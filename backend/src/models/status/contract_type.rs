#![allow(dead_code)]
//! 合同类型常量（大写值）。
//!
//! sales_contract 与 purchase_contract 两侧 contract_type 列共用本词表，与两端建单表单的
//! 三型选项同源；比较点与写侧取值必须以本模块常量为唯一来源，禁止另立第二套字面量。
//! 转单对话框当前提交的小写三值属存量兼容读面，不进本词表，写侧收口随前端转单选项对齐后进行。

/// 购销合同：销售侧建单表单首项，印花税按购销档计
pub const SALES: &str = "SALES";

/// 采购合同：采购侧建单表单首项
pub const PURCHASE: &str = "PURCHASE";

/// 框架合同：两侧建单表单同名列，购销性质按框架约定签约，印花税同按购销档计
pub const FRAMEWORK: &str = "FRAMEWORK";

/// 补充合同：对既有合同金额或条款的增补签约，印花税按购销档对增补额计
pub const SUPPLEMENT: &str = "SUPPLEMENT";

/// 全部合法取值（写侧与后续入参门取值域的唯一同源来源，逐字符敏感，不做大小写归一）
pub const ALL: &[&str] = &[SALES, PURCHASE, FRAMEWORK, SUPPLEMENT];
