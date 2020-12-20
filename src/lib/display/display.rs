use std::convert::TryFrom;

use crate::{config::DeviceConfig, logging::*, prelude::*, types::*};

/// Display is a i2c device paired with its configuration.
pub struct Display {
    device: Device,
    cfg: DeviceConfig,
}

pub struct Displays {
    displays: Vec<Display>,
}

impl Display {
    pub fn display_info(&mut self) -> Result<DeviceInfo> {
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
        for mut dev in devs {
            // earlier configs get priority
            for cfg in cfgs {
                let info = dev.display_info()?;

                if cfg.matcher.matches(&info) {
                    displays.push(Display {
                        device: dev,
                        cfg: cfg.clone(),
                    });
                    break;
                }
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
