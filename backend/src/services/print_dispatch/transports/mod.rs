//! 打印传输的具体实现集合
//!
//! 每个实现对应 [`crate::services::print_dispatch::transport::PrintTransport`] 的一种
//! `kind` 后端：CUPS `lp` 命令行、IPP over HTTP、raw-socket 网络打印、以及经 shell
//! 执行的外部命令。工厂 [`crate::services::print_dispatch::transport::build_transport`]
//! 按目标 `kind` 选择实例化哪一个。

pub mod cups;
pub mod ipp;
pub mod network;
pub mod system_command;
