mod gpu;
mod job;
mod stratum;

use std::io;
use stratum::StratumClient;

fn main() {
    println!("BTCC Rust Stratum Miner v0.2.0 (GPU)");
    println!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    // Try to initialize GPU miner
    let gpu_miner = gpu::GpuMiner::new();
    let use_gpu = gpu_miner.is_some();

    let server = "pool.btc-classic.org:63101";
    let username = "cc1qqp3808t2ejeew386drfdj2amyys7ntfexfysqq.worker1";
    let password = "x";

    println!("Server: stratum+tcp://{}", server);
    println!("Username: {}", username);
    println!(
        "Mining mode: {}",
        if use_gpu { "GPU (Metal)" } else { "CPU" }
    );

    let client = StratumClient::new(server, username, password, gpu_miner);
    client.run();

    println!("Miner started. Press Enter to stop...");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    println!("Shutting down...");
    client.stop();
    println!("Miner stopped.");
}
