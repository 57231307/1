//! 数据权限过滤工具
//!
//! 提供数据库层面的字段过滤和行级过滤功能

/// 数据权限过滤器
pub struct DataPermissionFilter {
    /// 允许查询的字段列表
    pub allowed_fields: Vec<String>,
    /// 需要隐藏的字段列表
    pub hidden_fields: Vec<String>,
}

impl DataPermissionFilter {
    /// 创建新的过滤器
    pub fn new(allowed: Vec<String>, hidden: Vec<String>) -> Self {
        Self {
            allowed_fields: allowed,
            hidden_fields: hidden,
        }
    }

    /// 获取允许查询的字段列表
    /// 如果有 allowed_fields，返回 allowed_fields
    /// 如果有 hidden_fields，返回所有字段减去 hidden_fields
    pub fn get_select_fields(&self, all_fields: &[&str]) -> Vec<String> {
        if !self.allowed_fields.is_empty() {
            return self.allowed_fields.clone();
        }

        if !self.hidden_fields.is_empty() {
            return all_fields
                .iter()
                .filter(|f| !self.hidden_fields.contains(&f.to_string()))
                .map(|f| f.to_string())
                .collect();
        }

        all_fields.iter().map(|f| f.to_string()).collect()
    }

    /// 检查是否为空过滤器（不需要过滤）
    pub fn is_empty(&self) -> bool {
        self.allowed_fields.is_empty() && self.hidden_fields.is_empty()
    }
}

/// 客户实体所有字段列表
pub const CUSTOMER_ALL_FIELDS: &[&str] = &[
    "id",
    "customer_code",
    "customer_name",
    "contact_person",
    "contact_phone",
    "contact_email",
    "address",
    "city",
    "province",
    "country",
    "postal_code",
    "credit_limit",
    "payment_terms",
    "tax_id",
    "bank_name",
    "bank_account",
    "status",
    "customer_type",
    "notes",
    "created_by",
    "created_at",
    "updated_at",
    "customer_industry",
    "main_products",
    "annual_purchase",
    "quality_requirement",
    "inspection_standard",
];

/// 默认隐藏的敏感字段（非管理员角色）
pub const DEFAULT_HIDDEN_FIELDS: &[&str] = &[
    "credit_limit",
    "payment_terms",
    "tax_id",
    "bank_name",
    "bank_account",
    "contact_phone",
    "contact_email",
    "address",
];
