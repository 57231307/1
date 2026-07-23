//! 凭证服务-编号生成与类型定义子模块（voucher_ops/numbering）
//!
//! 批次 488 D10-4 拆分：从原 `voucher_service.rs` L1258-1288 迁移。
//! 包含 2 个方法：
//! - generate_voucher_no（pub(crate)，crud.rs 的 create 调用）
//! - available_voucher_types（pub，voucher_handler::get_voucher_types 调用）

use crate::models::voucher;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

use crate::services::voucher_service::{VoucherService, VoucherTypeDefinition};

impl VoucherService {
    /// 生成凭证编号
    pub(crate) async fn generate_voucher_no(
        &self,
        voucher_type: &str,
        _voucher_date: chrono::NaiveDate,
    ) -> Result<String, AppError> {
        let prefix = match voucher_type {
            "记" => "JZ",
            "收" => "SK",
            "付" => "FK",
            "转" => "ZZ",
            _ => "JZ",
        };

        DocumentNumberGenerator::generate_no(
            &*self.db,
            prefix,
            voucher::Entity,
            voucher::Column::VoucherNo,
        )
        .await
    }

    /// 返回系统支持的凭证类型列表（v11 批次 155 P2-C：从 handler 下沉到 service 静态配置化）
    pub fn available_voucher_types() -> Vec<VoucherTypeDefinition> {
        vec![
            VoucherTypeDefinition::new("记", "记账凭证"),
            VoucherTypeDefinition::new("收", "收款凭证"),
            VoucherTypeDefinition::new("付", "付款凭证"),
            VoucherTypeDefinition::new("转", "转账凭证"),
        ]
    }
}
