//! Operating-system specifics.

use anyhow::{Result, bail};

/// tasker is a per-user tool: as root it would write root-owned files into the user's data folder,
/// or use root's instead. Containers that only have root can opt in with `TASKER_ALLOW_ROOT=1`.
pub fn ensure_not_root() -> Result<()> {
    if is_root() && std::env::var_os("TASKER_ALLOW_ROOT").is_none_or(|v| v != "1") {
        bail!(
            "tasker is a per-user app and doesn't run as root (sudo).\n\
             Run it as your normal user. Only if root is your only user (e.g. a container), set TASKER_ALLOW_ROOT=1."
        );
    }
    Ok(())
}

/// Whether the effective user is root.
#[cfg(unix)]
#[allow(unsafe_code)]
fn is_root() -> bool {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    // SAFETY: geteuid takes no arguments, has no preconditions and cannot fail.
    unsafe { geteuid() == 0 }
}

/// Windows has no root; an elevated shell still runs as the user's own account.
#[cfg(not(unix))]
fn is_root() -> bool {
    false
}
