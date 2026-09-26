import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';
import type { ProductColor } from './product';

/**
 * 色号懒加载（虚拟滚动），对应 GET /products/{id}/colors?keyword=&page_size=50。
 * 后端正在加 keyword/page_size 过滤，前端先透传。
 */
export function getProductColors(
  productId: number,
  params?: { keyword?: string; page_size?: number }
): Promise<ApiResponse<ProductColor[]>> {
  return request.get(`/products/${productId}/colors`, { params });
}

/**
 * 对齐后端 SkuMappingDto（snake_case，Decimal→string）。
 * 列表 handler 直接 serde_json::to_value(Vec<Model>) 出参键即此。
 */
export interface SkuMapping {
  id: number;
  product_id: number;
  product_code: string;
  product_color_id: number | null;
  color_no: string | null;
  supplier_id: number;
  supplier_name: string;
  supplier_product_id: number;
  supplier_product_code: string;
  supplier_product_color_id: number | null;
  supplier_color_no: string | null;
  supplier_price: string | null;
  min_order_quantity: string | null;
  lead_time: number | null;
  priority: number;
  is_primary: boolean;
  is_enabled: boolean;
}

export interface SkuMappingQueryParams {
  page?: number;
  page_size?: number;
  product_id?: number;
  supplier_id?: number;
  is_enabled?: boolean;
  keyword?: string;
}

/** 对齐后端 CreateMappingRequest：product_id/supplier_id/supplier_product_id 必填，其余可选。 */
export interface CreateSkuMappingPayload {
  product_id: number;
  product_color_id?: number | null;
  supplier_id: number;
  supplier_product_id: number;
  supplier_product_color_id?: number | null;
  supplier_price?: string | null;
  min_order_quantity?: string | null;
  lead_time?: number | null;
  is_primary?: boolean;
  priority?: number;
  is_enabled?: boolean;
  remarks?: string | null;
}

/** 对齐后端 UpdateMappingRequest：与创建同形（后端为整体替换语义，product_id/supplier_id/supplier_product_id 必填）。 */
export interface UpdateSkuMappingPayload {
  product_id: number;
  product_color_id?: number | null;
  supplier_id: number;
  supplier_product_id: number;
  supplier_product_color_id?: number | null;
  supplier_price?: string | null;
  min_order_quantity?: string | null;
  lead_time?: number | null;
  is_primary?: boolean;
  priority?: number;
  is_enabled?: boolean;
  remarks?: string | null;
}

export interface ResolveSkuMappingParams {
  product_id: number;
  color_id?: number | null;
  supplier_id: number;
}

/**
 * 后端 resolve 端点 data 直接是 ResolvedSku（无 `.mapping` 包装层），
 * 键对齐 services/sku_mapping_service.rs::ResolvedSku。
 */
export interface ResolveSkuMappingResult {
  mapping_id: number;
  supplier_product_id: number;
  supplier_product_color_id: number | null;
  supplier_product_code: string;
  supplier_color_no: string | null;
  supplier_price: string | null;
  lead_time: number | null;
}

export interface SkuMappingImportError {
  row: number;
  message: string;
}

/** 对齐后端 ImportMappingResult{total_count,success_count,error_count,errors}。 */
export interface SkuMappingImportResult {
  total_count: number;
  success_count: number;
  error_count: number;
  errors: SkuMappingImportError[];
}

export function getSkuMappingList(
  params?: SkuMappingQueryParams
): Promise<ApiResponse<PaginatedResponse<SkuMapping>>> {
  return request.get('/purchase/sku-mappings', { params });
}

export function getSkuMapping(id: number): Promise<ApiResponse<SkuMapping>> {
  return request.get(`/purchase/sku-mappings/${id}`);
}

export function createSkuMapping(data: CreateSkuMappingPayload): Promise<ApiResponse<SkuMapping>> {
  return request.post('/purchase/sku-mappings', data);
}

export function updateSkuMapping(
  id: number,
  data: UpdateSkuMappingPayload
): Promise<ApiResponse<SkuMapping>> {
  return request.put(`/purchase/sku-mappings/${id}`, data);
}

export function deleteSkuMapping(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/sku-mappings/${id}`);
}

export function resolveSkuMapping(
  params: ResolveSkuMappingParams
): Promise<ApiResponse<ResolveSkuMappingResult>> {
  return request.get('/purchase/sku-mappings/resolve', { params });
}

export function importSkuMappings(file: File): Promise<ApiResponse<SkuMappingImportResult>> {
  const formData = new FormData();
  formData.append('file', file);
  return request.post('/purchase/sku-mappings/import', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  });
}
