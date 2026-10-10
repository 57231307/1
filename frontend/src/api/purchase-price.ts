import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表出参 = 后端 PurchasePriceView（JOIN 富化行），详情/历史出参 = purchase_price::Model。
// status 词表权威 = backend/src/models/status/sales.rs 的 price_approval（全小写四值），
// 与 DB CHECK chk_purchase_price_status 恰等；审批理由两列（approval_reason/rejected_reason，
// 由 m0079 加列）当前无消费方展示、列表读模型也不携带 ⇒ 本行型刻意不声明，补键只会造出恒空假列。
// 读接口 Decimal 线格式：后端 rust_decimal 序列化为 JSON 字符串 ⇒ 前端如实声明 string，
// 数值解析与定点格式化统一走 composables/ppFmts.ts 的 formatCurrency。
// 注：本类型刻意不 export——api/index.ts 对价目两域 `export *`，同名导出会触发 TS2308 重复导出。
type DecimalString = string;

export interface PurchasePrice {
  id: number;
  product_id: number;
  supplier_id: number;
  // price DECIMAL(18,6) NOT NULL（建表 m0009_add_purchase_extensions）
  price: DecimalString;
  currency: string;
  unit: string;
  // min_order_qty DECIMAL(12,2) NOT NULL（同上建表）⇒ NOT NULL 列前端接口不写 `?`
  min_order_qty: DecimalString;
  price_type: string;
  effective_date: string;
  expiry_date: string | null;
  // 取值域 = 后端 price_approval 词表四值（见文件头注），与 chk_purchase_price_status 恰等
  status: 'pending' | 'approved' | 'rejected' | 'inactive';
  approved_by: number | null;
  approved_at: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
  /** 需要后端 JOIN：purchase_prices.product_id -> products.product_name（表内无名列） */
  product_name?: string | null;
  /** 需要后端 JOIN：purchase_prices.product_id -> products.product_code（表内无编码列） */
  product_code?: string | null;
  /** 需要后端 JOIN：purchase_prices.supplier_id -> suppliers.supplier_name（表内无名列） */
  supplier_name?: string | null;
}

/**
 * GET /purchase/purchase-prices 查询参数，逐键对齐后端 PurchasePriceQuery（snake_case，全可选）。
 * 后端不读 keyword/product_name/supplier_name，发出即被丢弃、筛选恒无效。
 */
export interface PurchasePriceQueryParams {
  product_id?: number;
  supplier_id?: number;
  status?: string;
  page?: number;
  page_size?: number;
}

/**
 * 创建采购价格请求，逐字段对齐后端 CreatePurchasePriceInput。
 * unit/price_type 必填：后端列 NOT NULL 无 DB 默认，缺失即 400。
 * Decimal 入参写线格式：十进制字符串精确且全域；浮点字面量后端也接受，
 * 但 DECIMAL(18,6) 末位会在 float ~17 位有效数字中静默失真。
 * 本别名暂以 `string | number` 放宽（既有调用点 usePp.ts 以 number 提交）。
 */
type DecimalWire = string | number;

export interface CreatePurchasePricePayload {
  product_id: number;
  supplier_id: number;
  price: DecimalWire;
  unit: string;
  price_type: string;
  currency?: string;
  min_order_qty?: DecimalWire;
  effective_date?: string;
  expiry_date?: string;
}

/**
 * 更新采购价格请求，逐字段对齐后端 UpdatePriceRequest。
 * 后端 price 为 String（非数值），必须以字符串提交（否则反序列化失败）。
 * status 透传前由后端按 price_approval::ALL 白名单校验，前端如实收敛为词表四值。
 */
export interface UpdatePurchasePricePayload {
  price: string;
  expiry_date?: string;
  status?: 'pending' | 'approved' | 'rejected' | 'inactive';
}

export function getPurchasePriceList(
  params?: PurchasePriceQueryParams
  // 后端 list_prices 出参 ApiResponse<Vec<PurchasePriceView>>（富化行，见文件头注）⇒ data 为裸数组
): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get('/purchase/purchase-prices', { params });
}

export function getPurchasePrice(id: number): Promise<ApiResponse<PurchasePrice>> {
  return request.get(`/purchase/purchase-prices/${id}`);
}

export function createPurchasePrice(
  data: CreatePurchasePricePayload
): Promise<ApiResponse<PurchasePrice>> {
  return request.post('/purchase/purchase-prices', data);
}

export function updatePurchasePrice(
  id: number,
  data: UpdatePurchasePricePayload
): Promise<ApiResponse<null>> {
  return request.put(`/purchase/purchase-prices/${id}`, data);
}

/**
 * 审批「通过」请求体，对齐后端 ApprovePriceRequest（snake_case 同名）。
 * approved 后端为 bool 非 Option ⇒ 线上必带该键；该端点只受理批准，
 * approved=false 被后端直接拒绝并指向 reject 端点 ⇒ 类型收敛为字面量 true，禁止兼职放行拒绝。
 * approval_reason 必填（缺失/空串/纯空白一律 400），值落 purchase_prices.approval_reason 列
 * ⇒ 前端必带且必采（采集见 composables/useActionPrompts.ts）。
 */
export interface ApprovePurchasePriceRequest {
  approved: true;
  approval_reason: string;
}

/** 批准采购价格：POST /purchase/purchase-prices/{id}/approve，pending → approved（仅 pending 可批）。 */
export function approvePurchasePrice(
  id: number,
  data: ApprovePurchasePriceRequest
): Promise<ApiResponse<null>> {
  return request.post(`/purchase/purchase-prices/${id}/approve`, data);
}

/**
 * 审批「拒绝」请求体，对齐后端 RejectPriceRequest：reason 为 String 非 Option ⇒ 必带体；
 * trim 后空串被后端 400，非空值落 purchase_prices.rejected_reason 列。
 */
export interface RejectPurchasePriceRequest {
  reason: string;
}

/** 拒绝采购价格：POST /purchase/purchase-prices/{id}/reject，pending → rejected 审批终态（rejected/approved 均无回退边）。 */
export function rejectPurchasePrice(
  id: number,
  data: RejectPurchasePriceRequest
): Promise<ApiResponse<null>> {
  return request.post(`/purchase/purchase-prices/${id}/reject`, data);
}

export function deletePurchasePrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/purchase-prices/${id}`);
}

export function getPurchasePriceHistory(productId: number): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get(`/purchase/purchase-prices/history/${productId}`);
}
