mod condvar;
mod mutex;
mod note;
mod once;
mod time;
/// # nsync-rs
/// A safe Rust wrapper around Google's nsync synchronization library.
/// This crate provides safe abstractions over nsync's synchronization primitives including:
///
/// Mutexes (with reader-writer support)
/// Condition variables
/// Once initialization
/// Notes (cancellable waits)
/// Counters
/// Time utilities
pub use condvar::Condvar;
pub use mutex::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use note::{Counter, Note};
pub use once::Once;
pub use time::{Duration, Time};

#[doc(hidden)]
pub mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    include!("bindings.rs");
}
