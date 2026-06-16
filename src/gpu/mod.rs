// GPU mining module.
//
// Platform support:
//   macOS + --features metal-gpu  → Metal (Apple Silicon)
//   Windows/Linux                 → OpenCL (NVIDIA/AMD/Intel, default feature)
//   Other / no GPU                → CPU fallback

#[cfg(all(target_os = "macos", feature = "metal-gpu"))]
mod metal_impl;
#[cfg(all(target_os = "macos", feature = "metal-gpu"))]
pub use metal_impl::GpuMiner;

#[cfg(all(
    not(all(target_os = "macos", feature = "metal-gpu")),
    feature = "opencl-gpu"
))]
mod opencl_impl;
#[cfg(all(
    not(all(target_os = "macos", feature = "metal-gpu")),
    feature = "opencl-gpu"
))]
pub use opencl_impl::GpuMiner;

#[cfg(not(any(
    all(target_os = "macos", feature = "metal-gpu"),
    feature = "opencl-gpu"
)))]
mod stub;
#[cfg(not(any(
    all(target_os = "macos", feature = "metal-gpu"),
    feature = "opencl-gpu"
)))]
pub use stub::GpuMiner;
