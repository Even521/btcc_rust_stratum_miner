// GPU mining module - Metal (Apple Silicon / macOS only)
//
// On macOS with `--features metal-gpu`, this module provides GPU-accelerated
// SHA-256d mining via Metal compute shaders.
//
// On other platforms, a stub GpuMiner is provided that always returns None.

#[cfg(all(target_os = "macos", feature = "metal-gpu"))]
mod metal_impl;

#[cfg(all(target_os = "macos", feature = "metal-gpu"))]
pub use metal_impl::GpuMiner;

#[cfg(not(all(target_os = "macos", feature = "metal-gpu")))]
mod stub;

#[cfg(not(all(target_os = "macos", feature = "metal-gpu")))]
pub use stub::GpuMiner;

