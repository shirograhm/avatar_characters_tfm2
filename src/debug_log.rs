//! A plain text log next to the mod's DLL, for chasing behaviour that only
//! shows up in a real match. It writes `avatar_characters_tfm2_debug.log` in
//! the deployed mod folder; `deploy.ps1` wipes that folder, so every deploy
//! starts a fresh log.
//!
//! Nothing here touches the simulation - it only reads and writes the file -
//! so leaving it on cannot change a match or a replay. Turn it off with
//! `ENABLED` once the question it is answering has been answered.
//!
//! Kept with no callers between investigations, hence the `allow`.
#![allow(dead_code)]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

pub const ENABLED: bool = false;

const FILE_NAME: &str = "avatar_characters_tfm2_debug.log";

fn log_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| dll_dir().unwrap_or_else(std::env::temp_dir).join(FILE_NAME))
        .clone()
}

/// The folder the mod's own DLL was loaded from. The game's working directory
/// is its install folder, not the mod's, so a relative path would land the log
/// somewhere nobody looks.
#[cfg(windows)]
fn dll_dir() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    extern "system" {
        fn GetModuleHandleExW(flags: u32, name: *const u16, module: *mut *mut std::ffi::c_void) -> i32;
        fn GetModuleFileNameW(module: *mut std::ffi::c_void, filename: *mut u16, size: u32) -> u32;
    }
    static ANCHOR: u8 = 0;
    let mut module = std::ptr::null_mut();
    // FROM_ADDRESS | UNCHANGED_REFCOUNT: the module that contains `ANCHOR`.
    if unsafe { GetModuleHandleExW(0x6, &ANCHOR as *const u8 as *const u16, &mut module) } == 0 {
        return None;
    }
    let mut buf = vec![0u16; 32768];
    let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if len == 0 {
        return None;
    }
    PathBuf::from(std::ffi::OsString::from_wide(&buf[..len]))
        .parent()
        .map(|dir| dir.to_path_buf())
}

#[cfg(not(windows))]
fn dll_dir() -> Option<PathBuf> {
    None
}

pub fn log(message: &str) {
    if !ENABLED {
        return;
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path()) {
        let _ = writeln!(file, "{message}");
    }
}
