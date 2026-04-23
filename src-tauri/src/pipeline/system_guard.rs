use std::path::Path;

use super::error::PipelineError;

/// Minimum buffer space in bytes (500 MB).
const BUFFER_BYTES: u64 = 500 * 1024 * 1024;

/// Check that the disk containing `output_dir` has enough free space for the
/// planned transcode operation.
///
/// Formula: needed = source_size_bytes × platform_count × 1.5 + 500 MB
pub fn check_disk_space(
    output_dir: &Path,
    source_size_bytes: u64,
    platform_count: usize,
) -> Result<(), PipelineError> {
    let needed = (source_size_bytes as f64 * platform_count as f64 * 1.5) as u64 + BUFFER_BYTES;
    let available = available_space(output_dir)?;

    if available < needed {
        return Err(PipelineError::InsufficientDiskSpace {
            needed_mb: needed / (1024 * 1024),
            available_mb: available / (1024 * 1024),
        });
    }
    Ok(())
}

/// Query available disk space for the volume that contains `path`.
fn available_space(path: &Path) -> Result<u64, PipelineError> {
    fs4_available_space(path).map_err(|e| {
        PipelineError::FileNotFound(format!(
            "Cannot determine free space for {}: {e}",
            path.display()
        ))
    })
}

/// Cross-platform available-space helper using std::fs metadata.
/// Falls back to platform-specific APIs.
#[cfg(unix)]
fn fs4_available_space(path: &Path) -> std::io::Result<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(stat.f_bavail as u64 * stat.f_frsize as u64)
    }
}

#[cfg(windows)]
fn fs4_available_space(path: &Path) -> std::io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;

    // GetDiskFreeSpaceExW
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            lpDirectoryName: *const u16,
            lpFreeBytesAvailableToCaller: *mut u64,
            lpTotalNumberOfBytes: *mut u64,
            lpTotalNumberOfFreeBytes: *mut u64,
        ) -> i32;
    }

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free_available: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;

    let ret = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_available,
            &mut total,
            &mut total_free,
        )
    };
    if ret == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(free_available)
}

// ---------------------------------------------------------------------------
// Sleep inhibitor
// ---------------------------------------------------------------------------

/// RAII guard that prevents the system from sleeping while held.
/// Automatically releases the inhibition when dropped.
pub struct SleepInhibitor {
    #[cfg(target_os = "macos")]
    assertion_id: u32,
    // Windows: no handle needed – we just reset the flags on drop.
    #[cfg(target_os = "windows")]
    _priv: (),
    // Linux: no-op
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    _priv: (),
}

/// Create a sleep inhibitor. The system will not enter sleep/suspend while the
/// returned guard is alive.
pub fn inhibit_sleep(reason: &str) -> Result<SleepInhibitor, PipelineError> {
    _inhibit_sleep(reason)
}

// -- macOS implementation ---------------------------------------------------
#[cfg(target_os = "macos")]
fn _inhibit_sleep(reason: &str) -> Result<SleepInhibitor, PipelineError> {
    use std::ffi::CString;

    // IOKit / CoreFoundation FFI
    type CFStringRef = *const std::ffi::c_void;
    type IOPMAssertionID = u32;

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOPMAssertionCreateWithName(
            assertion_type: CFStringRef,
            assertion_level: u32,
            reason_for_activity: CFStringRef,
            assertion_id: *mut IOPMAssertionID,
        ) -> i32;
        fn IOPMAssertionRelease(assertion_id: IOPMAssertionID) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithCString(
            alloc: *const std::ffi::c_void,
            c_str: *const i8,
            encoding: u32,
        ) -> CFStringRef;
        fn CFRelease(cf: *const std::ffi::c_void);
    }

    const K_CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;
    const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;

    let assertion_type_str =
        CString::new("PreventUserIdleSystemSleep").expect("CString::new failed");
    let reason_str = CString::new(reason).unwrap_or_else(|_| CString::new("HiddenShield").unwrap());

    unsafe {
        let assertion_type = CFStringCreateWithCString(
            std::ptr::null(),
            assertion_type_str.as_ptr(),
            K_CFSTRING_ENCODING_UTF8,
        );
        let reason_cf = CFStringCreateWithCString(
            std::ptr::null(),
            reason_str.as_ptr(),
            K_CFSTRING_ENCODING_UTF8,
        );

        let mut assertion_id: IOPMAssertionID = 0;
        let status = IOPMAssertionCreateWithName(
            assertion_type,
            K_IOPM_ASSERTION_LEVEL_ON,
            reason_cf,
            &mut assertion_id,
        );

        CFRelease(assertion_type);
        CFRelease(reason_cf);

        if status != 0 {
            return Err(PipelineError::SleepInhibitFailed(format!(
                "IOPMAssertionCreateWithName returned {status}"
            )));
        }

        Ok(SleepInhibitor { assertion_id })
    }
}

#[cfg(target_os = "macos")]
impl Drop for SleepInhibitor {
    fn drop(&mut self) {
        type IOPMAssertionID = u32;
        #[link(name = "IOKit", kind = "framework")]
        extern "C" {
            fn IOPMAssertionRelease(assertion_id: IOPMAssertionID) -> i32;
        }
        unsafe {
            IOPMAssertionRelease(self.assertion_id);
        }
    }
}

// -- Windows implementation -------------------------------------------------
#[cfg(target_os = "windows")]
fn _inhibit_sleep(_reason: &str) -> Result<SleepInhibitor, PipelineError> {
    #[link(name = "kernel32")]
    extern "system" {
        fn SetThreadExecutionState(es_flags: u32) -> u32;
    }

    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;

    let prev = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };
    if prev == 0 {
        return Err(PipelineError::SleepInhibitFailed(
            "SetThreadExecutionState failed".into(),
        ));
    }
    Ok(SleepInhibitor { _priv: () })
}

#[cfg(target_os = "windows")]
impl Drop for SleepInhibitor {
    fn drop(&mut self) {
        #[link(name = "kernel32")]
        extern "system" {
            fn SetThreadExecutionState(es_flags: u32) -> u32;
        }
        const ES_CONTINUOUS: u32 = 0x8000_0000;
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

// -- Linux / other: no-op ---------------------------------------------------
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn _inhibit_sleep(_reason: &str) -> Result<SleepInhibitor, PipelineError> {
    Ok(SleepInhibitor { _priv: () })
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl Drop for SleepInhibitor {
    fn drop(&mut self) {
        // no-op on Linux / other platforms
    }
}
