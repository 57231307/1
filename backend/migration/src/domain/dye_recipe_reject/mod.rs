//! 染色配方拒绝通道落地域（chk_dye_recipe_status 后继扩集 + rejected_reason 专列）
//!
//! - m0091_widen_dye_recipe_status_for_reject: v15 域尾建立的 chk_dye_recipe_status
//!   四值集（draft/pending_approval/approved/disabled）不含拒绝终态，dye_recipe 表
//!   亦无拒绝理由列；本域在同一约束名上把取值集扩至五值（+rejected）并补
//!   rejected_reason TEXT 可空列，守卫与回滚 fail-visible（照 price_vocab_extend 范式）。
//!
//! 注册位置（lib.rs 迁移链）必须在全链尾：生效 CHECK 的现行定义以本域 up 为准，
//! 若排在 v15 之前，会被 v15 域尾按同名约束重建的四值窄集覆盖回旧集。

mod m0091_widen_dye_recipe_status_for_reject;

pub use m0091_widen_dye_recipe_status_for_reject::Migration;
