import { request } from './request'

export interface InventoryBatch {
  id?: number
  batchNo?: string
  productId?: number
  productName?: string
  warehouseId?: number
  warehouseName?: string
  colorNo?: string
  colorName?: string
  dyeLotNo?: string
  grade?: string
  quantityOnHand?: number
  quantityAvailable?: number
  quantityReserved?: number
  quantityIncoming?: number
  quantityMeters?: number
  quantityKg?: number
  gramWeight?: number
  width?: number
  productionDate?: string
  expiryDate?: string
  supplierId?: number
  purchaseOrderNo?: string
  stockStatus?: string
  qualityStatus?: string
  remarks?: string
  createdAt?: string
  updatedAt?: string
}

export interface CreateBatchRequest {
  batchNo: string
  productId: number
  warehouseId: number
  colorNo: string
  colorName?: string
  dyeLotNo?: string
  grade: string
  quantityMeters: number
  quantityKg: number
  gramWeight?: number
  width?: number
  productionDate?: string
  expiryDate?: string
  supplierId?: number
  purchaseOrderNo?: string
  remarks?: string
}

export interface UpdateBatchRequest {
  colorNo?: string
  dyeLotNo?: string
  grade?: string
  gramWeight?: number
  width?: number
  expiryDate?: string
  remarks?: string
  stockStatus?: string
  qualityStatus?: string
}

export interface TransferBatchRequest {
  fromWarehouseId: number
  toWarehouseId: number
  quantityMeters: number
  quantityKg: number
  remarks?: string
}

export function listBatches(params?: any) {
  return request.get('/batches', { params })
}

export function getBatch(id: number) {
  return request.get(`/batches/${id}`)
}

export function createBatch(data: CreateBatchRequest) {
  return request.post('/batches', data)
}

export function updateBatch(id: number, data: UpdateBatchRequest) {
  return request.put(`/batches/${id}`, data)
}

export function deleteBatch(id: number) {
  return request.delete(`/batches/${id}`)
}

export function transferBatch(id: number, data: TransferBatchRequest) {
  return request.post(`/batches/${id}/transfer`, data)
}
