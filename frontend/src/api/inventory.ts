import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 库存行：字段名与后端 StockResponse 对齐
 * （数量按主/辅计量与四个业务量维度分开，面料维度为批次/色号/缸号/等级）。
 * 数量类字段是 rust_decimal 的字符串序列化（StockResponse 全列 Decimal，serde 输出为
 * 字符串；与 StockAlert 同口径），展示/计算前必须 Number() 归一，禁止直接 .toFixed。
 */
export interface InventoryStock {
  id: number;
  warehouse_id: number;
  product_id: number;
  /** 在库量（Decimal 串） */
  quantity_on_hand: string;
  /** 可用量（在库 - 预留，Decimal 串） */
  quantity_available: string;
  quantity_reserved: string;
  /** 已发货量（销售发货累计，Decimal 串） */
  quantity_shipped: string;
  /** 在途量（采购收货累计，Decimal 串） */
  quantity_incoming: string;
  reorder_point: string;
  max_stock_point: string;
  bin_location?: string | null;
  // ===== 面料四维 =====
  /** 批次/匹号：NOT NULL，出入库四维追溯恒必填（inv/fabric_class.rs::validate_fabric_trace） */
  batch_no: string;
  /** 色号：NOT NULL 列，空串=白坯（合法缺省，不是缺数据） */
  color_no: string;
  /** 缸号：白坯合法为 null；染色布（color_no 非空）后端强校验非空 */
  dye_lot_no?: string | null;
  grade: string;
  /** 台账状态：正常/报废/已删除（后端 inventory_stocks.stock_status 主数据值，见 constants/inventory-stock-status） */
  stock_status: string;
  /** 质量状态：合格/不合格/待检 */
  quality_status: string;
  quantity_meters: string;
  quantity_kg: string;
  /**
   * 乐观锁版本号（NOT NULL 列 inventory_stocks.version，StockResponse 直映输出）——
   * PUT /inventory/stock/{id} 必带的 version 唯一合法来源，缺它编辑链路不可达，禁止假值。
   */
  version: number;
  // ===== 主数据名称（后端按 ID 批量带出）=====
  product_code?: string | null;
  product_name?: string | null;
  warehouse_name?: string | null;
  created_at?: string;
  updated_at?: string;
}

/**
 * 预留列表行 = 后端 `inventory_reservation::Model` 直接序列化
 * （handlers/inventory_reservation_handler.rs::list_reservations → service 返回 Model 列表）。
 * 后端出参不含产品名/仓库名/订单号（无 JOIN 富化，名称富化缺口已列后端待串行清单），
 * 前端不得虚构这些键；quantity 为 Decimal 字符串，展示前 Number() 归一。
 * status 词表写入方 = models/status/purchase_inventory.rs::inventory_reservation（小写）：
 * pending/locked/consumed/released/cancelled。
 */
export interface InventoryReservation {
  id: number;
  order_id: number;
  product_id: number;
  warehouse_id: number;
  /** 预留数量（Decimal 串） */
  quantity: string;
  status: string;
  reserved_at: string;
  released_at: string | null;
  notes: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/**
 * 创建/锁定/释放预留的出参 = 后端 ReservationResponse
 * （handlers/inventory_reservation_handler.rs，比列表 Model 少 reserved_at/released_at/created_by 键，
 * 按真实形状声明，不复用 InventoryReservation 假装同形）。
 */
export interface ReservationMutationResponse {
  id: number;
  order_id: number;
  product_id: number;
  warehouse_id: number;
  quantity: string;
  status: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * 调拨单列表行：键名对应 services/inv/mod.rs 的 InventoryTransferDetail，
 * 仓库名与创建人名由 handlers/inventory_transfer_handler.rs 的 LEFT JOIN 产出，
 * 外键悬挂时为 null（行仍保留），故三处名称列可空；created_by 后端为 Option<i32>。
 * total_quantity/total_amount 为 Decimal 字符串，展示/计算前 Number() 归一。
 * 列表端点 items 恒为空数组（明细仅在详情端点返回）。
 */
export interface InventoryTransfer {
  id: number;
  transfer_no: string;
  from_warehouse_id: number;
  from_warehouse_name: string | null;
  to_warehouse_id: number;
  to_warehouse_name: string | null;
  transfer_date: string;
  /** pending/approved/rejected/shipped/completed（小写） */
  status: string;
  total_quantity: string;
  total_amount: string;
  notes: string | null;
  created_by: number | null;
  created_by_name: string | null;
  approved_by: number | null;
  approved_at: string | null;
  shipped_at: string | null;
  received_at: string | null;
  created_at: string;
  updated_at: string;
  items: InventoryTransferItem[];
}

/** 调拨明细行：InventoryTransferItemDetail 列 + products LEFT JOIN 的品名/编号/等级/单位；
 *  数量类为 Decimal 字符串，unit_cost 后端 Option → 出参 string | null */
export interface InventoryTransferItem {
  id: number;
  transfer_id: number;
  product_id: number;
  quantity: string;
  shipped_quantity: string;
  received_quantity: string;
  unit_cost: string | null;
  notes: string | null;
  /** 色号：NOT NULL，白坯为空串 */
  color_no: string;
  /** 缸号：白坯可为 null；染色布非空（validate_fabric_trace 口径） */
  dye_lot_no: string | null;
  /** 批次：恒非空（四维追溯，白坯也必填） */
  batch_no: string;
  created_at: string;
  updated_at: string;
  product_name: string | null;
  product_code: string | null;
  grade: string | null;
  unit: string | null;
}

/**
 * 库存列表/导出查询参数 = 后端 ListStockParams（handlers/inventory_stock_handler_dto.rs）。
 * 后端没有 low_stock 开关（低库存另有 /stock/low-stock 端点），传了会被 serde 静默丢弃，
 * 因此本类型不得声明该键（假筛选=静默失败）。
 */
export interface InventoryQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  warehouse_id?: number;
  product_id?: number;
  /** 台账状态筛选：取后端 stock_status 主数据值（见 constants/inventory-stock-status），越界值后端 400 */
  stock_status?: string;
  /** 色号筛选（后端下推 SQL） */
  color_no?: string;
  /** 缸号筛选（后端下推 SQL） */
  dye_lot_no?: string;
  /** 批次/匹号筛选（后端下推 SQL） */
  batch_no?: string;
}

/**
 * 库存调整建单入参：字段严格对齐后端
 * `handlers/inventory_adjustment_handler.rs::CreateAdjustmentRequestPayload`——
 * 该端点按「仓库 + 调整日期 + 调整类型 + 原因类型 + 明细项(stock_id+quantity)」建模，
 * 调整对象是既有库存行（stock_id），不是「产品+仓库」裸组合：
 * service 的 create_adjustment_items 会按 stock_id 读取 quantity_on_hand 作为调整前量。
 * 此前前端写 product_id/adjustment_quantity/reason，与后端必填
 * adjustment_date/reason_type/items 完全不同 → serde 反序列化缺字段 → 422，
 * 提交必被拒（02 盘盈/盘亏红根因）。quantity 为 Decimal 串，按 §3 出/入参口径传字符串。
 */
export interface StockAdjustmentData {
  warehouse_id: number;
  adjustment_date: string;
  adjustment_type: 'increase' | 'decrease';
  reason_type: string;
  reason_description?: string;
  notes?: string;
  items: Array<{
    stock_id: number;
    quantity: string;
    unit_cost?: string;
    notes?: string;
  }>;
}

/**
 * 创建预留入参 = 后端 CreateReservationRequest
 * （handlers/inventory_reservation_handler.rs，非 Option 必填：order_id/product_id/warehouse_id/quantity）。
 * 后端没有 order_no/expire_date 键（此前前端多传 = serde 静默丢弃的假字段）；
 * quantity 为 Decimal 入参，按本仓口径传字符串（整数 number 亦可反序列化，含小数的 JSON
 * 浮点在无 serde-float 的 rust_decimal 上会被拒 → 422「参数错误」）。
 * notes 为 Option<String>：空值省略该键，不传空串。
 */
export interface CreateReservationPayload {
  order_id: number;
  product_id: number;
  warehouse_id: number;
  quantity: string;
  notes?: string;
}

/** 预留列表查询参数 = 后端 ReservationQuery（handlers/inventory_reservation_handler.rs） */
export interface ReservationQueryParams {
  page?: number;
  page_size?: number;
  product_id?: number;
  warehouse_id?: number;
  /** pending/locked/consumed/released/cancelled（小写，后端按 status 等值下推） */
  status?: string;
}

/**
 * 库存调拨列表查询参数 = 后端 InventoryTransferQuery
 * （handlers/inventory_transfer_handler.rs）。
 * 后端只支持这四个筛选项 + 分页；keyword/产品/色号/缸号/批次/台账状态均不被本端点读取，
 * 不得混用台账的 InventoryQueryParams 传过去被静默丢弃。
 */
export interface InventoryTransferQueryParams {
  page?: number;
  page_size?: number;
  /** pending/approved/rejected/shipped/completed（小写，models/status/purchase_inventory.rs::inventory_transfer） */
  status?: string;
  from_warehouse_id?: number;
  to_warehouse_id?: number;
  transfer_no?: string;
}

/**
 * 库存汇总（stock/summary）查询参数 = 后端 ListStockFabricParams
 * （handlers/inventory_stock_handler_dto.rs；get_inventory_summary 只认这七个键，
 * 此前前端传的 category_id/date_from/date_to/report_type 全部被忽略）。
 * 与面料库存列表同构，直接复用 StockFabricQueryParams。
 */
export type InventorySummaryQueryParams = StockFabricQueryParams;

/**
 * 库存预警行：与后端 `StockAlertRow` 一一对应。
 *
 * 数量类字段是 Decimal 的字符串序列化（与库存台账同口径），展示前按需 Number() 转换；
 * 告警类型取值见 constants/stock-alert-type（后端 AlertType 的小写码）。
 */
export interface StockAlert {
  id: number;
  product_id: number;
  product_code?: string | null;
  product_name?: string | null;
  unit?: string | null;
  warehouse_id: number;
  warehouse_name?: string | null;
  quantity_on_hand: string;
  quantity_available: string;
  quantity_reserved: string;
  /** 补货点：可用量低于它即 low_stock 告警 */
  reorder_point: string;
  max_stock_point: string;
  expiry_date?: string | null;
  last_movement_date?: string | null;
  /** 台账状态：正常/报废/已删除 */
  stock_status: string;
  alert_type: string;
}

/**
 * 库存行更新入参 = 后端 UpdateStockWithVersionRequest
 * （handlers/inventory_stock_handler_dto.rs；update_stock handler 按此反序列化并做乐观锁比对）。
 * 该端点只接受数量/阈值/库位的纠偏字段——批次/色号/缸号/等级/仓库/产品等四维与归属列
 * 不可经 PUT 修改（四维改动须走红冲黑退的调整/出入库流程）。
 * version 必填：取自 GET 出参库存行的 version 列（StockResponse.version 直映
 * inventory_stocks.version 真实列），禁止以 0 之类假值蒙混乐观锁。
 * 数量类为 Decimal 入参：传字符串；Option 字段空值省略该键。
 * 三态语义（RFC 7386 JSON Merge Patch）：键缺席=保持原值、显式 null=清空、有值=覆盖。
 * - 六个数量/阈值列均为 NOT NULL（inventory_stocks 模型非 Option Decimal）：
 *   禁止送 null（后端 400「XX不能清空：该字段为必填项」）——不改即省略键；
 * - bin_location 为 DB 可空列：清空须显式送 null，禁止塌成省略。
 */
export interface UpdateStockRequest {
  quantity_on_hand?: string;
  quantity_available?: string;
  quantity_reserved?: string;
  reorder_point?: string;
  max_stock_point?: string;
  reorder_quantity?: string;
  bin_location?: string | null;
  version: number;
}

export const getStockList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<PaginatedResponse<InventoryStock>>>('/inventory/stock', {
    params,
  });

// D14 Batch 5b：原 inventoryApi.getStockById 转为风格 B 函数
export const getStockById = (id: number) =>
  request.get<ApiResponse<InventoryStock>>(`/inventory/stock/${id}`);

export const updateStock = (id: number, data: UpdateStockRequest) =>
  request.put<ApiResponse<InventoryStock>>(`/inventory/stock/${id}`, data);

// 批次 94 P2-12 修复：补全库存记录删除接口（原缺失，导致 StockTab 删除/批量删除占位）
// D14 Batch 5b：原 inventoryApi.deleteStock 转为风格 B 函数
export const deleteStock = (id: number) =>
  request.delete<ApiResponse<void>>(`/inventory/stock/${id}`);

// D14 Batch 5b：原 inventoryApi.getStockByProduct 转为风格 B 函数
export const getStockByProduct = (productId: number) =>
  request.get<ApiResponse<{ list: InventoryStock[]; total: number }>>(
    `/inventory/stock/product/${productId}`
  );

// D14 Batch 5b：原 inventoryApi.createStockAdjustment 转为风格 B 函数
export const createStockAdjustment = (data: StockAdjustmentData) =>
  request.post<ApiResponse<{ id: number; adjustment_no: string }>>('/inventory/adjustments', data);

// D14 Batch 5b：原 inventoryApi.getReservations 转为风格 B 函数
export const getReservationList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<{ list: InventoryReservation[]; total: number }>>(
    '/inventory/reservations',
    { params }
  );

// D14 Batch 5b：原 inventoryApi.createReservation 转为风格 B 函数
// 入参 = 后端 CreateReservationRequest，出参 = 后端 ReservationResponse（见上方两个接口注释）
export const createReservation = (data: CreateReservationPayload) =>
  request.post<ApiResponse<ReservationMutationResponse>>('/inventory/reservations', data);

// D14 Batch 5b：原 inventoryApi.cancelReservation 转为风格 B 函数
export const cancelReservation = (id: number) =>
  request.delete<ApiResponse<null>>(`/inventory/reservations/${id}`);

// D14 Batch 5b：原 inventoryApi.getTransfers 转为风格 B 函数
export const getInventoryTransferList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<PaginatedResponse<InventoryTransfer>>>('/inventory/transfers', {
    params,
  });

/** 批准体（后端 inventory_transfer_handler.rs::ApproveTransferRequest：approved 必填） */
export interface ApproveTransferPayload {
  approved: boolean;
  notes?: string;
}

export const approveInventoryTransfer = (id: number, data: ApproveTransferPayload) =>
  request.post<ApiResponse<null>>(`/inventory/transfers/${id}/approve`, data);

// D14 Batch 5b：原 inventoryApi.executeTransfer 转为风格 B 函数
export const executeInventoryTransfer = (id: number) =>
  request.post<ApiResponse<null>>(`/inventory/transfers/${id}/ship`);

// D14 Batch 5b：原 inventoryApi.getStockAlerts 转为风格 B 函数
/** 预警列表一次取多少条：预警面板无分页控件，取一屏上限（后端 clamp 到 500） */
export const STOCK_ALERT_PAGE_SIZE = 100;

export const getStockAlertList = (params?: {
  page?: number;
  page_size?: number;
  warehouse_id?: number;
  product_id?: number;
}) =>
  request.get<ApiResponse<PaginatedResponse<StockAlert>>>('/inventory/stock/alerts', { params });

/**
 * 库存汇总行：与后端 `InventorySummaryItem`（handlers/inventory_stock_handler_dto.rs，
 * 由 services/inventory_stock_query.rs::get_inventory_summary 按 产品+仓库+批次+色号+等级
 * GROUP BY 聚合产出）一一对应。
 * total_quantity_* 是 SUM(Decimal) 的字符串序列化（rust_decimal 出参口径，展示前 Number() 归一）；
 * product_name/warehouse_name 来自 INNER JOIN 主数据列、batch_no/color_no/grade 为 GROUP BY 列，
 * 后端均非 Option → 全部必填，不标可选。
 */
export interface InventorySummaryRow {
  product_id: number;
  product_name: string;
  batch_no: string;
  color_no: string;
  grade: string;
  total_quantity_meters: string;
  total_quantity_kg: string;
  warehouse_name: string;
}

// D14 Batch 5b：原 inventoryApi.getInventoryReport 转为风格 B 函数
/** 汇总端点入参 = 后端 ListStockFabricParams（与 InventorySummaryQueryParams 同构） */
export const getInventoryReport = (params: InventorySummaryQueryParams) =>
  request.get<ApiResponse<PaginatedResponse<InventorySummaryRow>>>('/inventory/stock/summary', {
    params,
  });

// ============== 库存创建/导出/面料/流水/预留锁定（Batch 补齐 API 封装）==============

/** 库存创建请求（对应后端 inventory_stock_handler_dto.rs::CreateStockFabricRequest） */
export interface CreateStockRequest {
  warehouse_id: number;
  product_id: number;
  batch_no: string;
  color_no: string;
  dye_lot_no?: string;
  grade: string;
  quantity_meters: number;
  quantity_kg?: number;
  gram_weight?: number;
  width?: number;
  location_id?: number;
  shelf_no?: string;
  layer_no?: string;
}

/** 库存基础响应（对应后端 inventory_stock_handler_dto.rs::StockResponse） */
export interface StockResponse {
  id: number;
  warehouse_id: number;
  product_id: number;
  quantity_on_hand: number;
  quantity_available: number;
  quantity_reserved: number;
  reorder_point: number;
  max_stock_point: number;
  bin_location?: string;
  created_at: string;
  updated_at: string;
}

/** 面料库存响应（对应后端 inventory_stock_handler_dto.rs::StockFabricResponse） */
export interface StockFabricResponse {
  id: number;
  warehouse_id: number;
  product_id: number;
  batch_no: string;
  color_no: string;
  dye_lot_no?: string;
  grade: string;
  quantity_on_hand: number;
  quantity_available: number;
  quantity_reserved: number;
  quantity_meters: number;
  quantity_kg: number;
  gram_weight?: number;
  width?: number;
  bin_location?: string;
  created_at: string;
  updated_at: string;
}

/** 库存事务响应（对应后端 inventory_stock_handler_dto.rs::TransactionResponse） */
export interface TransactionResponse {
  id: number;
  transaction_type: string;
  product_id: number;
  warehouse_id: number;
  batch_no: string;
  color_no: string;
  quantity_meters: number;
  quantity_kg: number;
  quantity_before_meters: number;
  quantity_before_kg: number;
  quantity_after_meters: number;
  quantity_after_kg: number;
  source_bill_type?: string;
  source_bill_no?: string;
  remarks?: string;
  created_at: string;
}

/** 面料库存查询参数（对应后端 ListStockFabricParams） */
export interface StockFabricQueryParams {
  page?: number;
  page_size?: number;
  warehouse_id?: number;
  product_id?: number;
  batch_no?: string;
  color_no?: string;
  grade?: string;
}

/** 库存事务查询参数（对应后端 ListTransactionParams） */
export interface TransactionQueryParams {
  page?: number;
  page_size?: number;
  product_id?: number;
  warehouse_id?: number;
  batch_no?: string;
  color_no?: string;
  transaction_type?: string;
  start_date?: string;
  end_date?: string;
}

/**
 * 创建库存记录
 * 后端路由：POST /api/v1/erp/inventory/stock（routes/inventory.rs stock_routes）
 */
export const createStock = (data: CreateStockRequest) =>
  request.post<ApiResponse<StockResponse>>('/inventory/stock', data);

/**
 * 导出库存列表 xlsx（V15 P0-S12；返回带水印的 Blob）
 * 后端路由：GET /api/v1/erp/inventory/stock/export（routes/inventory.rs stock_routes）
 */
export const exportStock = (params?: InventoryQueryParams) =>
  request.get<Blob>('/inventory/stock/export', { params, responseType: 'blob' });

/**
 * 面料库存列表（按批次+色号+仓库查询）
 * 后端路由：GET /api/v1/erp/inventory/stock/fabric（routes/inventory.rs stock_routes）
 */
export const getStockFabricList = (params?: StockFabricQueryParams) =>
  request.get<ApiResponse<StockFabricResponse[]>>('/inventory/stock/fabric', { params });

/**
 * 创建面料库存（提供克重与幅宽时后端自动换算公斤数）
 * 后端路由：POST /api/v1/erp/inventory/stock/fabric（routes/inventory.rs stock_routes）
 */
export const createStockFabric = (data: CreateStockRequest) =>
  request.post<ApiResponse<StockFabricResponse>>('/inventory/stock/fabric', data);

/**
 * 库存出入库流水（分页）
 * 后端路由：GET /api/v1/erp/inventory/stock/transactions（routes/inventory.rs stock_routes）
 */
export const getTransactionList = (params?: TransactionQueryParams) =>
  request.get<
    ApiResponse<{ items: TransactionResponse[]; total: number; page: number; page_size: number }>
  >('/inventory/stock/transactions', { params });

/**
 * 锁定预留（pending → locked）
 * 后端路由：POST /api/v1/erp/inventory/reservations/{id}/lock（routes/inventory.rs reservation_routes）
 */
export const lockReservation = (id: number) =>
  request.post<ApiResponse<InventoryReservation>>(`/inventory/reservations/${id}/lock`);

/**
 * 释放预留（locked/pending → released）
 * 后端路由：POST /api/v1/erp/inventory/reservations/{id}/release（routes/inventory.rs reservation_routes）
 */
export const releaseReservation = (id: number) =>
  request.post<ApiResponse<InventoryReservation>>(`/inventory/reservations/${id}/release`);
