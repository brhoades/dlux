use std::convert::TryFrom;
use std::fs::{read_link, File};

use ddc::Ddc;
use ddc_i2c::I2cDdc;
use i2c_linux::I2c;

use crate::{logging::*, prelude::*, types::*};

pub type I2CDevice = I2cDdc<I2c<File>>;
pub struct Device {
    name: String,
    inner: I2CDevice,
    max: Option<u16>,
}

impl TryFrom<I2CDevice> for Device {
    type Error = Error;
    fn try_from(dev: I2CDevice) -> Result<Self> {
        // dig out the device path from the file descriptor
        use std::os::unix::io::AsRawFd;
        let fd_num = dev.inner_ref().inner_ref().as_raw_fd().to_string();
        let fd = std::path::PathBuf::from("/proc/self/fd").join(fd_num);
        let path = read_link(fd)?;

        let name = path
            .to_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("[failed to convert path] {:?}", path));

        Ok(Self {
            name,
            inner: dev,
            max: None,
        })
    }
}

pub trait BrightnessHardware {
    /// Get the device's current relative brightness pecentage or return an error.
    fn brightness(&mut self) -> Result<f64>;
    /// Idempotently set brightness of display to the passed relative percentage of devices' max.
    fn set_brightness(&mut self, b: f64) -> Result<()>;
    /// Returns the raw, whole number maximum brightness value for the device.
    fn max_brightness(&mut self) -> Result<u16>;
}

impl BrightnessHardware for Device {
    fn brightness(&mut self) -> Result<f64> {
        let cap = self.inner.get_vcp_feature(0x10)?;

        Ok((cap.value() as f64)
            / (self
                .max_brightness()
                .context("couldn't calculate relative percentage")? as f64))
    }

    fn set_brightness(&mut self, b: f64) -> Result<()> {
        let rel_b = (b * self.max_brightness()? as f64) as u16;

        match self.inner.set_vcp_feature(0x10, rel_b) {
            Ok(_) => {
                debug!(
                    "set brightness for {} to {}% (absolute {})",
                    self,
                    b * 100.0,
                    rel_b,
                );
                Ok(())
            }
            Err(e) => {
                error!("failed to set monitor {} to {}%: {}", self, rel_b, e);
                Err(format_err!(
                    "failed to apply maximum brightness for {}: {}",
                    self,
                    e
                ))
            }
        }
    }

    fn max_brightness(&mut self) -> Result<u16> {
        if let Some(max) = self.max {
            return Ok(max);
        }

        match self.inner.get_vcp_feature(0x10) {
            Ok(cap) => {
                let max = cap.maximum();
                self.max = Some(max);
                Ok(max)
            }
            Err(e) => Err(format_err!(
                "failed to query maximum device brightness for {}: {}",
                self,
                e
            )),
        }
    }
}

impl Device {
    /// Ok if getting brightness was non-zero, otherwise Err with the error.
    pub fn try_brightness(&mut self) -> Result<()> {
        // XXX: refresh?
        self.inner.get_vcp_feature(0x10)?;
        Ok(())
    }

    pub fn display_info(&mut self) -> Result<DeviceInfo> {
        DeviceInfo::new(&mut self.inner)
    }
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
