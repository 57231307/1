// build.rs - 编译 proto 文件生成 Rust 代码
// 通过 tonic-build 将 notification.proto 编译为 Rust 结构体和 trait

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &["proto/notification.proto"],
            &["proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/notification.proto");
    Ok(())
}
