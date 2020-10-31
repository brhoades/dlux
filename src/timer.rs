use std::task::{Context, Poll};

use failure::{format_err, Error};
use chrono::{DateTime, TimeZone, Utc};
use nix::sys::time::TimeSpec;
use nix::sys::timerfd::{ClockId, Expiration, TimerFd, TimerFlags, TimerSetTimeFlags};
use tokio::io::unix::AsyncFd;

pub struct AsyncTimer {
    fd: TimerFd,
    afd: AsyncFd<TimerFd>,
    set: bool,
}

impl<'a> AsyncTimer {
    pub fn new() -> Result<Self, Error> {
        let fd = TimerFd::new(ClockId::CLOCK_BOOTTIME, TimerFlags::TFD_NONBLOCK)?;
        let t = Self {
            fd,
            afd: AsyncFd::new(fd)?,
            set: false,
        };
        Ok(t)
    }

    #[allow(dead_code)]
    pub fn set<T: chrono::TimeZone>(&mut self, dt: DateTime<T>) -> Result<(), Error> {
        self.reset(dt)
    }

    pub fn reset<T: chrono::TimeZone>(&mut self, dt: DateTime<T>) -> Result<(), Error> {
        let delay = (dt.with_timezone(&Utc) - Utc::now())
            .to_std()
            .unwrap_or(std::time::Duration::new(0, 100));

        self.fd.set(
            Expiration::OneShot(TimeSpec::from(delay)),
            TimerSetTimeFlags::empty(),
        )?;
        self.set = true;

        Ok(())
    }

    pub fn new_future(&'a mut self) -> Result<TimerFuture<'a>, Error> {
        if !self.set {
            return Err(format_err!("timer must be set before waiting"));
        }

        Ok(TimerFuture { afd: &mut self.afd })
    }
}

pub struct TimerFuture<'a> {
    afd: &'a mut AsyncFd<TimerFd>,
}

impl std::future::Future for TimerFuture<'_> {
    type Output = Result<(), Error>;

    fn poll(self: std::pin::Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.afd.poll_read_ready(ctx) {
            Poll::Ready(Ok(mut res)) => {
                // consume the guard
                res.clear_ready();
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(format_err!("failed to read fd: {}", e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

// suspend-aware wait until date. See `man timerfd_create(2)`.
// this leaks fds
#[allow(dead_code)]
fn wait_until<T: TimeZone>(dt: DateTime<T>) -> Result<(), Error> {

    let delay = (dt.with_timezone(&Utc) - Utc::now())
        .to_std()
        .unwrap_or(std::time::Duration::new(0, 100));

    let timer = TimerFd::new(ClockId::CLOCK_BOOTTIME, TimerFlags::TFD_CLOEXEC)?;
    timer.set(
        Expiration::OneShot(TimeSpec::from(delay)),
        TimerSetTimeFlags::empty(),
    )?;

    Ok(timer.wait()?)
}
