import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 读接口 Decimal 线格式：后端 rust_decimal 序列化为 JSON 字符串 ⇒ 前端如实声明 string，
// 数值解析与定点格式化统一走 composables/spFmts.ts 的 formatCurrency。
export type DecimalString = string;

// 读接口基型 SalesPrice = 后端 sales_price::Model 逐列映射；列表端点出参为富化读模型
// SalesPriceView（Model 全列 + JOIN 四列名称），其行型即下方 SalesPriceRow。
// 审批理由两列（approval_reason/rejected_reason，m0079 加列）刻意不声明：
// 当前无消费方展示、列表读模型也不携带，补键只会造出恒空的假列。
export interface SalesPrice {
  id: number;
  product_id: number;
  // customer_id 可空（标准价行无客户 ⇒ 响应为 null）
  customer_id: number | null;
  // price DECIMAL(18,6) NOT NULL（建表 m0011_add_sales_and_logistics_extensions）
  price: DecimalString;
  currency: string;
  unit: string;
  // min_order_qty DECIMAL(12,2) NOT NULL（同上建表）⇒ NOT NULL 列前端接口不写 `?`
  min_order_qty: DecimalString;
  price_type?: string;
  price_level?: string;
  effective_date: string;
  // 后端 Option<NaiveDate> ⇒ 响应可为 null
  expiry_date: string | null;
  // 销售侧取值全集 = pending/approved/rejected（词表权威 backend/src/models/status/sales.rs::price_approval；
  // inactive 无销售侧生产者，DB CHECK chk_sales_price_status 与本三值恰等，列表筛选白名单同集）
  status: 'pending' | 'approved' | 'rejected';
  created_at: string;
  updated_at: string;
}

// 写接口入参：逐字段对齐后端 CreateSalesPriceInput / UpdateSalesPriceInput，
// 不声明后端不接收的键（status/remark(s) 等在 DTO 中不存在，发出即死键）。
// price/min_order_qty 写线格式 = string：后端 Decimal 对整数也精确、对浮点不拒但丢精度，
// 十进制字符串是唯一精确且全域的形态；编辑态（el-input-number）为 number，
// 由 composables/useSp.ts 在提交边界转十进制字符串；缺值必须省略键（不许伪造 0/''）。
export interface SalesPriceCreateInput {
  product_id: number;
  customer_id?: number | null;
  customer_type?: string | null;
  // 后端 Decimal 非 Option ⇒ 必填；十进制字符串（如 "12.500000"、"0"）
  price: string;
  currency?: string;
  unit: string;
  price_type: string;
  // 后端 Option<Decimal>：缺省 = 不写键，create 路径由服务端主动写 0（非 DB 默认）
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

// 列表/导出共用查询入参（后端 SalesPriceQuery：全字段可选、snake_case，未知键静默忽略）。
// customer_id 等值筛选、keyword 为「产品名称/客户名称」模糊（后端真实接收并生效）。
// "未选"的编码 = 省略键：空串/纯空白由前端 serializeParams 与后端 normalize_empty_query_params 双侧剔除。
// download_token = 敏感导出 fail-closed 审批令牌，页面未暴露仍如实声明。
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

// 列表行 = SalesPrice 全列 + 四个 JOIN 名列（后端 SalesPriceView）。
export interface SalesPriceRow extends SalesPrice {
  // 四列均为后端 LEFT JOIN 产出：孤儿引用与无客户标准价行如实输出 NULL（键必存在、值可空）。
  // 显示约定：NULL 即空白——禁止 '未知'/'-' 造名，禁止前端本地缓存回填；列表显名唯一正解 = 后端 JOIN。
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

// 审批「通过」请求体，对齐后端 ApprovePriceRequest：approved 为 bool 必带该键，
// 该端点只受理批准（approved=false 直接 400 并指向 reject 端点）⇒ 类型收敛为字面量 true，
// 禁止用本端点兼职放行拒绝。approval_reason 必填（缺失/空串/纯空白一律 400），
// 值落 sales_prices.approval_reason 列 ⇒ 前端必带且必采（采集见 composables/useActionPrompts.ts）。
export interface ApproveSalesPriceRequest {
  approved: true;
  approval_reason: string;
}

/** 批准销售价格：POST /sales/sales-prices/{id}/approve，pending → approved（仅 pending 可批）。 */
export function approveSalesPrice(
  id: number,
  data: ApproveSalesPriceRequest
): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-prices/${id}/approve`, data);
}

// 审批「拒绝」请求体，对齐后端 RejectPriceRequest：reason 为 String 非 Option ⇒ 必带体；
// trim 后空串 400，非空值落 sales_prices.rejected_reason 列。
export interface RejectSalesPriceRequest {
  reason: string;
}

/** 拒绝销售价格：POST /sales/sales-prices/{id}/reject，pending → rejected 审批终态（与 approved 同为不可回退结论态）。 */
export function rejectSalesPrice(
  id: number,
  data: RejectSalesPriceRequest
): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-prices/${id}/reject`, data);
}

export function getPriceHistory(productId: number): Promise<ApiResponse<SalesPrice[]>> {
  return request.get(`/sales/sales-prices/history/${productId}`);
}
