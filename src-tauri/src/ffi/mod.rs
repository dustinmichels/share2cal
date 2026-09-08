#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod guard;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use guard::{FreeFn, NativeStringError, NativeStringGuard};
