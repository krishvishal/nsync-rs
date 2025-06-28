use crate::ffi;
use crate::time::Time;
use std::ptr::NonNull;

/// A note is a notification primitive that can be used to cancel waits
pub struct Note {
    ptr: NonNull<ffi::nsync_note_s_>,
}

unsafe impl Send for Note {}
unsafe impl Sync for Note {}

impl Note {
    /// Creates a new note with an optional parent and deadline
    pub fn new(parent: Option<&Note>, deadline: Time) -> Self {
        let parent_ptr = parent
            .map(|p| p.ptr.as_ptr())
            .unwrap_or(std::ptr::null_mut());
        let ptr = unsafe { ffi::nsync_note_new(parent_ptr, deadline.as_raw()) };
        Note {
            ptr: NonNull::new(ptr).expect("nsync_note_new returned null"),
        }
    }

    /// Notifies this note
    pub fn notify(&self) {
        unsafe { ffi::nsync_note_notify(self.ptr.as_ptr()) }
    }

    /// Checks if this note has been notified
    pub fn is_notified(&self) -> bool {
        unsafe { ffi::nsync_note_is_notified(self.ptr.as_ptr()) != 0 }
    }

    /// Waits for this note to be notified or until the deadline
    pub fn wait(&self, deadline: Time) -> bool {
        unsafe { ffi::nsync_note_wait(self.ptr.as_ptr(), deadline.as_raw()) == 0 }
    }

    /// Returns the expiry time of this note
    pub fn expiry(&self) -> Time {
        unsafe { Time(ffi::nsync_note_expiry(self.ptr.as_ptr())) }
    }
}

impl Drop for Note {
    fn drop(&mut self) {
        unsafe { ffi::nsync_note_free(self.ptr.as_ptr()) }
    }
}

/// A counter that can be waited on to reach zero
/// A counter that can be waited on to reach zero
pub struct Counter {
    ptr: NonNull<ffi::nsync_counter_s_>,
}

unsafe impl Send for Counter {}
unsafe impl Sync for Counter {}

impl Counter {
    /// Creates a new counter with the given initial value
    pub fn new(value: u32) -> Self {
        let ptr = unsafe { ffi::nsync_counter_new(value) };
        Counter {
            ptr: NonNull::new(ptr).expect("nsync_counter_new returned null"),
        }
    }
    /// Adds delta to the counter and returns the new value
    pub fn add(&self, delta: i32) -> u32 {
        unsafe { ffi::nsync_counter_add(self.ptr.as_ptr(), delta) }
    }

    /// Returns the current value of the counter
    pub fn value(&self) -> u32 {
        unsafe { ffi::nsync_counter_value(self.ptr.as_ptr()) }
    }

    /// Waits until the counter reaches zero or the deadline expires
    pub fn wait(&self, deadline: Time) -> u32 {
        unsafe { ffi::nsync_counter_wait(self.ptr.as_ptr(), deadline.as_raw()) }
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        unsafe { ffi::nsync_counter_free(self.ptr.as_ptr()) }
    }
}
