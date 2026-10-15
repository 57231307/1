// Desktop 入口：与 Android 共用 lib::run()（本机无 Android 工具链时可 `cargo tauri dev` 预览壳行为）。
// Android 构建走 lib.rs 的 mobile_entry_point，本 bin 不参与移动端编译。
fn main() {
    bingxi_mobile_lib::run();
}
