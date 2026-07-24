//! 采购入库-身份校验子模块（purchase_receipt_ops/auth）
//!
//! 批次 D10 拆分：从原 `purchase_receipt_service.rs` 迁移。
//! 包含 `PurchaseReceiptService` 的 1 个身份校验方法：
//! - `is_admin_user`：检查 user_id 是否为管理员（用于绕过 created_by owner 检查）
//!
//! 跨模块调用：`crud`（update_receipt / delete_receipt）与 `items`
//! （add_receipt_item / update_receipt_item / delete_receipt_item）共 5 处调用此方法，
//! 故声明为 `pub(crate)`。

use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::utils::error::AppError;

impl PurchaseReceiptService {
    /// P2 3-19 修复：检查 user_id 是否为管理员（用于绕过 created_by owner 检查）
    ///
    /// 原 update_receipt/delete_receipt/confirm_receipt/add_receipt_item/
    /// update_receipt_item/delete_receipt_item 6 处均硬编码 `created_by != user_id`
    /// 无管理员绕过，admin 无法管理他人创建的入库单。
    /// 新增此辅助方法，admin 角色可绕过 owner 检查。
    pub(crate) async fn is_admin_user(&self, user_id: i32) -> Result<bool, AppError> {
        use crate::models::user;
        use sea_orm::EntityTrait;

        let user = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("用户不存在"))?;
        if let Some(role_id) = user.role_id {
            Ok(crate::utils::admin_checker::is_admin_role(&self.db, role_id).await)
        } else {
            Ok(false)
        }
    }
}
