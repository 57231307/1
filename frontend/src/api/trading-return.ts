// trading-return.ts - 交易退货 API 桩
// 来源：拆分原 trading/index.vue 时统一接口调用而创建
// 后端真实路由：
//   采购退货 /purchase/returns（purchase.rs purchase_return_routes()）
//   销售退货 /sales/sales-returns（sales.rs，由 sales_return_handler::router() 提供）
//   旧 /trading/* 域仅保留 5 个 GET 列表端点（analytics.rs trading()）
import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

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

// 采购退货：后端 purchase_return_handler::list_purchase_returns 返回 PaginatedResponse
// （data = {items, total, page, page_size}）。
export const getTradingPurchaseReturnList = (params?: Record<string, unknown>) =>
  request.get<ApiResponse<PaginatedResponse<TradingReturn>>>('/purchase/returns', { params });

// 销售退货：后端 advanced::list_sales_returns 返回 Vec<SalesReturn>（data 为裸数组）。
export const getTradingSalesReturnList = () =>
  request.get<ApiResponse<TradingReturn[]>>('/trading/sales-returns');

// 兼容原调用点的分发函数：委托到上述两个类型明确的导出，避免同一函数体里出现两个不同
// 信封声明导致门禁按函数名匹配失败。
export const getTradingReturnList = (params: ListTradingReturnParams) =>
  params.type === 'sales' ? getTradingSalesReturnList() : getTradingPurchaseReturnList();

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
