#![allow(dead_code)]
//! PII 按需揭示留痕表 Entity（pii_reveal_audit）
//!
//! 唯一写入方：`services/crm/pii_reveal.rs::record_reveal`（揭示成功后插一行）。
//! 语义边界：本表**不存 PII 原文**，只记"谁（operator_user_id，会话注入）、何时
//! （revealed_at）、以何用途（reason）、揭示了哪条记录的哪些字段（record_type +
//! record_id + revealed_fields token 集合）"；业务端点不得删除或修改审计行，
//! `record_type`/`record_id`/`operator_user_id` 无外键（审计须比业务行与用户行
//! 存活久，判据与列宽依据见建表迁移文件头）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// PII 揭示留痕 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "pii_reveal_audit")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 记录类型（本波取值 customer，指向 customers.id）
    pub record_type: String,
    /// 被揭示记录 ID（与 customers.id 同宽 INTEGER）
    pub record_id: i32,
    /// 被揭示字段 token 集合（应用层白名单值域，text[]）
    pub revealed_fields: Vec<String>,
    /// 操作人用户 ID（服务端会话 AuthContext.user_id 注入）
    pub operator_user_id: i32,
    /// 本次查看用途（必填）
    pub reason: String,
    /// 揭示时间戳
    pub revealed_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
