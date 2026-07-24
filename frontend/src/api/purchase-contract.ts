import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface PurchaseContract {
  id: number
  contract_no: string
  supplier_id: number
  supplier_name: string
  contract_date: string
  start_date: string
  end_date: string
  total_amount: number
  currency: string
  status: 'draft' | 'pending' | 'active' | 'completed' | 'cancelled'
  items: ContractItem[]
  payment_terms: string
  delivery_terms: string
  created_by: number
  created_by_name: string
  created_at: string
  updated_at: string
}

export interface ContractItem {
  id: number
  contract_id: number
  product_id: number
  product_name: string
  product_code: string
  quantity: number
  unit: string
  price: number
  amount: number
  remark: string
}

export function getPurchaseContractList(
  params?: QueryParams
): Promise<ApiResponse<{ list: PurchaseContract[]; total: number }>> {
  return request.get('/purchase/purchase-contracts', { params })
}

export function getPurchaseContract(id: number): Promise<ApiResponse<PurchaseContract>> {
  return request.get(`/purchase/purchase-contracts/${id}`)
}

export function createPurchaseContract(
  data: Partial<PurchaseContract>
): Promise<ApiResponse<PurchaseContract>> {
  return request.post('/purchase/purchase-contracts', data)
}

export function updatePurchaseContract(
  id: number,
  data: Partial<PurchaseContract>
): Promise<ApiResponse<PurchaseContract>> {
  return request.put(`/purchase/purchase-contracts/${id}`, data)
}

export function deletePurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/purchase-contracts/${id}`)
}

export function approvePurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.post(`/purchase/purchase-contracts/${id}/approve`)
}

export function executePurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/purchase/purchase-contracts/${id}/execute`)
}

export function cancelPurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/purchase/purchase-contracts/${id}/cancel`)
}

// 批次 94 P2-12 修复：补全采购合同导出接口（原缺失，导致 usePcProc 导出占位假成功）
// 返回 blob，前端用 URL.createObjectURL 触发下载
export function exportPurchaseContracts(params?: QueryParams): Promise<Blob> {
  return request.get('/purchase/purchase-contracts/export', {
    params,
    responseType: 'blob',
  })
}
