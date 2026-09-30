import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 面料（产品档案）出参：真相源 backend/src/models/product.rs::Model
 * （list_products 用 serde_json::to_value(product::Model) 后叠加别名/富化键）。
 * 后端 rust_decimal 列一律序列化为字符串，价格/幅宽/克重渲染前用 Number() 归一。
 * category_name 由后端 attach_category_names 富化（handlers/product_handler.rs:306）。
 */
export interface Fabric {
  id: number;
  name: string;
  code: string;
  barcode?: string | null;
  category_id?: number | null;
  category_name?: string | null;
  specification?: string | null;
  unit: string;
  standard_price?: string | null;
  cost_price?: string | null;
  description?: string | null;
  status: string;
  is_deleted: boolean;
  product_type: string;
  fabric_composition?: string | null;
  yarn_count?: string | null;
  density?: string | null;
  width?: string | null;
  gram_weight?: string | null;
  structure?: string | null;
  finish?: string | null;
  min_order_quantity?: string | null;
  lead_time?: number | null;
  meters_per_piece?: string | null;
  meters_per_roll?: string | null;
  supplier_product_code?: string | null;
  supplier_id?: number | null;
  is_batch_managed?: boolean | null;
  batch_level?: string | null;
  execution_standard?: string | null;
  factory_name?: string | null;
  factory_address?: string | null;
  product_grade?: string | null;
  created_at: string;
  updated_at: string;
  // 后端 append_frontend_aliases 补的别名键（product_handler.rs:185）
  product_name?: string | null;
  product_code?: string | null;
  price?: string | null;
}

/**
 * 产品分类出参：backend/src/models/product_category.rs::Model。
 * 后端无 sort_order 列（DTO 亦不接收），排序需求见交付报告"需后端串行"。
 */
export interface FabricCategory {
  id: number;
  name: string;
  code: string;
  parent_id?: number | null;
  description?: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * 列表查询参数：严格对齐后端 handlers/product_handler.rs::ProductListQuery
 * （page/page_size/category_id/status/search，全部 Option，无 rename_all → snake_case）。
 * 关键词筛选的真实键名是 search；supplier_id / is_active 后端不读取，不再发送。
 */
export interface FabricQueryParams {
  page?: number;
  page_size?: number;
  category_id?: number;
  status?: string;
  search?: string;
}

/**
 * 导出查询参数：对齐 ExportProductsQuery（product_handler.rs:168，无分页键）。
 * download_token 为敏感导出审批令牌（fail-closed）。
 */
export interface ExportFabricsParams {
  category_id?: number;
  status?: string;
  search?: string;
  download_token?: string;
}

/**
 * 创建面料载荷：逐字段对齐 CreateProductRequest（product_handler.rs:34）。
 * 全部字段后端为 Option（编码留空由后端自动生成），故均可以省略；
 * 但不得再夹带实体回显键（id/created_at/…）——那是 Partial<实体> 反模式。
 */
export interface CreateFabricPayload {
  name?: string;
  code?: string;
  barcode?: string;
  category_id?: number;
  specification?: string;
  unit?: string;
  standard_price?: number;
  cost_price?: number;
  description?: string;
  status?: string;
  product_type?: string;
  fabric_composition?: string;
  yarn_count?: string;
  density?: string;
  width?: number;
  gram_weight?: number;
  structure?: string;
  finish?: string;
  min_order_quantity?: number;
  lead_time?: number;
  execution_standard?: string;
  factory_name?: string;
  factory_address?: string;
  product_grade?: string;
  meters_per_piece?: string | number;
  meters_per_roll?: string | number;
}

/**
 * 更新面料载荷：对齐 UpdateProductRequest（product_handler.rs:86）。
 * 注意：该 DTO 无 code 字段（编码创建后不可改），也不含 category_id。
 * 后端 #[validate(length)] 的 Option<String> 字段，空值必须整键省略
 * （Some("") 参与长度校验或落库空串），范式见 views/system/tabs/UserTab.vue。
 */
export interface UpdateFabricPayload {
  name?: string;
  barcode?: string;
  specification?: string;
  unit?: string;
  standard_price?: number;
  cost_price?: number;
  description?: string;
  status?: string;
  product_type?: string;
  fabric_composition?: string;
  yarn_count?: string;
  density?: string;
  width?: number;
  gram_weight?: number;
  structure?: string;
  finish?: string;
  min_order_quantity?: number;
  lead_time?: number;
  execution_standard?: string;
  factory_name?: string;
  factory_address?: string;
  product_grade?: string;
  meters_per_piece?: string | number;
  meters_per_roll?: string | number;
}

/** 创建分类载荷：name 必填（后端 min=1），code/parent_id/description 可选 */
export interface CreateFabricCategoryPayload {
  name: string;
  code?: string;
  parent_id?: number;
  description?: string;
}

/** 更新分类载荷：后端 UpdateProductCategoryRequest 全 Option */
export type UpdateFabricCategoryPayload = Partial<CreateFabricCategoryPayload>;

// D14 Batch 5b：原 fabricApi.list 转为风格 B 函数
export const getFabricList = (params?: FabricQueryParams) =>
  request.get<ApiResponse<PaginatedResponse<Fabric>>>('/products', { params });

// D14 Batch 5b：原 fabricApi.getById 转为风格 B 函数
export const getFabricById = (id: number) => request.get<ApiResponse<Fabric>>(`/products/${id}`);

// D14 Batch 5b：原 fabricApi.create 转为风格 B 函数
export const createFabric = (data: CreateFabricPayload) =>
  request.post<ApiResponse<Fabric>>('/products', data);

// D14 Batch 5b：原 fabricApi.update 转为风格 B 函数
export const updateFabric = (id: number, data: UpdateFabricPayload) =>
  request.put<ApiResponse<Fabric>>(`/products/${id}`, data);

// D14 Batch 5b：原 fabricApi.delete 转为风格 B 函数
export const deleteFabric = (id: number) => request.delete<ApiResponse<null>>(`/products/${id}`);

// D14 Batch 5b：原 fabricApi.getCategories 转为风格 B 函数
export const getFabricCategoryList = () =>
  request.get<ApiResponse<PaginatedResponse<FabricCategory>>>('/product-categories');

// D14 Batch 5b：原 fabricApi.createCategory 转为风格 B 函数
export const createFabricCategory = (data: CreateFabricCategoryPayload) =>
  request.post<ApiResponse<FabricCategory>>('/product-categories', data);

// D14 Batch 5b：原 fabricApi.updateCategory 转为风格 B 函数
export const updateFabricCategory = (id: number, data: UpdateFabricCategoryPayload) =>
  request.put<ApiResponse<FabricCategory>>(`/product-categories/${id}`, data);

// D14 Batch 5b：原 fabricApi.deleteCategory 转为风格 B 函数
export const deleteFabricCategory = (id: number) =>
  request.delete<ApiResponse<null>>(`/product-categories/${id}`);

// D14 Batch 5b：原 fabricApi.batchImport 转为风格 B 函数
export const batchImportFabrics = (data: Fabric[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>('/products/import', data);

// D14 Batch 5b：原 fabricApi.export 转为风格 B 函数
export const exportFabrics = (params?: ExportFabricsParams) =>
  request.get<Blob>('/products/export', { params, responseType: 'blob' });
