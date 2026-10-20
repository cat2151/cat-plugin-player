//! Test-only Windows IAT observation of the directories Surge actually enumerates.
//! Hooks are confined to this child test process; no plugin or dependency file changes.
use std::{ffi::c_void, sync::Mutex};

type Module = *mut c_void;
type Load = unsafe extern "system" fn(*const u16) -> Module;
type Find = unsafe extern "system" fn(*const u16, i32, Module, i32, Module, u32) -> Module;

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Module;
    fn VirtualProtect(address: Module, size: usize, flags: u32, old: *mut u32) -> i32;
    fn GetLastError() -> u32;
    fn SetLastError(error: u32);
}

struct Hook {
    module: usize,
    slot: usize,
    original: usize,
    replacement: usize,
}
static HOOKS: Mutex<Vec<Hook>> = Mutex::new(Vec::new());
static PATHS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LOAD: Mutex<usize> = Mutex::new(0);
static FIND: Mutex<usize> = Mutex::new(0);

unsafe fn wide(ptr: *const u16) -> String {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

unsafe fn write(slot: usize, value: usize) {
    let mut flags = 0;
    assert_ne!(VirtualProtect(slot as Module, 8, 4, &mut flags), 0);
    *(slot as *mut usize) = value;
    assert_ne!(VirtualProtect(slot as Module, 8, flags, &mut flags), 0);
}

// PE32+ import descriptors and thunks, as specified by the Windows PE format.
unsafe fn hook(module: Module, name: &[u8], replacement: usize) -> Option<usize> {
    let base = module as usize;
    let pe = base + *((base + 0x3c) as *const u32) as usize;
    assert_eq!(*((pe + 24) as *const u16), 0x20b);
    let rva = *((pe + 24 + 120) as *const u32) as usize;
    let mut desc = base + rva;
    while *((desc + 12) as *const u32) != 0 {
        let mut names = base + *(desc as *const u32) as usize;
        let mut slot = base + *((desc + 16) as *const u32) as usize;
        while *(names as *const u64) != 0 {
            let rva = *(names as *const u64);
            if rva >> 63 == 0 {
                let text = std::ffi::CStr::from_ptr((base + rva as usize + 2) as _);
                if text.to_bytes() == name {
                    let original = *(slot as *const usize);
                    if original == replacement {
                        return None;
                    }
                    let mut hooks = HOOKS.lock().unwrap();
                    hooks.retain(|h| h.slot != slot);
                    hooks.push(Hook {
                        module: base,
                        slot,
                        original,
                        replacement,
                    });
                    write(slot, replacement);
                    return Some(original);
                }
            }
            names += 8;
            slot += 8;
        }
        desc += 20;
    }
    None
}

unsafe extern "system" fn load(path: *const u16) -> Module {
    let real: Load = std::mem::transmute(*LOAD.lock().unwrap());
    let module = real(path);
    let error = GetLastError();
    let path = wide(path);
    if !module.is_null() && path.contains("Surge XT") {
        if let Some(original) = hook(module, b"FindFirstFileExW", find as *const () as usize) {
            *FIND.lock().unwrap() = original;
        }
    }
    SetLastError(error);
    module
}

unsafe extern "system" fn find(
    path: *const u16,
    level: i32,
    data: Module,
    search: i32,
    filter: Module,
    flags: u32,
) -> Module {
    let real: Find = std::mem::transmute(*FIND.lock().unwrap());
    let result = real(path, level, data, search, filter, flags);
    let error = GetLastError();
    if result as isize != -1 {
        PATHS.lock().unwrap().push(wide(path).replace('\\', "/"));
    }
    SetLastError(error);
    result
}

pub(super) struct CatalogProbe;
impl CatalogProbe {
    pub fn start() -> Self {
        PATHS.lock().unwrap().clear();
        let name: Vec<_> = "uapmd_shim.dll".encode_utf16().chain(Some(0)).collect();
        unsafe {
            let module = GetModuleHandleW(name.as_ptr());
            assert!(!module.is_null());
            *LOAD.lock().unwrap() = hook(module, b"LoadLibraryW", load as *const () as usize)
                .expect("shim must import LoadLibraryW");
        }
        Self
    }

    pub fn paths(&self) -> Vec<String> {
        PATHS.lock().unwrap().clone()
    }

    pub fn assert_normal(&self) {
        let paths = self.paths();
        for required in [
            "/patches_factory/Basses/",
            "/wavetables/",
            "/fx_presets/Airwindows/",
        ] {
            assert!(
                paths
                    .iter()
                    .any(|p| p.contains(required) && !p.contains("surge-data-min")),
                "Surge did not enumerate normal {required}: {paths:?}"
            );
        }
        assert!(
            !paths.iter().any(|p| p.contains("surge-data-min")),
            "{paths:?}"
        );
    }
}

impl Drop for CatalogProbe {
    fn drop(&mut self) {
        // Instances can unload/reload between observations. Restore only live slots
        // whose current value is still our hook (VirtualQuery avoids stale memory).
        #[repr(C)]
        struct Region {
            base: Module,
            allocation: Module,
            protection: u32,
            partition: u16,
            size: usize,
            state: u32,
            protect: u32,
            kind: u32,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn VirtualQuery(ptr: Module, info: *mut Region, len: usize) -> usize;
        }
        for h in HOOKS.lock().unwrap().drain(..).rev() {
            unsafe {
                let mut region: Region = std::mem::zeroed();
                if VirtualQuery(h.slot as Module, &mut region, std::mem::size_of::<Region>()) != 0
                    && region.state == 0x1000
                    && region.allocation as usize == h.module
                    && *(h.slot as *const usize) == h.replacement
                {
                    write(h.slot, h.original);
                }
            }
        }
    }
}
