import { request } from './request';

export interface OutsourcingOrder {
  id: number;
  order_no: string;
  order_type: string;
  status: string;
  [key: string]: unknown;
}

export interface CreateOutsourcingOrderPayload {
  order_no: string;
  order_type: string;
  supplier_id: number;
  production_order_id?: number;
  dye_batch_id?: number;
  color_no?: string;
  dye_lot_no?: string;
  issue_date: string;
  expected_return_date?: string;
  issue_quantity: number;
  issue_unit?: string;
  // 后端 CreateOutsourcingOrderRequest.material_cost 为 rust_decimal::Decimal 必填字段，
  // 与本文件其它金额字段(issue_quantity/unit_cost 等)一致，统一以 JSON number 传参序列化。
  material_cost: number;
  /**
   * 加工费/运费/进项税额：outsourcing_order 表 NOT NULL DECIMAL(14,4) 真实列
   * （backend migration/src/domain/v15/mod.rs:3247-3249）。后端 Create DTO 类型层
   * 非 Option（NOT NULL 列不标可选，显式 null 被 serde 类型校验拒绝）；
   * 建单缺省键=0 起步（serde(default)，与后端建单初始化同值），真实值须随单提交或
   * draft 期经 updateOutsourcingOrder 补录——结算 FEE 凭证金额=加工费+运费，
   * 不录入则成本链恒 0。
   */
  processing_fee?: number;
  freight_fee?: number;
  tax_amount?: number;
}

export function getOutsourcingOrderList(params?: Record<string, unknown>) {
  return request.get('/production/outsourcing-orders', { params });
}

export function createOutsourcingOrder(data: CreateOutsourcingOrderPayload) {
  return request.post('/production/outsourcing-orders', data);
}

export function getOutsourcingOrderByNo(no: string) {
  return request.get(`/production/outsourcing-orders/by-no/${no}`);
}

export function getOutsourcingOrderDetail(id: number) {
  return request.get(`/production/outsourcing-orders/${id}`);
}

/**
 * 更新委外订单载荷 —— 逐字段对齐后端 UpdateOutsourcingOrderRequest
 * （services/outsourcing_ops/types.rs，三态语义 RFC 7386 JSON Merge Patch）：
 * 键缺席=保持原值、显式 null=清空为 NULL（仅下方声明 `| null` 的 DB 可空列）、有值=覆盖。
 * 可空列依据 v15 outsourcing_order DDL（production_order_id/dye_batch_id/color_no/dye_lot_no/
 * expected_return_date/standard_loss_rate/remarks）。
 * NOT NULL 列（order_type/supplier_id/issue_date/issue_quantity/issue_unit/material_cost/
 * processing_fee/freight_fee/tax_amount）不声明 null——显式 null 会被后端
 * business_displayable 拒绝（"XX不能清空：该字段为必填项"）。
 * 委外单号 order_no 建单生成后不可改，后端更新结构无此字段，不得提交。
 * material_cost/issue_quantity/standard_loss_rate/三费 为 rust_decimal 入参，以 JSON number 提交。
 */
export interface UpdateOutsourcingOrderPayload {
  order_type?: string;
  supplier_id?: number;
  production_order_id?: number | null;
  dye_batch_id?: number | null;
  color_no?: string | null;
  dye_lot_no?: string | null;
  issue_date?: string;
  expected_return_date?: string | null;
  issue_quantity?: number;
  issue_unit?: string;
  material_cost?: number;
  /** 加工费（NOT NULL 列，后端 v15:3247）：有值覆盖/键缺席保持，禁显式 null */
  processing_fee?: number;
  /** 运费（NOT NULL 列，后端 v15:3248）：同上 */
  freight_fee?: number;
  /** 进项税额（NOT NULL 列，后端 v15:3249）：同上，结算 FEE 凭证 tax_amount 来源 */
  tax_amount?: number;
  standard_loss_rate?: number | null;
  remarks?: string | null;
}

export function updateOutsourcingOrder(id: number, data: UpdateOutsourcingOrderPayload) {
  return request.put(`/production/outsourcing-orders/${id}`, data);
}

export function deleteOutsourcingOrder(id: number) {
  return request.delete(`/production/outsourcing-orders/${id}`);
}

// 后端 issue/processing/settle handler 仅 Path(id) + State，无 Json<T> 提取器（状态机迁移，
// 载荷不参与业务），此前前端发送 `data ?? {}` 空体属多余请求体（被 axum 丢弃但契约占位）——
// 状态词表小写 received/settled/processing（models/status/wage_energy_chemical_business.rs:261），
// 前端比较值须逐字符一致。三端点均不带请求体调用。
export function issueOutsourcingOrder(id: number) {
  return request.post(`/production/outsourcing-orders/${id}/issue`);
}

export function processOutsourcingOrder(id: number) {
  return request.post(`/production/outsourcing-orders/${id}/processing`);
}

export function settleOutsourcingOrder(id: number) {
  return request.post(`/production/outsourcing-orders/${id}/settle`);
}

export function closeOutsourcingOrder(id: number) {
  return request.post(`/production/outsourcing-orders/${id}/close`);
}

export function cancelOutsourcingOrder(id: number) {
  return request.post(`/production/outsourcing-orders/${id}/cancel`);
}

export function getOutsourcingItems(orderId: number) {
  return request.get(`/production/outsourcing-orders/items/by-order/${orderId}`);
}

export function createOutsourcingItem(orderId: number, data: Record<string, unknown>) {
  return request.post('/production/outsourcing-orders/items', {
    outsourcing_order_id: orderId,
    ...data,
  });
}

export interface OutsourcingReceipt {
  id: number;
  status: string;
  [key: string]: unknown;
}

export function getOutsourcingReceiptList(params?: Record<string, unknown>) {
  return request.get('/production/outsourcing-receipts', { params });
}

export function createOutsourcingReceipt(data: Record<string, unknown>) {
  return request.post('/production/outsourcing-receipts', data);
}

export function confirmOutsourcingReceipt(id: number) {
  return request.post(`/production/outsourcing-receipts/${id}/confirm`);
}
