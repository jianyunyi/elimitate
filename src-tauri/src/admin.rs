//! 以管理员身份重新启动

use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::util::to_wide;

#[tauri::command]
pub fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("无法获取当前程序路径: {e}"))?
        .to_string_lossy()
        .into_owned();
    let exe_w = to_wide(&exe);
    let verb_w = to_wide("runas");
    unsafe {
        let result = ShellExecuteW(
            std::ptr::null_mut(),
            verb_w.as_ptr(),
            exe_w.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        // 返回值 <= 32 表示失败
        if result as isize <= 32 {
            return Err(format!("请求管理员权限失败（错误码 {}）", result as isize));
        }
    }
    Ok(())
}
