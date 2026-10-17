//! All C ABI pointers remain inside this module. Rust owns the copied result.
use super::model::{Event, Music, Track};
use std::{
    ffi::{c_char, c_void, CStr},
    ptr,
};

#[repr(C)]
struct RawEvent {
    delta: u32,
    status: u8,
    meta_type: u8,
    data: *const u8,
    size: usize,
}

unsafe extern "C" {
    fn smf_read(data: *const u8, size: usize, error: *mut c_char, error_size: usize)
        -> *mut c_void;
    fn smf_free(music: *mut c_void);
    fn smf_format(music: *const c_void) -> u8;
    fn smf_division(music: *const c_void) -> u16;
    fn smf_tracks(music: *const c_void) -> usize;
    fn smf_events(music: *const c_void, track: usize) -> usize;
    fn smf_event(music: *const c_void, track: usize, index: usize, out: *mut RawEvent);
}

struct Owned(*mut c_void);
impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle is uniquely owned and was returned by smf_read.
        unsafe { smf_free(self.0) }
    }
}

pub fn parse(bytes: &[u8]) -> Result<Music, String> {
    let mut error = [0 as c_char; 1024];
    // SAFETY: input/error slices are alive throughout the call. The shim catches exceptions.
    let handle = unsafe { smf_read(bytes.as_ptr(), bytes.len(), error.as_mut_ptr(), error.len()) };
    if handle.is_null() {
        // SAFETY: shim always terminates the error buffer.
        return Err(unsafe { CStr::from_ptr(error.as_ptr()) }
            .to_string_lossy()
            .into_owned());
    }
    let owned = Owned(handle);
    // SAFETY: indices come from shim counts; pointers remain valid until owned is dropped.
    unsafe {
        let mut tracks = Vec::new();
        for t in 0..smf_tracks(owned.0) {
            let mut events = Vec::new();
            let mut tick = 0u64;
            for i in 0..smf_events(owned.0, t) {
                let mut raw = RawEvent {
                    delta: 0,
                    status: 0,
                    meta_type: 0,
                    data: ptr::null(),
                    size: 0,
                };
                smf_event(owned.0, t, i, &mut raw);
                tick = tick
                    .checked_add(u64::from(raw.delta))
                    .ok_or("absolute tick overflow")?;
                let data = if raw.size == 0 {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(raw.data, raw.size).to_vec()
                };
                events.push(Event {
                    delta: raw.delta,
                    tick,
                    status: raw.status,
                    meta_type: raw.meta_type,
                    data,
                });
            }
            tracks.push(Track { events });
        }
        Ok(Music {
            format: smf_format(owned.0),
            division: smf_division(owned.0),
            tracks,
        })
    }
}
