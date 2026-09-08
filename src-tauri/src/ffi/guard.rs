use std::ffi::CStr;
use std::os::raw::c_char;
use thiserror::Error;

/// Function pointer type for native C string deallocators.
pub type FreeFn = unsafe extern "C" fn(*mut c_char);

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum NativeStringError {
    #[error("Native string pointer is null")]
    NullPointer,

    #[error("Invalid UTF-8 in native string: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

/// RAII memory guard for heap-allocated native C strings (`*mut c_char`).
///
/// Ensures the memory is deallocated using the provided `free_fn` when dropped,
/// avoiding memory leaks on normal returns, early returns, or panics.
#[derive(Debug)]
pub struct NativeStringGuard {
    ptr: *mut c_char,
    free_fn: FreeFn,
}

impl NativeStringGuard {
    /// Creates a new guard for a native string pointer and its deallocation function.
    pub fn new(ptr: *mut c_char, free_fn: FreeFn) -> Self {
        Self { ptr, free_fn }
    }

    /// Returns the raw pointer.
    pub fn as_ptr(&self) -> *const c_char {
        self.ptr
    }

    /// Returns the raw mutable pointer.
    pub fn as_mut_ptr(&self) -> *mut c_char {
        self.ptr
    }

    /// Returns `true` if the underlying pointer is null.
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Returns a reference to the underlying `CStr` if the pointer is non-null.
    pub fn as_c_str(&self) -> Option<&CStr> {
        if self.ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(self.ptr) })
        }
    }

    /// Converts the native C string to a lossy UTF-8 `String`, or `None` if null.
    pub fn to_string_lossy(&self) -> Option<String> {
        self.as_c_str().map(|c| c.to_string_lossy().into_owned())
    }

    /// Converts the native C string to a borrowed UTF-8 `&str`, returning `None` if null.
    pub fn to_str(&self) -> Result<Option<&str>, std::str::Utf8Error> {
        match self.as_c_str() {
            Some(c) => c.to_str().map(Some),
            None => Ok(None),
        }
    }

    /// Consumes the guard and converts the native C string into an owned `String`.
    ///
    /// If the pointer is null, returns `NativeStringError::NullPointer`.
    /// If the string contains invalid UTF-8, returns `NativeStringError::Utf8`.
    pub fn into_string(self) -> Result<String, NativeStringError> {
        if self.ptr.is_null() {
            return Err(NativeStringError::NullPointer);
        }
        let c_str = unsafe { CStr::from_ptr(self.ptr) };
        let s = c_str.to_str()?.to_owned();
        Ok(s)
    }

    /// Releases ownership of the pointer without freeing it.
    pub fn into_raw(mut self) -> *mut c_char {
        let ptr = self.ptr;
        self.ptr = std::ptr::null_mut();
        ptr
    }
}

impl Drop for NativeStringGuard {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                (self.free_fn)(self.ptr);
            }
            self.ptr = std::ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROP_FREED_COUNT: AtomicUsize = AtomicUsize::new(0);
    static INTO_RAW_FREED_COUNT: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn mock_free_drop(ptr: *mut c_char) {
        if !ptr.is_null() {
            DROP_FREED_COUNT.fetch_add(1, Ordering::SeqCst);
            let _ = CString::from_raw(ptr);
        }
    }

    unsafe extern "C" fn mock_free_into_raw(ptr: *mut c_char) {
        if !ptr.is_null() {
            INTO_RAW_FREED_COUNT.fetch_add(1, Ordering::SeqCst);
            let _ = CString::from_raw(ptr);
        }
    }

    unsafe extern "C" fn mock_noop_free(_ptr: *mut c_char) {}

    #[test]
    fn test_null_guard() {
        let guard = NativeStringGuard::new(std::ptr::null_mut(), mock_noop_free);
        assert!(guard.is_null());
        assert_eq!(guard.as_ptr(), std::ptr::null());
        assert_eq!(guard.to_string_lossy(), None);
        assert_eq!(guard.to_str(), Ok(None));
        assert_eq!(guard.into_string(), Err(NativeStringError::NullPointer));
    }

    #[test]
    fn test_valid_string_conversions() {
        let c_string = CString::new("Hello, world!").unwrap();
        let raw = c_string.into_raw();
        let guard = NativeStringGuard::new(raw, mock_noop_free);

        assert!(!guard.is_null());
        assert_eq!(guard.to_string_lossy(), Some("Hello, world!".to_string()));
        assert_eq!(guard.to_str(), Ok(Some("Hello, world!")));

        let owned = guard.into_string().unwrap();
        assert_eq!(owned, "Hello, world!");

        // Reclaim raw to avoid leak since mock_noop_free was used
        let c_string = unsafe { CString::from_raw(raw) };
        drop(c_string);
    }

    #[test]
    fn test_drop_calls_free() {
        let count_before = DROP_FREED_COUNT.load(Ordering::SeqCst);
        let c_string = CString::new("drop test").unwrap();
        let raw = c_string.into_raw();
        {
            let _guard = NativeStringGuard::new(raw, mock_free_drop);
        }
        assert_eq!(DROP_FREED_COUNT.load(Ordering::SeqCst), count_before + 1);
    }

    #[test]
    fn test_into_raw_disarms_drop() {
        let count_before = INTO_RAW_FREED_COUNT.load(Ordering::SeqCst);
        let c_string = CString::new("into_raw test").unwrap();
        let raw = c_string.into_raw();
        let extracted = {
            let guard = NativeStringGuard::new(raw, mock_free_into_raw);
            guard.into_raw()
        };
        assert_eq!(extracted, raw);
        assert_eq!(INTO_RAW_FREED_COUNT.load(Ordering::SeqCst), count_before);
        // Clean up
        unsafe { mock_free_into_raw(extracted) };
    }
}
