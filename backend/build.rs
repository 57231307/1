//! 编译期版本注入（任务 #116：修复"系统更新判断当前是否最新"的版本比较假阴性）。
//!
//! release tag 采用四段 CalVer `YYYY.M.D.HHMM`（由 `.github/workflows/ci-cd.yml` 版本 step
//! 产生），而 `Cargo.toml` 受 semver 三段约束被折叠为 `YYYY.MD.HHMM`（月日并入第二段）。
//! 后端 `get_current_version()` 若直接读 `CARGO_PKG_VERSION` 得到三段，与 GitHub latest 的四段
//! tag 逐段数值比较会误判"已最新"（假阴性）。
//!
//! 此 build script 在构建时读取 CI 注入的 `BINGXI_RELEASE_VERSION`（四段，与 tag 同格式），
//! 通过 `cargo:rustc-env` 写入二进制，使 `option_env!("BINGXI_RELEASE_VERSION")` 成为权威
//! 四段版本来源。未注入（本地开发/历史二进制）时该 env 不存在，运行期回退三段
//! `CARGO_PKG_VERSION` 并触发显式告警 + MD 反解兜底（见 `system_update_service` 归一逻辑）。

fn main() {
    // 始终声明对注入变量的依赖：无值（本地）也需在文档中显式登记，确保有值时正确重编译。
    println!("cargo:rerun-if-env-changed=BINGXI_RELEASE_VERSION");

    // 仅当 CI 显式注入且非空时才转发，避免写入空字符串使 option_env! 误判为 Some("")。
    if let Some(v) = std::env::var("BINGXI_RELEASE_VERSION")
        .ok()
        .filter(|s| !s.is_empty())
    {
        println!("cargo:rustc-env=BINGXI_RELEASE_VERSION={}", v);
    }
}
