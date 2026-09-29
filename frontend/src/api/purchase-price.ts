import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表/详情出参 = 后端 purchase_price::Model（services/purchase_price_service.rs get_prices_list/get_price）。
// status 由写入方决定：建单写 master_data::PENDING、批准写 APPROVED（purchase_price_service.rs:101/137），
// 词表模块 models/status/general.rs 的 master_data（小写）。表内无产品/供应商名称列，需后端 JOIN。
export interface PurchasePrice {
  id: number;
  product_id: number;
  supplier_id: number;
  price: number;
  currency: string;
  unit: string;
  min_order_qty: number;
  price_type: string;
  effective_date: string;
  expiry_date: string | null;
  status: string;
  approved_by: number | null;
  approved_at: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
  /** 需要后端 JOIN：purchase_prices.product_id -> products.product_name（表内无名列） */
  product_name?: string | null;
  /** 需要后端 JOIN：purchase_prices.product_id -> products.product_code（表内无编码列） */
  product_code?: string | null;
  /** 需要后端 JOIN：purchase_prices.supplier_id -> suppliers.supplier_name（表内无名列） */
  supplier_name?: string | null;
}

/**
 * GET /purchase/purchase-prices 查询参数。对应后端 purchase_price_handler::PurchasePriceQuery
 * （backend/src/handlers/purchase_price_handler.rs:16），无 rename_all → snake_case，全 Option → 可选。
 * 注意：后端不读 keyword/product_name/supplier_name（原 QueryParams/视图里的这些键被 Axum 静默丢弃）。
 */
export interface PurchasePriceQueryParams {
  product_id?: number;
  supplier_id?: number;
  status?: string;
  page?: number;
  page_size?: number;
}

/**
 * 创建采购价格请求（严格对齐 backend CreatePurchasePriceInput，
 * services/purchase_price_service.rs:25）。
 * unit/price_type 必填：后端列 NOT NULL 无 DB 默认（m0009），缺失即被 400 校验拒绝。
 */
export interface CreatePurchasePricePayload {
  product_id: number;
  supplier_id: number;
  price: number;
  unit: string;
  price_type: string;
  currency?: string;
  min_order_qty?: number;
  effective_date?: string;
  expiry_date?: string;
}

/**
 * 更新采购价格请求（严格对齐 backend UpdatePriceRequest，
 * handlers/purchase_price_handler.rs:33）。
 * 后端 price 是 String（非数值），必须以字符串形式提交（否则 Serde 反序列化失败）。
 */
export interface UpdatePurchasePricePayload {
  price: string;
  expiry_date?: string;
  status?: string;
}

export function getPurchasePriceList(
  params?: PurchasePriceQueryParams
  // 后端 purchase_price_handler::list_prices 返回 ApiResponse<Vec<Model>> ⇒ 裸数组
): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get('/purchase/purchase-prices', { params });
}

export function getPurchasePrice(id: number): Promise<ApiResponse<PurchasePrice>> {
  return request.get(`/purchase/purchase-prices/${id}`);
}

export function createPurchasePrice(
  data: CreatePurchasePricePayload
): Promise<ApiResponse<PurchasePrice>> {
  return request.post('/purchase/purchase-prices', data);
}

export function updatePurchasePrice(
  id: number,
  data: UpdatePurchasePricePayload
): Promise<ApiResponse<null>> {
  return request.put(`/purchase/purchase-prices/${id}`, data);
}

export function deletePurchasePrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/purchase-prices/${id}`);
}

export function getPurchasePriceHistory(productId: number): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get(`/purchase/purchase-prices/history/${productId}`);
}
