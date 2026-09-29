import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 供应商商品目录（后端序列化 models/supplier_product.rs::Model 直接出参，snake_case）。
 * 字段逐一对齐出参键，禁止编造。
 */
export interface SupplierProduct {
  id: number;
  supplier_id: number;
  product_code: string;
  product_name: string;
  product_description: string | null;
  unit: string;
  is_enabled: boolean;
  created_at: string;
  updated_at: string;
  created_by: number | null;
  updated_by: number | null;
  remarks: string | null;
}

/**
 * 供应商商品色号目录（后端序列化 models/supplier_product_color.rs::Model）。
 * extra_cost 为 rust_decimal，序列化为字符串，运算/展示前需 Number() 归一，禁止对字符串直接 .toFixed。
 */
export interface SupplierProductColor {
  id: number;
  supplier_product_id: number;
  color_no: string;
  color_name: string;
  pantone_code: string | null;
  extra_cost: string;
  is_enabled: boolean;
  created_at: string;
  updated_at: string;
  remarks: string | null;
}

/** GET /purchase/supplier-products 查询参数，对齐 handler::ListSupplierProductsQuery。 */
export interface SupplierProductQueryParams {
  supplier_id?: number;
  keyword?: string;
  is_enabled?: boolean;
  page?: number;
  page_size?: number;
}

/** GET /purchase/supplier-product-colors 查询参数，对齐 handler::ListSupplierProductColorsQuery。 */
export interface SupplierProductColorQueryParams {
  supplier_product_id?: number;
  keyword?: string;
  is_enabled?: boolean;
  page?: number;
  page_size?: number;
}

/** 创建供应商商品 body，对齐 service::CreateSupplierProductRequest。 */
export interface CreateSupplierProductPayload {
  supplier_id: number;
  product_code: string;
  product_name: string;
  product_description?: string | null;
  unit: string;
  is_enabled?: boolean;
  remarks?: string | null;
}

/** 更新供应商商品 body，对齐 service::UpdateSupplierProductRequest（未传 is_enabled 保留原值）。 */
export interface UpdateSupplierProductPayload {
  supplier_id: number;
  product_code: string;
  product_name: string;
  product_description?: string | null;
  unit: string;
  is_enabled?: boolean;
  remarks?: string | null;
}

/** 创建供应商商品色号 body，对齐 service::CreateSupplierProductColorRequest。 */
export interface CreateSupplierProductColorPayload {
  supplier_product_id: number;
  color_no: string;
  color_name: string;
  pantone_code?: string | null;
  /** 字符串承载 Decimal，默 '0' */
  extra_cost?: string | null;
  is_enabled?: boolean;
}

/** 更新供应商商品色号 body，对齐 service::UpdateSupplierProductColorRequest。 */
export interface UpdateSupplierProductColorPayload {
  supplier_product_id: number;
  color_no: string;
  color_name: string;
  pantone_code?: string | null;
  extra_cost?: string | null;
  is_enabled?: boolean;
}

export function getSupplierProductList(
  params?: SupplierProductQueryParams
): Promise<ApiResponse<PaginatedResponse<SupplierProduct>>> {
  return request.get('/purchase/supplier-products', { params });
}

export function getSupplierProduct(id: number): Promise<ApiResponse<SupplierProduct>> {
  return request.get(`/purchase/supplier-products/${id}`);
}

export function createSupplierProduct(
  data: CreateSupplierProductPayload
): Promise<ApiResponse<SupplierProduct>> {
  return request.post('/purchase/supplier-products', data);
}

export function updateSupplierProduct(
  id: number,
  data: UpdateSupplierProductPayload
): Promise<ApiResponse<SupplierProduct>> {
  return request.put(`/purchase/supplier-products/${id}`, data);
}

export function getSupplierProductColorList(
  params?: SupplierProductColorQueryParams
): Promise<ApiResponse<PaginatedResponse<SupplierProductColor>>> {
  return request.get('/purchase/supplier-product-colors', { params });
}

export function getSupplierProductColor(id: number): Promise<ApiResponse<SupplierProductColor>> {
  return request.get(`/purchase/supplier-product-colors/${id}`);
}

export function createSupplierProductColor(
  data: CreateSupplierProductColorPayload
): Promise<ApiResponse<SupplierProductColor>> {
  return request.post('/purchase/supplier-product-colors', data);
}

export function updateSupplierProductColor(
  id: number,
  data: UpdateSupplierProductColorPayload
): Promise<ApiResponse<SupplierProductColor>> {
  return request.put(`/purchase/supplier-product-colors/${id}`, data);
}
