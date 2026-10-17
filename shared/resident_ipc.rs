//! Link between the editor and a resident player.
//!
//! Control messages travel as one JSON value per line over two one-way named
//! pipes, so a thread blocked on receiving never stalls a send. Plugin states
//! are too large for a line and travel in a named shared-memory block that the
//! sender creates at the exact size; the line carries only a [`StateRef`].
//!
//! The editor listens and the player connects, both with the same name; the
//! editor sends [`ToPlayer`] lines and the player sends [`ToEditor`] lines.
//! When the peer closes its end, [`Receiver::receive`] returns `Ok(None)`.
use crate::host::PluginKey;
use crate::patch_identity::PatchIdentity;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::os::windows::io::FromRawHandle;

/// What the editor asks of a resident player.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToPlayer {
    /// Bring the browser for this target to the front.
    Open(Open),
    /// Open with editor-owned pane selections and an opaque song/track key.
    OpenFiltered {
        open: Open,
        context: String,
        panes: [String; 2],
        #[serde(default)]
        query: String,
        #[serde(default)]
        cursor: Option<PatchIdentity>,
    },
    /// The editor's answer to the last [`ToEditor::Selected`].
    Applied(Result<(), String>),
}

/// What a resident player tells the editor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToEditor {
    /// A patch or effect chosen in the browser, for the editor to play.
    Selected(Selected),
    /// Patch name under the cursor, independent of applied audio.
    Cursor {
        context: String,
        patch: Option<PatchIdentity>,
    },
    /// Pane labels changed, even when no sound was selected.
    Panes {
        context: String,
        panes: [String; 2],
        #[serde(default)]
        query: String,
    },
}

/// One plugin slot chosen for `target`. The player keeps the state shared
/// until the editor answers with [`ToPlayer::Applied`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selected {
    pub reference: Option<Box<crate::patch_reference::PatchReference>>,
    pub target: Open,
    pub plugin: PluginKey,
    pub preset_name: Option<String>,
    pub state: StateRef,
}

/// A browser and the part of the song it edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Open {
    Layer { track: usize, layer: u64, song: u64 },
    Patch { track: usize },
    Effect { track: usize, slot: Option<usize> },
    MasterEffect { slot: Option<usize> },
}

/// A state placed in shared memory, as written into a control message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRef {
    pub name: String,
    pub len: usize,
}

/// Keeps a shared state alive. Drop it after the peer has replied to the
/// message carrying [`SharedState::reference`]; until then the peer may still
/// be opening it.
pub struct SharedState {
    _mapping: Handle,
    pub reference: StateRef,
}

pub struct Sender {
    pipe: File,
    state_prefix: String,
    next_state: u64,
}

pub struct Receiver {
    pipe: BufReader<File>,
}

/// Pipes created by the editor, waiting for the player to connect.
pub struct Listener {
    to_player: Handle,
    to_editor: Handle,
    name: String,
}

impl Listener {
    pub fn create(name: &str) -> io::Result<Self> {
        Ok(Self {
            to_player: create_pipe(&pipe_path(name, "to-player"), PIPE_ACCESS_OUTBOUND)?,
            to_editor: create_pipe(&pipe_path(name, "to-editor"), PIPE_ACCESS_INBOUND)?,
            name: name.to_owned(),
        })
    }

    /// Blocks until the player has connected both pipes.
    pub fn accept(self) -> io::Result<(Sender, Receiver)> {
        connect_pipe(&self.to_player)?;
        connect_pipe(&self.to_editor)?;
        let sender = Sender::new(self.to_player.into_file(), &self.name, "editor");
        let receiver = Receiver::new(self.to_editor.into_file());
        Ok((sender, receiver))
    }
}

/// Connects the player to the pipes of a [`Listener`] with the same name.
pub fn connect(name: &str) -> io::Result<(Sender, Receiver)> {
    let receiver = Receiver::new(open_pipe(&pipe_path(name, "to-player"), GENERIC_READ)?);
    let sender = Sender::new(
        open_pipe(&pipe_path(name, "to-editor"), GENERIC_WRITE)?,
        name,
        "player",
    );
    Ok((sender, receiver))
}

impl Sender {
    fn new(pipe: File, name: &str, side: &str) -> Self {
        Self {
            pipe,
            state_prefix: format!("Local\\cat-plugin-player-{name}-{side}-state"),
            next_state: 0,
        }
    }

    pub fn send<T: Serialize>(&mut self, message: &T) -> io::Result<()> {
        let mut line = serde_json::to_vec(message).map_err(io::Error::other)?;
        line.push(b'\n');
        self.pipe.write_all(&line)?;
        self.pipe.flush()
    }

    /// Copies `bytes` into a new shared-memory block named after this link.
    pub fn share(&mut self, bytes: &[u8]) -> io::Result<SharedState> {
        let name = format!("{}-{}", self.state_prefix, self.next_state);
        self.next_state += 1;
        let size = bytes.len().max(1) as u64;
        let wide = wide(&name);
        let mapping = Handle::checked(unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                std::ptr::null_mut(),
                PAGE_READWRITE,
                (size >> 32) as u32,
                size as u32,
                wide.as_ptr(),
            )
        })?;
        if io::Error::last_os_error().raw_os_error() == Some(ERROR_ALREADY_EXISTS) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("shared state {name} already exists"),
            ));
        }
        let view = View::map(&mapping, FILE_MAP_WRITE, size as usize)?;
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), view.0, bytes.len()) };
        Ok(SharedState {
            _mapping: mapping,
            reference: StateRef {
                name,
                len: bytes.len(),
            },
        })
    }
}

impl Receiver {
    fn new(pipe: File) -> Self {
        Self {
            pipe: BufReader::new(pipe),
        }
    }

    /// Returns `Ok(None)` once the peer has closed its end.
    pub fn receive<T: DeserializeOwned>(&mut self) -> io::Result<Option<T>> {
        let mut line = String::new();
        if self.pipe.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        serde_json::from_str(&line)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

/// Copies a shared state out and closes it on this side.
pub fn read_state(reference: &StateRef) -> io::Result<Vec<u8>> {
    let wide = wide(&reference.name);
    let mapping = Handle::checked(unsafe { OpenFileMappingW(FILE_MAP_READ, 0, wide.as_ptr()) })?;
    let view = View::map(&mapping, FILE_MAP_READ, reference.len.max(1))?;
    let mut bytes = vec![0; reference.len];
    unsafe { std::ptr::copy_nonoverlapping(view.0, bytes.as_mut_ptr(), reference.len) };
    Ok(bytes)
}

fn pipe_path(name: &str, direction: &str) -> String {
    format!("\\\\.\\pipe\\cat-plugin-player-{name}-{direction}")
}

fn create_pipe(path: &str, access: u32) -> io::Result<Handle> {
    let wide = wide(path);
    Handle::checked(unsafe {
        CreateNamedPipeW(
            wide.as_ptr(),
            access | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            PIPE_BUFFER,
            PIPE_BUFFER,
            0,
            std::ptr::null_mut(),
        )
    })
}

fn connect_pipe(pipe: &Handle) -> io::Result<()> {
    if unsafe { ConnectNamedPipe(pipe.0, std::ptr::null_mut()) } != 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    // The client may connect between CreateNamedPipeW and ConnectNamedPipe.
    if error.raw_os_error() == Some(ERROR_PIPE_CONNECTED) {
        Ok(())
    } else {
        Err(error)
    }
}

fn open_pipe(path: &str, access: u32) -> io::Result<File> {
    let wide = wide(path);
    let handle = Handle::checked(unsafe {
        CreateFileW(
            wide.as_ptr(),
            access,
            0,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    })?;
    Ok(handle.into_file())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

struct Handle(*mut c_void);

// Kernel handles may be used and closed from any thread.
unsafe impl Send for Handle {}

impl Handle {
    fn checked(raw: *mut c_void) -> io::Result<Self> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }

    fn into_file(self) -> File {
        let file = unsafe { File::from_raw_handle(self.0) };
        std::mem::forget(self);
        file
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

struct View(*mut u8);

impl View {
    fn map(mapping: &Handle, access: u32, len: usize) -> io::Result<Self> {
        let address = unsafe { MapViewOfFile(mapping.0, access, 0, 0, len) };
        if address.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(address.cast()))
        }
    }
}

impl Drop for View {
    fn drop(&mut self) {
        unsafe { UnmapViewOfFile(self.0.cast()) };
    }
}

const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
const PIPE_ACCESS_INBOUND: u32 = 0x1;
const PIPE_ACCESS_OUTBOUND: u32 = 0x2;
const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
const PIPE_TYPE_BYTE: u32 = 0x0;
const PIPE_READMODE_BYTE: u32 = 0x0;
const PIPE_WAIT: u32 = 0x0;
const PIPE_REJECT_REMOTE_CLIENTS: u32 = 0x8;
const PIPE_BUFFER: u32 = 64 * 1024;
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const OPEN_EXISTING: u32 = 3;
const PAGE_READWRITE: u32 = 0x04;
const FILE_MAP_WRITE: u32 = 0x0002;
const FILE_MAP_READ: u32 = 0x0004;
const ERROR_ALREADY_EXISTS: i32 = 183;
const ERROR_PIPE_CONNECTED: i32 = 535;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateNamedPipeW(
        name: *const u16,
        open_mode: u32,
        pipe_mode: u32,
        max_instances: u32,
        out_buffer: u32,
        in_buffer: u32,
        timeout: u32,
        security: *mut c_void,
    ) -> *mut c_void;
    fn ConnectNamedPipe(pipe: *mut c_void, overlapped: *mut c_void) -> i32;
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *mut c_void,
        disposition: u32,
        flags: u32,
        template: *mut c_void,
    ) -> *mut c_void;
    fn CreateFileMappingW(
        file: *mut c_void,
        security: *mut c_void,
        protect: u32,
        size_high: u32,
        size_low: u32,
        name: *const u16,
    ) -> *mut c_void;
    fn OpenFileMappingW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
    fn MapViewOfFile(
        mapping: *mut c_void,
        access: u32,
        offset_high: u32,
        offset_low: u32,
        len: usize,
    ) -> *mut c_void;
    fn UnmapViewOfFile(address: *const c_void) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

#[cfg(test)]
#[path = "resident_ipc_tests.rs"]
mod tests;
