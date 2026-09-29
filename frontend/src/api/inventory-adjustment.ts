import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 库存调整单列表行出参：与后端
 * `backend/src/handlers/inventory_adjustment_handler.rs:85 AdjustmentSummary` 逐字段对齐
 * （list_adjustments 在 :311 逐字段构造该结构，信封键是 `adjustments`，见 :76
 * `AdjustmentListResponse{adjustments,total,page,page_size}`）。
 *
 * 主表 `models/inventory_adjustment.rs` 没有金额列，只有 `total_quantity`，
 * 因此列表不存在「金额」这一可回显字段（只有明细行的 amount = unit_cost × quantity）。
 * 数量是 rust_decimal `Decimal`，序列化为字符串，展示前按需 `Number()` 转换。
 */
export interface InventoryAdjustmentEntity {
  id: number;
  adjustment_no: string;
  warehouse_id: number;
  /** increase / decrease（models/inventory_adjustment.rs:28 注释的取值域） */
  adjustment_type: string;
  /** damage / sample / correction / other（models/inventory_adjustment.rs:31 注释的取值域） */
  reason_type: string;
  /** 取值见 utils/inventory-adjustment-status（models/status/purchase_inventory.rs:124） */
  status: string;
  total_quantity: string;
  created_at: string;
  /**
   * 需后端补出参：`AdjustmentSummary` 未包含 `adjustment_date`
   * （列存在：models/inventory_adjustment.rs:25，详情结构 AdjustmentResponse:49 有返回）。
   */
  adjustment_date: string | null;
  /** 需后端补出参：`AdjustmentResponse:52` 有 reason_description，`AdjustmentSummary` 未包含 */
  reason_description: string | null;
  /** 需后端 JOIN：`inventory_adjustments.warehouse_id` → warehouses.name */
  warehouse_name: string | null;
  /** 需后端 JOIN：`inventory_adjustments.created_by`（models/inventory_adjustment.rs:43）→ users.real_name */
  created_by_name: string | null;
}

/**
 * 调整明细行出参：与后端 `handlers/inventory_adjustment_handler.rs:62 AdjustmentItemResponse`
 * 逐字段对齐（id/stock_id/quantity/quantity_before/quantity_after/unit_cost/amount/notes）。
 * 明细只带 `stock_id`，产品主数据名称需后端 JOIN inventory_stocks → products。
 */
export interface AdjustmentItem {
  id: number;
  stock_id: number;
  quantity: string;
  quantity_before: string;
  quantity_after: string;
  unit_cost: string | null;
  amount: string | null;
  notes: string | null;
  /** 需后端 JOIN：`adjustment_items.stock_id` → inventory_stocks.product_id → products.name */
  product_name: string | null;
  /** 需后端 JOIN：同上 → products.code */
  product_code: string | null;
}

/**
 * 建单入参：与后端 `handlers/inventory_adjustment_handler.rs:23 CreateAdjustmentRequestPayload`
 * 对齐。`adjustment_date` 由 handler 用 `DateTime::parse_from_rfc3339` 口径的
 * `.parse::<DateTime<Utc>>()` 解析（:105），只接受 RFC3339；明细数量是字符串（:37）。
 * 单号由后端 `generate_adjustment_no()` 生成，前端不提交。
 */
export interface CreateInventoryAdjustmentPayload {
  warehouse_id: number;
  adjustment_date: string;
  adjustment_type: string;
  reason_type: string;
  reason_description?: string;
  notes?: string;
  items: AdjustmentItemPayload[];
}

/** 更新入参：与后端 `handlers/inventory_adjustment_handler.rs:382 UpdateAdjustmentRequestPayload` 对齐 */
export interface UpdateInventoryAdjustmentPayload {
  warehouse_id?: number;
  adjustment_date?: string;
  adjustment_type?: string;
  reason_type?: string;
  reason_description?: string;
  notes?: string;
}

/** 明细行入参：与后端 `handlers/inventory_adjustment_handler.rs:35 AdjustmentItemPayload` 对齐 */
export interface AdjustmentItemPayload {
  stock_id: number;
  quantity: string;
  unit_cost?: string;
  notes?: string;
}

/**
 * 列表查询参数：与后端 `handlers/inventory_adjustment_handler.rs:330 ListAdjustmentsParams`
 * 对齐——该结构当前只有 page/page_size，单号与状态筛选后端未接收（提交后被 serde 丢弃，
 * 即筛选恒返回全量）。见交付报告「需要后端补入参」。
 */
export interface InventoryAdjustmentQueryParams {
  page?: number;
  page_size?: number;
  adjustment_no?: string;
  status?: string;
}

export function getInventoryAdjustmentList(params?: InventoryAdjustmentQueryParams) {
  return request.get('/inventory/adjustments', { params });
}

export function getInventoryAdjustment(id: number) {
  return request.get(`/inventory/adjustments/${id}`);
}

export function createInventoryAdjustment(data: CreateInventoryAdjustmentPayload) {
  return request.post('/inventory/adjustments', data);
}

export function updateInventoryAdjustment(id: number, data: UpdateInventoryAdjustmentPayload) {
  return request.put(`/inventory/adjustments/${id}`, data);
}

export function deleteInventoryAdjustment(id: number) {
  return request.delete(`/inventory/adjustments/${id}`);
}

export function approveInventoryAdjustment(id: number) {
  return request.post(`/inventory/adjustments/${id}/approve`);
}

export function rejectInventoryAdjustment(id: number) {
  return request.post(`/inventory/adjustments/${id}/reject`);
}

export function getAdjustmentItems(id: number) {
  return request.get(`/inventory/adjustments/${id}`);
}

export function createAdjustmentItem(id: number, data: AdjustmentItemPayload) {
  return request.post(`/inventory/adjustments/${id}/items`, data);
}

export function updateAdjustmentItem(itemId: number, data: AdjustmentItemPayload) {
  return request.put(`/inventory/adjustments/items/${itemId}`, data);
}

export function deleteAdjustmentItem(itemId: number) {
  return request.delete(`/inventory/adjustments/items/${itemId}`);
}

/**
 * 生成库存调整单号
 * GET /inventory/adjustments/generate-no
 *
 * 单据号格式：`IA{yyyyMMdd}{4 位流水}`，例如 `IA202605140001`。
 * 后端通过 DocumentNumberGenerator 统计当日同前缀单据数量 + 1 计算流水。
 */
export const generateInventoryAdjustmentNo = (): Promise<ApiResponse<{ adjustment_no: string }>> =>
  request.get('/inventory/adjustments/generate-no');
