import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Product {
  id: number;
  product_code: string;
  product_name: string;
  category_id: number;
  category_name?: string;
  unit?: string;
  /** 售价（standard_price 出参别名；DECIMAL 列 NULL→null，入参可为数字或十进制字符串） */
  price?: number | string | null;
  /** 成本价（cost_price，DECIMAL 列 NULL→null，同上） */
  cost_price?: number | string | null;
  barcode?: string;
  specification?: string;
  description?: string;
  /**
   * 产品类型（面料行业字段，products.product_type，NOT NULL，存中文词表值：坯布/成品布/辅料）。
   * 词表与后端 models/status/general.rs::product_type 同源，前端经 constants/product-type.ts 消费。
   */
  product_type?: string | null;
  /** 纱支（yarn_count，可空文本主档列） */
  yarn_count?: string | null;
  /** 密度（density，可空文本主档列） */
  density?: string | null;
  /** 组织结构（structure，可空文本主档列） */
  structure?: string | null;
  /** 后整理（finish，可空文本主档列） */
  finish?: string | null;
  /**
   * 带针幅宽（cm，含针边，可空）。与 width（有效幅宽，净幅，不含针边）两列独立；
   * 开幅与预缩交付参考用本列，米↔公斤换算一律以 width 为源、本列不参与换算。
   * Decimal→string、NULL→null，并含表单入参数值。
   */
  width_needle_included_cm?: number | string | null;
  /** 克重公差（±g/m²，非负，可空）。目标值在 gram_weight，本列记允差幅度；Decimal→string、NULL→null。 */
  weight_tolerance?: number | string | null;
  /** 氨纶含量（%，0 到 100，可空）。弹性要求的行业口径；Decimal→string、NULL→null。 */
  spandex_percent?: number | string | null;
  /** 用途（自由文本，如 服装面料/家纺/产业用，可空） */
  usage?: string | null;
  /**
   * 安全类别（GB 18401-2010 的 A/B/C，可空即未定级）。A 类婴幼儿另须满足 GB 31701-2015。
   * 取值以 constants/product-spec-vocab.ts 与后端 product_safety_category 词表同源。
   */
  safety_category?: string | null;
  /** 最小起订量（min_order_quantity，DECIMAL 列：出参字符串、入参数值，空值 null） */
  min_order_quantity?: number | string | null;
  /** 交货期（天，lead_time，i32 列，JSON number，NULL→null） */
  lead_time?: number | null;
  /** 执行标准号（execution_standard，可空文本合规列） */
  execution_standard?: string | null;
  /** 生产厂名（factory_name，可空文本合规列） */
  factory_name?: string | null;
  /** 生产厂址（factory_address，可空文本合规列） */
  factory_address?: string | null;
  /** 产品等级（product_grade，可空文本合规列） */
  product_grade?: string | null;
  /**
   * 每公斤出米数（派生只读键 meters_per_kg，不入库、不回传后端）。
   * 由后端 DualUnitConverter::meters_per_kg 按克重与幅宽算出并随出参下发，
   * Decimal 序列化为字符串（如 "3.3"）、不可算（缺列或非正）为 null；
   * 表单即时值走 utils/fabric-yardage.ts 同源计算，两处同一口径。
   */
  meters_per_kg?: string | null;
  /**
   * 每匹米数（匹↔米换算元数据，可空）；仅换算视图出参与直连 API 与批量与导入的入参，
   * 不参与库存计量（models/product.rs）。产品新建与编辑表单已不再采集本值（每匹米数是
   * 库存实收口径的换算默认值，不是产品主档录入项）。一卷即一匹，按卷换算共用本列。
   * 后端 rust_decimal 序列化为字符串（如 "100.00"）、NULL→null。
   */
  meters_per_piece?: number | string | null;
  /**
   * 幅宽（cm，可空）。报价单转销售订单按「米数 × 克重 × 幅宽」派生公斤数，缺该值转单会点名拒绝；
   * 与 meters_per_piece 同口径：Decimal→string、NULL→null，并含表单入参数值。
   */
  width?: number | string | null;
  /**
   * 克重（g/m²，可空）。同上，是转单派生公斤数的必需元数据；Decimal→string、NULL→null。
   */
  gram_weight?: number | string | null;
  /**
   * 面料成分（如 "65%棉 35%涤"，可空）。对应后端 models/product.rs 的 fabric_composition（Option<String>，NULL→null）。
   * 报价明细编辑器选中产品后随编码/幅宽/克重一并只读带出展示；报价明细表无对应列，不落库，
   * 报价转销售订单的换算链按报价行实时回查 products 主档取值，不消费明细快照。
   * 本接口同时被产品表单入参复用（字符串），故可空即未配置。
   */
  fabric_composition?: string | null;
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
  /**
   * 色牢度等级（GB 18401 九档，含半级；可空即未考核或未录入）。
   * 取值以 constants/product-spec-vocab.ts 与后端 color_fastness 词表同源。
   */
  color_fastness_grade: string | null;
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
  /** 色牢度等级（可空即未录入），值为 GB 18401 九档之一 */
  color_fastness_grade?: string;
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
  /** 色牢度等级（键缺席即不改；空串按清空该列处理） */
  color_fastness_grade?: string;
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
 * meters_per_kg 是后端随查询出参下发的派生只读键（写侧 DTO 不收该键、列里也不存它），
 * 提交前摘除，避免把出参键当入参回传造成契约噪声。
 */
function toProductPayload(data: Partial<Product>) {
  const { product_name, product_code, is_active, price, ...rest } = data;
  const payload: Record<string, unknown> = { ...rest };
  delete payload.meters_per_kg;
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
// 后端批量创建的顶层 DTO 是带 products 键的结构体（bulk_product_handler 内定义），
// 裸数组反序列化必失败，这里按 DTO 包一层 products，元素载荷与单条创建同源经
// toProductPayload 映射，米数两键原样透传给后端做同源补齐与落库。
export const batchCreateProducts = (data: Partial<Product>[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/create', {
    products: data.map(toProductPayload),
  });

// D14 Batch 5b：原 productApi.batchUpdate 转为风格 B 函数
// 后端批量更新的顶层 DTO 同样是 products 包装（bulk_product_handler 内定义）。
// 元素的米数两键走三态更新，键缺席保持原值、显式 null 清空、有值覆盖，
// 只声明一边时由同源补齐复制到另一边。
export const batchUpdateProducts = (data: Partial<Product>[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/update', {
    products: data.map(toProductPayload),
  });

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
