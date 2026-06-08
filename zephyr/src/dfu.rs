//! Typed MCUboot + flash_img (DFU) helpers.
//!
//! Gated on `CONFIG_MCUBOOT_IMG_MANAGER`. On builds without it (e.g. `native_sim`) the functions
//! return the documented no-MCUboot sentinels so callers behave **identically** to the old C
//! `#else` stubs. The MCUboot arms are exercised on the real esp32c5 image (the link oracle); the
//! partition ids come from the C `tcu_slot_id()` trampoline (PARTITION_ID is a compile-time DT
//! macro with no Rust expansion).

// `c_int` is only referenced by the MCUboot arms (the trampoline + `boot_request_upgrade`); gate
// the import so a no-MCUboot build (e.g. native_sim) stays warning-clean under clippy `-D warnings`.
#[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
use core::ffi::c_int;

// The fixed-partition ids come from the app's `tcu_slot_id()` trampoline in `src/shim.c`
// (`PARTITION_ID` is a compile-time DT macro with no Rust expansion). Declared once here, used by
// `running_version` (slot0) and `Slot1Writer::begin` (slot1).
#[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
extern "C" {
    fn tcu_slot_id(which: c_int) -> u8;
}

/// Fixed-partition id for slot `which` (0 = slot0, 1 = slot1), via the app trampoline.
#[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
fn slot_id(which: c_int) -> u8 {
    // SAFETY: a pure DT-id lookup with no preconditions; `which` is constrained to {0, 1}.
    unsafe { tcu_slot_id(which) }
}

/// Confirm state of the running (slot0) image: 1 = confirmed / 0 = pending TEST / -1 = no MCUboot.
pub fn confirm_state() -> i32 {
    #[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
    {
        // SAFETY: read-only MCUboot query, no arguments.
        if unsafe { crate::raw::boot_is_img_confirmed() } {
            1
        } else {
            0
        }
    }
    #[cfg(not(CONFIG_MCUBOOT_IMG_MANAGER))]
    {
        -1
    }
}

/// MCUboot swap type (NONE=1 .. FAIL=5) or -1 if no MCUboot.
pub fn swap_type() -> i32 {
    #[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
    {
        // SAFETY: read-only MCUboot query.
        unsafe { crate::raw::mcuboot_swap_type() }
    }
    #[cfg(not(CONFIG_MCUBOOT_IMG_MANAGER))]
    {
        -1
    }
}

/// Confirm the running image: 0 ok / <0 errno / 1 = no-MCUboot sentinel.
pub fn confirm() -> i32 {
    #[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
    {
        // SAFETY: idempotent MCUboot confirm write.
        unsafe { crate::raw::boot_write_img_confirmed() }
    }
    #[cfg(not(CONFIG_MCUBOOT_IMG_MANAGER))]
    {
        1
    }
}

/// Request a TEST (`permanent=false`) / PERMANENT upgrade of slot1, then verify it went pending.
/// 0 ok+pending / -1 no-MCUboot / -2 request rc!=0 / -3 rc==0 but no pending swap.
pub fn request_upgrade(permanent: bool) -> i32 {
    #[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
    {
        let mode = if permanent {
            crate::raw::BOOT_UPGRADE_PERMANENT
        } else {
            crate::raw::BOOT_UPGRADE_TEST
        };
        // SAFETY: MCUboot upgrade request, integer arg.
        let rc = unsafe { crate::raw::boot_request_upgrade(mode as c_int) };
        if rc != 0 {
            return -2;
        }
        // SAFETY: read-only query to confirm the swap actually went pending.
        let swp = unsafe { crate::raw::mcuboot_swap_type() };
        if swp != crate::raw::BOOT_SWAP_TYPE_TEST as i32 && swp != crate::raw::BOOT_SWAP_TYPE_PERM as i32 {
            return -3;
        }
        0
    }
    #[cfg(not(CONFIG_MCUBOOT_IMG_MANAGER))]
    {
        let _ = permanent;
        -1
    }
}

/// Write the running (slot0) image's semver into `out` (>= 25 bytes), NUL-terminated.
/// 0 ok / negative rc on header-read error / -1 no MCUboot.
pub fn running_version(out: &mut [u8]) -> i32 {
    #[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
    {
        let slot0 = slot_id(0);
        // SAFETY: a zeroed mcuboot_img_header is valid; boot_read_bank_header fills it.
        let mut h: crate::raw::mcuboot_img_header = unsafe { core::mem::zeroed() };
        // SAFETY: read the slot0 image header into `h`.
        let rc = unsafe {
            crate::raw::boot_read_bank_header(
                slot0,
                &mut h,
                core::mem::size_of::<crate::raw::mcuboot_img_header>(),
            )
        };
        if rc != 0 {
            if !out.is_empty() {
                out[0] = 0;
            }
            return rc;
        }
        // SAFETY: mcuboot_version 1 -> the v1 arm of the header union is the valid member.
        let s = &unsafe { h.h.v1.as_ref() }.sem_ver;
        write_semver(out, s.major, s.minor, s.revision, s.build_num);
        0
    }
    #[cfg(not(CONFIG_MCUBOOT_IMG_MANAGER))]
    {
        // Mirror the old C #else stub which wrote "n/a" (so `ota status` prints version=n/a).
        const NA: &[u8] = b"n/a\0";
        let n = NA.len().min(out.len());
        out[..n].copy_from_slice(&NA[..n]);
        -1
    }
}

/// Format "major.minor.revision+build" into `out` as NUL-terminated ASCII (mirrors the C snprintf).
#[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
fn write_semver(out: &mut [u8], major: u8, minor: u8, revision: u16, build: u32) {
    use core::fmt::Write;
    struct Cursor<'a> {
        buf: &'a mut [u8],
        pos: usize,
    }
    impl core::fmt::Write for Cursor<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let b = s.as_bytes();
            let n = b.len().min(self.buf.len().saturating_sub(self.pos));
            self.buf[self.pos..self.pos + n].copy_from_slice(&b[..n]);
            self.pos += n;
            if n < b.len() {
                Err(core::fmt::Error)
            } else {
                Ok(())
            }
        }
    }
    let pos = {
        let mut c = Cursor { buf: out, pos: 0 };
        let _ = write!(c, "{}.{}.{}+{}", major, minor, revision, build);
        c.pos
    };
    let term = pos.min(out.len().saturating_sub(1));
    if !out.is_empty() {
        out[term] = 0;
    }
}

/// A streaming writer into the secondary MCUboot slot (slot1) via flash_img. esp32c5 / HIL only;
/// on native_sim `begin` returns the no-MCUboot sentinel and the writer is never constructed.
#[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
pub struct Slot1Writer {
    ctx: crate::raw::flash_img_context,
}

#[cfg(CONFIG_MCUBOOT_IMG_MANAGER)]
impl Slot1Writer {
    /// Initialise streaming into slot1. Returns the writer or a negative errno.
    pub fn begin() -> Result<Slot1Writer, i32> {
        // SAFETY: a zeroed flash_img_context is valid prior to init.
        let mut ctx: crate::raw::flash_img_context = unsafe { core::mem::zeroed() };
        // SAFETY: slot_id(1) -> slot1 partition id; flash_img_init_id initialises ctx.
        let rc = unsafe { crate::raw::flash_img_init_id(&mut ctx, slot_id(1)) };
        if rc == 0 {
            Ok(Slot1Writer { ctx })
        } else {
            Err(rc)
        }
    }

    /// Append `data` (buffered). Returns 0 or a negative errno.
    pub fn write(&mut self, data: &[u8]) -> i32 {
        // SAFETY: ctx is initialised; `data` is a valid readable slice copied into the staging buffer.
        unsafe { crate::raw::flash_img_buffered_write(&mut self.ctx, data.as_ptr(), data.len(), false) }
    }

    /// Flush the staging buffer to slot1. Returns 0 or a negative errno.
    pub fn finish(&mut self) -> i32 {
        // SAFETY: flush with a zero-length payload + flush=true.
        unsafe { crate::raw::flash_img_buffered_write(&mut self.ctx, core::ptr::null(), 0, true) }
    }
}

/// No-MCUboot sentinel writer so the app's HwFlashSink type-checks on native_sim (never used there).
#[cfg(not(CONFIG_MCUBOOT_IMG_MANAGER))]
pub struct Slot1Writer;

#[cfg(not(CONFIG_MCUBOOT_IMG_MANAGER))]
impl Slot1Writer {
    /// No-MCUboot: never succeeds (returns the sentinel 1).
    pub fn begin() -> Result<Slot1Writer, i32> {
        Err(1)
    }
    /// No-MCUboot: no-op sentinel.
    pub fn write(&mut self, _data: &[u8]) -> i32 {
        1
    }
    /// No-MCUboot: no-op sentinel.
    pub fn finish(&mut self) -> i32 {
        1
    }
}
