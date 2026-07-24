// trading-contract.ts - 交易合同 API 桩（统一采购/销售合同）
// 来源：拆分原 trading/index.vue 时统一接口调用而创建
// 实际后端路由：/trading/purchase-contracts, /trading/sales-contracts
import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface TradingContract {
  id: number
  contract_no: string
  supplier_name?: string
  customer_name?: string
  contract_date: string
  total_amount: number
  status: string
}

export interface ListTradingContractParams {
  type: 'purchase' | 'sales'
}

export const getTradingContractList = (params: ListTradingContractParams) => {
  if (params.type === 'purchase') {
    return request.get<ApiResponse<TradingContract[]>>('/trading/purchase-contracts')
  }
  return request.get<ApiResponse<TradingContract[]>>('/trading/sales-contracts')
}

export const getTradingContract = (id: number) =>
  request.get<ApiResponse<TradingContract>>(`/trading/contracts/${id}`)

export const createTradingContract = (data: Partial<TradingContract> & { type: 'purchase' | 'sales' }) => {
  if (data.type === 'purchase') {
    return request.post<ApiResponse<TradingContract>>('/trading/purchase-contracts', data)
  }
  return request.post<ApiResponse<TradingContract>>('/trading/sales-contracts', data)
}

export const updateTradingContract = (id: number, data: Partial<TradingContract>) =>
  request.put<ApiResponse<TradingContract>>(`/trading/contracts/${id}`, data)

export const deleteTradingContract = (id: number) =>
  request.delete<ApiResponse<null>>(`/trading/contracts/${id}`)

export const approveTradingContract = (id: number) =>
  request.post<ApiResponse<TradingContract>>(`/trading/contracts/${id}/approve`)

export const executeTradingContract = (id: number) =>
  request.post<ApiResponse<TradingContract>>(`/trading/contracts/${id}/execute`)
