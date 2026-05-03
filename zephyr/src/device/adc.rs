//! Device wrapper for ADC controllers.

use core::ffi::c_int;

use super::{NoStatic, Unique};
use crate::raw;

/// A wrapper around a Zephyr ADC device.
#[allow(dead_code)]
pub struct Adc {
    pub(crate) device: *const raw::device,
}

unsafe impl Send for Adc {}

impl Adc {
    /// Constructor, intended to be called by devicetree generated code.
    #[allow(dead_code)]
    pub(crate) unsafe fn new(
        unique: &Unique,
        _static: &NoStatic,
        device: *const raw::device,
    ) -> Option<Adc> {
        if !unique.once() {
            return None;
        }

        Some(Adc { device })
    }

    /// Check if the ADC device is ready.
    pub fn is_ready(&self) -> bool {
        unsafe { raw::device_is_ready(self.device) }
    }

    /// Configure an ADC channel.
    ///
    /// # Safety
    ///
    /// The caller must ensure `cfg` is properly initialized for the target ADC hardware.
    pub unsafe fn channel_setup(&self, cfg: &raw::adc_channel_cfg) -> c_int {
        raw::adc_channel_setup(self.device, cfg)
    }

    /// Perform an ADC read.
    ///
    /// # Safety
    ///
    /// The caller must ensure `seq` is properly initialized, including a valid buffer pointer.
    pub unsafe fn read(&self, seq: &raw::adc_sequence) -> c_int {
        raw::adc_read(self.device, seq)
    }
}
