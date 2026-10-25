//! 出参金额隐藏的 Service 层唯一实现。
//!
//! 【职责】收敛“某角色的出参要不要隐藏金额、隐藏哪些金额列”这一判定，
//! 供销售域与采购域的列表与详情出口共用，避免同一规则在各出口内联复制后分叉。
//!
//! 【边界与分工】本模块只负责金额列的隐藏门与移除动作，且只覆盖
//! “该角色对该资源没有数据权限配置行”这一条路径。三件事刻意不在这里做：
//! 一是数据权限配置行的读取与按配置过滤（allowed 白名单与 hidden 移除），
//! 由调用方沿用 DataPermissionService 的既有分支，配置行存在时不叠加默认隐藏；
//! 二是联系方式的掩码或整键移除，由各调用方按自己出参的真实列自行处理，
//! 本模块若代为处理，会给不含这些列的出参新增用户可见行为；
//! 三是行级可见范围（数据范围与归属判定），属查询侧收窄，与出参字段隐藏无关。
//!
//! 【隐藏门语义】与收编前各出口的内联判定逐字一致：调用方只在角色对该资源
//! 无配置行时进入本模块；进入后仅当该角色不是 admin（roles.code 为 admin，
//! 判定源 utils::admin_checker，查询失败按非 admin 处理）才移除金额列，
//! admin 命中则原值放行。角色缺失的出参路径不进入本模块，其现语义是
//! “不做任何处理”，由调用方外层分支结构保持；本模块不接管该形态，
//! 否则会把销售与采购两域的现语义收紧成另一种行为。
//!
//! 【列集合口径】列集合以各出参的序列化类型定义为唯一权威源逐列登记：
//! 一是该形状里的全部金额、价格与税率折扣率列都要覆盖，不许留半边
//! （留半边等于隐了一部分还把另一部分继续外发）；
//! 二是出参里不存在的键一律不收进来，对不存在的键做移除是恒不生效的空操作，
//! 留着只会让人误判该列已被保护。
//! 数量、重量、幅宽、规格、单号、状态、时间类列不属金额，保持原样外发。
//! 收编前个别出口把数量列一并移除了，按裁定这类列不属金额，撤销该动作；
//! 撤销会让这些数量列从被隐变为可见，属裁定明确批准的可见性变化，不是本模块自取。

use crate::utils::admin_checker;
use sea_orm::DatabaseConnection;
use serde_json::Value;

/// 参与默认金额隐藏的资源，取值范围与接入点一一对应，不为将来预留扩展位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultAmountResource {
    /// 销售订单主行及其嵌套明细
    SalesOrder,
    /// 采购订单主行及其嵌套明细
    PurchaseOrder,
    /// 采购入库单主行
    PurchaseReceipt,
    /// 采购入库明细行（行本身即明细，不携带嵌套）
    PurchaseReceiptItem,
}

/// 默认隐藏门的判定结果，调用方据此决定自己域特有的后续处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultAmountHiding {
    /// 该角色非 admin，金额列已按资源口径移除
    Hidden,
    /// 该角色是 admin，出参保持原值
    AdminVisible,
}

impl DefaultAmountResource {
    /// 日志用的资源标识，与调用方查询数据权限时传入的资源类型字面量同源。
    fn code(self) -> &'static str {
        match self {
            Self::SalesOrder => "sales_order",
            Self::PurchaseOrder => "purchase_order",
            Self::PurchaseReceipt => "purchase_receipt",
            Self::PurchaseReceiptItem => "purchase_receipt_item",
        }
    }

    /// 主行要移除的金额列。列名以该出参的序列化类型定义为唯一权威源。
    fn head_columns(self) -> &'static [&'static str] {
        match self {
            // 权威源 SalesOrderDetail：七个金额列全部在册，收编前无幻影键
            Self::SalesOrder => &[
                "subtotal",
                "tax_amount",
                "discount_amount",
                "shipping_cost",
                "total_amount",
                "paid_amount",
                "balance_amount",
            ],
            // 权威源 PurchaseOrderDto：已付与未付两键在该出参不存在（收编前是空转，
            // 故不收），已到货金额实名 received_amount 且此前漏隐，按真实列补入。
            // 两个数量列按裁定不属金额，不再列入隐藏集合。
            Self::PurchaseOrder => &["total_amount", "total_amount_foreign", "received_amount"],
            Self::PurchaseReceipt => &["total_amount"],
            // 权威源 purchase_receipt_item 模型：行金额键实名 amount，
            // 收编前写的 total_price 与 tax_amount 两键在该出参不存在（空转），不收。
            Self::PurchaseReceiptItem => &["unit_price", "amount"],
        }
    }

    /// 明细数组所在的键；返回空值表示该出参不携带嵌套明细。
    fn item_key(self) -> Option<&'static str> {
        match self {
            Self::SalesOrder | Self::PurchaseOrder => Some("items"),
            Self::PurchaseReceipt | Self::PurchaseReceiptItem => None,
        }
    }

    /// 明细行要移除的金额列。列名以明细行的序列化类型定义为唯一权威源。
    fn item_columns(self) -> &'static [&'static str] {
        match self {
            // 权威源 SalesOrderItemDetail：税率与行金额的实名分别是 tax_percent 与
            // total_amount，收编前写的 tax_rate 与 total_price 不存在（空转）；
            // 除已隐的单价外，小计、税额、折扣额、行金额、折扣率、税率与四个定价列
            // 此前全部漏隐，按真实列名一并补入。数量、重量、幅宽、纸管重与规格不属金额。
            Self::SalesOrder => &[
                "unit_price",
                "subtotal",
                "tax_amount",
                "discount_amount",
                "total_amount",
                "tax_percent",
                "discount_percent",
                "base_price",
                "color_extra_cost",
                "grade_price_diff",
                "final_price",
            ],
            // 权威源 purchase_order_item 模型：收编前六列全部真实存在，
            // 另漏隐折扣率与税率两列，按真实列名补入。收货数量与数量容差不属金额。
            Self::PurchaseOrder => &[
                "unit_price",
                "unit_price_foreign",
                "subtotal",
                "tax_amount",
                "discount_amount",
                "total_amount",
                "tax_percent",
                "discount_percent",
            ],
            Self::PurchaseReceipt | Self::PurchaseReceiptItem => &[],
        }
    }
}

/// 对单行出参按资源口径移除金额列。
///
/// 入参：resource 决定列集合；row 是一行出参，不是 JSON 对象时原样返回。
/// 返回：本行实际移除的键数，主行列与明细行列合并计数，用于区分
/// “确实移除”与“列集合对不上出参形状”两种情形，供日志与断言使用。
/// 本函数不查库、不判角色，只做形状操作；要不要隐藏的判断在
/// apply_default_amount_hiding 内完成。
pub fn hide_default_amount_columns(resource: DefaultAmountResource, row: &mut Value) -> usize {
    let Some(obj) = row.as_object_mut() else {
        return 0;
    };

    let mut removed_columns = 0usize;
    for column in resource.head_columns() {
        if obj.remove(*column).is_some() {
            removed_columns += 1;
        }
    }

    if let Some(item_key) = resource.item_key() {
        if let Some(items) = obj.get_mut(item_key).and_then(Value::as_array_mut) {
            for item in items {
                if let Some(item_obj) = item.as_object_mut() {
                    for column in resource.item_columns() {
                        if item_obj.remove(*column).is_some() {
                            removed_columns += 1;
                        }
                    }
                }
            }
        }
    }

    removed_columns
}

/// 默认金额隐藏门：判定、移除与日志的唯一入口。
///
/// 入参：db 供 admin 判定使用；role_id 是调用方已确认存在的角色，
/// 角色缺失的调用点不应到达本函数（现语义由调用方分支保持）；
/// resource 决定列集合；rows 是已序列化的出参数组，
/// 单条详情出口传单元素切片即可，不需要额外的单行入口。
/// 返回：DefaultAmountHiding。调用方拿到 Hidden 才继续执行自己域特有的
/// 后续处理（销售域在此条件下做收货联系电话掩码），拿到 AdminVisible
/// 则保持原值不做任何处理；采购域无后续处理，忽略返回值。
/// 副作用：命中与不命中各记一条 info 日志，带角色、资源与行数或移除列数，
/// 判定结果不静默丢弃。
pub async fn apply_default_amount_hiding(
    db: &DatabaseConnection,
    role_id: i32,
    resource: DefaultAmountResource,
    rows: &mut [Value],
) -> DefaultAmountHiding {
    if admin_checker::is_admin_role(db, role_id).await {
        tracing::info!(
            role_id,
            resource = resource.code(),
            rows = rows.len(),
            rule = "amount_default_hiding_admin_exempt",
            "出参金额默认隐藏未命中：admin 角色按既有契约原值放行"
        );
        return DefaultAmountHiding::AdminVisible;
    }

    let mut removed_columns = 0usize;
    for row in rows.iter_mut() {
        removed_columns += hide_default_amount_columns(resource, row);
    }

    tracing::info!(
        role_id,
        resource = resource.code(),
        rows = rows.len(),
        removed_columns,
        rule = "amount_default_hiding_applied",
        "出参金额默认隐藏命中：按该资源的金额列集合整键移除"
    );
    DefaultAmountHiding::Hidden
}
