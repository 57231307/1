import { request } from './request'
import type { ApiResponse } from './request'

export interface Batch {
  id: number
  batch_no: string
  product_id: number
  product_name: string
  product_code: string
  warehouse_id: number
  warehouse_name: string
  quantity: number
  available_quantity: number
  reserved_quantity: number
  unit?: string
  manufacturing_date?: string
  expiry_date?: string
  supplier_id?: number
  supplier_name?: string
  status: string
  created_at?: string
}

export interface BatchQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  warehouse_id?: number
  product_id?: number
  status?: string
  from_date?: string
  to_date?: string
}

export interface DyeBatch {
  id: number
  batch_no: string
  color_code: string
  color_name: string
  product_id: number
  product_name: string
  quantity: number
  unit?: string
  recipe_id?: number
  recipe_name?: string
  status: string
  start_date?: string
  expected_date?: string
  completed_date?: string
  operator?: string
  remark?: string
}

export interface GreigeFabric {
  id: number
  fabric_code: string
  fabric_name: string
  supplier_id: number
  supplier_name?: string
  quantity: number
  unit?: string
  unit_price?: number
  width?: number
  weight?: number
  color?: string
  status: string
  warehouse_id?: number
  warehouse_name?: string
}

export const batchApi = {
  list: (params?: BatchQueryParams) =>
    request.get<ApiResponse<{ list: Batch[]; total: number }>>('/batches', { params }),

  getById: (id: number) =>
    request.get<ApiResponse<Batch>>(`/batches/${id}`),

  create: (data: Partial<Batch>) =>
    request.post<ApiResponse<Batch>>('/batches', data),

  update: (id: number, data: Partial<Batch>) =>
    request.put<ApiResponse<Batch>>(`/batches/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<null>>(`/batches/${id}`),

  transfer: (id: number, data: { target_warehouse_id: number; quantity: number }) =>
    request.post<ApiResponse<any>>(`/batches/${id}/transfer`, data),

  listDyeBatches: (params?: any) =>
    request.get<ApiResponse<{ list: DyeBatch[]; total: number }>>('/dye-batches', { params }),

  createDyeBatch: (data: Partial<DyeBatch>) =>
    request.post<ApiResponse<DyeBatch>>('/dye-batches', data),

  getDyeBatch: (id: number) =>
    request.get<ApiResponse<DyeBatch>>(`/dye-batches/${id}`),

  updateDyeBatch: (id: number, data: Partial<DyeBatch>) =>
    request.put<ApiResponse<DyeBatch>>(`/dye-batches/${id}`, data),

  deleteDyeBatch: (id: number) =>
    request.delete<ApiResponse<null>>(`/dye-batches/${id}`),

  completeDyeBatch: (id: number) =>
    request.post<ApiResponse<any>>(`/dye-batches/${id}/complete`),

  getDyeBatchesByColor: (colorCode: string) =>
    request.get<ApiResponse<DyeBatch[]>>(`/dye-batches/by-color/${colorCode}`),

  listGreigeFabrics: (params?: any) =>
    request.get<ApiResponse<{ list: GreigeFabric[]; total: number }>>('/greige-fabrics', { params }),

  createGreigeFabric: (data: Partial<GreigeFabric>) =>
    request.post<ApiResponse<GreigeFabric>>('/greige-fabrics', data),

  getGreigeFabric: (id: number) =>
    request.get<ApiResponse<GreigeFabric>>(`/greige-fabrics/${id}`),

  updateGreigeFabric: (id: number, data: Partial<GreigeFabric>) =>
    request.put<ApiResponse<GreigeFabric>>(`/greige-fabrics/${id}`, data),

  deleteGreigeFabric: (id: number) =>
    request.delete<ApiResponse<null>>(`/greige-fabrics/${id}`),

  stockIn: (id: number, data: { quantity: number; warehouse_id: number }) =>
    request.post<ApiResponse<any>>(`/greige-fabrics/${id}/stock-in`, data),

  stockOut: (id: number, data: { quantity: number }) =>
    request.post<ApiResponse<any>>(`/greige-fabrics/${id}/stock-out`, data),
}
