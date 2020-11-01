mod alarm;

use std::convert::TryInto;

use alarm::Alarm;
use chrono::{DateTime, Duration, Local, Utc};
use ddc::Ddc;
use failure::{format_err, Error};
use humantime::format_duration;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opts {
    #[structopt(flatten)]
    devices: DeviceOpts,

    #[structopt(flatten)]
    geo: GeoOpts,

    /// out of 100, the brightness to target for the screen
    #[structopt(short, long)]
    brightness: u16,
    // fade in/out time at sunrise
    // #[structopt(short, long)]
    // fade_time: chrono::Duration
}

#[derive(StructOpt, Debug)]
struct GeoOpts {
    #[structopt(long)]
    lat: f64,

    #[structopt(long)]
    long: f64,

    #[structopt(long, default_value = "0.0")]
    height: f64,
}

#[derive(StructOpt, Debug)]
struct DeviceOpts {
    #[structopt(long)]
    model: Vec<String>,

    #[structopt(long)]
    all: bool,
}

#[derive(Debug)]
struct GeoSettings {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = Opts::from_args();
    let mut disps = Displays::new().unwrap();
    let mut alarm = Alarm::new()?;

    println!("found {} devices, beginning loop", disps.len());

    loop {
        update_monitors_from_time(&mut disps, &opts);

        let next_dt = get_next_event::<Local>(&opts.geo, Local::now());
        alarm.reset(next_dt)?;
        println!(
            "Waiting {} until {}",
            // round down
            format_duration(std::time::Duration::from_secs(
                (next_dt - Utc::now()).num_seconds().try_into().unwrap()
            )),
            next_dt.with_timezone(&Local)
        );

        alarm.future()?.await?;
        println!("awake, time is now: {}", Local::now());
    }
}

type Devs = Vec<ddc_i2c::I2cDdc<i2c_linux::I2c<std::fs::File>>>;

struct Displays {
    pub devs: Devs,
}

impl Displays {
    fn new() -> Result<Self, Error> {
        let devs = ddc_i2c::I2cDeviceEnumerator::new()?
            .filter_map(|mut i| {
                if i.get_vcp_feature(0x10).is_ok() {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Devs>();

        if devs.len() == 0 {
            Err(format_err!("failed to retrieve supported devices"))
        } else {
            Ok(Displays { devs })
        }
    }

    pub fn len(&self) -> usize {
        self.devs.len()
    }

    // Idempotently set brightness to the passed value..
    pub fn set_brightness(&mut self, b: u16) {
        for d in &mut self.devs {
            match d.set_vcp_feature(0x10, b) {
                Ok(_) => (),
                Err(e) => println!("failed to set to {}: {}", b, e),
            }
        }
    }
}

// A bit delicate: we need to check in local timezone so our dates are correct.
// Tomorrow in UTC != tomorrow Local.
fn get_next_event<T: chrono::TimeZone>(opts: &GeoOpts, now: chrono::DateTime<T>) -> DateTime<Utc> {
    let today = now.with_timezone(&Local);
    let geo = get_start_stop_at_date(opts, today.date());

    let next = if now >= geo.end {
        let tomorrow = today + Duration::days(1);
        get_start_stop_at_date(opts, tomorrow.date()).start
    } else if now < geo.end {
        geo.end
    } else {
        geo.start
    };
    next + Duration::milliseconds(100)
}

fn get_start_stop_at_date<T: chrono::TimeZone>(
    geo: &GeoOpts,
    date: chrono::Date<T>,
) -> GeoSettings {
    let (start, end) =
        sun_times::sun_times(date.with_timezone(&Utc), geo.lat, geo.long, geo.height);
    GeoSettings { start, end }
}

fn update_monitors_from_time(disps: &mut Displays, opts: &Opts) {
    let now = Local::now();
    let geo = get_start_stop_at_date(&opts.geo, now.date());

    let b = if now < geo.start || now > geo.end {
        opts.brightness
    } else {
        100
    };

    println!("updating brightness to {}", b);
    disps.set_brightness(b);
}
