pub mod config;
pub mod protocol;
pub mod redis;

use crate::config::AppConfig;
use crate::protocol::{ProtocolCodec};
use crate::redis::{SaveMessage, dispatch_to_redis, setup_redis};

use serde_json;

use std::time::Duration;
use std::{sync::Arc};

use dashmap::DashMap;
use nanoid::nanoid;

use tokio::io::{self};
use tokio::net::{TcpListener};
use tokio::sync::mpsc;
use tokio_util::codec::{Framed};
use futures::StreamExt;

use arc_swap::ArcSwap;
use notify_debouncer_mini::{DebounceEventResult, new_debouncer, notify::*};
use once_cell::sync::Lazy as LazyLock;


struct AppState {
    tag_to_id: DashMap<String, String>,
    packages: DashMap<String, Vec<u8>>,
}

static TRUNCATE_LEN: usize = 50;

static GLOBAL_APP_CONFIG: LazyLock<ArcSwap<AppConfig>> = LazyLock::new(|| {
    ArcSwap::from_pointee(AppConfig::load_from_file("config.json").expect("读取配置文件失败"))
});

#[tokio::main]
async fn main() -> io::Result<()> {
    let app_config = GLOBAL_APP_CONFIG.load().as_ref().clone();

    let redis_client = setup_redis(&app_config).await.expect("无法连接到 Redis");
    println!("[*] 成功连接到 Redis 服务器");
    println!("[*] Redis 地址: {}:{}", app_config.redis.as_ref().unwrap().host, app_config.redis.as_ref().unwrap().port);

    let (tx, mut rx) = mpsc::channel::<SaveMessage>(1024);

    // 热重载部分
    let mut debouncer = new_debouncer(
        Duration::from_millis(500), 
        |res: DebounceEventResult| {
            match res {
                Ok(events) => {
                    let target_file_str = "config.json";
                    let has_change = events.iter().any(|e|{
                        e.path.to_string_lossy().ends_with(target_file_str)
                    });
                    if has_change {
                        match AppConfig::load_from_file(target_file_str) {
                            Ok(new_config) => {
                                GLOBAL_APP_CONFIG.store(Arc::new(new_config));
                                println!("[*] 配置文件已重新加载");
                            }
                            Err(e) => {
                                eprintln!("[!] 重新加载配置文件失败: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => println!("[!] 监控配置文件变更时出错: {:?}", e),
            }
        }
    ).expect("无法初始化文件变更监控器");

    let _ = debouncer.watcher().watch(std::path::Path::new("config.json"), RecursiveMode::NonRecursive);
    println!("[*] 配置变更监控已启动");
    
    // 这个协程用于从队列中读取信息并分发
    let redis_client_clone = redis_client.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let app_config = GLOBAL_APP_CONFIG.load();
            if let Some(route_table) = &app_config.route_table {
                if let Some(target) = route_table.find_target(&msg.tag) {
                    println!("[{}] 路由到目标: {}", truncate_tag(&msg.tag, TRUNCATE_LEN), target);
                    match dispatch_to_redis(&redis_client_clone, target, &msg).await {
                        Ok(_) => {
                            println!("[{}] 分发到 Redis 成功，ID: {}", truncate_tag(&msg.tag, TRUNCATE_LEN), msg.id);
                        }
                        Err(e) => {
                            eprintln!("[{}] 分发到 Redis 失败，ID: {}，错误: {}", truncate_tag(&msg.tag, TRUNCATE_LEN), msg.id, e);
                        }
                    }
                } else {
                    println!("[{}] 启用路由表，丢弃信息", truncate_tag(&msg.tag, TRUNCATE_LEN));
                }
            } else {
                println!("[{}] 未配置路由表，丢弃信息", truncate_tag(&msg.tag, TRUNCATE_LEN));
            }
        }
    });

    // 用于将包封装为完整响应体
    let state = Arc::new(AppState {
        tag_to_id: DashMap::new(),
        packages: DashMap::new(),
    });

    let auth_key = app_config.auth_key.clone();

    // 绑定监听tcp
    let host = app_config.listener.host.clone();
    let port = app_config.listener.port;
    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    println!("[*] 使用认证密钥: {}", auth_key);
    println!("[*] Rust Codec Server 运行中...");
    println!("[*] 监听地址: {}:{}", host, port);

    // 开始监听tcp连接
    while let Ok((stream, _)) = listener.accept().await {

        // 监听到一个新的连接，开始处理
        let state = Arc::clone(&state);
        // 获取一个发送者的克隆
        let tx = tx.clone();
        // 获取魔术数字
        let auth_key = app_config.auth_key.clone();

        // 这里的异步协程用于处理每个连接，将接收到内容组包、校验、发送到缓存队列
        tokio::spawn(async move {

            // 定义一个基于codec的framed流
            let mut framed = Framed::new(stream, ProtocolCodec::new(&auth_key));

            // 一个粘包完成
            while let Some(Ok(package)) = framed.next().await {

                // 解析id
                let id = state.tag_to_id.entry(package.tag.clone())
                    .or_insert_with(|| nanoid!(8)).clone();
                
                // 处理 payload
                // 判断是否为EOF
                if package.end_flag {
                    // EOF 标志，表示数据接收完成，开始处理整个包内容
                    // 先累积当前包的数据
                    state.packages.entry(id.clone()).or_default().extend_from_slice(&package.payload);
                    
                    // 然后取出完整数据
                    if let Some((_, data)) = state.packages.remove(&id){
                        // 跳过空数据
                        if data.len() == 0 {
                            continue;
                        }
                        
                        let display_tag = truncate_tag(&package.tag, TRUNCATE_LEN);
                        println!("[{}] 数据接收完成: {} bytes，ID: {}", display_tag, data.len(), id);
                        match serde_json::from_slice::<serde_json::Value>(&data) {
                            Ok(json_content) => {
                                if let Err(e) = tx.send(SaveMessage {
                                    id: id.clone(),
                                    tag: package.tag.clone(),
                                    tag_app: package.tag_app.clone(),
                                    tag_url: package.tag_url.clone(),
                                    data: json_content,
                                }).await {
                                    eprintln!("[!] 发送到队列失败: {}", e);
                                } else {
                                    println!("[{}] 已发送到分发队列，ID: {}", display_tag, id);
                                }
                            }
                            Err(_e) => {
                                eprintln!("[{}] 数据解析为 JSON 失败，ID: {}", display_tag, id);
                                if let Err(e) = tx.send(SaveMessage {
                                    id: id.clone(),
                                    tag: "error|".to_string() + &package.tag,
                                    tag_app: package.tag_app.clone(),
                                    tag_url: package.tag_url.clone(),
                                    data: serde_json::json!({
                                        "error": "invalid_json",
                                        "app": package.tag_app,
                                        "url": package.tag_url,
                                        "message": data.iter().map(|&c| c as char).collect::<String>(),
                                    }),
                                }).await {
                                    eprintln!("[!] 发送错误信息到队列失败: {}", e);
                                } else {
                                    println!("[{}] 错误信息已发送到分发队列，ID: {}", display_tag, id);
                                }
                            }
                        }
                    }
                } else {
                    // 不是 EOF，继续累积数据
                    state.packages.entry(id).or_default().extend_from_slice(&package.payload);
                    println!("[{}] 接收数据块中: {} bytes", truncate_tag(&package.tag, TRUNCATE_LEN), package.payload.len());
                }
            }
        });
    }
    Ok(())
}

// 辅助函数：截断tag用于日志显示
fn truncate_tag(tag: &str, max_len: usize) -> String {
    if tag.len() <= max_len {
        tag.to_string()
    } else {
        format!("{}...", &tag[..max_len])
    }
}
