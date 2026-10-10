import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Product {
  id: number;
  product_code: string;
  product_name: string;
  category_id: number;
  category_name?: string;
  unit?: string;
  price?: number;
  cost_price?: number;
  barcode?: string;
  specification?: string;
  description?: string;
  /**
   * 每匹米数（匹↔米换算元数据，可空）；仅换算视图入参，不参与库存计量（models/product.rs:70）。
   * 后端 rust_decimal 序列化为字符串（如 "100.00"）、NULL→null；本接口亦用于创建/更新入参（表单数值），
   * 故取并集。消费点：ProductFormDialogTab 编辑回显需 Number() 归一后绑定 el-input-number。
   */
  meters_per_piece?: number | string | null;
  /**
   * 每卷米数（卷↔米换算元数据，可空）；仅换算视图入参，不参与库存计量（models/product.rs:74）。
   * 同上：Decimal→string，NULL→null，并含表单入参数值。
   */
  meters_per_roll?: number | string | null;
  is_active: boolean;
  created_at?: string;
  updated_at?: string;
}

/**
 * 产品色号响应模型 —— 逐字段对齐后端 models/product_color.rs::Model（直接 serde 出参，snake_case）。
 * 此前接口写有 color_code / rgb / price_adjustment 三个后端不存在的幽灵键，且缺
 * color_type / pantone_code / dye_formula / extra_cost —— 那三键读回恒 undefined（假字段）。
 * extra_cost 为 rust_decimal（NOT NULL），序列化为字符串（如 "0.00"），运算/展示前需 Number() 归一。
 * NOT NULL 列（color_no/color_name/color_type/extra_cost/is_active）不得标 `?`。
 */
export interface ProductColor {
  id: number;
  product_id: number;
  /** 色号（对应后端 product_color.color_no，NOT NULL） */
  color_no: string;
  color_name: string;
  /** 潘通色号（Option，NULL→null） */
  pantone_code: string | null;
  /** 色号类型（NOT NULL；DB 列默认 'STANDARD'，全仓唯一确证取值） */
  color_type: string;
  /** 染色配方（Option，NULL→null） */
  dye_formula: string | null;
  /** 特殊色号加价，rust_decimal 出参为字符串（"0.00"），Number() 归一后再展示/运算 */
  extra_cost: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

/**
 * 创建色号载荷 —— 逐字段对齐后端 product_handler.rs::CreateProductColorRequest。
 * 非 Option 必填：color_no / color_name / color_type / extra_cost（handler 无 #[validate]、无 validate()，
 * 但 serde 对非 Option 字段缺键即反序列化失败 422，故四者必发）。
 * extra_cost 为 f64 入参 → JSON number；pantone_code/dye_formula 为 Option，空值省略键。
 */
export interface CreateProductColorPayload {
  color_no: string;
  color_name: string;
  color_type: string;
  extra_cost: number;
  pantone_code?: string;
  dye_formula?: string;
}

/**
 * 更新色号载荷 —— 逐字段对齐后端 product_handler.rs::UpdateProductColorRequest（全部 Option）。
 * 后端更新结构无 color_no（色号编码不可经 PUT 改），故此处不含该键，避免误发被丢弃。
 */
export interface UpdateProductColorPayload {
  color_name?: string;
  color_type?: string;
  extra_cost?: number;
  pantone_code?: string;
  dye_formula?: string;
  is_active?: boolean;
}

/**
 * 产品分类响应模型 —— 对齐 models/product_category.rs::Model（列表）与
 * services/product_category_service.rs::CategoryTreeNode（/tree，含 children、无 code）。
 * 此前接口写有 level / sort_order 两个后端不存在的幽灵键，已移除。
 */
export interface ProductCategory {
  id: number;
  name: string;
  code: string;
  parent_id?: number;
  description?: string;
  children?: ProductCategory[];
}

/**
 * 创建分类载荷 —— 对齐 CreateProductCategoryRequest。name 非 Option 必填(min1)；
 * code/parent_id/description 为 Option，空值省略键（后端 code 未传时按名称生成 CAT-<ts> 落库）。
 */
export interface CreateProductCategoryPayload {
  name: string;
  code?: string;
  parent_id?: number;
  description?: string;
}

/**
 * 更新分类载荷 —— 对齐 UpdateProductCategoryRequest（全部 Option，未携带即不改）。
 */
export interface UpdateProductCategoryPayload {
  name?: string;
  code?: string;
  parent_id?: number;
  description?: string;
}

/**
 * GET /products 查询参数，对齐后端 handlers/product_handler.rs::ProductListQuery
 * （无 rename_all，snake_case；全部 Option）。
 */
export interface ProductQueryParams {
  page?: number;
  page_size?: number;
  category_id?: number;
  /** 后端 handlers/product_handler.rs ProductListQuery.status（products.status 为小写枚举） */
  status?: 'active' | 'inactive';
  /** 后端 ProductListQuery.search：名称/编码模糊匹配 */
  search?: string;
}

/**
 * GET /products/export 查询参数，对齐后端 product_handler.rs::ExportProductsQuery
 * （无 page/page_size；含敏感导出审批令牌 download_token）。
 */
export interface ExportProductsQuery {
  category_id?: number;
  status?: string;
  search?: string;
  download_token?: string;
}

/**
 * 产品导入结果。
 * 对齐后端 utils/import_export.rs::ImportResult（serde 序列化，snake_case）。
 */
export interface ProductImportError {
  row: number;
  column: string;
  message: string;
  value: string;
}

export interface ProductImportResult {
  total_count: number;
  success_count: number;
  error_count: number;
  errors: ProductImportError[];
}

// D14 Batch 5b：原 productApi.list 转为风格 B 函数
export const getProductList = (params?: ProductQueryParams) =>
  request.get<ApiResponse<{ items: Product[]; total: number }>>('/products', { params });

// D14 Batch 5b：原 productApi.getById 转为风格 B 函数
export const getProductById = (id: number) => request.get<ApiResponse<Product>>(`/products/${id}`);

/**
 * UI 侧 Product 模型沿用 product_name / product_code / is_active / price，
 * 后端 products 实体与 Create/UpdateProductRequest 的字段是 name / code / status /
 * standard_price。此前直接提交导致后端把名称当成缺省值（handler 里 name 缺省为
 * "产品_<时间戳>"），UI 新建的产品名称全被丢掉。这里在请求边界做一次完整映射，
 * 未提供的字段保持不发送（由后端按自身默认值处理），不做任何取值兜底。
 */
function toProductPayload(data: Partial<Product>) {
  const { product_name, product_code, is_active, price, ...rest } = data;
  const payload: Record<string, unknown> = { ...rest };
  if (product_name !== undefined) payload.name = product_name;
  if (product_code !== undefined) payload.code = product_code;
  if (is_active !== undefined) payload.status = is_active ? 'active' : 'inactive';
  if (price !== undefined) payload.standard_price = price;
  return payload;
}

// D14 Batch 5b：原 productApi.create 转为风格 B 函数
export const createProduct = (data: Partial<Product>) =>
  request.post<ApiResponse<Product>>('/products', toProductPayload(data));

// D14 Batch 5b：原 productApi.update 转为风格 B 函数
export const updateProduct = (id: number, data: Partial<Product>) =>
  request.put<ApiResponse<Product>>(`/products/${id}`, toProductPayload(data));

// D14 Batch 5b：原 productApi.delete 转为风格 B 函数
export const deleteProduct = (id: number) => request.delete<ApiResponse<null>>(`/products/${id}`);

// D14 Batch 5b：原 productApi.batchCreate 转为风格 B 函数
export const batchCreateProducts = (data: Partial<Product>[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>(
    '/products/batch/create',
    data.map(toProductPayload)
  );

// D14 Batch 5b：原 productApi.batchUpdate 转为风格 B 函数
export const batchUpdateProducts = (data: Partial<Product>[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>(
    '/products/batch/update',
    data.map(toProductPayload)
  );

// D14 Batch 5b：原 productApi.batchDelete 转为风格 B 函数
export const batchDeleteProducts = (ids: number[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/delete', {
    ids,
  });

// D14 Batch 5b：原 productApi.getCategories 转为风格 B 函数
/** 宏 define_crud_handlers! 生成的 list 返回 json!({"items","total"})（crud_macro.rs:202-207） */
export const getProductCategoryList = () =>
  request.get<ApiResponse<{ items: ProductCategory[]; total: number }>>('/product-categories');

// D14 Batch 5b：原 productApi.createCategory 转为风格 B 函数
export const createProductCategory = (data: CreateProductCategoryPayload) =>
  request.post<ApiResponse<ProductCategory>>('/product-categories', data);

// D14 Batch 5b：原 productApi.updateCategory 转为风格 B 函数
export const updateProductCategory = (id: number, data: UpdateProductCategoryPayload) =>
  request.put<ApiResponse<ProductCategory>>(`/product-categories/${id}`, data);

// D14 Batch 5b：原 productApi.deleteCategory 转为风格 B 函数
export const deleteProductCategory = (id: number) =>
  request.delete<ApiResponse<null>>(`/product-categories/${id}`);

// D14 Batch 5b：原 productApi.getCategoryTree 转为风格 B 函数
export const getProductCategoryTree = () =>
  request.get<ApiResponse<ProductCategory[]>>('/product-categories/tree');

// D14 Batch 5b：原 productApi.getColors 转为风格 B 函数
export const getProductColorList = (productId: number) =>
  request.get<ApiResponse<ProductColor[]>>(`/products/${productId}/colors`);

// D14 Batch 5b：原 productApi.createColor 转为风格 B 函数
export const createProductColor = (productId: number, data: CreateProductColorPayload) =>
  request.post<ApiResponse<ProductColor>>(`/products/${productId}/colors`, data);

// D14 Batch 5b：原 productApi.updateColor 转为风格 B 函数
export const updateProductColor = (
  productId: number,
  colorId: number,
  data: UpdateProductColorPayload
) => request.put<ApiResponse<ProductColor>>(`/products/${productId}/colors/${colorId}`, data);

// D14 Batch 5b：原 productApi.deleteColor 转为风格 B 函数
export const deleteProductColor = (productId: number, colorId: number) =>
  request.delete<ApiResponse<null>>(`/products/${productId}/colors/${colorId}`);

// D14 Batch 5b：原 productApi.batchCreateColors 转为风格 B 函数
// 后端 body 是 Json<BatchCreateColorsRequest>（product_handler.rs:161-163），结构只有一个键
// colors: Vec<CreateProductColorRequest>——此前发裸数组与 DTO 顶层形状不符，serde 反序列化
// 直接失败（必 422）。这里按 DTO 包一层 { colors }；元素载荷与单条创建同构，复用
// CreateProductColorPayload（Partial<ProductColor> 是响应模型，extra_cost 为字符串且缺
// color_type 必填语义，不能冒充创建契约）。
export const batchCreateProductColors = (productId: number, colors: CreateProductColorPayload[]) =>
  request.post<ApiResponse<ProductColor[]>>(`/products/${productId}/colors/batch`, { colors });

// D14 Batch 5b：原 productApi.getImportTemplate 转为风格 B 函数
export const getProductImportTemplate = () =>
  request.get<Blob>('/products/import-template', { responseType: 'blob' });

// D14 Batch 5b：原 productApi.importProducts 转为风格 B 函数
// P2-16 修复：导入结果 ApiResponse<any> → ProductImportResult
export const importProducts = (file: File) => {
  const formData = new FormData();
  formData.append('file', file);
  return request.post<ApiResponse<ProductImportResult>>('/products/import', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  });
};

// D14 Batch 5b：原 productApi.export 转为风格 B 函数
export const exportProducts = (params?: ExportProductsQuery) =>
  request.get<Blob>('/products/export', { params, responseType: 'blob' });
