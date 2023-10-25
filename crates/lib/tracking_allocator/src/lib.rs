use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

pub fn run_guarded<F>(f: F)
where
    F: FnOnce(),
{
    thread_local! {
        static GUARD: Cell<bool> = Cell::new(false);
    }

    GUARD.with(|guard| {
        if !guard.replace(true) {
            f();
            guard.set(false)
        }
    })
}

pub struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        run_guarded(|| {
            eprintln!("Allocated {} bytes", layout.size());
        });
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		run_guarded(|| {
            eprintln!("Deallocated {} bytes", layout.size());
        });
        System.dealloc(ptr, layout)
    }
}
