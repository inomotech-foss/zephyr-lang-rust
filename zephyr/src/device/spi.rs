//! Device wrapper for SPI controllers. Peripherals on the bus are addressed by the `slave`/`cs`
//! fields of the `spi_config` passed to [`Spi::transceive`].

use super::{NoStatic, Unique};
use crate::raw;

/// A wrapper around a Zephyr SPI controller device.
pub struct Spi {
    pub(crate) device: *const raw::device,
}

// SAFETY: a Zephyr device pointer is a 'static singleton; the driver mediates concurrent access.
unsafe impl Send for Spi {}

impl Spi {
    /// Constructor, intended to be called by devicetree-generated code.
    pub(crate) unsafe fn new(
        unique: &Unique,
        _static: &NoStatic,
        device: *const raw::device,
    ) -> Option<Spi> {
        if !unique.once() {
            return None;
        }
        Some(Spi { device })
    }

    /// Construct from a raw device pointer (e.g. a devicetree `get_instance_raw()`). Unlike
    /// `get_instance()`, this does NOT consume the device's `Unique`, so it can be called
    /// repeatedly — appropriate for on-demand self-tests. The caller guarantees `device` is valid.
    pub fn from_device(device: *const raw::device) -> Self {
        Spi { device }
    }

    /// Whether the underlying device is ready.
    pub fn is_ready(&self) -> bool {
        // SAFETY: device_is_ready accepts any (incl. null) device pointer and only reads state.
        unsafe { raw::device_is_ready(self.device) }
    }

    /// Synchronous transceive: `tx` and `rx` are `spi_buf_set`s; the peripheral is selected by the
    /// `slave`/`cs` fields of `config`. Returns 0 or a negative errno.
    pub fn transceive(
        &self,
        config: &raw::spi_config,
        tx: &raw::spi_buf_set,
        rx: &raw::spi_buf_set,
    ) -> i32 {
        // SAFETY: `device` is a valid SPI controller; config/tx/rx are read for the duration of the
        // call and describe valid buffers.
        unsafe { raw::spi_transceive(self.device, config, tx, rx) }
    }
}
