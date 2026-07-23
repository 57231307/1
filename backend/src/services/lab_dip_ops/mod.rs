//! 化验室打样业务实现子模块（lab_dip_ops）
//!
//! 批次 D10 拆分：从原 `lab_dip_service.rs` 拆出 3 个 Service impl 块。
//! - `request`：LabDipRequestService impl（打样通知单 CRUD + 状态流转 + 校验）
//! - `sample`：LabDipSampleService impl（打样小样 CRUD + 对色结果记录 + ABCD 多版样管理）
//! - `resample`：LabDipResampleService impl（复样记录 CRUD + 复样结果判定 + 染色技术卡开具）
//! - `types`：9 个 DTO struct（请求体 / 查询参数）
//!
//! 3 个 Service struct 定义与 `new` 构造函数、纯函数（单号生成 / 版本标识生成 / 状态校验）
//! 保留在 facade `lab_dip_service` 中，impl 业务方法块分散到本子模块，Rust 允许同一 crate 多文件多 impl 块。
//! `db` 字段声明为 `pub(crate)` 以便本子模块访问；跨模块调用的纯函数亦为 `pub(crate)`。

pub mod request;
pub mod resample;
pub mod sample;
pub mod types;

// re-export DTOs，facade 通过 `pub use` 二次 re-export 保持外部引用路径不变
pub use types::{
    CreateLabDipRequestRequest, CreateLabDipSampleRequest, CreateResampleRequest,
    IssueTechCardRequest, LabDipRequestQuery, RecordMatchingResultRequest,
    RecordResampleResultRequest, UpdateLabDipRequestRequest, UpdateLabDipSampleRequest,
};
