#![no_std]
#![doc = "Negative fixture: a functional core with ambient mutable state."]

use core::sync::atomic::AtomicUsize;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
static mut LEGACY_NEXT_ID: usize = 0;

/// Keeps both violating statics reachable in this fixture.
pub fn ambient_state_exists() -> bool {
    NEXT_ID.load(core::sync::atomic::Ordering::Relaxed) == 0
        && core::ptr::addr_of!(LEGACY_NEXT_ID).is_null()
}
