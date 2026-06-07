# 数据模型层 (Models)

数据模型层是冰溪 ERP 后端的数据访问层，负责定义数据库实体、数据传输对象（DTO）和数据访问接口。数据模型层使用 SeaORM 框架实现 ORM 映射。

## 模块职责

- 定义数据库实体
- 实现数据访问接口
- 定义数据传输对象（DTO）
- 管理数据库迁移
- 提供数据验证

## 结构

```
models/
├── mod.rs                    # 模块导出
├── user.rs                   # 用户实体
├── role.rs                   # 角色实体
├── role_permission.rs        # 角色权限实体
├── data_permission.rs        # 数据权限实体
├── field_permission.rs       # 字段权限实体
├── product.rs                # 产品实体
├── product_category.rs       # 产品分类实体
├── product_color.rs          # 产品颜色实体
├── product_code_mapping.rs   # 产品编码映射实体
├── sales_order.rs            # 销售订单实体
├── sales_order_item.rs       # 销售订单项实体
├── sales_contract.rs         # 销售合同实体
├── sales_return.rs           # 销售退货实体
├── sales_delivery.rs         # 销售发货实体
├── sales_price.rs            # 销售价格实体
├── purchase_order.rs         # 采购订单实体
├── purchase_order_item.rs    # 采购订单项实体
├── purchase_contract.rs      # 采购合同实体
├── purchase_receipt.rs       # 采购收货实体
├── purchase_return.rs        # 采购退货实体
├── purchase_inspection.rs    # 采购检验实体
├── purchase_price.rs         # 采购价格实体
├── inventory_stock.rs        # 库存实体
├── inventory_transfer.rs     # 库存调拨实体
├── inventory_count.rs        # 库存盘点实体
├── inventory_adjustment.rs   # 库存调整实体
├── inventory_reservation.rs  # 库存预留实体
├── inventory_piece.rs        # 库存匹号实体
├── finance_invoice.rs        # 财务发票实体
├── finance_payment.rs        # 财务付款实体
├── voucher.rs                # 财务凭证实体
├── voucher_item.rs           # 财务凭证项实体
├── account_subject.rs        # 会计科目实体
├── account_balance.rs        # 科目余额实体
├── accounting_period.rs      # 会计期间实体
├── ap_invoice.rs             # 应付发票实体
├── ap_payment.rs             # 应付付款实体
├── ap_payment_request.rs     # 应付付款申请实体
├── ap_reconciliation.rs      # 应付对账实体
├── ap_verification.rs        # 应付核销实体
├── ar_invoice.rs             # 应收发票实体
├── ar_collection.rs          # 应收收款实体
├── ar_reconciliation.rs      # 应收对账实体
├── ar_aging_analysis.rs      # 应收账龄分析实体
├── production_order.rs       # 生产订单实体
├── bom.rs                    # BOM 实体
├── bom_item.rs               # BOM 项实体
├── mrp_result.rs             # MRP 结果实体
├── work_center.rs            # 工作中心实体
├── scheduling_result.rs      # 排程结果实体
├── supplier.rs               # 供应商实体
├── supplier_contact.rs       # 供应商联系人实体
├── supplier_evaluation.rs    # 供应商评估实体
├── supplier_qualification.rs # 供应商资质实体
├── supplier_product.rs       # 供应商产品实体
├── customer.rs               # 客户实体
├── customer_contact.rs       # 客户联系人实体
├── customer_credit.rs        # 客户信用实体
├── crm_lead.rs               # CRM 线索实体
├── crm_opportunity.rs        # CRM 商机实体
├── assignment_history.rs     # 分配历史实体
├── bpm_process_definition.rs # BPM 流程定义实体
├── bpm_process_instance.rs   # BPM 流程实例实体
├── bpm_task.rs               # BPM 任务实体
├── approval_template.rs      # 审批模板实体
├── budget_management.rs      # 预算管理实体
├── budget_plan.rs            # 预算计划实体
├── fixed_asset.rs            # 固定资产实体
├── fund_management.rs        # 资金管理实体
├── fund_account.rs           # 资金账户实体
├── tenant.rs                 # 租户实体
├── tenant_user.rs            # 租户用户实体
├── tenant_config.rs          # 租户配置实体
├── tenant_plan.rs            # 租户计划实体
├── tenant_subscription.rs    # 租户订阅实体
├── tenant_usage.rs           # 租户使用实体
├── audit_log.rs              # 审计日志实体
├── omni_audit_log.rs         # 全链路审计日志实体
├── operation_log.rs          # 操作日志实体
├── log_login.rs              # 登录日志实体
├── log_api_access.rs         # API 访问日志实体
├── notification.rs           # 通知实体
├── notification_setting.rs   # 通知设置实体
├── email_template.rs         # 邮件模板实体
├── email_log.rs              # 邮件日志实体
├── warehouse.rs              # 仓库实体
├── department.rs             # 部门实体
├── currency.rs               # 币种实体
├── exchange_rate.rs          # 汇率实体
├── quality_standard.rs       # 质量标准实体
├── quality_inspection.rs     # 质量检验实体
├── cost_collection.rs        # 成本归集实体
├── report_template.rs        # 报表模板实体
├── report_subscription.rs    # 报表订阅实体
├── greige_fabric.rs          # 坯布实体
├── dye_batch.rs              # 缸号实体
├── dye_recipe.rs             # 染色配方实体
├── batch_dye_lot.rs          # 批次缸号实体
├── piece_mapping.rs          # 匹号映射实体
├── five_dimension.rs         # 五维管理实体
├── dual_unit_converter.rs    # 双单位换算实体
├── trading.rs                # 交易实体
├── advanced.rs               # 高级功能实体
├── dto/                      # 数据传输对象
│   ├── mod.rs
│   ├── user_dto.rs
│   ├── product_dto.rs
│   ├── sales_order_dto.rs
│   ├── purchase_order_dto.rs
│   ├── inventory_dto.rs
│   ├── voucher_dto.rs
│   └── ...
└── models/                   # SeaORM codegen 生成的实体
    ├── mod.rs
    ├── prelude.rs
    └── ...
```

## 关键文件

| 文件 | 目的 |
|------|------|
| `user.rs` | 用户实体定义，包含认证和权限 |
| `product.rs` | 产品实体定义，包含五维管理 |
| `sales_order.rs` | 销售订单实体定义，包含订单生命周期 |
| `inventory_stock.rs` | 库存实体定义，包含批次管理 |
| `voucher.rs` | 财务凭证实体定义，包含借贷平衡 |
| `tenant.rs` | 租户实体定义，包含多租户支持 |

## 依赖

**本模块依赖**:
- `sea-orm` - ORM 框架
- `serde` - 序列化/反序列化
- `uuid` - UUID 生成
- `chrono` - 日期时间处理
- `rust_decimal` - 精确数值计算

**依赖本模块的**:
- `services/` - 业务逻辑层使用数据模型
- `handlers/` - 处理器层使用 DTO
- `database/` - 数据库连接池

## 规范

### 文件命名

- 实体: `[entity].rs`（如 `user.rs`）
- DTO: `[entity]_dto.rs`（如 `user_dto.rs`）
- 关联: `[entity1]_[entity2].rs`（如 `role_permission.rs`）

### 代码模式

**实体定义模式**:
```rust
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub status: String,
    pub tenant_id: Option<Uuid>,
    pub last_login_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::role::Entity")]
    Role,
    #[sea_orm(has_many = "super::sales_order::Entity")]
    SalesOrder,
    #[sea_orm(belongs_to = "super::tenant::Entity", from = "Column::TenantId", to = "super::tenant::Column::Id")]
    Tenant,
}

impl Related<super::role::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_role::Relation::Role.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

**DTO 定义模式**:
```rust
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 8))]
    pub password: String,
    
    pub full_name: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
}

impl From<Model> for UserResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            email: model.email,
            full_name: model.full_name,
            phone: model.phone,
            status: model.status,
            created_at: model.created_at,
        }
    }
}
```

### 数据验证

```rust
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, max = 50))]
    pub code: String,
    
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    
    #[validate(range(min = 0.01))]
    pub price: Decimal,
    
    pub category: String,
    pub unit: String,
}

impl CreateProductRequest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // 自定义验证逻辑
        if !["fabric", "yarn", "accessory"].contains(&self.category.as_str()) {
            return Err(ValidationError::new("invalid_category"));
        }
        
        if !["meter", "kilogram", "piece"].contains(&self.unit.as_str()) {
            return Err(ValidationError::new("invalid_unit"));
        }
        
        Ok(())
    }
}
```

## 测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_model_serialization() {
        let user = Model {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            full_name: Some("Test User".to_string()),
            phone: None,
            avatar: None,
            status: "active".to_string(),
            tenant_id: None,
            last_login_at: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };
        
        let json = serde_json::to_string(&user).unwrap();
        let deserialized: Model = serde_json::from_str(&json).unwrap();
        
        assert_eq!(user, deserialized);
    }

    #[test]
    fn test_create_user_request_validation() {
        let valid_request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "Password123".to_string(),
            full_name: None,
            phone: None,
        };
        
        assert!(valid_request.validate().is_ok());
        
        let invalid_request = CreateUserRequest {
            username: "te".to_string(), // 太短
            email: "invalid-email".to_string(), // 无效邮箱
            password: "123".to_string(), // 太短
            full_name: None,
            phone: None,
        };
        
        assert!(invalid_request.validate().is_err());
    }
}
```

## 添加新模型

### 添加新 [实体] 模型

1. 创建 `models/[entity].rs` 文件
2. 定义实体结构体
3. 实现关联关系
4. 创建 `models/[entity]_dto.rs` DTO 文件
5. 从 `models/mod.rs` 导出
6. 创建数据库迁移
7. 添加单元测试

**检查清单**:
- [ ] 遵循命名约定
- [ ] 实现关联关系
- [ ] 定义 DTO
- [ ] 添加数据验证
- [ ] 有对应测试文件
- [ ] 从 mod.rs 导出
- [ ] 创建数据库迁移

## 模型文件清单(自动维护)

<!-- AUTO-GENERATED-START: models -->
- `backend/src/models/exchange_rate.rs`
- `backend/src/models/ar_reconciliation.rs`
- `backend/src/models/mrp_result.rs`
- `backend/src/models/report_subscription.rs`
- `backend/src/models/business_trace.rs`
- `backend/src/models/dye_recipe.rs`
- `backend/src/models/assist_accounting_record.rs`
- `backend/src/models/ap_payment.rs`
- `backend/src/models/inventory_adjustment_item.rs`
- `backend/src/models/log_api_access.rs`
- `backend/src/models/ap_verification.rs`
- `backend/src/models/crm_opportunity.rs`
- `backend/src/models/sales_price.rs`
- `backend/src/models/supplier_product_color.rs`
- `backend/src/models/location.rs`
- `backend/src/models/ar_collection.rs`
- `backend/src/models/financial_analysis.rs`
- `backend/src/models/inventory_piece.rs`
- `backend/src/models/crm_lead.rs`
- `backend/src/models/webhook.rs`
- `backend/src/models/supplier_contact.rs`
- `backend/src/models/account_subject.rs`
- `backend/src/models/tenant_plan.rs`
- `backend/src/models/bpm_task.rs`
- `backend/src/models/business_trace_view.rs`
- `backend/src/models/report_definition.rs`
- `backend/src/models/purchase_price.rs`
- `backend/src/models/ar_invoice.rs`
- `backend/src/models/sales_delivery.rs`
- `backend/src/models/bpm_process_definition.rs`
- `backend/src/models/sales_return_item.rs`
- `backend/src/models/ap_payment_request_item.rs`
- `backend/src/models/inventory_transfer_item.rs`
- `backend/src/models/tenant_invoice.rs`
- `backend/src/models/finance_payment.rs`
- `backend/src/models/supplier_evaluation_record.rs`
- `backend/src/models/purchase_inspection.rs`
- `backend/src/models/customer_followup.rs`
- `backend/src/models/bom.rs`
- `backend/src/models/tenant_usage.rs`
- `backend/src/models/fixed_asset.rs`
- `backend/src/models/fixed_asset_disposal.rs`
- `backend/src/models/voucher_item.rs`
- `backend/src/models/finance_invoice.rs`
- `backend/src/models/production_order.rs`
- `backend/src/models/inventory_count.rs`
- `backend/src/models/supplier_product.rs`
- `backend/src/models/product_color.rs`
- `backend/src/models/product_code_mapping.rs`
- `backend/src/models/customer.rs`
- `backend/src/models/fund_management.rs`
- `backend/src/models/batch_dye_lot.rs`
- `backend/src/models/report_template.rs`
- `backend/src/models/product_category.rs`
- `backend/src/models/business_trace_assist_link.rs`
- `backend/src/models/accounting_period.rs`
- `backend/src/models/bpm_process_instance.rs`
- `backend/src/models/unqualified_product.rs`
- `backend/src/models/cost_collection.rs`
- `backend/src/models/supplier_grade.rs`
- `backend/src/models/supplier_category.rs`
- `backend/src/models/budget_management.rs`
- `backend/src/models/product_supplier_mapping.rs`
- `backend/src/models/omni_audit_log.rs`
- `backend/src/models/audit_log.rs`
- `backend/src/models/scheduling_result.rs`
- `backend/src/models/inventory_count_item.rs`
- `backend/src/models/sales_order.rs`
- `backend/src/models/data_permission.rs`
- `backend/src/models/supplier_evaluation.rs`
- `backend/src/models/system_version.rs`
- `backend/src/models/department.rs`
- `backend/src/models/inventory_transaction.rs`
- `backend/src/models/oa_announcement.rs`
- `backend/src/models/ap_invoice.rs`
- `backend/src/models/approval_instance.rs`
- `backend/src/models/user_notification_setting.rs`
- `backend/src/models/inventory_adjustment.rs`
- `backend/src/models/bom_item.rs`
- `backend/src/models/ap_reconciliation.rs`
- `backend/src/models/status.rs`
- `backend/src/models/purchase_return.rs`
- `backend/src/models/business_trace_snapshot.rs`
- `backend/src/models/purchase_order.rs`
- `backend/src/models/batch_trace_log.rs`
- `backend/src/models/inventory_transfer.rs`
- `backend/src/models/supplier_qualification.rs`
- `backend/src/models/customer_credit.rs`
- `backend/src/models/supplier.rs`
- `backend/src/models/sales_order_item.rs`
- `backend/src/models/notification.rs`
- `backend/src/models/ar_reconciliation_item.rs`
- `backend/src/models/tenant_user.rs`
- `backend/src/models/log_login.rs`
- `backend/src/models/sales_return.rs`
- `backend/src/models/assignment_history.rs`
- `backend/src/models/sales_analysis.rs`
- `backend/src/models/tenant_config.rs`
- `backend/src/models/tenant_subscription.rs`
- `backend/src/models/ap_verification_item.rs`
- `backend/src/models/ap_payment_request.rs`
- `backend/src/models/work_center.rs`
- `backend/src/models/fund_transfer_record.rs`
- `backend/src/models/piece_mapping.rs`
- `backend/src/models/api_key.rs`
- `backend/src/models/purchase_return_item.rs`
- `backend/src/models/operation_log.rs`
- `backend/src/models/assist_accounting_summary.rs`
- `backend/src/models/purchase_order_item.rs`
- `backend/src/models/quality_inspection_record.rs`
- `backend/src/models/approval_template.rs`
- `backend/src/models/tenant.rs`
- `backend/src/models/inventory_stock.rs`
- `backend/src/models/product.rs`
- `backend/src/models/greige_fabric.rs`
- `backend/src/models/approval_log.rs`
- `backend/src/models/inventory_reservation.rs`
- `backend/src/models/account_balance.rs`
- `backend/src/models/currency.rs`
- `backend/src/models/business_trace_chain.rs`
- `backend/src/models/voucher.rs`
- `backend/src/models/email_log.rs`
- `backend/src/models/purchase_receipt_item.rs`
- `backend/src/models/fund_account.rs`
- `backend/src/models/user.rs`
- `backend/src/models/log_system.rs`
- `backend/src/models/quality_inspection.rs`
- `backend/src/models/dye_batch.rs`
- `backend/src/models/logistics_waybill.rs`
- `backend/src/models/color_code_mapping.rs`
- `backend/src/models/assist_accounting_dimension.rs`
- `backend/src/models/role.rs`
- `backend/src/models/budget_adjustment.rs`
- `backend/src/models/ar_aging_analysis.rs`
- `backend/src/models/sales_order_change_history.rs`
- `backend/src/models/financial_analysis_result.rs`
- `backend/src/models/purchase_receipt.rs`
- `backend/src/models/purchase_contract.rs`
- `backend/src/models/field_permission.rs`
- `backend/src/models/mod.rs`
- `backend/src/models/budget_execution.rs`
- `backend/src/models/purchase_contract_execution.rs`
- `backend/src/models/budget_plan.rs`
- `backend/src/models/sales_delivery_item.rs`
- `backend/src/models/audit_alert_rule.rs`
- `backend/src/models/notification_setting.rs`
- `backend/src/models/cost_analysis.rs`
- `backend/src/models/role_permission.rs`
- `backend/src/models/sales_contract.rs`
- `backend/src/models/email_template.rs`
- `backend/src/models/approval_node.rs`
- `backend/src/models/dye_lot_mapping.rs`
- `backend/src/models/supplier_blacklist.rs`
- `backend/src/models/quality_standard.rs`
- `backend/src/models/warehouse.rs`
<!-- AUTO-GENERATED-END: models -->
