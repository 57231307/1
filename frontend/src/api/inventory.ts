import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 库存行：字段名与后端 StockResponse 对齐
 * （数量按主/辅计量与四个业务量维度分开，面料维度为批次/色号/缸号/等级）
 */
export interface InventoryStock {
  id: number;
  warehouse_id: number;
  product_id: number;
  /** 在库量 */
  quantity_on_hand: number;
  /** 可用量（在库 - 预留） */
  quantity_available: number;
  quantity_reserved: number;
  /** 已发货量（销售发货累计） */
  quantity_shipped: number;
  /** 在途量（采购收货累计） */
  quantity_incoming: number;
  reorder_point: number;
  max_stock_point: number;
  bin_location?: string | null;
  // ===== 面料四维 =====
  batch_no: string;
  color_no: string;
  dye_lot_no?: string | null;
  grade: string;
  /** 台账状态：正常/报废/已删除（后端 inventory_stocks.stock_status 主数据值，见 constants/inventory-stock-status） */
  stock_status: string;
  /** 质量状态：合格/不合格/待检 */
  quality_status: string;
  quantity_meters: number;
  quantity_kg: number;
  // ===== 主数据名称（后端按 ID 批量带出）=====
  product_code?: string | null;
  product_name?: string | null;
  warehouse_name?: string | null;
  created_at?: string;
  updated_at?: string;
}

export interface InventoryReservation {
  id: number;
  order_id: number;
  order_no: string;
  product_id: number;
  product_name: string;
  warehouse_id: number;
  warehouse_name: string;
  quantity: number;
  reserved_quantity: number;
  available_quantity: number;
  status: string;
  expire_date?: string;
}

export interface InventoryTransfer {
  id: number;
  transfer_no: string;
  from_warehouse_id: number;
  from_warehouse_name: string;
  to_warehouse_id: number;
  to_warehouse_name: string;
  status: string;
  total_quantity: number;
  creator_name: string;
  created_at: string;
  items: InventoryTransferItem[];
}

export interface InventoryTransferItem {
  id: number;
  product_id: number;
  product_name: string;
  quantity: number;
  from_location?: string;
  to_location?: string;
}

export interface InventoryQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  warehouse_id?: number;
  product_id?: number;
  /** 台账状态筛选：取后端 stock_status 主数据值（见 constants/inventory-stock-status） */
  stock_status?: string;
  low_stock?: boolean;
  /** 色号筛选（后端下推 SQL） */
  color_no?: string;
  /** 缸号筛选（后端下推 SQL） */
  dye_lot_no?: string;
  /** 批次/匹号筛选（后端下推 SQL） */
  batch_no?: string;
}

export interface StockAdjustmentData {
  warehouse_id: number;
  product_id: number;
  batch_no?: string;
  adjustment_quantity: number;
  adjustment_type: 'increase' | 'decrease';
  reason: string;
  remark?: string;
}

export interface ReservationData {
  order_id: number;
  order_no: string;
  product_id: number;
  warehouse_id: number;
  quantity: number;
  expire_date?: string;
  /** 备注（对应后端 CreateReservationRequest::notes） */
  notes?: string;
}

export interface TransferData {
  from_warehouse_id: number;
  to_warehouse_id: number;
  items: {
    product_id: number;
    quantity: number;
    from_location?: string;
    to_location?: string;
  }[];
  remark?: string;
}

export interface InventoryReportParams {
  warehouse_id?: number;
  product_id?: number;
  category_id?: number;
  date_from?: string;
  date_to?: string;
  report_type?: 'summary' | 'detail' | 'movement';
}

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

/// 说明：与后端 PaginatedResponse 同构的分页包装，供预警等列表端点复用
export interface Paginated<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

// P2-9b 修复（批次 82 v1 复审）：库存报表返回类型强类型化，替代 { summary: any; details: any[] }
export interface InventoryReportSummary {
  total_quantity: number;
  total_amount: number;
  warehouse_count: number;
  product_count: number;
  low_stock_count: number;
  alert_count: number;
}

export interface InventoryReportDetail {
  product_id: number;
  product_name: string;
  product_code: string;
  warehouse_id: number;
  warehouse_name: string;
  quantity: number;
  amount: number;
  unit?: string;
  status?: string;
  batch_no?: string;
}

// D14 Batch 5b：原 inventoryApi.getStockList 转为风格 B 函数
export const getStockList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<{ items: InventoryStock[]; total: number }>>('/inventory/stock', {
    params,
  });

// D14 Batch 5b：原 inventoryApi.getStockById 转为风格 B 函数
export const getStockById = (id: number) =>
  request.get<ApiResponse<InventoryStock>>(`/inventory/stock/${id}`);

// 批次 94 P2-12 修复：补全库存记录更新接口（原缺失，导致 StockTab 编辑占位）
// D14 Batch 5b：原 inventoryApi.updateStock 转为风格 B 函数
export const updateStock = (id: number, data: Partial<InventoryStock>) =>
  request.put<ApiResponse<InventoryStock>>(`/inventory/stock/${id}`, data);

// 批次 94 P2-12 修复：补全库存记录删除接口（原缺失，导致 StockTab 删除/批量删除占位）
// D14 Batch 5b：原 inventoryApi.deleteStock 转为风格 B 函数
export const deleteStock = (id: number) =>
  request.delete<ApiResponse<void>>(`/inventory/stock/${id}`);

// D14 Batch 5b：原 inventoryApi.getStockByProduct 转为风格 B 函数
export const getStockByProduct = (productId: number) =>
  request.get<ApiResponse<InventoryStock[]>>(`/inventory/stock/product/${productId}`);

// D14 Batch 5b：原 inventoryApi.createStockAdjustment 转为风格 B 函数
export const createStockAdjustment = (data: StockAdjustmentData) =>
  request.post<ApiResponse<{ id: number; adjustment_no: string }>>('/inventory/adjustments', data);

// D14 Batch 5b：原 inventoryApi.getReservations 转为风格 B 函数
export const getReservationList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<{ items: InventoryReservation[]; total: number }>>(
    '/inventory/reservations',
    { params }
  );

// D14 Batch 5b：原 inventoryApi.createReservation 转为风格 B 函数
export const createReservation = (data: ReservationData) =>
  request.post<ApiResponse<InventoryReservation>>('/inventory/reservations', data);

// D14 Batch 5b：原 inventoryApi.cancelReservation 转为风格 B 函数
export const cancelReservation = (id: number) =>
  request.delete<ApiResponse<null>>(`/inventory/reservations/${id}`);

// D14 Batch 5b：原 inventoryApi.getTransfers 转为风格 B 函数
export const getInventoryTransferList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<{ items: InventoryTransfer[]; total: number }>>('/inventory/transfers', {
    params,
  });

// D14 Batch 5b：原 inventoryApi.createTransfer 转为风格 B 函数
export const createInventoryTransfer = (data: TransferData) =>
  request.post<ApiResponse<InventoryTransfer>>('/inventory/transfers', data);

// D14 Batch 5b：原 inventoryApi.approveTransfer 转为风格 B 函数
export const approveInventoryTransfer = (id: number) =>
  request.post<ApiResponse<null>>(`/inventory/transfers/${id}/approve`);

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
}) => request.get<ApiResponse<Paginated<StockAlert>>>('/inventory/stock/alerts', { params });

// D14 Batch 5b：原 inventoryApi.getInventoryReport 转为风格 B 函数
export const getInventoryReport = (params: InventoryReportParams) =>
  request.get<ApiResponse<{ summary: InventoryReportSummary; details: InventoryReportDetail[] }>>(
    '/inventory/stock/summary',
    {
      params,
    }
  );

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
