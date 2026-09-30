//! A process tree started in a Job dies when the job handle is dropped, which is what happens
//! to the sidecars when the app exits or crashes.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use desktop_lib::job::Job;
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
};

/// Pid of a direct child of `parent` named `exe` (case-insensitive), if one exists now.
fn child_named(parent: u32, exe: &str) -> Option<u32> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        assert_ne!(snapshot, INVALID_HANDLE_VALUE);
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = None;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if entry.th32ParentProcessID == parent && name.eq_ignore_ascii_case(exe) {
                found = Some(entry.th32ProcessID);
                break;
            }
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        found
    }
}

struct Process(HANDLE);

impl Process {
    fn open(pid: u32) -> Process {
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        assert!(!handle.is_null(), "open process {pid}");
        Process(handle)
    }

    /// True if the process exits within `ms`.
    fn exits_within(&self, ms: u32) -> bool {
        match unsafe { WaitForSingleObject(self.0, ms) } {
            WAIT_OBJECT_0 => true,
            WAIT_TIMEOUT => false,
            other => panic!("WaitForSingleObject returned {other}"),
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

#[test]
fn dropping_the_job_kills_the_child_and_its_children() {
    let job = Job::new().unwrap();
    let mut command = Command::new("cmd");
    command
        .args(["/c", "ping -n 30 127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut cmd = job.spawn(&mut command).unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    let ping_pid = loop {
        if let Some(pid) = child_named(cmd.id(), "PING.EXE") {
            break pid;
        }
        assert!(Instant::now() < deadline, "cmd never started ping");
        std::thread::sleep(Duration::from_millis(50));
    };
    // Held open across the kill, so the pid cannot be reused under us.
    let ping = Process::open(ping_pid);
    assert!(
        !ping.exits_within(500),
        "ping should still be running while the job is alive"
    );
    assert!(cmd.try_wait().unwrap().is_none());

    drop(job);

    assert!(ping.exits_within(5000), "grandchild survived the job");
    let deadline = Instant::now() + Duration::from_secs(5);
    while cmd.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "child survived the job");
        std::thread::sleep(Duration::from_millis(50));
    }
}
