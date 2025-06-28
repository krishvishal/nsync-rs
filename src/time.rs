use crate::ffi;
use std::cmp::Ordering;
use std::ops::{Add, Sub};
use std::time::Duration as StdDuration;

/// A point in time
#[derive(Copy, Clone)]
pub struct Time(pub(super) ffi::nsync_time);

/// A duration of time
#[derive(Copy, Clone)]
pub struct Duration(ffi::nsync_time);
impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::nsync_time_cmp(self.0, other.0) == 0 }
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        let result = unsafe { ffi::nsync_time_cmp(self.0, other.0) };
        match result {
            x if x < 0 => Ordering::Less,
            x if x > 0 => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}

impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::nsync_time_cmp(self.0, other.0) == 0 }
    }
}

impl Eq for Duration {}

impl PartialOrd for Duration {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Duration {
    fn cmp(&self, other: &Self) -> Ordering {
        let result = unsafe { ffi::nsync_time_cmp(self.0, other.0) };
        match result {
            x if x < 0 => Ordering::Less,
            x if x > 0 => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}
impl Time {
    /// Returns the current time
    pub fn now() -> Self {
        unsafe { Time(ffi::nsync_time_now()) }
    }

    /// Creates a time that represents "no deadline" for wait operations
    pub fn no_deadline() -> Self {
        unsafe { Time(ffi::nsync_time_no_deadline) }
    }

    /// Returns the zero time
    pub fn zero() -> Self {
        unsafe { Time(ffi::nsync_time_zero) }
    }

    /// Sleeps until this time
    pub fn sleep_until(self) -> Time {
        unsafe { Time(ffi::nsync_time_sleep(self.0)) }
    }

    /// Returns the raw nsync_time
    pub(crate) fn as_raw(&self) -> ffi::nsync_time {
        self.0
    }
}

impl Duration {
    /// Creates a duration from milliseconds
    pub fn from_millis(ms: u32) -> Self {
        unsafe { Duration(ffi::nsync_time_ms(ms)) }
    }

    /// Creates a duration from microseconds
    pub fn from_micros(us: u32) -> Self {
        unsafe { Duration(ffi::nsync_time_us(us)) }
    }

    /// Creates a duration from seconds and nanoseconds
    pub fn from_secs_nanos(secs: i64, nanos: u32) -> Self {
        unsafe { Duration(ffi::nsync_time_s_ns(secs, nanos)) }
    }

    /// Sleeps for this duration
    pub fn sleep(self) {
        unsafe {
            ffi::nsync_time_sleep(self.0);
        }
    }
}

impl From<StdDuration> for Duration {
    fn from(d: StdDuration) -> Self {
        Duration::from_secs_nanos(d.as_secs() as i64, d.subsec_nanos())
    }
}

impl Add<Duration> for Time {
    type Output = Time;

    fn add(self, rhs: Duration) -> Self::Output {
        unsafe { Time(ffi::nsync_time_add(self.0, rhs.0)) }
    }
}

impl Sub<Time> for Time {
    type Output = Duration;
    fn sub(self, rhs: Time) -> Self::Output {
        unsafe { Duration(ffi::nsync_time_sub(self.0, rhs.0)) }
    }
}
