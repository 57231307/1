/// Tauri v2 Android 客户端 Rust 内核。
///
/// 形态：WebView 载入远端 Bingxi ERP Web（同域复用现有 httpOnly Cookie + CSRF 会话，
/// 故 Web 端全部业务功能在 App 内零改动继承）。Rust 层只做两件事：
/// 1) 承载窗口生命周期与安全配置；
/// 2) 暴露原生能力桥接（扫码/相机、通知、打印机目标、文件导出落盘等），由前端经
///    `window.__TAURI__` 调用（需 tauri.conf 开 `withGlobalTauri`）。
///
/// 当前为最小可构建骨架：仅注册一个 `app_info` 探针命令验证 IPC 通路。
/// 各原生能力命令按需在本模块追加，并在 capabilities 里放行对应权限。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("Bingxi ERP 移动端启动失败");
}

/// IPC 探针：确认 WebView↔Rust 通路可用，供前端识别"运行在原生壳内"。
#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({
        "app": "bingxi-erp-mobile",
        "shell": "tauri-v2",
    })
}
