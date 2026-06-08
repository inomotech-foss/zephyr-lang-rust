//! Device wrapper for the Zephyr `sensor` subsystem.

use super::{NoStatic, Unique};
use crate::raw;

/// A wrapper around a Zephyr `sensor` device.
pub struct Sensor {
    pub(crate) device: *const raw::device,
}

// SAFETY: a Zephyr device pointer is a 'static singleton; the driver mediates concurrent access.
unsafe impl Send for Sensor {}

impl Sensor {
    /// Constructor, intended to be called by devicetree-generated code.
    pub(crate) unsafe fn new(
        unique: &Unique,
        _static: &NoStatic,
        device: *const raw::device,
    ) -> Option<Sensor> {
        if !unique.once() {
            return None;
        }
        Some(Sensor { device })
    }

    /// Construct from a raw device pointer (e.g. a devicetree `get_instance_raw()`). Unlike
    /// `get_instance()`, this does NOT consume the device's `Unique`, so it can be called
    /// repeatedly — appropriate for on-demand self-tests. The caller guarantees `device` is valid.
    pub fn from_device(device: *const raw::device) -> Self {
        Sensor { device }
    }

    /// Whether the underlying device is ready.
    pub fn is_ready(&self) -> bool {
        // SAFETY: device_is_ready accepts any (incl. null) device pointer and only reads state.
        unsafe { raw::device_is_ready(self.device) }
    }

    /// Set a channel attribute (e.g. sampling frequency). Returns 0 or a negative errno.
    pub fn attr_set(
        &self,
        chan: raw::sensor_channel,
        attr: raw::sensor_attribute,
        val: &raw::sensor_value,
    ) -> i32 {
        // SAFETY: `device` is a valid sensor device; sensor_attr_set reads `val` (a const ptr in C)
        // and does not retain it.
        unsafe { raw::sensor_attr_set(self.device, chan, attr, val) }
    }

    /// Fetch a fresh sample for all channels. Returns 0 or a negative errno.
    pub fn sample_fetch(&self) -> i32 {
        // SAFETY: `device` is a valid sensor device; takes the device only.
        unsafe { raw::sensor_sample_fetch(self.device) }
    }

    /// Read `out.len()` `sensor_value`s for `chan`. Returns 0 or a negative errno. The caller sizes
    /// `out` to the channel's value count (e.g. 3 for `SENSOR_CHAN_ACCEL_XYZ`).
    pub fn channel_get(&self, chan: raw::sensor_channel, out: &mut [raw::sensor_value]) -> i32 {
        // SAFETY: `device` is valid; sensor_channel_get writes the channel's values into `out`.
        unsafe { raw::sensor_channel_get(self.device, chan, out.as_mut_ptr()) }
    }
}

/// `sensor_value` as f64 (`val1 + val2 / 1e6`) — mirrors the C inline `sensor_value_to_double`.
pub fn value_to_f64(v: &raw::sensor_value) -> f64 {
    v.val1 as f64 + v.val2 as f64 / 1_000_000.0
}
