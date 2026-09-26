import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 库存盘点单列表行出参：与后端
 * `backend/src/handlers/inventory_count_handler.rs:142 CountSummary` 逐字段对齐
 * （list_counts 在 :242 逐字段构造；信封键是 `counts`，见 :133 `CountListResponse`）。
 */
export interface InventoryCountEntity {
  id: number;
  count_no: string;
  warehouse_id: number;
  count_date: string;
  /** 取值见 utils/inventory-count-status（models/status/purchase_inventory.rs:82） */
  status: string;
  total_items: number;
  counted_items: number;
  variance_items: number;
  created_at: string;
  /** 需后端补出参：`CountSummary` 未包含 notes（详情 `CountResponse:58` 有） */
  notes: string | null;
  /**
   * 需后端补出参：主表有 `completed_at` 列（models/inventory_count.rs:25，Option，
   * 审批通过时由 finalize_count_completion 写入），但 `CountSummary` 未包含，
   * 详情结构 `CountResponse:62` 才有——故列表「完成时间」列恒空。
   */
  completed_at: string | null;
  /** 需后端 JOIN：`inventory_counts.warehouse_id`（models/inventory_count.rs:32 Relation::Warehouse）→ warehouses.name */
  warehouse_name: string | null;
  /** 需后端 JOIN：`inventory_counts.created_by`（models/inventory_count.rs:22）→ users.real_name */
  created_by_name: string | null;
}

/**
 * 盘点单详情出参：与后端 `handlers/inventory_count_handler.rs:49 CountResponse` 逐字段对齐
 * （get_count 返回该结构，含 items）。`warehouse_name`/`created_by_name` 仍需后端 JOIN。
 */
export interface InventoryCountDetail {
  id: number;
  count_no: string;
  warehouse_id: number;
  count_date: string;
  status: string;
  total_items: number;
  counted_items: number;
  variance_items: number;
  notes: string | null;
  created_by: number | null;
  approved_by: number | null;
  approved_at: string | null;
  completed_at: string | null;
  created_at: string;
  updated_at: string;
  items: CountItem[];
  /** 需后端 JOIN：→ warehouses.name */
  warehouse_name: string | null;
  /** 需后端 JOIN：→ users.real_name */
  created_by_name: string | null;
}

/**
 * 盘点明细行出参：与后端 `handlers/inventory_count_handler.rs:94 CountItemResponse`
 * 逐字段对齐。数量是 rust_decimal `Decimal`（序列化为字符串）。
 */
export interface CountItem {
  id: number;
  count_id: number;
  stock_id: number;
  product_id: number;
  warehouse_id: number;
  quantity_before: string;
  quantity_actual: string;
  quantity_difference: string;
  unit_cost: string;
  total_cost: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
  /** 需后端 JOIN：`inventory_count_items.product_id` → products.name */
  product_name: string | null;
  /** 需后端 JOIN：同上 → products.code */
  product_code: string | null;
}

/**
 * 建单入参：与后端 `handlers/inventory_count_handler.rs:37 CreateCountPayload` 对齐。
 * `count_date` 由 handler `.parse::<DateTime<Utc>>()`（:202）解析，只接受 RFC3339；
 * 单号由后端生成（:214 create_count → generate_count_no），前端不提交 count_no；
 * 初始状态由服务写 pending（inventory_count_service.rs:155），前端不提交 status。
 */
export interface CreateInventoryCountPayload {
  warehouse_id: number;
  count_date: string;
  notes?: string;
  stock_ids?: number[];
}

/** 更新入参：与后端 `handlers/inventory_count_handler.rs:190 UpdateCountPayload` 对齐 */
export interface UpdateInventoryCountPayload {
  count_date?: string;
  notes?: string;
}

/** 实盘录入行入参：与后端 `handlers/inventory_count_handler.rs:180 RecordItemInput` 对齐（数量是字符串） */
export interface RecordCountItemPayload {
  stock_id: number;
  quantity_actual: string;
  notes?: string | null;
}

/**
 * 列表查询参数：与后端 `handlers/inventory_count_handler.rs:157 ListCountsParams` 对齐
 * （只有 page/page_size/warehouse_id/status；单号 count_no 后端不接收，提交后被 serde 丢弃，
 * 即单号筛选恒返回全量）。见交付报告「需要后端补入参」。
 */
export interface InventoryCountQueryParams {
  page?: number;
  page_size?: number;
  warehouse_id?: number;
  status?: string;
}

export const getInventoryCountList = (params?: InventoryCountQueryParams) =>
  request.get('/inventory/counts', { params });

export const getInventoryCount = (id: number) => request.get(`/inventory/counts/${id}`);

export const createInventoryCount = (data: CreateInventoryCountPayload) =>
  request.post('/inventory/counts', data);

export const updateInventoryCount = (id: number, data: UpdateInventoryCountPayload) =>
  request.put(`/inventory/counts/${id}`, data);

export const deleteInventoryCount = (id: number) => request.delete(`/inventory/counts/${id}`);

export const approveInventoryCount = (id: number) =>
  request.post(`/inventory/counts/${id}/approve`);

/// 提交盘点明细（POST /counts/:id/record），对齐后端 record_count_items 端点
export const recordCountItems = (id: number, items: RecordCountItemPayload[]) =>
  request.post(`/inventory/counts/${id}/record`, { items });

/// 提交盘点单审批（POST /counts/:id/submit），对齐后端 submit_for_approval 端点（写 in_review）
export const submitInventoryCount = (id: number) => request.post(`/inventory/counts/${id}/submit`);

/// 驳回盘点单（POST /counts/:id/reject）：后端 reject_count 只有 Path<i32>，无 JSON 体
export const rejectInventoryCount = (id: number) => request.post(`/inventory/counts/${id}/reject`);

export const getCountItems = (id: number) => request.get(`/inventory/counts/${id}`);

export const updateCountItem = (
  itemId: number,
  data: { quantity_actual?: string; notes?: string | null }
) => request.put(`/inventory/counts/items/${itemId}`, data);

export const deleteCountItem = (itemId: number) =>
  request.delete(`/inventory/counts/items/${itemId}`);

/**
 * 生成库存盘点单号
 * GET /inventory/counts/generate-no
 *
 * 单据号格式：`IC{yyyyMMdd}{4 位流水}`，例如 `IC202605140001`。
 * 后端通过 DocumentNumberGenerator 统计当日同前缀单据数量 + 1 计算流水。
 */
export const generateInventoryCountNo = (): Promise<ApiResponse<{ count_no: string }>> =>
  request.get('/inventory/counts/generate-no');
