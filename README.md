# Cronet Receiver

> [!CAUTION]
> ## ⚠️ 免责声明 / DISCLAIMER
> 
> **本项目仅供学习、研究和技术交流使用，严禁用于任何非法用途。**
> 
> - **研究性质**：本项目是逆向工程研究的**额外成果**，用于学习网络协议设计和实现
> - **教育目的**：代码仅用于教学和技术研讨，帮助理解自定义协议的工作原理
> - **禁止滥用**：严禁将本项目用于未经授权的数据拦截、窃取或其他违法行为
> - **法律责任**：使用者必须遵守所在地区的法律法规，因滥用本项目导致的任何法律后果由使用者自行承担
> - **安全风险**：本项目未实现完整的安全机制（无加密、无认证），**不建议在生产环境或公网使用**
> - **隐私保护**：请勿使用本项目处理包含敏感信息或个人隐私的数据
> - **无担保声明**：本软件按"原样"提供，不提供任何明示或暗示的保证，作者不对使用本软件造成的任何损害负责
> 
> **使用本项目即表示您已阅读、理解并同意上述条款。若不同意，请立即停止使用。**
> 
> ---

一个基于 Rust 的高性能 TCP 协议服务器，用于接收、解析和存储自定义协议数据到 Redis。

## 📋 功能特性

- ✅ **自定义二进制协议**：支持基于 Magic Number 的协议校验
- ✅ **流式数据处理**：支持分块接收大数据，自动组装
- ✅ **异步高性能**：基于 Tokio 的异步运行时
- ✅ **零拷贝解析**：使用 BytesMut 优化内存使用
- ✅ **Redis Stream 存储**：数据持久化到 Redis Stream
- ✅ **防扫描机制**：Magic Number 校验防止随机扫描
- ✅ **粘包处理**：内置 Codec 自动处理 TCP 粘包问题

## 🏗️ 架构设计

```
┌─────────────┐         ┌──────────────────┐         ┌─────────────┐
│   客户端     │ ──TCP──→│  Cronet Receiver │ ─────→ │    Redis    │
│  (Python)   │         │   (Rust Server)  │  Stream │   Storage   │
└─────────────┘         └──────────────────┘         └─────────────┘
     ↓                           ↓
  发送协议包                  解析 + 组装
  - Magic验证                 - 校验Magic
  - 分块/单包                 - 累积数据块
  - JSON payload             - JSON解析
                              - 异步分发
```

## 📦 协议格式

```
┌────────────┬──────────────┬─────────────┬──────────────┬──────────┬────────────┬──────────┬──────────┐
│ MagicNumber│ TagPackageLen│  TagUrlLen  │ PayloadLen   │ EndFlag  │ TagPackage │  TagUrl  │ Payload  │
│    4 B     │     2 B      │    2 B      │     4 B      │   4 B    │   M bytes  │ N bytes  │ K bytes  │
└────────────┴──────────────┴─────────────┴──────────────┴──────────┴────────────┴──────────┴──────────┘
```

### 字段说明

| 字段 | 长度 | 类型 | 说明 |
|-----|------|------|------|
| MagicNumber | 4B | 固定 | 魔数，默认 "MAGI" (截取前4字节) |
| TagPackageLen | 2B | u16 | TagPackage 字段长度 (大端序) |
| TagUrlLen | 2B | u16 | TagUrl 字段长度 (大端序) |
| PayloadLen | 4B | u32 | Payload 字段长度 (大端序) |
| EndFlag | 4B | u32 | 结束标志：0=继续，非0=结束 |
| TagPackage | 可变 | UTF-8 | 应用标签 (如 "app1") |
| TagUrl | 可变 | UTF-8 | URL标签 (如 "/api/test") |
| Payload | 可变 | bytes | 有效载荷数据 (通常为JSON) |

## 🚀 快速开始

### 环境要求

- **Rust**: 1.70+ (推荐 1.80+)
- **Redis**: 5.0+ (支持 Stream)
- **Python**: 3.8+ (仅用于测试客户端)

### 安装步骤

1. **克隆项目**
   ```bash
   git clone <repository-url>
   cd cronet-receiver
   ```

2. **配置文件**
   
   编辑 `config.json`:
   ```json
   {
       "auth_key": "my_secret_key",
       "listener": {
           "host": "0.0.0.0",
           "port": 9000
       },
       "redis": {
           "host": "localhost",
           "port": 6379
       },
       "route_table": { // 尚未实现
           "app1": "handler1",
           "app2": "handler2"
       }
   }
   ```

3. **启动 Redis**
   ```bash
   redis-server
   ```

4. **编译运行**
   ```bash
   # 开发模式
   cargo run
   
   # 生产模式（优化编译）
   cargo build --release
   ./target/release/cronet-receiver
   ```

### 测试验证

```bash
# 安装 Python 依赖
pip install redis

# 运行测试客户端
python3 test_protocol.py

# 查询 Redis 数据
python3 query_redis.py list
```

## 📚 使用文档

### 服务器启动

```bash
cargo run
```

输出示例：
```
[*] 成功连接到 Redis 服务器
[*] Redis 地址: localhost:6379
[*] Rust Codec Server 运行中...
[*] 监听地址: 0.0.0.0:9000
```

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

### 开发流程

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/your-feature`
3. 提交更改：`git commit -am 'Add some feature'`
4. 推送分支：`git push origin feature/your-feature`
5. 提交 Pull Request

### 代码规范

- 遵循 Rust 标准格式：`cargo fmt`
- 通过 Clippy 检查：`cargo clippy`
- 添加必要的测试

## 📄 许可证

MIT License - 详见 LICENSE 文件

## 🙏 致谢

- [Tokio](https://tokio.rs/) - 异步运行时
- [Redis](https://redis.io/) - 数据存储
- [Bytes](https://github.com/tokio-rs/bytes) - 字节操作库

---

**Made with ❤️ using Rust**
