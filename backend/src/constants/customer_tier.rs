//! `customers.tier` 唯一词表模块
//!
//! 本模块是 **customers 表 tier 列**允许值集合在源码中的唯一出现点；
//! 各写入口与判据一律引此处，不得再抄内联白名单。
//!
//! ## 本列承载的语义：客户分层（tier）
//!
//! `ALLOWED` 四值一律是**分层** token，与渠道列 `customer_type`
//! （`constants::customer_type`，retail/wholesale/distributor/manufacturer/other）
//! 是**两回事**：渠道回答"从哪条销路走"，分层回答"这客户按等级算不算大客户"。
//! 分层词不得写进渠道列，渠道词也不得写进本列。
//!
//! 本词表**复用**仓内既有客户等级词表（不新造第五套）：DB 侧
//! `chk_tier_customer_level`（`color_price_tiers.customer_level` 的 CHECK）与应用侧
//! `services/color_price_crud_service.rs` 校验、`utils/price_calculator.rs` 折扣消费
//! 均为同一组大写 token——大小写必须逐字符一致，任何一侧改小写/换拼法即契约漂移。
//!
//! ⚠️ DB 侧对本列的 CHECK 约束名为 `chk_customers_tier`（与本 `ALLOWED` 逐值同源，
//! 另放行 NULL），迁移侧与本模块必须同一集合，任何一侧单独增删值都算契约漂移。
//!
//! ## NULL 的语义：未定档（不是档位）
//!
//! `tier IS NULL` = 没有可由既有评级数据复核出来的档位（无评级行，或评级落在六值
//! 词表外），**不得**把 NULL 当 NORMAL 消费，也不得给列加 DEFAULT 替存量造档。
//! NORMAL 与 NULL 的区别是"有低档依据"与"无依据"。NULL 判不进任何档位、判不进大客户。
//!
//! ## `MAJOR`：大客户高档集合（二级审批唯一判据）
//!
//! 大客户转单二级审批（`services/crm/customer_transfer_approval_service.rs`
//! 的 `check_large_customer`）**只**以 `tier IN MAJOR` 判定。`MAJOR` 是分层阶梯的
//! 最高两档（VIP、GOLD；序位依据：SILVER/NORMAL 在等级折扣与评级映射中恒低于
//! GOLD/VIP），集中为模块常量、与 `ALLOWED` 同文件同源，禁止在判据处内联第二套
//! token 清单，也禁止再以预估金额、信用额度等其它维度代理判"大客户"。

/// 最高档 token——分层阶梯顶端
pub const VIP: &str = "VIP";

/// 次高档 token
pub const GOLD: &str = "GOLD";

/// 第三档 token
pub const SILVER: &str = "SILVER";

/// 最低实档 token——"有评级依据但落低档"，与 NULL（未定档）语义严格区分
pub const NORMAL: &str = "NORMAL";

/// customers.tier 当前允许值集合（分层维度唯一出现点，与 DB 侧 `chk_customers_tier`
/// 逐值同源、外加放行 NULL；不增不减，渠道词永不并入本列）。
pub const ALLOWED: &[&str] = &[VIP, GOLD, SILVER, NORMAL];

/// 大客户高档集合（二级审批唯一判据）：分层阶梯最高两档。
/// 调整本集合 = 调整业务口径，必须与决策方确认；判据代码只许引用本常量。
pub const MAJOR: &[&str] = &[VIP, GOLD];

/// 高档判定：逐字符精确匹配 `MAJOR`（不 trim、不做大小写归一——词表与 DB CHECK
/// 均为逐字符敏感，归一会把词表外变体静默吞进判据）。
/// NULL/缺席由调用方按"非大客户"处理（未定档不判大客户），本函数不承担 Option。
pub fn is_major(raw: &str) -> bool {
    MAJOR.contains(&raw)
}
