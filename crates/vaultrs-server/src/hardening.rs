//! Production hardening: memory pinning and core dump prevention.
//!
//! On Unix systems, this module provides two critical security measures:
//!
//! 1. **`disable_core_dumps`** — Sets `RLIMIT_CORE` to 0, preventing the OS
//!    from writing core dump files that could contain key material.
//!
//! 2. **`lock_memory`** — Calls `mlockall(MCL_CURRENT | MCL_FUTURE)` to pin
//!    all current and future memory pages, preventing the OS from swapping
//!    sensitive data (unseal keys, root keys, transit keys) to disk.
//!
//! Both functions are no-ops on non-Unix platforms.

/// Disable core dumps by setting `RLIMIT_CORE` to 0.
///
/// Core dumps can contain key material in plaintext. In a secrets manager,
/// this is unacceptable. Call this early in `main()` before any keys are
/// loaded into memory.
///
/// # Errors
///
/// Returns an error string if the `setrlimit` syscall fails.
#[cfg(unix)]
pub fn disable_core_dumps() -> Result<(), String> {
    // SAFETY: `setrlimit` is a POSIX syscall that sets resource limits for
    // the current process. We pass a valid `rlimit` struct with both fields
    // set to 0. This is a well-defined operation with no memory safety
    // implications — it only affects the kernel's willingness to write core
    // dump files for this process.
    #[allow(unsafe_code)]
    let result = unsafe {
        let rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::setrlimit(libc::RLIMIT_CORE, &rlim)
    };

    if result == 0 {
        Ok(())
    } else {
        Err(format!(
            "setrlimit(RLIMIT_CORE, 0) failed with errno {}",
            std::io::Error::last_os_error()
        ))
    }
}

/// No-op on non-Unix platforms.
#[cfg(not(unix))]
pub fn disable_core_dumps() -> Result<(), String> {
    Ok(())
}

/// Pin all current and future memory pages with `mlockall`.
///
/// This prevents the OS from swapping any of the process's memory to disk,
/// which could expose key material. Requires `CAP_IPC_LOCK` on Linux or
/// running as root. In development, set `VAULTRS_DISABLE_MLOCK=true` to
/// skip this step.
///
/// # Errors
///
/// Returns an error string if the `mlockall` syscall fails.
#[cfg(unix)]
pub fn lock_memory() -> Result<(), String> {
    // SAFETY: `mlockall` is a POSIX syscall that locks all current and
    // future mapped pages into RAM. We pass `MCL_CURRENT | MCL_FUTURE`
    // which are well-defined flags. The call has no memory safety
    // implications — it only instructs the kernel to keep pages resident.
    // Failure (e.g., insufficient privileges) is handled gracefully.
    #[allow(unsafe_code)]
    let result = unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) };

    if result == 0 {
        Ok(())
    } else {
        Err(format!(
            "mlockall(MCL_CURRENT | MCL_FUTURE) failed with errno {}",
            std::io::Error::last_os_error()
        ))
    }
}

/// No-op on non-Unix platforms.
#[cfg(not(unix))]
pub fn lock_memory() -> Result<(), String> {
    Ok(())
}
