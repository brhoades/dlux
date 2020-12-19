use std::convert::TryFrom;
use std::fs::{read_link, File};

use ddc::Ddc;
use ddc_i2c::I2cDdc;
use i2c_linux::I2c;
use log::{debug, error, trace, warn};

use crate::{config::DeviceConfig, edid::DisplayInfo, types::*};

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

    pub fn display_info(&mut self) -> Result<DisplayInfo> {
        DisplayInfo::new(&mut self.inner)
    }
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Display is a i2c device paired with its configuration.
pub struct Display {
    device: Device,
    cfg: DeviceConfig,
}

pub struct Displays {
    displays: Vec<Display>,
}

impl Display {
    pub fn display_info(&mut self) -> Result<DisplayInfo> {
        self.device.display_info()
    }
}

impl std::fmt::Display for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.device.fmt(f)
    }
}

impl Displays {
    pub fn new<C: AsRef<Vec<DeviceConfig>>>(cfgs: C) -> Result<Self> {
        let cfgs = cfgs.as_ref();

        let (devs, unavail_devs) = ddc_i2c::I2cDeviceEnumerator::new()?
            .map(TryFrom::try_from)
            .map_results(|mut i: Device| match i.try_brightness() {
                Ok(_) => {
                    trace!("found device {}", i);
                    Left(i)
                }
                Err(e) => Right((i, e)),
            })
            .fold_results((vec![], vec![]), |(mut lefts, mut rights), e| {
                match e {
                    Left(l) => lefts.push(l),
                    Right(r) => rights.push(r),
                }

                (lefts, rights)
            })?;

        if devs.len() == 0 {
            let cnt = unavail_devs.len();
            for (i, e) in unavail_devs {
                warn!("\t{}: {}\n", i, e);
            }

            return if cnt != 0 {
                Err(format_err!("failed to discover compatible devices: no compatible monitors were found (of {}). Do your monitors support ddc?", cnt))
            } else {
                Err(format_err!("failed to query any devices: is the i2c-dev module loaded and can your user write to /dev/i2c?"))
            };
        }

        // Pair discovered devices to matching configs.
        let mut displays = Vec::with_capacity(devs.len());
        for dev in devs {
            // earlier configs get priority
            for cfg in cfgs {
                trace!("device {} {}", dev, cfg.matcher);
                displays.push(Display {
                    device: dev,
                    cfg: cfg.clone(),
                });
                break;
            }
        }

        Ok(Displays { displays })
    }

    pub fn len(&self) -> usize {
        self.displays.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Display> {
        self.displays.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Display> {
        self.displays.iter_mut()
    }
}

pub trait BrightnessOps {
    /// Idempotently update brightness of display based on config.
    fn update_brightness(&mut self, is_daytime: bool) -> Result<()>;
}

impl BrightnessOps for Display {
    fn update_brightness(&mut self, is_daytime: bool) -> Result<()> {
        self.device.set_brightness(if is_daytime {
            self.cfg.day_brightness as f64
        } else {
            self.cfg.night_brightness as f64
        })
    }
}
