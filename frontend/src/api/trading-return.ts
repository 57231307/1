// trading-return.ts - 交易退货 API 桩
// 来源：拆分原 trading/index.vue 时统一接口调用而创建
import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface TradingReturn {
  id: number
  return_no: string
  customer_name: string
  order_no?: string
  return_date: string
  total_amount: number
  reason?: string
  status: string
}

export interface ListTradingReturnParams {
  type: 'purchase' | 'sales'
}

export const getTradingReturnList = (params: ListTradingReturnParams) => {
  if (params.type === 'sales') {
    return request.get<ApiResponse<TradingReturn[]>>('/trading/sales-returns')
  }
  return request.get<ApiResponse<TradingReturn[]>>('/trading/purchase-returns')
}

export const getTradingReturn = (id: number) =>
  request.get<ApiResponse<TradingReturn>>(`/trading/returns/${id}`)

export const createTradingReturn = (data: Partial<TradingReturn> & { type: 'purchase' | 'sales' }) => {
  if (data.type === 'sales') {
    return request.post<ApiResponse<TradingReturn>>('/trading/sales-returns', data)
  }
  return request.post<ApiResponse<TradingReturn>>('/trading/purchase-returns', data)
}

export const updateTradingReturn = (id: number, data: Partial<TradingReturn>) =>
  request.put<ApiResponse<TradingReturn>>(`/trading/returns/${id}`, data)

export const deleteTradingReturn = (id: number) =>
  request.delete<ApiResponse<null>>(`/trading/returns/${id}`)

export const approveTradingReturn = (id: number) =>
  request.post<ApiResponse<TradingReturn>>(`/trading/returns/${id}/approve`)
