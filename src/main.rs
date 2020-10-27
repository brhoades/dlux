use failure::Error;
use structopt::StructOpt;
use chrono::{DateTime, Utc, Duration, Local};

use ddc::Ddc;

#[derive(StructOpt, Debug)]
struct Opts {
    #[structopt(flatten)]
    devices: DeviceOpts,

    #[structopt(flatten)]
    geo: GeoOpts,

    /// out of 100, the brightness to target for the screen
    #[structopt(short, long)]
    brightness: u16

    // /// fade in/out time at sunrise
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

fn get_start_stop(geo: &GeoOpts) -> GeoSettings {
    let (start, end) = sun_times::sun_times(Utc::now().date(), geo.lat, geo.long, geo.height);
    GeoSettings{
        start,
        end,
    }
}

fn main() -> Result<(), Error> {
    let opts = Opts::from_args();
    let mut last_date = Utc::now().date();
    let mut geo = get_start_stop(&opts.geo);
    let mut devs = ddc_i2c::I2cDeviceEnumerator::new()?.map(|mut i| {
        match i.get_vcp_feature(0x10) {
            Ok(_) => Some(i),
            Err(_) => None,
        }
    })
        .filter(Option::is_some)
        .map(Option::unwrap)
        .collect::<Vec<_>>();

    println!(
        "sunrise at {}, sunset at {}",
        geo.start.with_timezone(&Local),
        geo.end.with_timezone(&Local),
    );
    println!("found {} devices, beginning loop", devs.len());

    loop {
        let now = Utc::now();
        if last_date != now.date() {
            last_date = now.date();
            geo = get_start_stop(&opts.geo);
            println!(
                "sunrise at {}, sunset at {}",
                geo.start.with_timezone(&Local),
                geo.end.with_timezone(&Local),
            );
        }

        for d in &mut devs {
            let cap = match d.get_vcp_feature(0x10) {
                Ok(cap) => cap,
                Err(e) => {
                    println!("failed to query monitor: {}", e);
                    continue;
                }
            };

            let b = if now < geo.start || now > geo.end {
                opts.brightness
            } else {
                100
            };

            if cap.value() != b {
                match d.set_vcp_feature(0x10, b) {
                    Ok(_) => println!("set brightness to {}", b),
                    Err(e) => println!("failed to set to {}: {}", b, e),
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(60 * 1_000));
    }
}

