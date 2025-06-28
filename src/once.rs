use crate::ffi;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

/// A synchronization primitive which can be used to run a one-time global
/// initialization.
#[derive(Default)]
pub struct Once {
    inner: UnsafeCell<ffi::nsync_once>,
    done: AtomicBool,
}

unsafe impl Send for Once {}
unsafe impl Sync for Once {}

impl Once {
    pub const fn new() -> Once {
        Once {
            inner: UnsafeCell::new(0),
            done: AtomicBool::new(false),
        }
    }

    /// Performs an initialization routine idempotently.
    pub fn call_once<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if self.done.load(Ordering::Acquire) {
            return;
        }

        self.call_once_slow(f);
    }

    #[cold]
    fn call_once_slow<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        struct Closure<F: FnOnce()> {
            func: Option<F>,
        }
        unsafe extern "C" fn run_closure<F: FnOnce()>(p: *mut std::os::raw::c_void) {
            let closure = unsafe { &mut *(p as *mut Closure<F>) };
            let func = closure.func.take().unwrap();
            func();
        }
        let mut closure = Closure { func: Some(f) };
        unsafe {
            ffi::nsync_run_once_arg(
                self.inner.get(),
                Some(run_closure::<F>),
                &mut closure as *mut _ as *mut std::os::raw::c_void,
            );
        }

        self.done.store(true, Ordering::Release);
    }

    /// Returns `true` if some `call_once` call has completed successfully.
    pub fn is_completed(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }
}
