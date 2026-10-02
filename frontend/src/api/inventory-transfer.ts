import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';
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
 * 调拨明细行出参：与后端 `backend/src/services/inv/mod.rs InventoryTransferItemDetail`
 * 逐字段对齐。面料四维（色号/缸号/批次/匹号）后端全部回传，
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
  /**
   * 匹号（出库第四维）：DB 可空列 inventory_transfer_items.piece_no（m0066）——
   * 白坯行为合法 NULL；染色布建单已强制必填，回显恒有值（mod.rs:110 Option<String>）。
   */
  piece_no: string | null;
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
 * 明细行入参：与后端 `services/inv/mod.rs InventoryTransferItemRequest` 对齐。
 * quantity/unit_cost 是 `Option<Decimal>`，按 e2e 造数口径以字符串提交避免精度丢失。
 * piece_no（出库第四维）：染色布（色号非空）必填——后端 fabric_class::normalize_outbound_piece_no
 * 强制，缺失回 400 VALIDATION_ERROR，且建单期 piece_domain_service::validate_dyed_piece_for_outbound
 * 按 产品+调出仓+缸号+批次+匹号 全 tuple 校验须命中真实 AVAILABLE 匹（BUSINESS 族）；
 * 白坯免填——无值时**省略该键**（禁发 null/空串占位），取值仅允许来自 GET /inventory/pieces。
 */
export interface InventoryTransferItemPayload {
  product_id?: number;
  quantity?: string;
  notes?: string;
  color_no?: string;
  dye_lot_no?: string;
  batch_no?: string;
  unit_cost?: string;
  piece_no?: string;
}

/**
 * 更新入参：与后端 `services/inv/mod.rs UpdateInventoryTransferRequest` 对齐——
 * 只有 status/notes/items 三字段，表单里改动的仓库与调拨日期不会被该端点接收。
 * 三态语义（RFC 7386 JSON Merge Patch）：键缺席=保持原值、显式 null=清空、有值=覆盖。
 * - status 映射非 Option 模型列：禁止送 null（后端 400「调拨状态不能清空」）；
 * - notes 为 DB 可空列（m0001 DDL）：清空须显式送 null；
 * - items 为明细整表替换数组：清空明细须传空数组 `[]`，不支持 `null` 清全表。
 */
export interface UpdateInventoryTransferPayload {
  status?: string;
  notes?: string | null;
  items?: InventoryTransferItemPayload[];
}

/**
 * 更新调拨明细入参：与后端 `services/inv/mod.rs UpdateInventoryTransferItemRequest` 对齐
 * （PUT /inventory/transfers/items/{item_id}）。三态语义：键缺席=保持、显式 null=清空
 * （仅 DB 可空列）、有值=覆盖。
 * - product_id/quantity/color_no/batch_no 映射 NOT NULL 列：禁止送 null
 *   （后端 400「XX不能清空：该字段为必填项」）；"色号改回白坯"提交空串而非 null；
 * - notes/unit_cost/dye_lot_no 为 DB 可空列：清空须显式送 null；
 *   染色布行（生效色号非空）清空缸号按四维追溯不变量被拒。
 * - piece_no 为 DB 可空列（Option<Option<String>>，mod.rs:217）：清空仅对白坯行合法，
 *   染色布行清空/缺匹号按四维追溯不变量被拒（400）。
 */
export interface UpdateTransferItemPayload {
  product_id?: number;
  quantity?: string;
  notes?: string | null;
  unit_cost?: string | null;
  color_no?: string;
  dye_lot_no?: string | null;
  batch_no?: string;
  piece_no?: string | null;
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

export function updateTransferItem(itemId: number, data: UpdateTransferItemPayload) {
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

/**
 * 匹状态词表（出库第四维选择器用到的取值）。唯一事实来源 = 后端写入方
 * `models/status/purchase_inventory.rs::inventory_piece`（大写 token，与
 * `handlers/inventory_piece_handler.rs` 的 PIECE_STATUS_DOMAIN 逐字符相同）。
 * 出库/调拨选择器只透传 AVAILABLE（现存可出库匹），状态集合语义由后端权威判定，
 * 前端不推断、不改写。
 */
export const INVENTORY_PIECE_STATUS = {
  AVAILABLE: 'AVAILABLE',
  RESERVED: 'RESERVED',
  SHIPPED: 'SHIPPED',
  DEFECT: 'DEFECT',
  UNAVAILABLE: 'UNAVAILABLE',
  SAMPLE: 'SAMPLE',
} as const;

/**
 * 可出库匹行：与后端 `handlers/inventory_piece_handler.rs:45 PieceResponse` 逐字段对齐
 * （GET /inventory/pieces -> PaginatedResponse<PieceResponse>）。
 * length/weight 是 rust_decimal `Decimal`，序列化为字符串，展示前 Number() 归一，禁 .toFixed 造数。
 * dye_lot_no/color_no 后端包 Some(...) 但类型 Option<String>，故 `string | null`。
 */
export interface InventoryPieceRow {
  id: number;
  piece_no: string;
  /** greige=生产匹 / dyed=染色匹 */
  piece_type: string;
  dye_lot_id: number | null;
  dye_lot_no: string | null;
  machine_no: string | null;
  machine_operator: string | null;
  warehouse_in_at: string | null;
  /** 匹长（Decimal 串） */
  length: string;
  /** 匹重（Decimal 串，可空） */
  weight: string | null;
  batch_no: string;
  color_no: string | null;
  product_id: number;
  warehouse_id: number;
  warehouse_name: string | null;
  warehouse_type: string | null;
  parent_piece_id: number | null;
  piece_seq: number | null;
  status: string;
  quality_status: string | null;
  created_at: string;
}

/** GET /inventory/pieces 查询参数（键与后端 ListPieceParams 同名，inventory_piece_handler.rs:22） */
export interface InventoryPieceQueryParams {
  page?: number;
  page_size?: number;
  piece_no?: string;
  piece_type?: string;
  product_id?: number;
  warehouse_id?: number;
  batch_no?: string;
  dye_lot_no?: string;
  /** 取值域 = INVENTORY_PIECE_STATUS；词表外后端 400 拒绝 */
  status?: string;
}

/**
 * 查询「该调出仓 + 该产品 + 该缸 + 该批」现存可出库真实匹（出库第四维数据源）。
 * status=AVAILABLE 由调用方下推——匹是否可出库是后端权威语义，此处只透传不判定。
 */
export function getAvailablePieces(params: InventoryPieceQueryParams) {
  return request.get<ApiResponse<PaginatedResponse<InventoryPieceRow>>>('/inventory/pieces', {
    params,
  });
}
