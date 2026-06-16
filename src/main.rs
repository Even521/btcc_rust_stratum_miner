mod gpu;
mod job;
mod stratum;

use std::io;
use stratum::StratumClient;

#[derive(Debug, Clone, Copy, PartialEq)]
enum MiningMode {
    Auto,
    Gpu,
    Cpu,
}

fn print_usage() {
    eprintln!("Usage: btcc_rust_stratum_miner [OPTIONS]");
    eprintln!("Options:");
    eprintln!("  --gpu        Force GPU mining (fail if no GPU available)");
    eprintln!("  --cpu        Force CPU mining");
    eprintln!("  --help       Show this help message");
    eprintln!("  (no args)    Auto-detect: try GPU first, fall back to CPU");
    eprintln!();
    eprintln!("Note: GPU support requires the appropriate feature flag at build time:");
    eprintln!("  Windows/Linux: cargo run --release --features opencl-gpu");
    eprintln!("  macOS:          cargo run --release --features metal-gpu");
    eprintln!("  CPU-only:       cargo run --release (default, no GPU features)");
}

fn parse_args() -> MiningMode {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        match arg.as_str() {
            "--gpu" => return MiningMode::Gpu,
            "--cpu" => return MiningMode::Cpu,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown option: {}", other);
                print_usage();
                std::process::exit(1);
            }
        }
    }
    MiningMode::Auto
}

fn main() {
    let mode = parse_args();

    println!("BTCC Rust Stratum Miner v0.3.0");
    println!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    // Initialize GPU miner based on mode
    let gpu_miner = match mode {
        MiningMode::Cpu => {
            eprintln!("CPU mode forced via --cpu flag");
            None
        }
        MiningMode::Gpu => {
            eprintln!("GPU mode forced via --gpu flag");
            let miner = gpu::GpuMiner::new();
            if miner.is_none() {
                eprintln!("FATAL: --gpu specified but no GPU device found!");
                eprintln!("  - Make sure your GPU drivers are installed");
                eprintln!("  - For NVIDIA: install CUDA or OpenCL runtime");
                eprintln!("  - For AMD: install ROCm or AMD APP SDK");
                eprintln!("  - For Intel: install Intel OpenCL Runtime");
                eprintln!("  - Try --cpu to fall back to CPU mining");
                std::process::exit(1);
            }
            miner
        }
        MiningMode::Auto => {
            let miner = gpu::GpuMiner::new();
            if miner.is_none() {
                eprintln!("No GPU support compiled in or no GPU device found.");
                eprintln!("  To enable GPU: rebuild with --features opencl-gpu (Windows/Linux)");
                eprintln!("  or --features metal-gpu (macOS)");
                eprintln!("  To force CPU mode: use --cpu");
            }
            miner
        }
    };

    let use_gpu = gpu_miner.is_some();

    let server = "pool.btc-classic.org:63101";
    let username = "cc1qqp3808t2ejeew386drfdj2amyys7ntfexfysqq.worker1";
    let password = "x";

    println!("Server: stratum+tcp://{}", server);
    println!("Username: {}", username);

    let mode_str = if use_gpu {
        #[cfg(all(target_os = "macos", feature = "metal-gpu"))]
        {
            "GPU (Metal)"
        }
        #[cfg(not(all(target_os = "macos", feature = "metal-gpu")))]
        {
            "GPU (OpenCL)"
        }
    } else {
        "CPU"
    };
    println!("Mining mode: {}", mode_str);

    let client = StratumClient::new(server, username, password, gpu_miner);
    client.run();

    println!("Miner started. Press Enter to stop...");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    println!("Shutting down...");
    client.stop();
    println!("Miner stopped.");
}
