import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

// 采购合同
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

export function listPurchaseContracts(params?: QueryParams): Promise<ApiResponse<PurchaseContract[]>> {
  return request.get('/purchase-contracts', { params })
}

export function getPurchaseContract(id: number): Promise<ApiResponse<PurchaseContract>> {
  return request.get(`/purchase-contracts/${id}`)
}

export function createPurchaseContract(data: Partial<PurchaseContract>): Promise<ApiResponse<PurchaseContract>> {
  return request.post('/purchase-contracts', data)
}

export function updatePurchaseContract(id: number, data: Partial<PurchaseContract>): Promise<ApiResponse<PurchaseContract>> {
  return request.put(`/purchase-contracts/${id}`, data)
}

export function deletePurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase-contracts/${id}`)
}

export function approvePurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.post(`/purchase-contracts/${id}/approve`)
}

export function executePurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/purchase-contracts/${id}/execute`)
}

export function cancelPurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/purchase-contracts/${id}/cancel`)
}

// 销售合同
export interface SalesContract {
  id: number
  contract_no: string
  customer_id: number
  customer_name: string
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

export function listSalesContracts(params?: QueryParams): Promise<ApiResponse<SalesContract[]>> {
  return request.get('/sales-contracts', { params })
}

export function getSalesContract(id: number): Promise<ApiResponse<SalesContract>> {
  return request.get(`/sales-contracts/${id}`)
}

export function createSalesContract(data: Partial<SalesContract>): Promise<ApiResponse<SalesContract>> {
  return request.post('/sales-contracts', data)
}

export function updateSalesContract(id: number, data: Partial<SalesContract>): Promise<ApiResponse<SalesContract>> {
  return request.put(`/sales-contracts/${id}`, data)
}

export function deleteSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/sales-contracts/${id}`)
}

export function approveSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales-contracts/${id}/approve`)
}

export function executeSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/sales-contracts/${id}/execute`)
}

export function cancelSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/sales-contracts/${id}/cancel`)
}

// 采购价格
export interface PurchasePrice {
  id: number
  product_id: number
  product_name: string
  product_code: string
  supplier_id: number
  supplier_name: string
  price: number
  currency: string
  unit: string
  effective_date: string
  expiry_date: string
  status: 'active' | 'inactive'
  remark: string
  created_at: string
  updated_at: string
}

export function listPurchasePrices(params?: QueryParams): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get('/purchase-prices', { params })
}

export function getPurchasePrice(id: number): Promise<ApiResponse<PurchasePrice>> {
  return request.get(`/purchase-prices/${id}`)
}

export function createPurchasePrice(data: Partial<PurchasePrice>): Promise<ApiResponse<PurchasePrice>> {
  return request.post('/purchase-prices', data)
}

export function updatePurchasePrice(id: number, data: Partial<PurchasePrice>): Promise<ApiResponse<PurchasePrice>> {
  return request.put(`/purchase-prices/${id}`, data)
}

export function deletePurchasePrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase-prices/${id}`)
}

// 销售价格
export interface SalesPrice {
  id: number
  product_id: number
  product_name: string
  product_code: string
  customer_id: number
  customer_name: string
  price: number
  currency: string
  unit: string
  effective_date: string
  expiry_date: string
  status: 'active' | 'inactive'
  remark: string
  created_at: string
  updated_at: string
}

export function listSalesPrices(params?: QueryParams): Promise<ApiResponse<SalesPrice[]>> {
  return request.get('/sales-prices', { params })
}

export function getSalesPrice(id: number): Promise<ApiResponse<SalesPrice>> {
  return request.get(`/sales-prices/${id}`)
}

export function createSalesPrice(data: Partial<SalesPrice>): Promise<ApiResponse<SalesPrice>> {
  return request.post('/sales-prices', data)
}

export function approveSalesPrice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales-prices/${id}/approve`)
}

export function getPriceHistory(productId: number): Promise<ApiResponse<SalesPrice[]>> {
  return request.get(`/sales-prices/history/${productId}`)
}

export function listPricingStrategies(): Promise<ApiResponse<any[]>> {
  return request.get('/sales-prices/strategies')
}

// 销售退货
export interface SalesReturn {
  id: number
  return_no: string
  customer_id: number
  customer_name: string
  order_id: number
  order_no: string
  return_date: string
  total_amount: number
  reason: string
  status: 'draft' | 'pending' | 'approved' | 'rejected' | 'completed'
  items: ReturnItem[]
  created_by: number
  created_by_name: string
  approved_by: number
  approved_by_name: string
  approved_at: string
  created_at: string
  updated_at: string
}

export interface ReturnItem {
  id: number
  return_id: number
  product_id: number
  product_name: string
  product_code: string
  quantity: number
  unit: string
  price: number
  amount: number
  reason: string
}

export function listSalesReturns(params?: QueryParams): Promise<ApiResponse<SalesReturn[]>> {
  return request.get('/sales-returns', { params })
}

export function getSalesReturn(id: number): Promise<ApiResponse<SalesReturn>> {
  return request.get(`/sales-returns/${id}`)
}

export function createSalesReturn(data: Partial<SalesReturn>): Promise<ApiResponse<SalesReturn>> {
  return request.post('/sales-returns', data)
}
