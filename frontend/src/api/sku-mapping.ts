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

/**
 * 对齐后端 utils/import_export.rs::ImportError（serialize 无 rename，逐键为
 * row/column/message/value；后端 services/sku_mapping_service.rs::ImportMappingResult
 * 的 errors 元素即该类型）。对话框当前只渲染 row/message，但键形状按后端真实输出全量声明，
 * 不允许声明成缺键或可选的宽松形态。
 */
export interface SkuMappingImportError {
  row: number;
  column: string;
  message: string;
  value: string;
}

/** 对齐后端 ImportMappingResult{total_count,success_count,error_count,errors}。 */
export interface SkuMappingImportResult {
  total_count: number;
  success_count: number;
  error_count: number;
  errors: SkuMappingImportError[];
}

/**
 * 对齐后端 BatchCreateMappingsRequest（handlers/sku_mapping_handler.rs）：
 * 每行与单条新建同形（后端行 DTO 就是 CreateMappingRequest），行号即 items 的 1 基下标，
 * 上送体不带行号字段，避免「界面行序」与「上送行号」两套事实。
 */
export interface BatchCreateSkuMappingPayload {
  items: CreateSkuMappingPayload[];
}

/**
 * 对齐后端 BatchCreateMappingResult（services/sku_mapping_service.rs）。
 * 整批原子：created_count 要么等于 total_count（全成），要么为 0（任一行不合格，零落库）。
 * errors[].row 是 items 的 1 基下标，同一行可有多条（多列不合格）。
 */
export interface SkuMappingBatchResult {
  total_count: number;
  created_count: number;
  failed_count: number;
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

/**
 * 批量新增 SKU 对照（界面多行录入），POST /purchase/sku-mappings/batch。
 * 后端整批原子：任一行不合格则零落库并在 data.errors 里逐行点名行号与字段。
 */
export function batchCreateSkuMappings(
  data: BatchCreateSkuMappingPayload
): Promise<ApiResponse<SkuMappingBatchResult>> {
  return request.post('/purchase/sku-mappings/batch', data);
}
