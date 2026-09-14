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
}

export function getOutsourcingOrderList(params?: Record<string, unknown>) {
  return request.get('/outsourcing-orders', { params });
}

export function createOutsourcingOrder(data: CreateOutsourcingOrderPayload) {
  return request.post('/outsourcing-orders', data);
}

export function getOutsourcingOrderByNo(no: string) {
  return request.get(`/outsourcing-orders/by-no/${no}`);
}

export function getOutsourcingOrderDetail(id: number) {
  return request.get(`/outsourcing-orders/${id}`);
}

export function updateOutsourcingOrder(id: number, data: Partial<CreateOutsourcingOrderPayload>) {
  return request.put(`/outsourcing-orders/${id}`, data);
}

export function deleteOutsourcingOrder(id: number) {
  return request.delete(`/outsourcing-orders/${id}`);
}

export function issueOutsourcingOrder(id: number, data?: Record<string, unknown>) {
  return request.post(`/outsourcing-orders/${id}/issue`, data ?? {});
}

export function processOutsourcingOrder(id: number, data?: Record<string, unknown>) {
  return request.post(`/outsourcing-orders/${id}/processing`, data ?? {});
}

export function settleOutsourcingOrder(id: number, data?: Record<string, unknown>) {
  return request.post(`/outsourcing-orders/${id}/settle`, data ?? {});
}

export function closeOutsourcingOrder(id: number) {
  return request.post(`/outsourcing-orders/${id}/close`);
}

export function cancelOutsourcingOrder(id: number) {
  return request.post(`/outsourcing-orders/${id}/cancel`);
}

export function getOutsourcingItems(orderId: number) {
  return request.get(`/outsourcing-orders/items/by-order/${orderId}`);
}

export function createOutsourcingItem(orderId: number, data: Record<string, unknown>) {
  return request.post('/outsourcing-orders/items', { outsourcing_order_id: orderId, ...data });
}

export interface OutsourcingReceipt {
  id: number;
  status: string;
  [key: string]: unknown;
}

export function getOutsourcingReceiptList(params?: Record<string, unknown>) {
  return request.get('/outsourcing-receipts', { params });
}

export function createOutsourcingReceipt(data: Record<string, unknown>) {
  return request.post('/outsourcing-receipts', data);
}

export function confirmOutsourcingReceipt(id: number) {
  return request.post(`/outsourcing-receipts/${id}/confirm`);
}

export const OUTSOURCING_STATUS_LABEL: Record<string, string> = {
  draft: '草稿',
  issued: '已发出',
  processing: '加工中',
  settled: '已结算',
  closed: '已关闭',
  cancelled: '已取消',
};
