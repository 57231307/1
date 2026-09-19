import { request } from './request';

export interface SalesOrderHit {
  order_no: string;
  customer_id: number;
  customer_name: string;
  total_amount: number;
  status: string;
  created_at: string;
}

export interface CustomerHit {
  id: number;
  code: string;
  name: string;
  contact_person?: string;
  phone?: string;
  tier: string;
}

export interface ProductHit {
  id: number;
  code: string;
  name: string;
  category?: string;
  status: string;
}

export interface SearchResponse<T> {
  total: number;
  took_ms: number;
  hits: T[];
}

export function searchSalesOrders(
  q: string,
  params?: { from?: number; size?: number; status?: string }
) {
  return request.get<SearchResponse<SalesOrderHit>>('/search/sales-orders', {
    params: { q, ...params },
  });
}

export function searchCustomers(
  q: string,
  params?: { from?: number; size?: number; tier?: string }
) {
  return request.get<SearchResponse<CustomerHit>>('/search/customers', {
    params: { q, ...params },
  });
}

export function searchProducts(
  q: string,
  params?: { from?: number; size?: number; category?: string }
) {
  return request.get<SearchResponse<ProductHit>>('/search/products', { params: { q, ...params } });
}
