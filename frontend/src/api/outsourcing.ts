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
 * （services/outsourcing_ops/types.rs）。委外单号 order_no 建单生成后不可改，后端更新结构无此字段，
 * 故此前用 Partial<CreateOutsourcingOrderPayload>（含 order_no）冒充更新契约是错的：order_no 被 serde 丢弃、
 * 且 standard_loss_rate/remarks 在类型层缺席。所有字段均 Option（未携带即不改）。
 * material_cost/issue_quantity/standard_loss_rate 为 rust_decimal 入参，以 JSON number 提交。
 */
export interface UpdateOutsourcingOrderPayload {
  order_type?: string;
  supplier_id?: number;
  production_order_id?: number;
  dye_batch_id?: number;
  color_no?: string;
  dye_lot_no?: string;
  issue_date?: string;
  expected_return_date?: string;
  issue_quantity?: number;
  issue_unit?: string;
  material_cost?: number;
  standard_loss_rate?: number;
  remarks?: string;
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
