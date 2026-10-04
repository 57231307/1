import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 读接口 Decimal 线格式类型：
// rust_decimal 在 backend/Cargo.toml 仅启用 serde feature、未启用 serde-float ⇒ 序列化为 JSON 字符串
// （模型证据 backend/src/models/sales_price.rs::Model 的 price / min_order_qty 字段均为 Rust Decimal）。
// 前端如实声明为 string，数值解析与定点格式化统一走 composables/spFmts.ts 的 formatCurrency。
export type DecimalString = string;

// 读接口 = 后端 sales_price::Model 逐列如实映射（backend/src/models/sales_price.rs::Model）。
// 已删除历史幽灵键 product_name/product_code/customer_name/remark：销售侧 handler 全部响应点
// 直接序列化 Model（sales_price_handler.rs 的 list_prices/get_price/create_price/update_price/
// get_price_history/list_strategies）；sales_price_service.rs::get_prices_list 主体为
// Entity::find()，keyword 筛选走 LeftJoin(product/customer) 仅作谓词、不追加 SELECT 列（响应仍整
// Model，无名列输出）；信封（utils/response.rs::ApiResponse/PaginatedResponse）不补键 ⇒ 这些键从未
// 出现在响应里。
// purchase 侧同名字段是真实的（purchase_price_service.rs::get_prices_list 以 column_as+LEFT JOIN 产出
// PurchasePriceView），两域接口此前同构抄写，即漂移根源。
export interface SalesPrice {
  id: number;
  product_id: number;
  customer_id: number;
  // DB price DECIMAL(18,6) NOT NULL（migration/src/domain/business/m0011_add_sales_and_logistics_extensions.rs 建表 sales_prices 列定义）
  price: DecimalString;
  currency: string;
  unit: string;
  // DB min_order_qty DECIMAL(12,2) NOT NULL（同上建表列定义，无 DEFAULT）⇒ NOT NULL 列前端接口不写 `?`
  min_order_qty: DecimalString;
  price_type?: string;
  price_level?: string;
  effective_date: string;
  // 后端 Option<NaiveDate>（models/sales_price.rs::Model.expiry_date；DDL m0011 建表 "expiry_date" DATE 可空）⇒ 响应可为 null。
  // 消费侧（SalesPriceView.vue 的 SpViewData.expiry_date）已同步放宽为 `string | null`；
  // el-table 各 prop="expiry_date" 列对 null/undefined 同为空白，无连动残留。
  expiry_date: string | null;
  // 销售侧写入方取值全集 = pending/approved：
  // sales_price_service.rs::create_price 写 price_approval::PENDING、::approve_price 写 APPROVED；
  // inactive 无销售侧生产者（词表权威 backend/src/models/status/sales.rs::price_approval；
  // 契约锁 backend/tests/contract_wave8_price_status_parity_test.rs 已存在，钉 词表常量==DB CHECK
  // 与 list_strategies 判据，含旁路写 inactive 被 chk_sales_price_status 拒绝的负例；
  // 该测试不读前端文件 ⇒ 前端数组一致性由本接口字面量类型与 composables/spFmts.ts 数组同文维持）。
  // 旧值 active/expired 无服务层写入方；建表 DEFAULT 'ACTIVE' 属词表外历史值，
  // 已由 migration/src/domain/price_vocab_check 收敛默认值为 'pending' 并回填存量。
  // DB CHECK chk_sales_price_status IN ('pending','approved') 即该迁移施加，与本取值集逐项相等。
  status: 'pending' | 'approved';
  created_at: string;
  updated_at: string;
}

// 写接口入参（收口后终态，替代原过渡放宽的 DecimalWire=`string|number`）：
// 逐字段对齐后端 sales_price_service.rs::CreateSalesPriceInput
// / ::UpdateSalesPriceInput，不声明后端不接收的键（price_level/status/remark(s)
// 在两 DTO 中不存在，serde 默认静默忽略——声明出去只会制造"写了就会保存"的假象）。
// price/min_order_qty 线格式终态 = string：rust_decimal 1.42.1（版本钉死见 backend/Cargo.lock 的
// rust_decimal 条目）在本构建图仅启用 base serde feature（backend/Cargo.toml rust_decimal 依赖行；
// sea-orm/sea-query/sqlx 不追加 serde-str/serde-float/
// arbitrary-precision），入参走 deserialize_any+DecimalVisitor：
//   visit_str 精确解析（registry rust_decimal-1.42.1/src/serde.rs:362-368）、
//   visit_i64/u64 精确（:322-340）、visit_f64 经 f64::to_string 兜底（:342-348，不拒但
//   丢精度——DECIMAL(18,6) 合法域 18 位有效数字 > f64 ~17 位，number 无法精确表达）。
// ⇒ string 是唯一精确且全域的写线格式（依据即上列 serde.rs 行号的实测实现）。
// 编辑态（el-input-number）仍为 number，由 composables/useSp.ts 在提交边界转十进制字符串；
// 缺值必须省略键（不许伪造 0/''，教训：提交 bd6c407c）。
export interface SalesPriceCreateInput {
  product_id: number;
  customer_id?: number | null;
  customer_type?: string | null;
  // 后端 Decimal 非 Option ⇒ 必填；十进制字符串（如 "12.500000"、"0"）
  price: string;
  currency?: string;
  unit: string;
  price_type: string;
  // 后端 Option<Decimal>，缺省 = 不写键（create 路径由 sales_price_service.rs::create_price 的
  // min_order_qty.unwrap_or_default() 主动写 0，非 DB 默认值兜底——m0011 建表该列无 DEFAULT）
  min_order_qty?: string;
  effective_date?: string;
  expiry_date?: string;
}

export interface SalesPriceUpdateInput {
  product_id?: number;
  customer_id?: number | null;
  customer_type?: string | null;
  price?: string;
  currency?: string;
  min_order_qty?: string;
  effective_date?: string;
  expiry_date?: string;
}

// 后端 sales_price_handler::SalesPriceQuery（list_prices 的 Query<T>，全字段 Option、snake_case）。
// 后端本轮已新增接收 customer_id（等值筛选）与 keyword（产品/客户名模糊，LeftJoin 谓词），本接口
// 尚未声明这两键（筛选栏一直在传、此前后端不收的假筛选已修）——补键属类型改动，交回主编排定夺。
// download_token 为敏感导出 fail-closed 审批令牌：页面未暴露不等于类型不该有，仍如实声明。
export interface SalesPriceQuery {
  product_id?: number;
  customer_type?: string;
  status?: string;
  page?: number;
  page_size?: number;
  download_token?: string;
}

// 后端 sales_price_handler::list_prices 返回 ApiResponse<Vec<Model>> ⇒ 裸数组
export function getSalesPriceList(params?: SalesPriceQuery): Promise<ApiResponse<SalesPrice[]>> {
  return request.get('/sales/sales-prices', { params });
}

export function getSalesPrice(id: number): Promise<ApiResponse<SalesPrice>> {
  return request.get(`/sales/sales-prices/${id}`);
}

export function createSalesPrice(data: SalesPriceCreateInput): Promise<ApiResponse<SalesPrice>> {
  return request.post('/sales/sales-prices', data);
}

export function updateSalesPrice(
  id: number,
  data: SalesPriceUpdateInput
): Promise<ApiResponse<SalesPrice>> {
  return request.put(`/sales/sales-prices/${id}`, data);
}

export function deleteSalesPrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/sales/sales-prices/${id}`);
}

// 审批销售定价请求体：对齐后端 sales_price_handler::ApprovePriceRequest（approved: bool + remark: Option<String>）。
// 后端仅处理批准：approved=false 在 handler::approve_price 直接 400（"审批拒绝请使用专用拒绝接口"，
// 销售价目无 reject 路由）；状态门（仅 pending 可批）在 sales_price_service.rs::approve_price，
// 契约锁 backend/tests/contract_wave8_price_approve_gate_test.rs 钉采购侧批准门与两侧列表筛选 400 化。
// remark 不落库（Model 无 remark 列），仅入 tracing 日志。⚠️ 前端 promptApproval 的"不通过"分支
// 对本端点必 400 —— 拒绝语义缺后端承接，属产品待决策，不在本注释收口内擅改。
export interface ApproveSalesPriceRequest {
  approved: boolean;
  remark?: string;
}

export function approveSalesPrice(
  id: number,
  data: ApproveSalesPriceRequest
): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-prices/${id}/approve`, data);
}

export function getPriceHistory(productId: number): Promise<ApiResponse<SalesPrice[]>> {
  return request.get(`/sales/sales-prices/history/${productId}`);
}

// 后端 sales_price_handler::list_strategies 返回
// ApiResponse<PaginatedResponse<sales_price::Model>> ⇒ {items,total,page,page_size}；
// service 层语义 = “当前生效价目”筛选判据：status=approved + effective_date<=今天
// 且 expiry_date 为空或大于今天（sales_price_service.rs::list_strategies；契约锁
// backend/tests/contract_wave8_price_status_parity_test.rs 的判据①②③与“结果行必全
// approved”用例钉死此语义）。路由 /strategies 原样保留（挂载于 backend/src/routes/sales.rs →
// sales_price_handler::list_strategies；是否删除待用户单独授权）。
// 旧 PricingStrategy/PricingStrategyRule 接口（name/description/rules 等字段）对应 price_strategies
// 表——现迁移链已不建该表，仅 legacy 快照 .monkeycode/docs/database/legacy-migration-snapshots/
// backend-database-migration/001_consolidated_schema.sql 的 price_strategies 表定义可开证；
// 返回行按 sales_price::Model 如实用 SalesPrice[] 承载。
export function getPricingStrategyList(): Promise<
  ApiResponse<{ items: SalesPrice[]; total: number; page: number; page_size: number }>
> {
  return request.get('/sales/sales-prices/strategies');
}
