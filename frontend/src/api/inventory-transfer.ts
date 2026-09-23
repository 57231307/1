import { request } from './request';
import type { ApiResponse } from '@/types/api';
import type { ApproveTransferPayload } from './inventory';

/**
 * 库存调拨单出参形状：与后端 `backend/src/services/inv/mod.rs:37 InventoryTransferDetail`
 * 逐字段对齐（列表与详情共用同一结构：`services/inv/inventory_move.rs:79`（列表，items 恒为
 * 空数组）、`:111 get_transfer_detail`（详情带 items）；handler 直接
 * `serde_json::to_value(detail)`，见 `handlers/inventory_transfer_handler.rs:66/89`）。
 *
 * 数量/金额是 rust_decimal `Decimal`，序列化为字符串（与 StockAlertRow 同口径），展示前按需
 * `Number()` 转换。
 *
 * 末尾 4 个键后端当前不返回（该结构里没有名称列，也没有 total_amount），
 * 需按 `services/po/order_ops/crud.rs:463 PurchaseOrderDto` 的
 * `column_as + LeftJoin + into_model::<Dto>()` 范式补齐，补齐前对应列必然为空。
 */
export interface InventoryTransferEntity {
  id: number;
  transfer_no: string;
  from_warehouse_id: number;
  to_warehouse_id: number;
  /** DateTime<Utc>（NOT NULL） */
  transfer_date: string;
  /** 取值见 utils/inventory-transfer-status（models/status/purchase_inventory.rs:63） */
  status: string;
  total_quantity: string;
  notes: string | null;
  created_by: number | null;
  approved_by: number | null;
  approved_at: string | null;
  shipped_at: string | null;
  received_at: string | null;
  created_at: string;
  updated_at: string;
  /** 列表接口固定返回空数组，只有详情接口填实 */
  items: TransferItem[];
  /** 需后端 JOIN：`models/inventory_transfer.rs:45 Relation::FromWarehouse` → warehouses.warehouse_name */
  from_warehouse_name: string | null;
  /** 需后端 JOIN：`models/inventory_transfer.rs:51 Relation::ToWarehouse` → warehouses.warehouse_name */
  to_warehouse_name: string | null;
  /** 需后端 JOIN：`inventory_transfers.created_by` → users.real_name */
  created_by_name: string | null;
  /**
   * 需后端补出参：`models/inventory_transfer.rs:36` 有 total_amount 列（Decimal NOT NULL），
   * 但 `services/inv/mod.rs:37 InventoryTransferDetail` 未把它带出。
   */
  total_amount: string | null;
}

/**
 * 调拨明细行出参：与后端 `backend/src/services/inv/mod.rs:57 InventoryTransferItemDetail`
 * 逐字段对齐。面料四维（色号/缸号/批次）后端全部回传，
 * 产品主数据名称（code/name/等级/单位）不在该结构里。
 */
export interface TransferItem {
  id: number;
  transfer_id: number;
  product_id: number;
  quantity: string;
  shipped_quantity: string;
  received_quantity: string;
  unit_cost: string | null;
  notes: string | null;
  color_no: string;
  dye_lot_no: string | null;
  batch_no: string;
  created_at: string;
  updated_at: string;
  /** 需后端 JOIN：`inventory_transfer_items.product_id` → products.product_code */
  product_code: string | null;
  /** 需后端 JOIN：`inventory_transfer_items.product_id` → products.product_name */
  product_name: string | null;
  /** 需后端 JOIN：`inventory_transfer_items.product_id` → products.grade */
  grade: string | null;
  /** 需后端 JOIN：`inventory_transfer_items.product_id` → products.unit */
  unit: string | null;
}

/**
 * 建单入参：与后端 `services/inv/mod.rs:77 CreateInventoryTransferRequest` 对齐——
 * 该结构每个字段都是 `Option<…>`，故前端逐字段可选（缺字段由服务侧报参数缺失，
 * 不在界面上用默认值兜底）。`transfer_date` 是 `Option<DateTime<Utc>>`，
 * serde chrono 只接受 RFC3339（`YYYY-MM-DD` 反序列化失败 → 400），
 * 日期控件取值需 `new Date(v).toISOString()`。
 */
export interface CreateInventoryTransferPayload {
  from_warehouse_id?: number;
  to_warehouse_id?: number;
  transfer_date?: string;
  status?: string;
  notes?: string;
  items?: InventoryTransferItemPayload[];
}

/**
 * 明细行入参：与后端 `services/inv/mod.rs:87 InventoryTransferItemRequest` 对齐。
 * quantity/unit_cost 是 `Option<Decimal>`，按 e2e 造数口径以字符串提交避免精度丢失。
 */
export interface InventoryTransferItemPayload {
  product_id?: number;
  quantity?: string;
  notes?: string;
  color_no?: string;
  dye_lot_no?: string;
  batch_no?: string;
  unit_cost?: string;
}

/**
 * 更新入参：与后端 `services/inv/mod.rs:100 UpdateInventoryTransferRequest` 对齐——
 * 只有 status/notes/items 三字段，表单里改动的仓库与调拨日期不会被该端点接收。
 */
export interface UpdateInventoryTransferPayload {
  status?: string;
  notes?: string;
  items?: InventoryTransferItemPayload[];
}

// 列表查询参数键与后端 `handlers/inventory_transfer_handler.rs:23 InventoryTransferQuery`
// 同名（transfer_no 走 contains 模糊匹配）。
export interface InventoryTransferQueryParams {
  page?: number;
  page_size?: number;
  status?: string;
  from_warehouse_id?: number;
  to_warehouse_id?: number;
  transfer_no?: string;
}

export function getInventoryTransfer(id: number) {
  return request.get(`/inventory/transfers/${id}`);
}

export function createInventoryTransfer(data: CreateInventoryTransferPayload) {
  return request.post('/inventory/transfers', data);
}

export function updateInventoryTransfer(id: number, data: UpdateInventoryTransferPayload) {
  return request.put(`/inventory/transfers/${id}`, data);
}

export function deleteInventoryTransfer(id: number) {
  return request.delete(`/inventory/transfers/${id}`);
}

// 体形状同 api/inventory.ts::ApproveTransferPayload（后端 ApproveTransferRequest）
export function approveInventoryTransfer(id: number, data: ApproveTransferPayload) {
  return request.post(`/inventory/transfers/${id}/approve`, data);
}

export function getTransferItems(id: number) {
  return request.get(`/inventory/transfers/${id}`);
}

export function createTransferItem(id: number, data: InventoryTransferItemPayload) {
  return request.post(`/inventory/transfers/${id}/items`, data);
}

export function updateTransferItem(itemId: number, data: InventoryTransferItemPayload) {
  return request.put(`/inventory/transfers/items/${itemId}`, data);
}

export function deleteTransferItem(itemId: number) {
  return request.delete(`/inventory/transfers/items/${itemId}`);
}

/**
 * 生成库存调拨单号
 * GET /inventory/transfers/generate-no
 *
 * 单据号格式：`IT{yyyyMMdd}{4 位流水}`，例如 `IT202605140001`。
 * 后端通过 DocumentNumberGenerator 统计当日同前缀单据数量 + 1 计算流水。
 */
export const generateInventoryTransferNo = (): Promise<ApiResponse<{ transfer_no: string }>> =>
  request.get('/inventory/transfers/generate-no');
