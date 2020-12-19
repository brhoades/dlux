use std::convert::TryInto;

use anyhow::{Error, Result};
use futures::future::{select, try_join_all, Either};
use log::*;
use structopt::StructOpt;
use tokio::time::sleep;

use adaptive_backoff::prelude::*;
use chrono::{DateTime, Duration, Local, Utc};
use humantime::format_duration;

use lib::{
    alarm::Alarm,
    config::{Config, GeoOpts},
    device::{BrightnessOps, Display, Displays},
};

#[derive(StructOpt, Debug)]
pub struct Opts {
    pub config: std::path::PathBuf,
}

pub async fn run(cfg: lib::config::Config) -> Result<(), Error> {
    let mut disps = Displays::new(&cfg.devices)?;
    let mut alarm = Alarm::new()?;

    info!("discovered {} monitors", disps.len());

    loop {
        update_monitors_from_time(&mut disps, &cfg).await;

        let next_dt = get_next_event::<Local>(&cfg.geo, Local::now());
        alarm.reset(next_dt)?;
        info!(
            "sleeping for {} until {}",
            // round down
            format_duration(std::time::Duration::from_secs(
                (next_dt - Utc::now()).num_seconds().try_into().unwrap()
            )),
            next_dt.with_timezone(&Local)
        );
        std::mem::forget(next_dt);

        alarm.future()?.await?;
        debug!("awake, time is now: {}", Local::now());
    }
}

// A bit delicate: we need to check in local timezone so our dates are correct.
// Tomorrow in UTC != tomorrow Local.
fn get_next_event<T: chrono::TimeZone>(opts: &GeoOpts, now: chrono::DateTime<T>) -> DateTime<Utc> {
    let today = now.with_timezone(&Local);
    let geo = get_start_stop_at_date(opts, today.date());

    let next = if now >= geo.1 {
        let tomorrow = today + Duration::days(1);
        get_start_stop_at_date(opts, tomorrow.date()).0
    } else if now < geo.1 {
        geo.1
    } else {
        geo.0
    };
    next + Duration::milliseconds(100)
}

fn get_start_stop_at_date<T: chrono::TimeZone>(
    geo: &GeoOpts,
    date: chrono::Date<T>,
) -> (DateTime<Utc>, DateTime<Utc>) {
    let (start, end) = sun_times::sun_times(
        date.with_timezone(&Utc),
        geo.latitude,
        geo.longitude,
        geo.altitude,
    );
    (start, end)
}

async fn update_monitors_from_time(disps: &mut Displays, cfg: &Config) {
    let now = Local::now();
    let geo = get_start_stop_at_date(&cfg.geo, now.date());
    let is_daytime = now < geo.0 || now > geo.1;
    info!(
        "updating brightness of all displays to {} value",
        if is_daytime { "daytime" } else { "nighttime" }
    );

    // Run all updates in parallel, retrying, and if any error bail completely.
    // When resuming from suspend, monitors may not wake up consistently and this
    // ensures they eventually are set properly.
    let set_displays = try_join_all(
        disps
            .iter_mut()
            .map(|d| retry_monitor(d, is_daytime))
            .collect::<Vec<_>>(),
    );

    match select(set_displays, sleep(std::time::Duration::from_secs(300))).await {
        Either::Left((Err(e), _)) => {
            error!("failed to set display brightness: {}", e);
            panic!(e);
        }
        Either::Left((Ok(_), _)) => {
            debug!("finished setting monitor brightness");
        }
        Either::Right(_) => {
            error!("timed out setting monitor brightness after 5 mintues");
        }
    };
}

/// retry_monitor retires setting brightness on failure indefinely. It's not expected
/// that errors should return except when dependencies fail.
async fn retry_monitor(disp: &mut Display, is_day: bool) -> Result<()> {
    let mut backoff = ExponentialBackoffBuilder::default()
        .factor(1.1)
        .max(5.0)
        .build()
        .unwrap();

    let mut tries = 1;
    loop {
        if let Err(e) = disp.update_brightness(is_day) {
            debug!(
                "failed to set brightness for {} on try {}: {}",
                disp, tries, e
            );
            let delay = backoff.wait()?;
            tries += 1;
            trace!(
                "backing off {} for {}",
                disp,
                humantime::format_duration(delay)
            );

            sleep(delay).await;
        } else {
            break;
        }
    }

    Ok(())
}
