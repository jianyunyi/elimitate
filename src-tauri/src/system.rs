//! 系统信息与管理员权限

use serde::Serialize;

use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW};
use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;
use windows_sys::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE};
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

use crate::util::to_wide;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub letter: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os_version: String,
    pub is_admin: bool,
    pub drives: Vec<DriveInfo>,
}

#[tauri::command]
pub fn system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        os_version: os_version()?,
        is_admin: is_admin(),
        drives: list_drives(),
    })
}

pub fn is_admin() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

fn os_version() -> Result<String, String> {
    unsafe {
        let mut info: OSVERSIONINFOW = std::mem::zeroed();
        info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        let status = RtlGetVersion(&mut info);
        if status != 0 {
            return Err(format!("RtlGetVersion 失败: 0x{status:x}"));
        }
        let build = info.dwBuildNumber;
        let label = match (info.dwMajorVersion, info.dwMinorVersion) {
            (10, 0) if build >= 22000 => "11".to_string(),
            (10, 0) => "10".to_string(),
            (6, 3) => "8.1".to_string(),
            (6, 2) => "8".to_string(),
            (6, 1) => "7".to_string(),
            (6, 0) => "Vista".to_string(),
            (5, 1) => "XP".to_string(),
            (maj, min) => format!("{maj}.{min}"),
        };
        let sp = crate::util::wide_to_string(info.szCSDVersion.as_ptr());
        let mut out = format!("Windows {label} (Build {build})");
        if !sp.is_empty() {
            out.push_str(&format!(" {sp}"));
        }
        Ok(out)
    }
}

fn list_drives() -> Vec<DriveInfo> {
    let mut out = Vec::new();
    for c in b'A'..=b'Z' {
        let letter = format!("{}:", c as char);
        let root = format!("{}:\\", c as char);
        let rootw = to_wide(&root);
        let ty = unsafe { GetDriveTypeW(rootw.as_ptr()) };
        if ty != DRIVE_FIXED && ty != DRIVE_REMOVABLE {
            continue;
        }
        let mut free_avail: u64 = 0;
        let mut total: u64 = 0;
        let mut total_free: u64 = 0;
        let ok = unsafe { GetDiskFreeSpaceExW(rootw.as_ptr(), &mut free_avail, &mut total, &mut total_free) };
        if ok == 0 {
            continue;
        }
        out.push(DriveInfo {
            letter,
            total_bytes: total,
            free_bytes: free_avail,
        });
    }
    out
}
