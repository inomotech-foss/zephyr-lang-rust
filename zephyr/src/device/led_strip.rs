//! Device wrapper for WS2812-style LED strips (the `led_strip` subsystem).

use super::{NoStatic, Unique};
use crate::raw;

/// A wrapper around a Zephyr `led_strip` device.
pub struct LedStrip {
    pub(crate) device: *const raw::device,
}

// SAFETY: a Zephyr device pointer is a 'static singleton; the driver mediates concurrent access.
unsafe impl Send for LedStrip {}

impl LedStrip {
    /// Constructor, intended to be called by devicetree-generated code.
    pub(crate) unsafe fn new(
        unique: &Unique,
        _static: &NoStatic,
        device: *const raw::device,
    ) -> Option<LedStrip> {
        if !unique.once() {
            return None;
        }
        Some(LedStrip { device })
    }

    /// Construct from a raw device pointer (e.g. a devicetree `get_instance_raw()`). Unlike
    /// `get_instance()`, this does NOT consume the device's `Unique`, so it can be called
    /// repeatedly — appropriate for on-demand self-tests.
    ///
    /// # Safety
    /// `device` must be a valid, `'static` Zephyr device pointer (e.g. from `get_instance_raw()`):
    /// [`update_rgb`](Self::update_rgb) passes it to the C driver, which dereferences it.
    pub unsafe fn from_device(device: *const raw::device) -> Self {
        LedStrip { device }
    }

    /// Whether the underlying device is ready.
    pub fn is_ready(&self) -> bool {
        // SAFETY: device_is_ready accepts any (incl. null) device pointer and only reads state.
        unsafe { raw::device_is_ready(self.device) }
    }

    /// Update the strip with `pixels` (one `led_rgb` per LED). Returns 0 or a negative errno.
    pub fn update_rgb(&self, pixels: &[raw::led_rgb]) -> i32 {
        // SAFETY: `device` is a valid led_strip device; led_strip_update_rgb reads exactly
        // `pixels.len()` elements through the pointer and does not retain it.
        unsafe {
            raw::led_strip_update_rgb(
                self.device,
                pixels.as_ptr() as *mut raw::led_rgb,
                pixels.len(),
            )
        }
    }
}
