# CLAUDE.md

本文件为在此代码库中使用 Claude Code (claude.ai/code) 提供指导。

## 关于 rust-eco

rust-eco 是一个受 [lua-eco](https://github.com/zhaojh329/lua-eco) 启发的 Rust 异步运行时，为高性能应用程序提供轻量级协程和内置模块。

### 特性

- **异步运行时**: 基于 Tokio 构建，为常见任务提供简化的 API
- **并发任务**: 轻松生成和管理轻量级异步任务
- **文件 I/O**: 提供全面工具的异步文件操作
- **网络**: TCP/UDP 套接字、HTTP 客户端/服务器、WebSocket 支持
- **协议**: MQTT 客户端、WebSocket 客户端/服务器
- **系统工具**: 进程管理、系统信息、环境变量
- **同步**: 异步友好的互斥锁、通道、等待组、条件变量
- **DNS 解析**: 支持缓存的异步 DNS 查找
- **日志**: 多种输出格式的结构化日志

## 开发命令

### 构建和运行

```bash
# 构建库
cargo build

# 优化构建
cargo build --release

# 运行 CLI 演示
cargo run

# 启用详细日志运行 CLI
cargo run -- -v

# 静默模式运行 CLI
cargo run -- -q

# 构建 eco CLI 工具
cargo build --release --bin eco
./target/release/eco
```

### 示例

```bash
# 运行单个示例
cargo run --example basic_concurrency
cargo run --example file_operations
cargo run --example http_client
cargo run --example tcp_server
cargo run --example websocket_echo
cargo run --example channels_sync
```

### 测试

```bash
# 运行所有测试
cargo test

# 显示输出运行测试
cargo test -- --nocapture

# 运行特定测试
cargo test test_name
```

## 架构概述

rust-eco 是一个受 lua-eco 启发的异步运行时库，基于 Tokio 构建。架构采用模块化设计，每个模块为常见任务提供异步友好的 API。

### 核心组件

#### 运行时模块 (`src/runtime.rs`)

- `Eco` 结构体：包装 Tokio 并提供简化 API 的主运行时
- 任务生成和管理，内置关闭处理
- 使用 `tokio::select!` 进行优雅关闭协调

#### 模块结构

- 每个模块（file、socket、http 等）提供高级异步 API
- 所有模块使用 `crate::Result<T>` 类型别名进行一致的错误处理
- 基于 Tokio 原语但具有简化接口

### 关键设计模式

#### 错误处理

- 使用 `thiserror` 的自定义 `EcoError` 枚举进行结构化错误
- 所有模块间一致的 `Result<T>` 类型
- 通过 `#[from]` 属性自动转换 IO 错误

#### 异步任务管理

- `spawn()` 函数提供轻量级任务创建
- 运行时处理任务生命周期和清理
- 通过 mpsc 通道进行关闭协调

#### 模块组织

- `lib.rs` 重新导出核心功能（`Eco`、`spawn`、`sleep` 等）
- 每个模块是自包含的，具有清晰的 API 边界
- 协议模块包含 MQTT、WebSocket 等子模块

## 模块参考

### 运行时 (`rust_eco::runtime`)

- `Eco` - 管理异步任务的主运行时
- `spawn()` - 生成轻量级异步任务
- `sleep()` - 异步睡眠函数
- `Watcher` - 事件监听工具

### 时间 (`rust_eco::time`)

- `sleep_secs()`、`sleep_millis()` - 睡眠函数
- `now()`、`monotonic()` - 时间工具
- `Timer` - 周期性定时器
- `with_timeout()` - 超时包装器

### 文件 I/O (`rust_eco::file`)

- `read()`、`write()`、`append()` - 基本文件操作
- `mkdir()`、`remove_dir()` - 目录操作
- `copy()`、`rename()` - 文件操作
- `list_dir()`、`walk_dir()` - 目录遍历
- `stat()` - 文件元数据

### 网络 (`rust_eco::socket`)

- `TcpServer`、`TcpClient` - TCP 网络
- `UdpClient` - UDP 网络
- `UnixServer`、`UnixClient` - Unix 域套接字（仅 Unix）

### HTTP (`rust_eco::http`)

- `HttpClient` - 支持 GET、POST、PUT、DELETE 的 HTTP 客户端
- `HttpServer` - 简单 HTTP 服务器
- JSON 和表单数据支持
- 自定义头部和请求构建

### 协议 (`rust_eco::protocols`)

- `MqttClient` - MQTT 3.1.1 客户端
- `WebSocketClient`、`WebSocketServer` - WebSocket 支持
- 消息路由和连接管理

### 系统 (`rust_eco::sys`)

- `pid()`、`hostname()` - 系统信息
- `getenv()`、`setenv()` - 环境变量
- `Process` - 进程生成和管理
- `exec()`、`exec_shell()` - 命令执行
- 信号处理（仅 Unix）

### 同步 (`rust_eco::sync`)

- `Mutex` - 异步互斥锁
- `ReadWriteLock` - 异步读写锁
- `WaitGroup` - 等待多个任务
- `Condition` - 条件变量
- `Once` - 一次性初始化

### 通道 (`rust_eco::channel`)

- `Channel` - 无界 MPSC 通道
- `BoundedChannel` - 有界 MPSC 通道
- `Broadcast` - 广播通道
- `OneShot` - 一次性通道

### DNS (`rust_eco::dns`)

- `resolve()` - DNS 解析
- `resolve_ipv4()`、`resolve_ipv6()` - IP 版本特定
- `reverse_lookup()` - 反向 DNS
- `CachedDnsResolver` - 带缓存的 DNS

### 日志 (`rust_eco::log`)

- `Logger` - 基础日志器
- `FileLogger` - 基于文件的日志
- `StructuredLogger` - 结构化日志
- 多个日志级别和输出

## CLI 工具

CLI 工具 (`src/bin/eco.rs`) 演示了运行时功能，既可作为演示也可作为潜在的脚本执行器（脚本执行计划中但尚未实现）。

```bash
# 构建 CLI
cargo build --release

# 运行演示
./target/release/eco

# 启用详细日志运行
./target/release/eco -v

# 直接执行代码（TODO）
./target/release/eco -e "println!('Hello, rust-eco!');"

# 运行脚本文件（TODO）
./target/release/eco script.rs
```