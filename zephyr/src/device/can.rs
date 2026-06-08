//! Device wrapper for CAN controllers.

use core::ffi::c_void;

use super::{NoStatic, Unique};
use crate::raw;

/// A wrapper around a Zephyr CAN device.
pub struct Can {
    pub(crate) device: *const raw::device,
}

// SAFETY: a Zephyr device pointer is a 'static singleton; the driver mediates concurrent access.
unsafe impl Send for Can {}

impl Can {
    /// Constructor, intended to be called by devicetree-generated code.
    pub(crate) unsafe fn new(
        unique: &Unique,
        _static: &NoStatic,
        device: *const raw::device,
    ) -> Option<Can> {
        if !unique.once() {
            return None;
        }
        Some(Can { device })
    }

    /// Whether the underlying device is ready.
    pub fn is_ready(&self) -> bool {
        // SAFETY: device_is_ready accepts any (incl. null) device pointer and only reads state.
        unsafe { raw::device_is_ready(self.device) }
    }

    /// Set the controller mode (e.g. `CAN_MODE_LOOPBACK`). Returns 0 or a negative errno.
    pub fn set_mode(&self, mode: raw::can_mode_t) -> i32 {
        // SAFETY: `device` is a valid CAN device.
        unsafe { raw::can_set_mode(self.device, mode) }
    }

    /// Start the controller. Returns 0 or a negative errno.
    pub fn start(&self) -> i32 {
        // SAFETY: `device` is a valid CAN device.
        unsafe { raw::can_start(self.device) }
    }

    /// Stop the controller. Returns 0 or a negative errno.
    pub fn stop(&self) -> i32 {
        // SAFETY: `device` is a valid CAN device.
        unsafe { raw::can_stop(self.device) }
    }

    /// Queue `frame` for transmission, blocking up to `timeout` (no completion callback). Returns 0
    /// or a negative errno.
    pub fn send(&self, frame: &raw::can_frame, timeout: raw::k_timeout_t) -> i32 {
        // SAFETY: `device` is valid; can_send reads `frame` for the duration of the call.
        unsafe { raw::can_send(self.device, frame, timeout, None, core::ptr::null_mut()) }
    }

    /// Register an RX filter with a C-ABI callback. Returns the filter id (>= 0) or a negative errno.
    pub fn add_rx_filter(
        &self,
        callback: raw::can_rx_callback_t,
        user_data: *mut c_void,
        filter: &raw::can_filter,
    ) -> i32 {
        // SAFETY: `device` is valid; `callback` is a valid C-ABI fn invoked from driver context;
        // `filter` is read for the duration of the call.
        unsafe { raw::can_add_rx_filter(self.device, callback, user_data, filter) }
    }

    /// Remove a previously-added RX filter.
    pub fn remove_rx_filter(&self, filter_id: i32) {
        // SAFETY: `device` is valid; `filter_id` was returned by `add_rx_filter`.
        unsafe { raw::can_remove_rx_filter(self.device, filter_id) };
    }
}
