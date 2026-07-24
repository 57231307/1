// trading-price.ts - 交易价格 API 桩
// 来源：拆分原 trading/index.vue 时统一接口调用而创建
import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface TradingPrice {
  id: number
  product_name: string
  supplier_name?: string
  customer_name?: string
  price: number
  currency: string
  unit: string
  effective_date: string
  expiry_date?: string
  status: string
}

export interface ListTradingPriceParams {
  type: 'purchase' | 'sales'
}

export const getTradingPriceList = (params: ListTradingPriceParams) => {
  if (params.type === 'purchase') {
    return request.get<ApiResponse<TradingPrice[]>>('/trading/purchase-prices')
  }
  return request.get<ApiResponse<TradingPrice[]>>('/trading/sales-prices')
}

export const getTradingPrice = (id: number) =>
  request.get<ApiResponse<TradingPrice>>(`/trading/prices/${id}`)

export const createTradingPrice = (data: Partial<TradingPrice> & { type: 'purchase' | 'sales' }) => {
  if (data.type === 'purchase') {
    return request.post<ApiResponse<TradingPrice>>('/trading/purchase-prices', data)
  }
  return request.post<ApiResponse<TradingPrice>>('/trading/sales-prices', data)
}

export const updateTradingPrice = (id: number, data: Partial<TradingPrice>) =>
  request.put<ApiResponse<TradingPrice>>(`/trading/prices/${id}`, data)

export const deleteTradingPrice = (id: number) =>
  request.delete<ApiResponse<null>>(`/trading/prices/${id}`)
