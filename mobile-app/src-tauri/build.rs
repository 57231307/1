fn main() {
    // 生成 app 命令的 ACL 权限（allow-*）。远端 ERP 页面属于非本地 origin，
    // 一旦存在 app 权限清单，app 命令就必须在 capabilities 里逐条授权。
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "app_info",
            "remember_gateway",
            "reopen_gateway",
            "bluetooth_request_permission",
            "bluetooth_open_settings",
            "bluetooth_list_printers",
            "bluetooth_connect",
            "bluetooth_print_text",
            "bluetooth_disconnect",
        ]),
    ))
    .expect("failed to run tauri-build");
}
