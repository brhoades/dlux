use std::convert::TryFrom;

use crate::{config::DeviceConfig, logging::*, prelude::*, types::*};

/// Display is a i2c device paired with its configuration.
pub struct Display<'a> {
    device: Device,
    cfg: &'a DeviceConfig,
}

pub struct Displays<'a> {
    displays: Vec<Display<'a>>,
}

impl<'a> Display<'a> {
    pub fn display_info(&mut self) -> Result<DeviceInfo> {
        self.device.display_info()
    }
}

impl<'a> std::fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.device.fmt(f)
    }
}

impl<'a> Displays<'a> {
    /// Create a new set of displays from device configs, matching up
    /// displays to their appropriate configuration. Unmatched displays
    /// will be discarded.
    pub fn new<C: IntoIterator<Item = &'a DeviceConfig>>(cfgs: C) -> Result<Self> {
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
        let cfgs: Vec<_> = cfgs.into_iter().collect();
        for mut dev in devs {
            // earlier configs get priority
            for cfg in &cfgs {
                let info = dev.display_info()?;

                if cfg.matcher.matches(&info) {
                    displays.push(Display { device: dev, cfg });
                    break;
                }
            }
        }

        Ok(Displays { displays })
    }

    pub fn len(&self) -> usize {
        self.displays.len()
    }

    pub fn iter<'b>(&'b self) -> impl Iterator<Item = &'b Display<'a>> {
        self.displays.iter()
    }

    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = &'b mut Display<'a>> {
        self.displays.iter_mut()
    }
}

pub trait BrightnessOps {
    /// Idempotently update brightness of display based on config.
    fn update_brightness(&mut self, is_daytime: bool) -> Result<()>;
}

impl<'a> BrightnessOps for Display<'a> {
    fn update_brightness(&mut self, is_daytime: bool) -> Result<()> {
        self.device.set_brightness(if is_daytime {
            self.cfg.day_brightness as f64
        } else {
            self.cfg.night_brightness as f64
        })
    }
}
