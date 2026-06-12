# BTCC Rust Stratum Miner

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

基于 Rust 实现的 BTCC (Bitcoin-Classic) Stratum 矿池挖矿客户端，支持 **Apple Silicon GPU (Metal)** 加速和 **多线程 CPU** 回退。

## 特性

- **Metal GPU 挖矿** — 在 Apple M1/M2/M3/M4 系列芯片上使用 GPU 进行 SHA-256d 哈希计算
- **CPU 多线程回退** — 非 macOS 平台或无 GPU 时自动使用 CPU 多线程挖矿
- **Stratum v1 协议** — 完整的 `mining.subscribe` / `mining.authorize` / `mining.notify` / `mining.submit` 实现
- **自动重连** — 连接断开后自动重连，无需人工干预
- **实时算力统计** — 每 10 秒输出当前算力
- **GPU 性能优化** — Midstate 预计算 + 双缓冲命令流水线 + 自动调参
- **单一二进制** — 纯 Rust 实现，无外部运行时依赖

## 系统要求

| 平台 | GPU 支持 | CPU 支持 |
|------|---------|---------|
| macOS (Apple Silicon) | ✅ Metal GPU | ✅ |
| macOS (Intel) | ❌ | ✅ |
| Linux | ❌ | ✅ |
| Windows | ❌ | ✅ |

- Rust 1.70+
- macOS 12+（GPU 挖矿需要）
- Xcode Command Line Tools（GPU 挖矿需要，`xcode-select --install`）

## 快速开始

### 1. 克隆项目

```bash
git clone <repo-url>
cd btcc_rust_stratum_miner
```

### 2. 修改钱包地址

编辑 `src/main.rs`，将 `username` 改为你的 BTCC 钱包地址：

```rust
let username = "你的BTCC地址.worker1";
```

### 3. 编译运行

**macOS（GPU 挖矿）：**

```bash
cargo build --release --features metal-gpu
./target/release/btcc_rust_stratum_miner
```

**其他平台（CPU 挖矿）：**

```bash
cargo build --release
./target/release/btcc_rust_stratum_miner
```

### 4. 停止挖矿

按 `Enter` 键优雅退出。

## 命令行参数

当前版本通过修改 `src/main.rs` 中的常量来配置：

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `server` | `pool.btc-classic.org:63101` | 矿池地址 |
| `username` | `your_btcc_address.worker1` | BTCC 钱包地址.矿工名 |
| `password` | `x` | 矿池密码（通常填 `x`） |

## 编译选项

### Feature Flags

| Feature | 说明 |
|---------|------|
| `metal-gpu` | 启用 Metal GPU 挖矿（仅 macOS） |

### Release 优化

`Cargo.toml` 中预配置了针对 Apple M2 的编译优化：

```toml
[profile.release]
opt-level = 3      # 最高优化级别
lto = true         # 链接时优化
codegen-units = 1  # 单代码生成单元（更好的内联）

[target.aarch64-apple-darwin]
rustflags = ["-C", "target-cpu=apple-m2"]  # M2 专用指令优化
```

## 项目结构

```
btcc_rust_stratum_miner/
├── Cargo.toml              # 项目配置与依赖
├── src/
│   ├── main.rs             # 入口：初始化日志、连接矿池、启动挖矿
│   ├── job.rs              # 挖矿作业：coinbase 构建、Merkle 根、区块头、SHA-256d
│   ├── stratum.rs          # Stratum 协议：TCP 连接、JSON-RPC、多线程/GPU 挖矿调度
│   └── gpu/
│       ├── mod.rs          # GPU 模块入口（条件编译）
│       ├── metal_impl.rs   # Metal GPU 实现（SHA-256d kernel + 双缓冲流水线）
│       └── stub.rs         # 非 macOS 平台的 GPU 桩实现
└── docs/
    ├── README.md           # 本文件
    ├── ARCHITECTURE.md     # 架构设计文档
    └── PERFORMANCE.md      # 性能分析与对比
```

## 工作原理

### Stratum 协议流程

```
┌──────────┐                    ┌──────────┐
│  Miner   │                    │   Pool   │
└────┬─────┘                    └────┬─────┘
     │                               │
     │  mining.subscribe ──────────► │
     │  mining.authorize ──────────► │
     │                               │
     │  ◄────────── mining.notify    │  (新作业)
     │                               │
     │  [GPU/CPU 搜索 nonce]         │
     │                               │
     │  mining.submit ─────────────► │  (提交 share)
     │                               │
     │  ◄────────── result (accept)  │
```

### GPU 挖矿流程

```
┌─────────────────────────────────────────────────────┐
│                    CPU (Rust)                        │
│                                                     │
│  1. 接收 mining.notify → 解析 job                   │
│  2. 构建 80 字节区块头                               │
│  3. 预计算 midstate (chunk1 的 SHA-256 中间状态)     │
│  4. 将 midstate + tail_words + target 写入 GPU buffer│
│  5. 提交 GPU compute dispatch                       │
│  6. 等待 GPU 完成 → 读取结果                         │
│  7. CPU 复核 hash → 提交 share                      │
│                                                     │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│                  GPU (Metal)                         │
│                                                     │
│  每个线程处理一个 nonce:                              │
│    SHA256_compress(chunk2, midstate) → hash1         │
│    SHA256_compress(hash1, 初始状态)  → 最终 hash      │
│    比较 hash ≤ target → 原子 CAS 写入结果             │
│                                                     │
└─────────────────────────────────────────────────────┘
```

## 性能

| 硬件 | 模式 | 算力 |
|------|------|------|
| Apple M2 (10 GPU 核) | GPU (Metal) | ~180 MH/s |
| Apple M2 Pro | GPU (Metal) | ~350-400 MH/s |
| Apple M2 Max | GPU (Metal) | ~650-700 MH/s |
| Apple M2 (8 CPU 核) | CPU | ~5-8 MH/s |

> 详细性能分析见 [PERFORMANCE.md](PERFORMANCE.md)

## 后台运行

### nohup（最简单）

```bash
nohup ./target/release/btcc_rust_stratum_miner > miner.log 2>&1 &
echo $! > miner.pid

# 查看日志
tail -f miner.log

# 停止
kill "$(cat miner.pid)"
```

### caffeinate（防止休眠）

```bash
nohup caffeinate -i ./target/release/btcc_rust_stratum_miner > miner.log 2>&1 &
```

### tmux（SSH 远程推荐）

```bash
tmux new -s miner
caffeinate -i ./target/release/btcc_rust_stratum_miner
# Ctrl+B, D 分离
# tmux attach -t miner  重新连接
```

## 常见问题

### 连接失败

```
# 测试矿池连通性
nc -vz pool.btc-classic.org 63101

# 查看详细日志
RUST_LOG=debug ./target/release/btcc_rust_stratum_miner
```

### GPU 不可用

非 macOS 平台或未启用 `--features metal-gpu` 时，程序会自动回退到 CPU 挖矿。

### 授权失败

确认 `username` 格式为 `钱包地址.矿工名`，BTCC 地址以 `cc1` 开头。

## 致谢

- Metal SHA-256d kernel 参考了 [BTCC_apple-gpu-miner](https://github.com/wendell1224/BTCC_apple-gpu-miner)
- Midstate 优化技术源自 cgminer/bfgminer 的成熟方案

## License

MIT