//! Minimal typed FAT / disk-access helper (the ELM-FAT filesystem over a disk-access drive).
//!
//! This is not a devicetree device, so there is no `zephyr::device` wrapper — just functions over
//! the `fs_*` / `disk_access_*` C API. The FAT context (`FATFS`) is kept as an opaque, over-sized,
//! 8-aligned stack buffer so we don't need to bind `<ff.h>`.

use core::ffi::{c_void, CStr};

use crate::raw;

/// Whether this build uses the RAM-disk backend (vs a real SD card).
pub fn is_ram_disk() -> bool {
    cfg!(CONFIG_DISK_DRIVER_RAM)
}

/// Mount `disk` (FAT) at `mnt`, write `payload` to `file`, read it back into `out`, then unmount.
/// Returns the number of bytes read, or a negative errno (`-ENODEV` if the disk can't initialise).
pub fn fat_rw(
    disk: &CStr,
    mnt: &CStr,
    file: &CStr,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, i32> {
    // SAFETY: disk_access_init takes a NUL-terminated drive name and returns 0 on success.
    if unsafe { raw::disk_access_init(disk.as_ptr()) } != 0 {
        return Err(-(raw::ENODEV as i32));
    }

    // Opaque backing for the FAT context (`FATFS`): a small fixed header followed by a
    // `win[FF_MAX_SS]` sector cache, where `FF_MAX_SS == CONFIG_FS_FATFS_MAX_SS`. Sizing it from
    // that Kconfig (+128 B headroom for the fixed fields — a multiple of 8, so `/ 8` is exact and
    // the buffer stays `u64`/8-aligned) means raising `CONFIG_FS_FATFS_MAX_SS` can never silently
    // overflow this stack buffer. Lives on this stack frame across mount..unmount (the FS core
    // retains the pointer until fs_unmount).
    const FATFS_BUF_U64S: usize = (crate::kconfig::CONFIG_FS_FATFS_MAX_SS as usize + 128) / 8;
    let mut fatfs_buf = [0u64; FATFS_BUF_U64S];
    // SAFETY: an all-zero fs_mount_t is valid (null list node / fs ptr); the FS-core fields are set
    // below, matching the C designated initializer.
    let mut mp: raw::fs_mount_t = unsafe { core::mem::zeroed() };
    mp.type_ = raw::FS_FATFS as i32;
    mp.fs_data = fatfs_buf.as_mut_ptr() as *mut c_void;
    mp.mnt_point = mnt.as_ptr();
    mp.storage_dev = disk.as_ptr() as *const u8 as *mut c_void;

    // SAFETY: mp + fatfs_buf outlive the mount..unmount calls below (same stack frame).
    let rc = unsafe { raw::fs_mount(&mut mp) };
    if rc != 0 {
        return Err(rc);
    }

    let result = fat_write_read(file, payload, out);

    // SAFETY: mp is currently mounted.
    unsafe { raw::fs_unmount(&mut mp) };

    if result < 0 {
        Err(result)
    } else {
        Ok(result as usize)
    }
}

/// open+write+close, then open+read+close. Returns bytes read (>= 0) or a negative errno.
fn fat_write_read(file: &CStr, payload: &[u8], out: &mut [u8]) -> i32 {
    let create = (raw::FS_O_CREATE | raw::FS_O_WRITE | raw::FS_O_TRUNC) as raw::fs_mode_t;

    // SAFETY: an all-zero fs_file_t is the documented init state (equivalent to fs_file_t_init).
    let mut f: raw::fs_file_t = unsafe { core::mem::zeroed() };
    // SAFETY: f/file are valid; fs_open initialises the handle on success.
    let rc = unsafe { raw::fs_open(&mut f, file.as_ptr(), create) };
    if rc != 0 {
        return rc;
    }
    // SAFETY: f is an open file; payload is a valid readable slice.
    let w = unsafe { raw::fs_write(&mut f, payload.as_ptr() as *const c_void, payload.len()) };
    // SAFETY: f is open.
    unsafe { raw::fs_close(&mut f) };
    if w < 0 {
        return w as i32;
    }

    // SAFETY: all-zero fs_file_t init.
    let mut f: raw::fs_file_t = unsafe { core::mem::zeroed() };
    // SAFETY: f/file valid.
    let rc = unsafe { raw::fs_open(&mut f, file.as_ptr(), raw::FS_O_READ as raw::fs_mode_t) };
    if rc != 0 {
        return rc;
    }
    // SAFETY: f is open; out is a valid writable slice.
    let n = unsafe { raw::fs_read(&mut f, out.as_mut_ptr() as *mut c_void, out.len()) };
    // SAFETY: f is open.
    unsafe { raw::fs_close(&mut f) };
    n as i32
}
