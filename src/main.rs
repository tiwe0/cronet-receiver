pub mod config;
pub mod protocol;
pub mod redis;

use crate::config::AppConfig;
use crate::protocol::{ProtocolCodec};
use crate::redis::{SaveMessage, dispatch_to_redis, setup_redis};

use serde_json;

use std::{sync::Arc};

use dashmap::DashMap;
use nanoid::nanoid;

use tokio::io::{self};
use tokio::net::{TcpListener};
use tokio::sync::mpsc;
use tokio_util::codec::{Framed};
use futures::StreamExt;

struct AppState {
    tag_to_id: DashMap<String, String>,
    packages: DashMap<String, Vec<u8>>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let app_config: AppConfig = serde_json::from_str(std::fs::read_to_string("config.json").expect("读取配置文件失败").as_str()).expect("解析配置文件失败");
    let redis_client = setup_redis(&app_config).await.expect("无法连接到 Redis");
    println!("[*] 成功连接到 Redis 服务器");
    println!("[*] Redis 地址: {}:{}", app_config.redis.as_ref().unwrap().host, app_config.redis.as_ref().unwrap().port);

    let (tx, mut rx) = mpsc::channel::<SaveMessage>(1024);
    
    // 这个协程用于从队列中读取信息并分发
    let redis_client_clone = redis_client.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match dispatch_to_redis(&redis_client_clone, &msg).await {
                Ok(_) => {
                    println!("[{}] 分发到 Redis 成功，ID: {}", msg.tag, msg.id);
                }
                Err(e) => {
                    eprintln!("[{}] 分发到 Redis 失败，ID: {}，错误: {}", msg.tag, msg.id, e);
                }
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
                        println!("[{}] 数据块接收完成，总大小: {} bytes，ID: {}", package.tag, data.len(), id);
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
                                    println!("[{}] 已发送到分发队列，ID: {}", package.tag, id);
                                }
                            }
                            Err(e) => {
                                eprintln!("[!] 解析 JSON 失败: {}", e);
                            }
                        }
                    }
                } else {
                    // 不是 EOF，继续累积数据
                    state.packages.entry(id).or_default().extend_from_slice(&package.payload);
                    println!("[{}] 接受数据块中: {} bytes", package.tag, package.payload.len());
                }
            }
        });
    }
    Ok(())
}
