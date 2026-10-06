//! State bridge. All operations run on the UI thread with audio stopped.
use super::{Host, UhHost, UH_OK};
use std::ffi::{c_char, c_void, CStr};

extern "C" {
    fn uh_state_save(
        host: *mut UhHost,
        id: i32,
        receive: unsafe extern "C" fn(*mut c_void, *const u8, u64),
        user: *mut c_void,
        error: *mut c_char,
        error_size: i32,
    ) -> i32;
    fn uh_state_load(
        host: *mut UhHost,
        id: i32,
        data: *const u8,
        size: u64,
        error: *mut c_char,
        error_size: i32,
    ) -> i32;
}

unsafe extern "C" fn receive(user: *mut c_void, data: *const u8, size: u64) {
    let bytes = &mut *(user as *mut Vec<u8>);
    if size > 0 {
        bytes.extend_from_slice(std::slice::from_raw_parts(data, size as usize));
    }
}

fn result(code: i32, error: &[c_char]) -> Result<(), String> {
    if code == UH_OK {
        Ok(())
    } else {
        let message = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy();
        Err(format!("{message} (code {code})"))
    }
}

impl Host {
    /// Caller must stop audio rendering until this operation returns.
    pub fn save_state(&self, id: i32) -> Result<Vec<u8>, String> {
        let mut data = Vec::new();
        let mut error = [0; 2048];
        let code = unsafe {
            uh_state_save(
                self.raw,
                id,
                receive,
                &mut data as *mut Vec<u8> as *mut c_void,
                error.as_mut_ptr(),
                error.len() as i32,
            )
        };
        result(code, &error)?;
        Ok(data)
    }

    /// Caller must stop audio rendering until this operation returns.
    pub fn load_state(&self, id: i32, data: &[u8]) -> Result<(), String> {
        let mut error = [0; 2048];
        let code = unsafe {
            uh_state_load(
                self.raw,
                id,
                data.as_ptr(),
                data.len() as u64,
                error.as_mut_ptr(),
                error.len() as i32,
            )
        };
        result(code, &error)
    }
}
