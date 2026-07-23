//! 产品 Service ES 同步辅助子模块（product_ops/sync）
//!
//! 批次 D10 拆分：从原 `product_service.rs` 迁移。
//! 包含 `ProductService` 的 2 个 ES 同步辅助方法：
//! - `build_product_doc`：将 `product::Model` 转换为 `ProductDoc` 用于 ES 索引（私有）
//! - `sync_product_to_es`：PG 事务提交后同步到 ES（最终一致性策略，`pub(crate)`）
//!
//! `sync_product_to_es` 声明为 `pub(crate)`，供 `crud` 子模块的
//! create/update/delete 方法跨子模块调用。`build_product_doc` 仅在本模块内
//! 被 `sync_product_to_es` 调用，保持私有。

use crate::models::product;
use crate::search::ProductDoc;
use crate::services::product_service::ProductService;

impl ProductService {
    /// 将 product::Model 转换为 ProductDoc 用于 ES 索引
    ///
    /// 批次 125 v8 复审 P1 修复：字段映射规则
    /// - category: join product_category 表取 name（category_id 索引意义不大）
    /// - color_no/pantone_code: 暂设 None（一对多关联复杂，后续迭代优化）
    /// - price: standard_price Decimal → f64
    /// - spec: specification 字段
    fn build_product_doc(&self, model: &product::Model) -> ProductDoc {
        ProductDoc {
            id: model.id,
            code: model.code.clone(),
            name: model.name.clone(),
            category: None, // 批次 125：暂设 None，后续迭代 join product_category
            spec: model.specification.clone(),
            unit: model.unit.clone(),
            color_no: None, // 批次 125：暂设 None，后续迭代 join product_color
            pantone_code: None, // 批次 125：暂设 None，后续迭代 join product_color
            price: model
                .standard_price
                .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
                .unwrap_or(0.0),
        }
    }

    /// 同步产品到 ES（最终一致性策略）
    ///
    /// 批次 125 v8 复审 P1 修复：ES 同步失败仅记录日志，不回滚 PG 事务。
    ///
    /// `pub(crate)`：`crud` 子模块的 create/update/delete 方法跨子模块调用。
    pub(crate) async fn sync_product_to_es(&self, model: &product::Model, operation: &str) {
        let doc = self.build_product_doc(model);
        if let Err(e) = self.search_syncer.sync_product(&doc).await {
            tracing::warn!(
                error = %e,
                product_id = model.id,
                product_code = %model.code,
                operation = operation,
                "ES 产品同步失败（PG 已提交，最终一致性靠补偿任务修复）"
            );
        }
    }
}
