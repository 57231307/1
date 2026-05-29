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

export const inventoryApi = {
  getStockList: (params?: InventoryQueryParams) =>
    request.get<ApiResponse<{ list: InventoryStock[]; total: number }>>('/inventory/stock', {
      params,
    }),

  getStockById: (id: number) => request.get<ApiResponse<InventoryStock>>(`/inventory/stock/${id}`),

  getStockByProduct: (productId: number) =>
    request.get<ApiResponse<InventoryStock[]>>(`/inventory/stock/product/${productId}`),

  createStockAdjustment: (data: StockAdjustmentData) =>
    request.post<ApiResponse<{ id: number; adjustment_no: string }>>(
      '/inventory/adjustments',
      data
    ),

  getReservations: (params?: InventoryQueryParams) =>
    request.get<ApiResponse<{ list: InventoryReservation[]; total: number }>>(
      '/inventory/reservations',
      { params }
    ),

  createReservation: (data: ReservationData) =>
    request.post<ApiResponse<InventoryReservation>>('/inventory/reservations', data),

  cancelReservation: (id: number) =>
    request.delete<ApiResponse<null>>(`/inventory/reservations/${id}`),

  getTransfers: (params?: InventoryQueryParams) =>
    request.get<ApiResponse<{ list: InventoryTransfer[]; total: number }>>('/inventory/transfers', {
      params,
    }),

  createTransfer: (data: TransferData) =>
    request.post<ApiResponse<InventoryTransfer>>('/inventory/transfers', data),

  approveTransfer: (id: number) =>
    request.post<ApiResponse<null>>(`/inventory/transfers/${id}/approve`),

  executeTransfer: (id: number) =>
    request.post<ApiResponse<null>>(`/inventory/transfers/${id}/ship`),

  getStockAlerts: () => request.get<ApiResponse<StockAlert[]>>('/inventory/stock/alerts'),

  getInventoryReport: (params: InventoryReportParams) =>
    request.get<ApiResponse<{ summary: any; details: any[] }>>('/inventory/stock/summary', {
      params,
    }),
}
