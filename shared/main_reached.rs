//! main に到達した瞬間を、process 作成からの経過時間つきで標準出力と `startup.log` へ出す。
//! main より前（exe と DLL の読み込み）で待たされているかを、体感と突き合わせるため。
use std::io::Write;
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct FileTime {
    low: u32,
    high: u32,
}

impl FileTime {
    fn ticks(self) -> u64 {
        (u64::from(self.high) << 32) | u64::from(self.low)
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> isize;
    fn GetProcessTimes(
        process: isize,
        creation: *mut FileTime,
        exit: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
    fn GetSystemTimePreciseAsFileTime(now: *mut FileTime);
}

/// process 作成から今までの経過ミリ秒。
pub(crate) fn since_process_creation_ms() -> Option<f64> {
    let [mut creation, mut exit, mut kernel, mut user, mut now] = [FileTime::default(); 5];
    // SAFETY: すべての出力先は有効な FileTime を指す。疑似 handle は閉じなくてよい。
    let ok = unsafe {
        let ok = GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        );
        GetSystemTimePreciseAsFileTime(&mut now);
        ok
    };
    (ok != 0).then(|| now.ticks().saturating_sub(creation.ticks()) as f64 / 10_000.0)
}

fn append(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(
        file,
        "[{}.{:03} UTC unix] {line}",
        time.as_secs(),
        time.subsec_millis()
    )
}

/// `log` は書き込み先の `startup.log`。取得できなかったときはその理由。
pub(crate) fn report(since_process_creation_ms: Option<f64>, log: Result<PathBuf, String>) {
    let line = match since_process_creation_ms {
        Some(ms) => format!("main到達: process作成から {ms:.1} ms"),
        None => "main到達: process作成時刻を取得できません".to_owned(),
    };
    println!("{line}");
    let path = match log {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Cannot write startup.log: {error}");
            return;
        }
    };
    if let Err(error) = append(&path, &line) {
        eprintln!("Cannot write {}: {error}", path.display());
    }
}
