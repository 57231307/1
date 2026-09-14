// trading-price.ts - 交易价格 API 桩
// 来源：拆分原 trading/index.vue 时统一接口调用而创建
// 后端真实路由：
//   采购价格 /purchase/purchase-prices（purchase.rs purchase_prices()）
//   销售价格 /sales/sales-prices（sales.rs sales_prices()）
//   旧 /trading/* 域仅保留 5 个 GET 列表端点（analytics.rs trading()）
import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface TradingPrice {
  id: number;
  product_name: string;
  supplier_name?: string;
  customer_name?: string;
  price: number;
  currency: string;
  unit: string;
  effective_date: string;
  expiry_date?: string;
  status: string;
}

export interface ListTradingPriceParams {
  type: 'purchase' | 'sales';
}

export const getTradingPriceList = (params: ListTradingPriceParams) => {
  if (params.type === 'purchase') {
    return request.get<ApiResponse<TradingPrice[]>>('/trading/purchase-prices');
  }
  return request.get<ApiResponse<TradingPrice[]>>('/trading/sales-prices');
};

export const getTradingPrice = (id: number, type: 'purchase' | 'sales') =>
  type === 'purchase'
    ? request.get<ApiResponse<TradingPrice>>(`/purchase/purchase-prices/${id}`)
    : request.get<ApiResponse<TradingPrice>>(`/sales/sales-prices/${id}`);

export const createTradingPrice = (
  data: Partial<TradingPrice> & { type: 'purchase' | 'sales' }
) => {
  if (data.type === 'purchase') {
    return request.post<ApiResponse<TradingPrice>>('/purchase/purchase-prices', data);
  }
  return request.post<ApiResponse<TradingPrice>>('/sales/sales-prices', data);
};

export const updateTradingPrice = (
  id: number,
  data: Partial<TradingPrice>,
  type: 'purchase' | 'sales'
) =>
  type === 'purchase'
    ? request.put<ApiResponse<TradingPrice>>(`/purchase/purchase-prices/${id}`, data)
    : request.put<ApiResponse<TradingPrice>>(`/sales/sales-prices/${id}`, data);

export const deleteTradingPrice = (id: number, type: 'purchase' | 'sales') =>
  type === 'purchase'
    ? request.delete<ApiResponse<null>>(`/purchase/purchase-prices/${id}`)
    : request.delete<ApiResponse<null>>(`/sales/sales-prices/${id}`);
