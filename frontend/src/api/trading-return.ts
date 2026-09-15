// trading-return.ts - 交易退货 API 桩
// 来源：拆分原 trading/index.vue 时统一接口调用而创建
// 后端真实路由：
//   采购退货 /purchase/returns（purchase.rs purchase_return_routes()）
//   销售退货 /sales/sales-returns（sales.rs，由 sales_return_handler::router() 提供）
//   旧 /trading/* 域仅保留 5 个 GET 列表端点（analytics.rs trading()）
import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface TradingReturn {
  id: number;
  return_no: string;
  customer_name: string;
  order_no?: string;
  return_date: string;
  total_amount: number;
  reason?: string;
  status: string;
}

export interface ListTradingReturnParams {
  type: 'purchase' | 'sales';
}

export const getTradingReturnList = (params: ListTradingReturnParams) => {
  if (params.type === 'sales') {
    return request.get<ApiResponse<TradingReturn[]>>('/trading/sales-returns');
  }
  // 后端采购退货列表真实路由：GET /purchase/returns（purchase.rs purchase_return_routes()）
  return request.get<ApiResponse<TradingReturn[]>>('/purchase/returns');
};

export const getTradingReturn = (id: number, type: 'purchase' | 'sales') =>
  type === 'purchase'
    ? request.get<ApiResponse<TradingReturn>>(`/purchase/returns/${id}`)
    : request.get<ApiResponse<TradingReturn>>(`/sales/sales-returns/${id}`);

export const createTradingReturn = (
  data: Partial<TradingReturn> & { type: 'purchase' | 'sales' }
) => {
  if (data.type === 'sales') {
    return request.post<ApiResponse<TradingReturn>>('/sales/sales-returns', data);
  }
  return request.post<ApiResponse<TradingReturn>>('/purchase/returns', data);
};

export const updateTradingReturn = (
  id: number,
  data: Partial<TradingReturn>,
  type: 'purchase' | 'sales'
) =>
  type === 'purchase'
    ? request.put<ApiResponse<TradingReturn>>(`/purchase/returns/${id}`, data)
    : request.put<ApiResponse<TradingReturn>>(`/sales/sales-returns/${id}`, data);

export const deleteTradingReturn = (id: number, type: 'purchase' | 'sales') =>
  type === 'purchase'
    ? request.delete<ApiResponse<null>>(`/purchase/returns/${id}`)
    : request.delete<ApiResponse<null>>(`/sales/sales-returns/${id}`);

export const approveTradingReturn = (id: number, type: 'purchase' | 'sales') =>
  type === 'purchase'
    ? request.post<ApiResponse<TradingReturn>>(`/purchase/returns/${id}/approve`)
    : request.post<ApiResponse<TradingReturn>>(`/sales/sales-returns/${id}/approve`);
