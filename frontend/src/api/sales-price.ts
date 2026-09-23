import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface SalesPrice {
  id: number;
  product_id: number;
  product_name: string;
  product_code: string;
  customer_id: number;
  customer_name: string;
  price: number;
  currency: string;
  unit: string;
  min_order_qty?: number;
  price_type?: string;
  price_level?: string;
  effective_date: string;
  expiry_date: string;
  status: 'pending' | 'active' | 'expired' | 'inactive';
  remark: string;
  created_at: string;
  updated_at: string;
}

export interface PricingStrategy {
  id: number;
  name: string;
  description: string;
  type: 'tiered' | 'volume' | 'contract';
  rules: PricingStrategyRule[];
  status: 'active' | 'inactive';
  created_at: string;
  updated_at: string;
}

export interface PricingStrategyRule {
  id: number;
  strategy_id: number;
  min_quantity: number;
  max_quantity?: number;
  discount_rate: number;
  price?: number;
}

// 后端 sales_price_handler::SalesPriceQuery（list_prices 的 Query<T>，全字段 Option、snake_case）。
// download_token 为敏感导出 fail-closed 审批令牌：页面未暴露不等于类型不该有，仍如实声明。
export interface SalesPriceQuery {
  product_id?: number;
  customer_type?: string;
  status?: string;
  page?: number;
  page_size?: number;
  download_token?: string;
}

// 后端 sales_price_handler::list_prices 返回 ApiResponse<Vec<Model>> ⇒ 裸数组
export function getSalesPriceList(params?: SalesPriceQuery): Promise<ApiResponse<SalesPrice[]>> {
  return request.get('/sales/sales-prices', { params });
}

export function getSalesPrice(id: number): Promise<ApiResponse<SalesPrice>> {
  return request.get(`/sales/sales-prices/${id}`);
}

export function createSalesPrice(data: Partial<SalesPrice>): Promise<ApiResponse<SalesPrice>> {
  return request.post('/sales/sales-prices', data);
}

export function updateSalesPrice(
  id: number,
  data: Partial<SalesPrice>
): Promise<ApiResponse<SalesPrice>> {
  return request.put(`/sales/sales-prices/${id}`, data);
}

export function deleteSalesPrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/sales/sales-prices/${id}`);
}

// 审批销售定价请求体：对齐后端 sales_price_handler::ApprovePriceRequest。
// approved 必填布尔（通过/拒绝均需显式留痕）；remark 可选审批意见。
export interface ApproveSalesPriceRequest {
  approved: boolean;
  remark?: string;
}

export function approveSalesPrice(
  id: number,
  data: ApproveSalesPriceRequest
): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-prices/${id}/approve`, data);
}

export function getPriceHistory(productId: number): Promise<ApiResponse<SalesPrice[]>> {
  return request.get(`/sales/sales-prices/history/${productId}`);
}

// 后端 sales_price_handler::list_strategies 返回 PaginatedResponse ⇒ {items,total,page,page_size}
export function getPricingStrategyList(): Promise<
  ApiResponse<{ items: PricingStrategy[]; total: number; page: number; page_size: number }>
> {
  return request.get('/sales/sales-prices/strategies');
}
