// OpenCL GPU miner implementation for Windows / Linux.
//
// Supports NVIDIA, AMD, and Intel GPUs via the OpenCL standard.
// Core SHA-256d logic is identical to the Metal implementation.

use ocl::{flags, Buffer, Context, Device, DeviceType, Kernel, Platform, Program, Queue};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::job::{diff_to_target, hash_meets_target, MiningJob};
use crate::stratum::FoundShare;

fn ts() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("[{:02}:{:02}:{:02}]", h, m, s)
}

const SHA256D_KERNEL: &str = r#"
__constant uint K[64] = {
    0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u,
    0x3956c25bu, 0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u,
    0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u,
    0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u,
    0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu,
    0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
    0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u,
    0xc6e00bf3u, 0xd5a79147u, 0x06ca6351u, 0x14292967u,
    0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u,
    0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
    0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u,
    0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
    0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u,
    0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu, 0x682e6ff3u,
    0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u,
    0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u,
};

__constant uint H0[8] = {
    0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
    0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u,
};

inline uint rotr32(uint x, uint n) { return (x >> n) | (x << (32 - n)); }
inline uint bswap32_u(uint x) {
    return ((x & 0x000000FFu) << 24) | ((x & 0x0000FF00u) << 8)
         | ((x & 0x00FF0000u) >> 8)  | ((x & 0xFF000000u) >> 24);
}

inline void sha256_compress(uint h[8], const uint w_in[16]) {
    uint w[64];
    for (uint i = 0; i < 16; i++) w[i] = w_in[i];
    for (uint i = 16; i < 64; i++) {
        uint s0 = rotr32(w[i-15], 7) ^ rotr32(w[i-15], 18) ^ (w[i-15] >> 3);
        uint s1 = rotr32(w[i-2], 17) ^ rotr32(w[i-2], 19) ^ (w[i-2] >> 10);
        w[i] = w[i-16] + s0 + w[i-7] + s1;
    }
    uint a=h[0], b=h[1], c=h[2], d=h[3], e=h[4], f=h[5], g=h[6], hh=h[7];
    for (uint i = 0; i < 64; i++) {
        uint S1 = rotr32(e,6) ^ rotr32(e,11) ^ rotr32(e,25);
        uint ch = (e & f) ^ ((~e) & g);
        uint t1 = hh + S1 + ch + K[i] + w[i];
        uint S0 = rotr32(a,2) ^ rotr32(a,13) ^ rotr32(a,22);
        uint mj = (a & b) ^ (a & c) ^ (b & c);
        uint t2 = S0 + mj;
        hh = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    h[0]+=a; h[1]+=b; h[2]+=c; h[3]+=d; h[4]+=e; h[5]+=f; h[6]+=g; h[7]+=hh;
}

__kernel void sha256d_search(
    __global const uint *midstate,
    __global const uint *tail_words,
    __global const uint *target_be,
    uint start_nonce,
    __global volatile uint *result_flag,
    __global uint *result_nonce,
    __global uint *result_hash
) {
    uint nonce_val = start_nonce + get_global_id(0);

    uint w[16];
    w[0] = tail_words[0];
    w[1] = tail_words[1];
    w[2] = tail_words[2];
    w[3] = bswap32_u(nonce_val);
    w[4] = 0x80000000u;
    w[5] = 0; w[6] = 0; w[7] = 0;
    w[8] = 0; w[9] = 0; w[10] = 0; w[11] = 0;
    w[12] = 0; w[13] = 0; w[14] = 0;
    w[15] = 640u;

    uint h[8];
    for (uint i = 0; i < 8; i++) h[i] = midstate[i];
    sha256_compress(h, w);

    uint w2[16];
    for (uint i = 0; i < 8; i++) w2[i] = h[i];
    w2[8]  = 0x80000000u;
    w2[9]  = 0; w2[10] = 0; w2[11] = 0;
    w2[12] = 0; w2[13] = 0; w2[14] = 0;
    w2[15] = 256u;

    uint h2[8];
    for (uint i = 0; i < 8; i++) h2[i] = H0[i];
    sha256_compress(h2, w2);

    bool below = false;
    bool decided = false;
    for (uint i = 0; i < 8; i++) {
        uint d = bswap32_u(h2[7 - i]);
        if (!decided) {
            if (d < target_be[i])      { below = true;  decided = true; }
            else if (d > target_be[i]) { below = false; decided = true; }
        }
    }
    if (!decided) below = true;

    if (below) {
        uint expected = 0u;
        if (atomic_cmpxchg(result_flag, expected, 1u) == expected) {
            *result_nonce = nonce_val;
            for (uint i = 0; i < 8; i++) {
                result_hash[i] = bswap32_u(h2[7 - i]);
            }
        }
    }
}
"#;

pub struct GpuMiner {
    context: Context,
    queue: Queue,
    program: Program,
    workgroup_size: usize,
    per_dispatch: u32,
}

impl Clone for GpuMiner {
    fn clone(&self) -> Self {
        GpuMiner {
            context: self.context.clone(),
            queue: self.queue.clone(),
            program: self.program.clone(),
            workgroup_size: self.workgroup_size,
            per_dispatch: self.per_dispatch,
        }
    }
}

impl GpuMiner {
    pub fn new() -> Option<Self> {
        // Find first GPU device across all platforms
        let mut device: Option<Device> = None;
        for platform in Platform::list() {
            if let Ok(devices) = Device::list(&platform, Some(DeviceType::GPU)) {
                if let Some(d) = devices.into_iter().next() {
                    device = Some(d);
                    break;
                }
            }
        }
        let device = device?;

        let name = device.name().unwrap_or_else(|_| "unknown".into());
        let vendor = device.vendor().unwrap_or_else(|_| "unknown".into());
        eprintln!("GPU: {} (OpenCL), vendor={}", name, vendor);

        let context = Context::builder().devices(device).build().ok()?;

        let queue = Queue::new(&context, device, None).ok()?;

        let program = Program::builder()
            .src(SHA256D_KERNEL)
            .devices(device)
            .build(&context)
            .map_err(|e| {
                eprintln!("[ERROR] Failed to build OpenCL program: {}", e);
            })
            .ok()?;

        let max_wg = device.max_wg_size().unwrap_or(256);
        let workgroup_size = if max_wg >= 256 { 256 } else { max_wg };

        // Auto-tune per-dispatch: ~2M nonces per compute unit, clamped to [4M, 64M]
        let cu = device
            .info(ocl::enums::DeviceInfo::MaxComputeUnits)
            .map(|r| match r {
                ocl::enums::DeviceInfoResult::MaxComputeUnits(n) => n as u64,
                _ => 8,
            })
            .unwrap_or(8);
        let per_dispatch = (cu * (1u64 << 21)).max(1u64 << 22).min(1u64 << 26);

        eprintln!(
            "OpenCL GPU miner: workgroup={}, per_dispatch={}M, compute_units={}",
            workgroup_size,
            per_dispatch / 1_048_576,
            cu
        );

        Some(GpuMiner {
            context,
            queue,
            program,
            workgroup_size,
            per_dispatch: per_dispatch as u32,
        })
    }

    pub fn run(
        &self,
        current_job: Arc<Mutex<Option<MiningJob>>>,
        running: Arc<AtomicBool>,
        hashrate: Arc<Mutex<f64>>,
        share_tx: mpsc::Sender<FoundShare>,
        subscription: Arc<Mutex<Option<crate::stratum::Subscription>>>,
        difficulty: Arc<Mutex<f64>>,
    ) {
        let context = self.context.clone();
        let queue = self.queue.clone();
        let program = self.program.clone();
        let workgroup_size = self.workgroup_size;
        let per_dispatch = self.per_dispatch;

        thread::spawn(move || {
            let mut hash_count: u64 = 0;
            let mut last_report = Instant::now();
            let mut nonce_base: u32 = 0;
            let mut extranonce2_counter: u64 = 0;

            // Pre-allocate buffers
            let mid_buf = Buffer::<u32>::builder()
                .queue(queue.clone())
                .flags(flags::MEM_READ_ONLY | flags::MEM_HOST_WRITE_ONLY)
                .len(8)
                .fill_val(0u32)
                .build()
                .unwrap();

            let tail_buf = Buffer::<u32>::builder()
                .queue(queue.clone())
                .flags(flags::MEM_READ_ONLY | flags::MEM_HOST_WRITE_ONLY)
                .len(3)
                .fill_val(0u32)
                .build()
                .unwrap();

            let tgt_buf = Buffer::<u32>::builder()
                .queue(queue.clone())
                .flags(flags::MEM_READ_ONLY | flags::MEM_HOST_WRITE_ONLY)
                .len(8)
                .fill_val(0u32)
                .build()
                .unwrap();

            let flag_buf: [Buffer<u32>; 2] = [
                Buffer::<u32>::builder()
                    .queue(queue.clone())
                    .flags(flags::MEM_READ_WRITE | flags::MEM_HOST_READ_ONLY)
                    .len(1)
                    .fill_val(0u32)
                    .build()
                    .unwrap(),
                Buffer::<u32>::builder()
                    .queue(queue.clone())
                    .flags(flags::MEM_READ_WRITE | flags::MEM_HOST_READ_ONLY)
                    .len(1)
                    .fill_val(0u32)
                    .build()
                    .unwrap(),
            ];
            let nonce_buf: [Buffer<u32>; 2] = [
                Buffer::<u32>::builder()
                    .queue(queue.clone())
                    .flags(flags::MEM_READ_WRITE | flags::MEM_HOST_READ_ONLY)
                    .len(1)
                    .fill_val(0u32)
                    .build()
                    .unwrap(),
                Buffer::<u32>::builder()
                    .queue(queue.clone())
                    .flags(flags::MEM_READ_WRITE | flags::MEM_HOST_READ_ONLY)
                    .len(1)
                    .fill_val(0u32)
                    .build()
                    .unwrap(),
            ];
            let hash_buf: [Buffer<u32>; 2] = [
                Buffer::<u32>::builder()
                    .queue(queue.clone())
                    .flags(flags::MEM_READ_WRITE | flags::MEM_HOST_READ_ONLY)
                    .len(8)
                    .fill_val(0u32)
                    .build()
                    .unwrap(),
                Buffer::<u32>::builder()
                    .queue(queue.clone())
                    .flags(flags::MEM_READ_WRITE | flags::MEM_HOST_READ_ONLY)
                    .len(8)
                    .fill_val(0u32)
                    .build()
                    .unwrap(),
            ];

            loop {
                if !running.load(Ordering::Relaxed) {
                    break;
                }

                let job_data = {
                    let guard = current_job.lock().unwrap();
                    guard.clone()
                };

                if let Some(job) = job_data {
                    let diff = *difficulty.lock().unwrap();
                    let target = diff_to_target(diff);
                    let extranonce1 = {
                        let sub = subscription.lock().unwrap();
                        sub.as_ref()
                            .map(|s| s.extranonce1.clone())
                            .unwrap_or_default()
                    };

                    extranonce2_counter = extranonce2_counter.wrapping_add(1);
                    let extranonce2 = {
                        let bytes = extranonce2_counter.to_le_bytes();
                        hex::encode(bytes)
                    };

                    let merkle_root = {
                        let coinbase = job.build_coinbase(&extranonce1, &extranonce2);
                        job.compute_merkle_root(&coinbase)
                    };
                    let ntime = {
                        let job_ntime = u32::from_str_radix(&job.ntime, 16).unwrap_or(0);
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as u32;
                        std::cmp::max(job_ntime, now)
                    };
                    let header = job.build_header(&merkle_root, ntime, 0);

                    let midstate = compute_midstate(&header);
                    let tail_words = [
                        u32::from_be_bytes([header[64], header[65], header[66], header[67]]),
                        u32::from_be_bytes([header[68], header[69], header[70], header[71]]),
                        u32::from_be_bytes([header[72], header[73], header[74], header[75]]),
                    ];
                    let target_be: [u32; 8] = std::array::from_fn(|i| {
                        let o = i * 4;
                        u32::from_be_bytes([target[o], target[o + 1], target[o + 2], target[o + 3]])
                    });

                    mid_buf.write(&midstate[..]).enq().ok();
                    tail_buf.write(&tail_words[..]).enq().ok();
                    tgt_buf.write(&target_be[..]).enq().ok();

                    let result = run_gpu_search(
                        &context,
                        &queue,
                        &program,
                        &mid_buf,
                        &tail_buf,
                        &tgt_buf,
                        &flag_buf,
                        &nonce_buf,
                        &hash_buf,
                        nonce_base,
                        per_dispatch as u64,
                        workgroup_size,
                    );

                    match result {
                        Ok((found_nonce, checked)) => {
                            hash_count += checked;
                            if let Some(nonce) = found_nonce {
                                let header = job.build_header(&merkle_root, ntime, nonce);
                                let hash = crate::job::double_sha256(&header);
                                let meets = hash_meets_target(&hash, &target);
                                if meets {
                                    eprintln!(
                                        "{} \x1b[33m✔ GPU FOUND SHARE!\x1b[0m nonce={:08x}, hash={}",
                                        ts(),
                                        nonce,
                                        hex::encode(hash)
                                    );
                                    let share = FoundShare {
                                        job_id: job.job_id.clone(),
                                        extranonce2: extranonce2.clone(),
                                        ntime: format!("{:08x}", ntime),
                                        nonce,
                                    };
                                    let _ = share_tx.send(share);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("{} [ERROR] GPU batch error: {}", ts(), e);
                        }
                    }

                    nonce_base = nonce_base.wrapping_add(per_dispatch);

                    let elapsed = last_report.elapsed().as_secs_f64();
                    if elapsed >= 10.0 {
                        let hr = hash_count as f64 / elapsed;
                        *hashrate.lock().unwrap() = hr;
                        eprintln!(
                            "{} GPU Hashrate: {:.2} H/s ({:.2} MH/s)",
                            ts(),
                            hr,
                            hr / 1_000_000.0
                        );
                        hash_count = 0;
                        last_report = Instant::now();
                    }
                } else {
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        });
    }
}

fn compute_midstate(header: &[u8; 80]) -> [u32; 8] {
    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            header[i * 4],
            header[i * 4 + 1],
            header[i * 4 + 2],
            header[i * 4 + 3],
        ]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut a = h[0];
    let mut b = h[1];
    let mut c = h[2];
    let mut d = h[3];
    let mut e = h[4];
    let mut f = h[5];
    let mut g = h[6];
    let mut hh = h[7];

    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let t1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(k[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    h[0] = h[0].wrapping_add(a);
    h[1] = h[1].wrapping_add(b);
    h[2] = h[2].wrapping_add(c);
    h[3] = h[3].wrapping_add(d);
    h[4] = h[4].wrapping_add(e);
    h[5] = h[5].wrapping_add(f);
    h[6] = h[6].wrapping_add(g);
    h[7] = h[7].wrapping_add(hh);

    h
}

fn run_gpu_search(
    context: &Context,
    queue: &Queue,
    program: &Program,
    mid_buf: &Buffer<u32>,
    tail_buf: &Buffer<u32>,
    tgt_buf: &Buffer<u32>,
    flag_buf: &[Buffer<u32>; 2],
    nonce_buf: &[Buffer<u32>; 2],
    hash_buf: &[Buffer<u32>; 2],
    start_nonce: u32,
    total_count: u64,
    workgroup_size: usize,
) -> Result<(Option<u32>, u64), String> {
    let mut committed: u64 = 0;
    let mut done: u64 = 0;
    let mut slot: usize = 0;
    let mut prev_queue: Option<Queue> = None;
    let mut prev_slot: usize = 0;
    let mut prev_batch: u64 = 0;
    let mut found_nonce: Option<u32> = None;

    while committed < total_count {
        let left = total_count - committed;
        let batch = left.min(u32::MAX as u64) as u32;
        let batch_start = start_nonce.wrapping_add(committed as u32);

        flag_buf[slot].write(&[0u32][..]).enq().ok();

        let global = ((batch as usize + workgroup_size - 1) / workgroup_size) * workgroup_size;

        let q = Queue::new(context, queue.device().clone(), None)
            .map_err(|e| format!("Failed to create queue: {}", e))?;

        let kernel = Kernel::builder()
            .program(program)
            .name("sha256d_search")
            .queue(q.clone())
            .global_work_size(global)
            .arg(mid_buf)
            .arg(tail_buf)
            .arg(tgt_buf)
            .arg(&batch_start)
            .arg(&flag_buf[slot])
            .arg(&nonce_buf[slot])
            .arg(&hash_buf[slot])
            .build()
            .map_err(|e| format!("Kernel build error: {}", e))?;

        unsafe {
            kernel
                .enq()
                .map_err(|e| format!("Kernel enqueue error: {}", e))?;
        }

        committed += batch as u64;

        if let Some(prev_q) = prev_queue.take() {
            prev_q
                .finish()
                .map_err(|e| format!("Queue finish error: {}", e))?;
            done += prev_batch;

            let mut flag = vec![0u32; 1];
            flag_buf[prev_slot].read(&mut flag).enq().ok();
            queue.finish().ok();

            if flag[0] != 0 {
                let mut nonce = vec![0u32; 1];
                nonce_buf[prev_slot].read(&mut nonce).enq().ok();
                queue.finish().ok();
                found_nonce = Some(nonce[0]);
                q.finish().ok();
                done += batch as u64;
                break;
            }
        }

        prev_queue = Some(q);
        prev_slot = slot;
        prev_batch = batch as u64;
        slot = 1 - slot;
    }

    if found_nonce.is_none() {
        if let Some(prev_q) = prev_queue.take() {
            prev_q
                .finish()
                .map_err(|e| format!("Queue finish error: {}", e))?;
            done += prev_batch;

            let mut flag = vec![0u32; 1];
            flag_buf[prev_slot].read(&mut flag).enq().ok();
            queue.finish().ok();

            if flag[0] != 0 {
                let mut nonce = vec![0u32; 1];
                nonce_buf[prev_slot].read(&mut nonce).enq().ok();
                queue.finish().ok();
                found_nonce = Some(nonce[0]);
            }
        }
    }

    Ok((found_nonce, done))
}
