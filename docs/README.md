# BTCC Rust Stratum Miner

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

基于 Rust 实现的 BTCC (Bitcoin-Classic) Stratum 矿池挖矿客户端，支持 **Metal GPU (macOS)**、**OpenCL GPU (Windows/Linux)** 加速和 **多线程 CPU** 回退。

## 特性

- **Metal GPU 挖矿** — macOS 上使用 Apple Silicon GPU 进行 SHA-256d 哈希计算
- **OpenCL GPU 挖矿** — Windows/Linux 上使用 NVIDIA/AMD/Intel GPU 加速
- **CPU 多线程回退** — 无 GPU 或未启用 GPU feature 时自动使用 CPU 多线程挖矿
- **CLI 参数控制** — `--gpu` / `--cpu` 显式选择挖矿模式，默认自动检测
- **Stratum v1 协议** — 完整的 `mining.subscribe` / `mining.authorize` / `mining.notify` / `mining.submit` 实现
- **自动重连** — 连接断开后自动重连，GPU 线程复用不重复创建
- **实时算力统计** — 每 10 秒输出带时间戳的当前算力
- **GPU 性能优化** — Midstate 预计算 + 双缓冲命令流水线 + 自动调参
- **单一二进制** — 纯 Rust 实现，无外部运行时依赖

## 系统要求

| 平台 | GPU 支持 | CPU 支持 | Feature Flag |
|------|---------|---------|-------------|
| macOS (Apple Silicon) | ✅ Metal GPU | ✅ | `--features metal-gpu` |
| macOS (Intel) | ❌ | ✅ | (default) |
| Windows (NVIDIA/AMD/Intel) | ✅ OpenCL GPU | ✅ | `--features opencl-gpu` |
| Linux (NVIDIA/AMD/Intel) | ✅ OpenCL GPU | ✅ | `--features opencl-gpu` |

- Rust 1.70+
- macOS 12+（Metal GPU 挖矿需要）
- Xcode Command Line Tools（macOS GPU 挖矿需要，`xcode-select --install`）
- OpenCL 运行时（Windows/Linux GPU 挖矿需要）：
  - NVIDIA：驱动自带 OpenCL.dll / libOpenCL.so
  - AMD：需安装 ROCm 或 AMD APP SDK
  - Intel：需安装 Intel OpenCL Runtime
- **Windows 编译 GPU 版本还需 OpenCL SDK**（提供 `OpenCL.lib` 链接文件）：
  - [AMD APP SDK](https://www.amd.com/en/developer/rocm.html) 或
  - [Intel OpenCL SDK](https://www.intel.com/content/www/us/en/developer/tools/opencl-sdk/overview.html)

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

| 平台 | 命令 | 挖矿模式 |
|------|------|----------|
| macOS (Apple Silicon) | `cargo run --release --features metal-gpu` | GPU Metal |
| macOS (Intel) | `cargo run --release` | CPU 多线程 |
| Windows (GPU) | `cargo run --release --features opencl-gpu` | GPU OpenCL |
| Windows (CPU) | `cargo run --release` | CPU 多线程 |
| Linux (GPU) | `cargo run --release --features opencl-gpu` | GPU OpenCL |
| Linux (CPU) | `cargo run --release` | CPU 多线程 |

> **注意**：默认编译（不加 feature flag）为 CPU-only 模式。GPU 加速需要显式启用对应的 feature。
>
> 也可以先编译再运行：`cargo build --release --features opencl-gpu && ./target/release/btcc_rust_stratum_miner`

### 4. 运行时参数

```bash
# 自动检测（默认）：优先 GPU，无 GPU 则回退 CPU
./btcc_rust_stratum_miner

# 强制 GPU 模式（无 GPU 则报错退出）
./btcc_rust_stratum_miner --gpu

# 强制 CPU 模式
./btcc_rust_stratum_miner --cpu

# 查看帮助
./btcc_rust_stratum_miner --help
```

### 5. 停止挖矿

按 `Enter` 键优雅退出。

## 命令行参数

| 参数 | 说明 |
|------|------|
| `--gpu` | 强制 GPU 挖矿（无可用 GPU 时退出并提示安装驱动） |
| `--cpu` | 强制 CPU 挖矿 |
| `--help` / `-h` | 显示帮助信息 |
| (无参数) | 自动检测：优先 GPU，不可用时回退 CPU |

当前版本通过修改 `src/main.rs` 中的常量来配置矿池连接：

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `server` | `pool.btc-classic.org:63101` | 矿池地址 |
| `username` | `your_btcc_address.worker1` | BTCC 钱包地址.矿工名 |
| `password` | `x` | 矿池密码（通常填 `x`） |

## 编译选项

### Feature Flags

| Feature | 说明 | 适用平台 |
|---------|------|----------|
| `metal-gpu` | 启用 Metal GPU 挖矿 | 仅 macOS |
| `opencl-gpu` | 启用 OpenCL GPU 挖矿 | Windows / Linux |
| (default) | CPU-only 模式 | 所有平台 |

### Release 优化

`Cargo.toml` 中预配置了编译优化：

```toml
[profile.release]
opt-level = 3      # 最高优化级别
lto = true         # 链接时优化
codegen-units = 1  # 单代码生成单元（更好的内联）
```

`.cargo/config.toml` 中配置了平台特定优化：

```toml
# Apple Silicon (M1/M2/M3/M4) 特定优化
[target.aarch64-apple-darwin]
rustflags = ["-C", "target-cpu=apple-m2"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "target-cpu=native"]
```

## 项目结构

```
btcc_rust_stratum_miner/
├── Cargo.toml              # 项目配置与依赖
├── .cargo/
│   └── config.toml         # 平台特定编译优化
├── src/
│   ├── main.rs             # 入口：CLI 参数解析、初始化、连接矿池
│   ├── job.rs              # 挖矿作业：coinbase 构建、Merkle 根、区块头、SHA-256d
│   ├── stratum.rs          # Stratum 协议：TCP 连接、JSON-RPC、GPU/CPU 挖矿调度
│   └── gpu/
│       ├── mod.rs          # GPU 模块入口（条件编译）
│       ├── metal_impl.rs   # Metal GPU 实现（macOS / Apple Silicon）
│       ├── opencl_impl.rs  # OpenCL GPU 实现（Windows / Linux）
│       └── stub.rs         # 无 GPU 平台的桩实现
├── docs/
│   ├── README.md              # 项目说明（本文件）
│   ├── stratum_protocol.md    # Stratum 协议详解
│   └── mining_algorithm.md    # SHA-256d 挖矿核心算法
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
│                  GPU (Metal / OpenCL)                 │
│                                                     │
│  每个线程处理一个 nonce:                              │
│    SHA256_compress(chunk2, midstate) → hash1         │
│    SHA256_compress(hash1, 初始状态)  → 最终 hash      │
│    比较 hash ≤ target → 原子 CAS 写入结果             │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### 断连重连与 GPU 线程复用

每次 TCP 连接断开后，程序自动等待 5 秒重连。**GPU 挖矿线程只在首次连接时启动一次**，后续重连复用已有线程，避免多个 GPU 线程争抢资源导致算力下降。

## 性能

| 硬件 | 模式 | 算力 |
|------|------|------|
| Apple M2 (8 GPU 核) | GPU (Metal) | ~188 MH/s |
| Apple M2 Pro | GPU (Metal) | ~350-400 MH/s |
| Apple M2 Max | GPU (Metal) | ~650-700 MH/s |
| Apple M2 (8 CPU 核) | CPU | ~5-8 MH/s |
| NVIDIA GTX 1060 | GPU (OpenCL) | ~150-200 MH/s |
| NVIDIA RTX 3060 | GPU (OpenCL) | ~400-600 MH/s |

> 注：M2 基础款实测约 188 MH/s。算力不受矿池 vardiff 难度调整影响。Pro/Max 款需实现 `detect_gpu_cores()` 获取真实核心数才能达到标称算力。OpenCL 性能因驱动和硬件而异。

## 日志格式

所有日志带 `[HH:MM:SS]` 时间戳，输出到 stderr：

```
[14:32:05] Connecting to pool.btc-classic.org:63101...
[14:32:06] Connected to pool.btc-classic.org:63101
[14:32:06] GPU: NVIDIA GeForce RTX 3060 (OpenCL), vendor=NVIDIA Corporation
[14:32:06] OpenCL GPU miner: workgroup=256, per_dispatch=16M, compute_units=28
[14:32:06] Starting GPU miner...
[14:32:06] Subscribed: extranonce1=30005acb, extranonce2_size=8
[14:32:06] Authorized successfully
[14:32:07] New job: id=00000eda, prev_hash=7c7f8948, nbits=1902ee94
[14:32:17] GPU Hashrate: 187976622.83 H/s (187.98 MH/s)
[14:45:30] [WARN] Server closed connection
[14:45:35] Reconnecting in 5 seconds...
[14:45:40] Connected to pool.btc-classic.org:63101
[14:45:40] GPU miner already running, reusing existing thread
```

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

### caffeinate（macOS 防止休眠）

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

### 连接频繁断开

Stratum 矿池可能因空闲超时或负载均衡主动断开连接，属正常行为。程序会自动重连，GPU 线程复用不重复创建。观察日志中 `Server closed connection` 的时间间隔可判断是否有规律。

### 算力逐渐下降

如果每次重连后算力递减，说明 GPU 线程被重复创建。v0.2.1 已修复此问题，重连时输出 `GPU miner already running, reusing existing thread`。

### 连接失败

```bash
# 测试矿池连通性
nc -vz pool.btc-classic.org 63101
```

### GPU 不可用

- **macOS**：确保使用 `--features metal-gpu` 编译，且为 Apple Silicon 设备
- **Windows/Linux**：确保使用 `--features opencl-gpu` 编译，且已安装 GPU 驱动和 OpenCL 运行时
- 使用 `--cpu` 参数可强制回退到 CPU 挖矿
- 默认编译（无 feature flag）为 CPU-only 模式

### Windows OpenCL 编译错误 `LNK1181: 无法打开输入文件"OpenCL.lib"`

需要安装 OpenCL SDK 提供链接库文件：
- [AMD APP SDK](https://www.amd.com/en/developer/rocm.html)
- [Intel OpenCL SDK](https://www.intel.com/content/www/us/en/developer/tools/opencl-sdk/overview.html)

或使用 CPU-only 模式：`cargo run --release`（不加 `--features opencl-gpu`）

### 授权失败

确认 `username` 格式为 `钱包地址.矿工名`，BTCC 地址以 `cc1` 开头。

## 更新日志

### v0.3.0

- 新增 OpenCL GPU 支持（Windows / Linux，NVIDIA/AMD/Intel）
- 新增 CLI 参数 `--gpu` / `--cpu` / `--help` 控制挖矿模式
- 默认编译改为 CPU-only（`default = []`），GPU 需显式启用 feature
- 修复 prev_hash 字节序（每 4 字节 word 内部反转）
- 修复 merkle_root 字节序（SHA-256d 输出直接拷贝）
- 修复 hash_meets_target 使用 display hash 比较
- 新增单元测试覆盖关键字节序逻辑
- 完善文档：Stratum 协议详解 + SHA-256d 算法详解

### v0.2.1

- 所有日志添加 `[HH:MM:SS]` 时间戳
- 修复断连重连时 GPU 线程重复创建导致算力递减的问题
- 适配 `metal-rs` 0.29 API（`ComputePipelineDescriptor`、`CommandBuffer`）

### v0.2.0

- Metal GPU 挖矿支持（midstate 优化 + 双缓冲流水线）
- CPU 多线程回退
- Stratum v1 协议完整实现

## 免责声明

**本软件仅限学习用途，严禁用于任何商业或盈利目的。**

- **法律合规**：请务必遵守所在国家/地区的法律法规。部分国家/地区禁止或限制加密货币挖矿，任何违反法律法规的行为，后果由使用者自行承担。
- **电费成本**：GPU 挖矿功耗较高，请自行评估。
- **硬件损耗**：长时间满载运行可能加速硬件老化或导致过热降频。
- **收益风险**：挖矿收益受币价、难度、矿池政策等多因素影响，不保证盈利。

**使用本软件即表示您已了解并同意自行承担所有风险，作者不对任何直接或间接损失及法律后果负责。**

## 致谢

- Metal SHA-256d kernel 参考了 [BTCC_apple-gpu-miner](https://github.com/wendell1224/BTCC_apple-gpu-miner)
- Midstate 优化技术源自 cgminer/bfgminer 的成熟方案

## License

MIT