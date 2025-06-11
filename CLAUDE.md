# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About rust-eco

A Rust async runtime inspired by [lua-eco](https://github.com/zhaojh329/lua-eco), providing lightweight coroutines and built-in modules for high-performance applications.

### Features

- **Async Runtime**: Built on top of Tokio with simplified APIs for common tasks
- **Concurrent Tasks**: Easy spawning and management of lightweight async tasks
- **File I/O**: Async file operations with comprehensive utilities
- **Networking**: TCP/UDP sockets, HTTP client/server, WebSocket support
- **Protocols**: MQTT client, WebSocket client/server
- **System Utilities**: Process management, system information, environment variables
- **Synchronization**: Async-friendly mutexes, channels, wait groups, condition variables
- **DNS Resolution**: Async DNS lookup with caching support
- **Logging**: Structured logging with multiple output formats

## Development Commands

### Building and Running

```bash
# Build the library
cargo build

# Build with optimizations
cargo build --release

# Run the CLI demo
cargo run

# Run CLI with verbose logging
cargo run -- -v

# Run CLI in quiet mode
cargo run -- -q

# Build the eco CLI tool
cargo build --release --bin eco
./target/release/eco
```

### Examples

```bash
# Run individual examples
cargo run --example basic_concurrency
cargo run --example file_operations
cargo run --example http_client
cargo run --example tcp_server
cargo run --example websocket_echo
cargo run --example channels_sync
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Architecture Overview

rust-eco is an async runtime library inspired by lua-eco, built on top of Tokio. The architecture follows a modular design with each module providing async-friendly APIs for common tasks.

### Core Components

#### Runtime Module (`src/runtime.rs`)

- `Eco` struct: Main runtime that wraps Tokio with simplified APIs
- Task spawning and management with built-in shutdown handling
- Uses `tokio::select!` for graceful shutdown coordination

#### Module Structure

- Each module (file, socket, http, etc.) provides high-level async APIs
- All modules use `crate::Result<T>` type alias for consistent error handling
- Built on Tokio primitives but with simplified interfaces

### Key Design Patterns

#### Error Handling

- Custom `EcoError` enum with `thiserror` for structured errors
- Consistent `Result<T>` type across all modules
- IO errors automatically converted via `#[from]` attribute

#### Async Task Management

- `spawn()` function provides lightweight task creation
- Runtime handles task lifecycle and cleanup
- Shutdown coordination through mpsc channels

#### Module Organization

- `lib.rs` re-exports core functionality (`Eco`, `spawn`, `sleep`, etc.)
- Each module is self-contained with clear API boundaries
- Protocols module contains sub-modules for MQTT, WebSocket, etc.

## Module Reference

### Runtime (`rust_eco::runtime`)

- `Eco` - Main runtime for managing async tasks
- `spawn()` - Spawn lightweight async tasks
- `sleep()` - Async sleep functions
- `Watcher` - Event watching utilities

### Time (`rust_eco::time`)

- `sleep_secs()`, `sleep_millis()` - Sleep functions
- `now()`, `monotonic()` - Time utilities
- `Timer` - Periodic timer
- `with_timeout()` - Timeout wrapper

### File I/O (`rust_eco::file`)

- `read()`, `write()`, `append()` - Basic file operations
- `mkdir()`, `remove_dir()` - Directory operations
- `copy()`, `rename()` - File manipulation
- `list_dir()`, `walk_dir()` - Directory traversal
- `stat()` - File metadata

### Networking (`rust_eco::socket`)

- `TcpServer`, `TcpClient` - TCP networking
- `UdpClient` - UDP networking
- `UnixServer`, `UnixClient` - Unix domain sockets (Unix only)

### HTTP (`rust_eco::http`)

- `HttpClient` - HTTP client with GET, POST, PUT, DELETE
- `HttpServer` - Simple HTTP server
- JSON and form data support
- Custom headers and request building

### Protocols (`rust_eco::protocols`)

- `MqttClient` - MQTT 3.1.1 client
- `WebSocketClient`, `WebSocketServer` - WebSocket support
- Message routing and connection management

### System (`rust_eco::sys`)

- `pid()`, `hostname()` - System information
- `getenv()`, `setenv()` - Environment variables
- `Process` - Process spawning and management
- `exec()`, `exec_shell()` - Command execution
- Signal handling (Unix only)

### Synchronization (`rust_eco::sync`)

- `Mutex` - Async mutex
- `ReadWriteLock` - Async RW lock
- `WaitGroup` - Wait for multiple tasks
- `Condition` - Condition variables
- `Once` - One-time initialization

### Channels (`rust_eco::channel`)

- `Channel` - Unbounded MPSC channels
- `BoundedChannel` - Bounded MPSC channels
- `Broadcast` - Broadcast channels
- `OneShot` - One-shot channels

### DNS (`rust_eco::dns`)

- `resolve()` - DNS resolution
- `resolve_ipv4()`, `resolve_ipv6()` - IP version specific
- `reverse_lookup()` - Reverse DNS
- `CachedDnsResolver` - DNS with caching

### Logging (`rust_eco::log`)

- `Logger` - Basic logger
- `FileLogger` - File-based logging
- `StructuredLogger` - Structured logging
- Multiple log levels and outputs

## CLI Tool

The CLI tool (`src/bin/eco.rs`) demonstrates the runtime capabilities and serves as both a demo and a potential script executor (script execution is planned but not yet implemented).

```bash
# Build the CLI
cargo build --release

# Run the demo
./target/release/eco

# Run with verbose logging
./target/release/eco -v

# Execute code directly (TODO)
./target/release/eco -e "println!('Hello, rust-eco!');"

# Run a script file (TODO)
./target/release/eco script.rs
```