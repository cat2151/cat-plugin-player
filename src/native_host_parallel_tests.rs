//! Keep the parallel-host contract independent of process-lifetime device discovery.
use std::sync::{Arc, Barrier};

#[test]
fn separate_threads_can_create_and_destroy_hosts_concurrently() {
    if super::process::isolated() {
        return;
    }
    let _library = super::load().unwrap();
    let gate = Arc::new(Barrier::new(4));
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let gate = gate.clone();
                scope.spawn(move || {
                    extern "C" fn wake(_: *mut std::ffi::c_void) {}
                    gate.wait();
                    // No callbacks retain a context, and each thread owns its host.
                    let host = unsafe { crate::ffi::Host::new(wake, std::ptr::null_mut()) };
                    // Every thread reaches the barrier even if construction failed.
                    gate.wait();
                    drop(host.expect("parallel native host construction"));
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
