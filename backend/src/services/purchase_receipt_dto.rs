//! 采购入库服务请求/响应 DTO 结构体
//!
//! 拆分自 purchase_receipt_service.rs：原 4 个 DTO 独立成文件。

/// 创建采购入库单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseReceiptRequest {
    /// 采购订单 ID
    pub order_id: Option<i32>,

    /// 供应商 ID
    pub supplier_id: i32,

    /// 入库日期
    pub receipt_date: chrono::NaiveDate,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 部门 ID
    pub department_id: Option<i32>,

    /// 质检员 ID
    pub inspector_id: Option<i32>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 入库明细
    #[validate(length(min = 1, message = "入库单至少需要一行明细"))]
    pub items: Vec<CreateReceiptItemRequest>,
}

/// 更新采购入库单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseReceiptRequest {
    pub supplier_id: Option<i32>,
    pub receipt_date: Option<chrono::NaiveDate>,
    pub department_id: Option<i32>,
    pub inspector_id: Option<i32>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

/// 创建入库明细请求
#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct CreateReceiptItemRequest {
    /// 订单明细 ID
    pub order_item_id: Option<i32>,

    /// 行号
    pub line_no: i32,

    /// 物料 ID
    pub material_id: i32,

    /// 物料编码
    pub material_code: String,

    /// 物料名称
    pub material_name: String,

    /// 批次号
    pub batch_no: Option<String>,

    /// 色号
    pub color_code: Option<String>,

    /// 缸号
    pub lot_no: Option<String>,

    /// 等级
    pub grade: Option<String>,

    /// 克重
    pub gram_weight: Option<Decimal>,

    /// 幅宽
    pub width: Option<Decimal>,

    /// 入库数量（主单位）
    pub quantity: Decimal,

    /// 入库数量（辅助单位）
    pub quantity_alt: Decimal,

    /// 主单位
    pub unit_master: String,

    /// 辅助单位
    pub unit_alt: Option<String>,

    /// 单价
    pub unit_price: Option<Decimal>,

    /// 库位编码
    pub location_code: Option<String>,

    /// 包号
    pub package_no: Option<String>,

    /// 生产日期
    pub production_date: Option<chrono::NaiveDate>,

    /// 保质期（天）
    pub shelf_life: Option<i32>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新入库明细请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdateReceiptItemRequest {
    pub line_no: Option<i32>,
    pub material_id: Option<i32>,
    pub material_code: Option<String>,
    pub material_name: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub quantity: Option<Decimal>,
    pub quantity_alt: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub location_code: Option<String>,
    pub notes: Option<String>,
}
