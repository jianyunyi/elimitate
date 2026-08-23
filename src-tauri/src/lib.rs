mod admin;
mod junk;
mod large;
mod system;
mod uninstall;
mod util;

/// 应用入口：注册全部 Tauri 命令
pub fn run() {
    tauri::Builder::default()
        .manage(large::ScanState::default())
        .invoke_handler(tauri::generate_handler![
            system::system_info,
            admin::relaunch_as_admin,
            junk::scan::scan_junk,
            junk::clean::clean_junk,
            junk::clean::clean_all,
            uninstall::enumerate::list_installed_apps,
            uninstall::residue::scan_residue,
            uninstall::remove::delete_residue,
            uninstall::remove::delete_paths,
            uninstall::remove::uninstall_app,
            large::scan_large_files,
            large::cancel_large_scan,
            large::open_in_explorer,
            large::precheck_locked,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Elimitate");
}
