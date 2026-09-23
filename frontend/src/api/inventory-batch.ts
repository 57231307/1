import { request } from './request';

/**
 * 库存批次出参 = `PaginatedResponse<inventory_stock::Model>`：列表 handler
 * （`backend/src/handlers/inventory_batch_handler.rs` list_batches →
 * `backend/src/services/inventory_stock_service.rs` list_batches 直接返回
 * `Vec<inventory_stock::Model>`，无 JOIN）逐字段等于
 * `backend/src/models/inventory_stock.rs` 的列名（snake_case）。
 *
 * `product_name` / `warehouse_name` 两列 inventory_stocks 表并不存在，由后端在
 * `services/inventory_stock_service.rs::list_batches` 里按 Relation::Product /
 * Relation::Warehouse 做单次 LEFT JOIN（`column_as + into_model::<InventoryBatchView>`，
 * 照抄 PurchaseOrderDto 范式、无 N+1）富化后返回，`Option<String>` 承载。
 * 注意：详情端点 `get_batch` 返回裸 `inventory_stock::Model`（无 JOIN），此两字段缺省。
 *
 * 数量类字段（quantity_* / gram_weight / width）后端为 `Decimal`，`rust_decimal` 未启用
 * serde-with-float，序列化为 JSON 字符串（如 "1250.00"），故类型标注为 `string`；
 * 展示需格式化、回传给 f64 入参处需显式 `Number(...)`。
 */
export interface InventoryBatch {
  id: number;
  batch_no: string;
  product_id: number;
  warehouse_id: number;
  color_no: string;
  grade: string;
  quantity_on_hand: string;
  quantity_available: string;
  quantity_reserved: string;
  quantity_incoming: string;
  quantity_meters: string;
  quantity_kg: string;
  stock_status: string;
  quality_status: string;
  created_at: string;
  updated_at: string;
  dye_lot_no?: string | null;
  gram_weight?: string | null;
  width?: string | null;
  production_date?: string | null;
  expiry_date?: string | null;
  product_name?: string | null;
  warehouse_name?: string | null;
}

/** 创建批次：字段与 `inventory_batch_handler.rs` 的 `CreateBatchRequest`（无 rename_all，wire 为 snake_case）逐一同名 */
export interface CreateBatchRequest {
  batch_no: string;
  product_id: number;
  warehouse_id: number;
  color_no: string;
  color_name?: string;
  dye_lot_no?: string;
  grade: string;
  quantity_meters: number;
  quantity_kg: number;
  gram_weight?: number;
  width?: number;
  production_date?: string;
  expiry_date?: string;
  supplier_id?: number;
  purchase_order_no?: string;
  remarks?: string;
}

/** 更新批次：字段对齐 `inventory_batch_handler.rs` 的 `UpdateBatchRequest`（全部可选） */
export interface UpdateBatchRequest {
  color_no?: string;
  dye_lot_no?: string;
  grade?: string;
  gram_weight?: number;
  width?: number;
  expiry_date?: string;
  remarks?: string;
  stock_status?: string;
  quality_status?: string;
}

/** 批次调拨：字段对齐 `inventory_batch_handler.rs` 的 `TransferBatchRequest` */
export interface TransferBatchRequest {
  from_warehouse_id: number;
  to_warehouse_id: number;
  quantity_meters: number;
  quantity_kg: number;
  remarks?: string;
}

/** 列表查询参数：与 `inventory_batch_handler.rs` 的 `BatchListQuery` 同名字段集（无 keyword/qualityStatus 之类后端不接受的键） */
export interface InventoryBatchQueryParams {
  page?: number;
  page_size?: number;
  product_id?: number;
  batch_no?: string;
  color_no?: string;
  grade?: string;
  warehouse_id?: number;
  start_date?: string;
  end_date?: string;
}

export function getBatchList(params?: InventoryBatchQueryParams) {
  return request.get('/inventory/batches', { params });
}

export function getBatch(id: number) {
  return request.get(`/inventory/batches/${id}`);
}

export function createBatch(data: CreateBatchRequest) {
  return request.post('/inventory/batches', data);
}

export function updateBatch(id: number, data: UpdateBatchRequest) {
  return request.put(`/inventory/batches/${id}`, data);
}

export function deleteBatch(id: number) {
  return request.delete(`/inventory/batches/${id}`);
}

export function transferBatch(id: number, data: TransferBatchRequest) {
  return request.post(`/inventory/batches/${id}/transfer`, data);
}
