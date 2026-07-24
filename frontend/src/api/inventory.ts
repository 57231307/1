import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface InventoryStock {
  id: number
  product_id: number
  product_name: string
  product_code: string
  warehouse_id: number
  warehouse_name: string
  batch_no?: string
  color_code?: string
  color_name?: string
  lot_no?: string
  quantity: number
  quantity_alt?: number
  unit?: string
  unit_alt?: string
  gram_weight?: number
  width?: number
  location?: string
  status: string
  created_at?: string
  updated_at?: string
}

export interface InventoryReservation {
  id: number
  order_id: number
  order_no: string
  product_id: number
  product_name: string
  warehouse_id: number
  warehouse_name: string
  quantity: number
  reserved_quantity: number
  available_quantity: number
  status: string
  expire_date?: string
}

export interface InventoryTransfer {
  id: number
  transfer_no: string
  from_warehouse_id: number
  from_warehouse_name: string
  to_warehouse_id: number
  to_warehouse_name: string
  status: string
  total_quantity: number
  creator_name: string
  created_at: string
  items: InventoryTransferItem[]
}

export interface InventoryTransferItem {
  id: number
  product_id: number
  product_name: string
  quantity: number
  from_location?: string
  to_location?: string
}

export interface InventoryQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  warehouse_id?: number
  product_id?: number
  status?: string
  low_stock?: boolean
}

export interface StockAdjustmentData {
  warehouse_id: number
  product_id: number
  batch_no?: string
  adjustment_quantity: number
  adjustment_type: 'increase' | 'decrease'
  reason: string
  remark?: string
}

export interface ReservationData {
  order_id: number
  order_no: string
  product_id: number
  warehouse_id: number
  quantity: number
  expire_date?: string
}

export interface TransferData {
  from_warehouse_id: number
  to_warehouse_id: number
  items: {
    product_id: number
    quantity: number
    from_location?: string
    to_location?: string
  }[]
  remark?: string
}

export interface InventoryReportParams {
  warehouse_id?: number
  product_id?: number
  category_id?: number
  date_from?: string
  date_to?: string
  report_type?: 'summary' | 'detail' | 'movement'
}

export interface StockAlert {
  id: number
  product_id: number
  product_name: string
  product_code: string
  warehouse_id: number
  warehouse_name: string
  current_quantity: number
  min_quantity: number
  unit?: string
  alert_level: 'warning' | 'danger'
}

// P2-9b 修复（批次 82 v1 复审）：库存报表返回类型强类型化，替代 { summary: any; details: any[] }
export interface InventoryReportSummary {
  total_quantity: number
  total_amount: number
  warehouse_count: number
  product_count: number
  low_stock_count: number
  alert_count: number
}

export interface InventoryReportDetail {
  product_id: number
  product_name: string
  product_code: string
  warehouse_id: number
  warehouse_name: string
  quantity: number
  amount: number
  unit?: string
  status?: string
  batch_no?: string
}

// D14 Batch 5b：原 inventoryApi.getStockList 转为风格 B 函数
export const getStockList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<{ list: InventoryStock[]; total: number }>>('/inventory/stock', {
    params,
  })

// D14 Batch 5b：原 inventoryApi.getStockById 转为风格 B 函数
export const getStockById = (id: number) =>
  request.get<ApiResponse<InventoryStock>>(`/inventory/stock/${id}`)

// 批次 94 P2-12 修复：补全库存记录更新接口（原缺失，导致 StockTab 编辑占位）
// D14 Batch 5b：原 inventoryApi.updateStock 转为风格 B 函数
export const updateStock = (id: number, data: Partial<InventoryStock>) =>
  request.put<ApiResponse<InventoryStock>>(`/inventory/stock/${id}`, data)

// 批次 94 P2-12 修复：补全库存记录删除接口（原缺失，导致 StockTab 删除/批量删除占位）
// D14 Batch 5b：原 inventoryApi.deleteStock 转为风格 B 函数
export const deleteStock = (id: number) =>
  request.delete<ApiResponse<void>>(`/inventory/stock/${id}`)

// D14 Batch 5b：原 inventoryApi.getStockByProduct 转为风格 B 函数
export const getStockByProduct = (productId: number) =>
  request.get<ApiResponse<InventoryStock[]>>(`/inventory/stock/product/${productId}`)

// D14 Batch 5b：原 inventoryApi.createStockAdjustment 转为风格 B 函数
export const createStockAdjustment = (data: StockAdjustmentData) =>
  request.post<ApiResponse<{ id: number; adjustment_no: string }>>(
    '/inventory/adjustments',
    data
  )

// D14 Batch 5b：原 inventoryApi.getReservations 转为风格 B 函数
export const getReservationList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<{ list: InventoryReservation[]; total: number }>>(
    '/inventory/reservations',
    { params }
  )

// D14 Batch 5b：原 inventoryApi.createReservation 转为风格 B 函数
export const createReservation = (data: ReservationData) =>
  request.post<ApiResponse<InventoryReservation>>('/inventory/reservations', data)

// D14 Batch 5b：原 inventoryApi.cancelReservation 转为风格 B 函数
export const cancelReservation = (id: number) =>
  request.delete<ApiResponse<null>>(`/inventory/reservations/${id}`)

// D14 Batch 5b：原 inventoryApi.getTransfers 转为风格 B 函数
export const getInventoryTransferList = (params?: InventoryQueryParams) =>
  request.get<ApiResponse<{ list: InventoryTransfer[]; total: number }>>('/inventory/transfers', {
    params,
  })

// D14 Batch 5b：原 inventoryApi.createTransfer 转为风格 B 函数
export const createInventoryTransfer = (data: TransferData) =>
  request.post<ApiResponse<InventoryTransfer>>('/inventory/transfers', data)

// D14 Batch 5b：原 inventoryApi.approveTransfer 转为风格 B 函数
export const approveInventoryTransfer = (id: number) =>
  request.post<ApiResponse<null>>(`/inventory/transfers/${id}/approve`)

// D14 Batch 5b：原 inventoryApi.executeTransfer 转为风格 B 函数
export const executeInventoryTransfer = (id: number) =>
  request.post<ApiResponse<null>>(`/inventory/transfers/${id}/ship`)

// D14 Batch 5b：原 inventoryApi.getStockAlerts 转为风格 B 函数
export const getStockAlertList = () =>
  request.get<ApiResponse<StockAlert[]>>('/inventory/stock/alerts')

// D14 Batch 5b：原 inventoryApi.getInventoryReport 转为风格 B 函数
export const getInventoryReport = (params: InventoryReportParams) =>
  request.get<
    ApiResponse<{ summary: InventoryReportSummary; details: InventoryReportDetail[] }>
  >('/inventory/stock/summary', {
    params,
  })
