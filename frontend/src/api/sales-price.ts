import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 读接口 Decimal 线格式类型：
// rust_decimal 在 backend/Cargo.toml 仅启用 serde feature、未启用 serde-float ⇒ 序列化为 JSON 字符串
// （模型证据 backend/src/models/sales_price.rs::Model 的 price / min_order_qty 字段均为 Rust Decimal）。
// 前端如实声明为 string，数值解析与定点格式化统一走 composables/spFmts.ts 的 formatCurrency。
export type DecimalString = string;

// 读接口基型 SalesPrice = 后端 sales_price::Model 逐列如实映射（backend/src/models/sales_price.rs::Model），
// 唯一例外 = 审批理由两列（approval_reason/rejected_reason）刻意不声明，理由见下方 status 字段注。
// 详情/新建/更新/历史端点仍整 Model 直接序列化（sales_price_handler.rs 的 get_price/create_price/
// update_price/get_price_history）；列表端点 list_prices 已换为富化读模型 SalesPriceView
// （backend/src/services/sales_price_service.rs:92-115：Model 全列 + LEFT JOIN 四列
// product_name/product_code/customer_name/customer_code），其前端行型即下方 SalesPriceRow。
// 历史幽灵键 remark 已删（Model 无该列，任何端点都不输出）。
// purchase 侧同构读模型是 PurchasePriceView（purchase_price_service.rs），两域此前"接口同构抄写、
// 后端实态不同"即漂移根源，现两侧后端均已真实 JOIN 富化。
export interface SalesPrice {
  id: number;
  product_id: number;
  // DB customer_id INTEGER 可空（models/sales_price.rs::Model.customer_id = Option<i32>，
  // m0011 建表无 NOT NULL 且无 FK）；标准价行无客户 ⇒ 响应为 null，如实声明 number | null。
  customer_id: number | null;
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
  // 销售侧写入方取值全集 = pending/approved/rejected：
  // sales_price_service.rs::create_price 写 price_approval::PENDING、::approve_price 写 APPROVED、
  // ::reject_price 写 REJECTED；
  // inactive 无销售侧生产者（词表权威 backend/src/models/status/sales.rs::price_approval；
  // 契约锁 backend/tests/contract_wave8_price_status_parity_test.rs 钉 词表常量==DB CHECK
  // 与跨表不对称负例（旁路写 inactive 被销售侧 chk_sales_price_status 拒绝）；
  // 该测试不读前端文件 ⇒ 前端数组一致性由本接口字面量类型与 composables/spFmts.ts 数组同文维持）。
  // 旧值 active/expired 无服务层写入方；建表 DEFAULT 'ACTIVE' 属词表外历史值，
  // 已由 migration/src/domain/price_vocab_check 收敛默认值为 'pending' 并回填存量。
  // DB CHECK chk_sales_price_status IN ('pending','approved','rejected')：窄集由 price_vocab_check
  // 施加、rejected 由后继 migration/src/domain/price_vocab_extend/m0080_extend_price_status_check_rejected.rs
  // 扩集，与本取值集逐项相等；列表 status 筛选白名单同集（handlers/sales_price_handler.rs:62-66）。
  // 审批理由两列（approval_reason/rejected_reason，m0079 加列）本接口刻意不声明：详情端点虽会输出，
  // 但当前无消费方展示，且列表读模型 SalesPriceView 不携带这两列
  // （services/sales_price_service.rs:92-115），在行型上补键只会造出恒空的假列。
  status: 'pending' | 'approved' | 'rejected';
  created_at: string;
  updated_at: string;
}

// 写接口入参（收口后终态，替代原过渡放宽的 DecimalWire=`string|number`）：
// 逐字段对齐后端 sales_price_service.rs::CreateSalesPriceInput
// / ::UpdateSalesPriceInput，不声明后端不接收的键（status/remark(s)
// 在两 DTO 中不存在，serde 默认静默忽略——声明出去只会制造"写了就会保存"的假象；
// price_level 后端 DTO 真实存在（写链在途），本侧未把它加入入参型与提交载荷，交回主编排对齐）。
// price/min_order_qty 线格式终态 = string：rust_decimal 1.42.1（版本钉死见 backend/Cargo.lock 的
// rust_decimal 条目）在本构建图仅启用 base serde feature（backend/Cargo.toml rust_decimal 依赖行；
// sea-orm/sea-query/sqlx 不追加 serde-str/serde-float/
// arbitrary-precision），入参走 deserialize_any+DecimalVisitor：
//   JSON 字符串经 visit_str 精确解析、整数精确；浮点字面量经 f64::to_string 兜底，不拒但
//   丢精度——DECIMAL(18,6) 合法域 18 位有效数字 > f64 ~17 位，number 无法精确表达。
// ⇒ string 是唯一精确且全域的写线格式。
// 编辑态（el-input-number）仍为 number，由 composables/useSp.ts 在提交边界转十进制字符串；
// 缺值必须省略键（不许伪造 0/''）。
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

// 后端 sales_price_handler::SalesPriceQuery（backend/src/handlers/sales_price_handler.rs 的 Query<T>，
// 全字段 Option、snake_case、serde 无 rename/default/deny_unknown_fields ⇒ 未知键静默忽略、缺键=None；
// 列表 list_prices 与导出 export_prices 共用同一结构体 = 同口径）。
// customer_id（Option<i32> 等值筛选）与 keyword（Option<String>，语义=「产品名称/客户名称」模糊，
// service 侧 LeftJoin(product/customer) 仅作谓词、trim 去空后下推）后端已真实接收并生效。
// 空串筛选由双侧契约剔除：request.ts serializeParams 不发空串/纯空白键，后端
// normalize_empty_query_params 中间件再兜一层；"未选"的正确编码 = 省略键（不是发 null——
// query string 没有 null 线格式，customer_id= 空串或 "null" 字符串都会破坏 Option<i32> 解析语义）。
// download_token 为敏感导出 fail-closed 审批令牌：页面未暴露不等于类型不该有，仍如实声明。
// 注：列表运行时链路走 useTableApi（params 为 Record<string, unknown>），本接口是该端点查询形态的
// 声明层契约（getSalesPriceList 的入参类型），改动必须与后端结构体逐键对照。
export interface SalesPriceQuery {
  product_id?: number;
  customer_id?: number;
  keyword?: string;
  customer_type?: string;
  status?: string;
  page?: number;
  page_size?: number;
  download_token?: string;
}

// 后端 sales_price_handler::list_prices 返回 ApiResponse<Vec<SalesPriceView>> ⇒ 裸数组行对象。
// 列表行 = SalesPrice 全列 + 四个 JOIN 名列（SalesPriceView，sales_price_service.rs:92-115）。
export interface SalesPriceRow extends SalesPrice {
  // 四列均为 LEFT JOIN 产出：两表无外键 ⇒ 孤儿引用与无客户标准价行如实输出 NULL（键必存在、值可空）。
  // 消费端显示约定：NULL 即空白，禁止 '未知'/'-' 造名，也禁止用本地缓存的 products/customers 回填
  // （列表显名唯一正解 = 后端 JOIN；防漂移锁 backend/tests/sales_price_read_enrichment_drift_test.rs）。
  product_name: string | null;
  product_code: string | null;
  customer_name: string | null;
  customer_code: string | null;
}

export function getSalesPriceList(params?: SalesPriceQuery): Promise<ApiResponse<SalesPriceRow[]>> {
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

// 审批「通过」请求体：对齐后端 sales_price_handler::ApprovePriceRequest（handlers/sales_price_handler.rs:43-50）。
// approved 为 `bool`（非 Option）⇒ 线上必带该键；handler:160-165 对 approved=false 直接 400
// "审批拒绝请提交至价格拒绝接口"，本端点只受理批准 ⇒ 类型收敛为字面量 true，杜绝误发 false。
// approval_reason 后端为 Option<String> 仅为了不脱化为无信封裸 400，必填语义在 handler:169-176 收口：
// 缺失/空串/纯空白一律 400 VALIDATION_ERROR，非空值真实落 sales_prices.approval_reason 列
// （services/sales_price_service.rs:408-412）⇒ 前端必带且必采（采集见 composables/useActionPrompts.ts）。
// 旧键 remark 已从后端 DTO 删除 ⇒ 不再声明（历史注释称"后端真实字段但不落库"，该前提已不成立）。
export interface ApproveSalesPriceRequest {
  approved: true;
  approval_reason: string;
}

/** 批准销售价格（pending → approved，状态门 services/sales_price_service.rs::approve_price）。 */
export function approveSalesPrice(
  id: number,
  data: ApproveSalesPriceRequest
): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-prices/${id}/approve`, data);
}

// 审批「拒绝」请求体：对齐后端 sales_price_handler::RejectPriceRequest（handlers/sales_price_handler.rs:52-56，
// reason: String 非 Option 且抽取器为强类型 `Json<T>` ⇒ 必带体，缺失即解码失败）。
// handler:200-204 对 trim 后空串回 400；非空值真实落 sales_prices.rejected_reason 列
// （services/sales_price_service.rs:460-461，状态门仅 pending 起拒）。
export interface RejectSalesPriceRequest {
  reason: string;
}

/** 拒绝销售价格（pending → rejected 终态，后端 routes/sales.rs:200-203 挂载）。 */
export function rejectSalesPrice(
  id: number,
  data: RejectSalesPriceRequest
): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-prices/${id}/reject`, data);
}

export function getPriceHistory(productId: number): Promise<ApiResponse<SalesPrice[]>> {
  return request.get(`/sales/sales-prices/history/${productId}`);
}
