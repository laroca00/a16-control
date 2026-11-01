//! Elevation helpers. GiENEMY always runs elevated, started by its logon task.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject, INFINITE,
};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

use crate::setup::TASK_NAME;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Absolute path of a Windows tool, e.g. `system_tool("schtasks.exe")`.
/// A bare name would be searched in our own folder first, and this process
/// runs elevated.
pub fn system_tool(relative: &str) -> PathBuf {
    let mut buf = [0u16; 260];
    let len = unsafe { GetSystemDirectoryW(Some(&mut buf)) } as usize;
    let dir = if len > 0 && len < buf.len() {
        PathBuf::from(String::from_utf16_lossy(&buf[..len]))
    } else {
        PathBuf::from(r"C:\Windows\System32")
    };
    dir.join(relative)
}

/// A System32 tool that runs without flashing a console window.
pub fn tool(relative: &str) -> Command {
    let mut cmd = Command::new(system_tool(relative));
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[derive(Debug, PartialEq, Eq)]
pub enum StartupAction {
    Cli,
    RelaunchViaTask,
    RunApp { needs_setup: bool },
}

pub fn startup_action(elevated: bool, task_exists: bool, has_cli_args: bool) -> StartupAction {
    if has_cli_args {
        StartupAction::Cli
    } else if !elevated && task_exists {
        StartupAction::RelaunchViaTask
    } else {
        StartupAction::RunApp { needs_setup: !task_exists }
    }
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut info = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut info as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && info.TokenIsElevated != 0
    }
}

/// Re-runs this exe with `args` through a UAC prompt and waits for it.
pub fn run_elevated_and_wait(args: &str) -> Result<i32, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let file = HSTRING::from(exe.as_os_str());
    let params = HSTRING::from(args);
    let verb = HSTRING::from("runas");
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    unsafe {
        // Fails when the user declines the UAC prompt.
        ShellExecuteExW(&mut info).map_err(|e| e.message())?;
        if info.hProcess.is_invalid() {
            return Err("no process handle".into());
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 1u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        Ok(code as i32)
    }
}

pub fn launch_via_task() -> Result<(), String> {
    let out = tool("schtasks.exe").args(["/run", "/tn", TASK_NAME])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_tools_resolve_inside_system32() {
        let s = system_tool("schtasks.exe").display().to_string().to_lowercase();
        assert!(s.ends_with(r"\system32\schtasks.exe"), "{s}");
        assert!(std::path::Path::new(&s).is_absolute());
        let m = system_tool(r"wbem\mofcomp.exe").display().to_string().to_lowercase();
        assert!(m.ends_with(r"\system32\wbem\mofcomp.exe"), "{m}");
    }

    #[test]
    fn startup_action_cli_wins() {
        assert_eq!(startup_action(false, false, true), StartupAction::Cli);
    }

    #[test]
    fn startup_action_relaunches_via_task() {
        assert_eq!(startup_action(false, true, false), StartupAction::RelaunchViaTask);
    }

    #[test]
    fn startup_action_needs_setup_without_task() {
        assert_eq!(startup_action(false, false, false), StartupAction::RunApp { needs_setup: true });
    }

    #[test]
    fn startup_action_runs_when_elevated() {
        assert_eq!(startup_action(true, true, false), StartupAction::RunApp { needs_setup: false });
    }
}
