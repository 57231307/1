import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 大货处方（production_recipe）状态
 * 后端状态机：draft→approved→closed；cancelled 仅草稿可作废
 */
export type ProductionRecipeStatus = 'draft' | 'approved' | 'closed' | 'cancelled';

/**
 * 大货处方物料明细项（出参形状）= 后端 models/production_recipe.rs::RecipeMaterialItem
 * （handler list/get/calculate 均直出该 Serialize 结构）。concentration/amount 后端为
 * Rust Decimal（Option<Decimal> / Decimal），序列化为 JSON 字符串，渲染处需显式转换；
 * 提交方向（建单/更新 recipe_detail、试算 items）另用 RecipeMaterialItemInput，勿共用。
 */
export interface RecipeMaterialItem {
  material_code: string;
  material_name: string;
  /** 后端 Option<Decimal> ⇒ 出参 string | null */
  concentration: string | null;
  unit: string;
  /** 后端 Decimal ⇒ 出参 string */
  amount: string;
  category: string;
}

/**
 * 大货处方物料明细项（提交入参形状）：后端 RecipeMaterialItem 的 concentration 为
 * Option<Decimal>、amount 为 Decimal——rust_decimal 反序列化同时接受 JSON number 与
 * 字符串（rust_decimal-1.42.1/src/serde.rs visit_f64），表单数值直接按 number 提交合法。
 * amount 后端无 serde(default)，键必须存在（缺键后端请求边界显式 422）。
 */
export interface RecipeMaterialItemInput {
  material_code: string;
  material_name: string;
  concentration?: number | null;
  unit: string;
  amount: number;
  category: string;
}

/**
 * 大货处方出参 = 后端 models/production_recipe.rs::Model 直接序列化
 * （handlers/production_recipe_handler.rs 各端点返回类型直书 production_recipe::Model）。
 * fabric_width/gram_weight/fabric_weight/bath_volume/adjustment_factor/total_dye_cost/
 * total_auxiliary_cost 均为 Rust Decimal（多数 Option），序列化为 JSON 字符串
 * （liquor_ratio 后端本体是 String "1:N"），渲染/计算前需显式 Number() 转换。
 */
export interface ProductionRecipe {
  id: number;
  recipe_no: string;
  work_order_id: number | null;
  dye_batch_id: number | null;
  source_recipe_id: number | null;
  lab_dip_resample_id: number | null;
  customer_id: number | null;
  color_no: string | null;
  fabric_name: string | null;
  fabric_spec: string | null;
  fabric_width: string | null;
  gram_weight: string | null;
  fabric_weight: string;
  equipment_no: string | null;
  liquor_ratio: string;
  bath_volume: string | null;
  adjustment_factor: string | null;
  recipe_detail: RecipeMaterialItem[] | null;
  total_dye_cost: string | null;
  total_auxiliary_cost: string | null;
  status: ProductionRecipeStatus;
  approved_by: number | null;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * 创建大货处方（对齐后端 CreateProductionRecipeRequest，production_recipe_service.rs:39-）。
 * fabric_weight 备布重量 kg（必填，用量计算依据）、liquor_ratio 浴比如 1:8（必填）；其余可选。
 * 建单人 created_by 与开单人 issued_by 都不是该后端请求结构体的入参
 * （service 按 handler 传入的会话用户 AuthContext.user_id 落库），前端不得上送这两个身份键。
 */
export interface CreateProductionRecipePayload {
  work_order_id?: number;
  dye_batch_id?: number;
  source_recipe_id?: number;
  lab_dip_resample_id?: number;
  customer_id?: number;
  color_no?: string;
  fabric_name?: string;
  fabric_spec?: string;
  fabric_width?: number;
  gram_weight?: number;
  fabric_weight: number;
  equipment_no?: string;
  liquor_ratio: string;
  bath_volume?: number;
  adjustment_factor?: number;
  recipe_detail?: RecipeMaterialItemInput[];
  total_dye_cost?: number;
  total_auxiliary_cost?: number;
  remarks?: string;
}

/**
 * 配方试算（对齐后端 CalculateAmountsRequest，production_recipe_service.rs:109）。
 * fabric_weight、liquor_ratio、items（物料明细，需含 concentration）均必填；adjustment_factor 可选。
 */
export interface CalculateAmountsPayload {
  fabric_weight: number;
  liquor_ratio: string;
  adjustment_factor?: number;
  items: RecipeMaterialItemInput[];
}

/**
 * 更新大货处方请求体（对齐后端 UpdateProductionRecipeRequest，
 * services/production_recipe_service.rs:67-86；仅 draft 态可更新，由 service 的状态门拒绝）。
 * 与新建载荷分开声明：后端 UpdateProductionRecipeRequest 全字段可选（仅 draft 态可改），
 * 且新建/更新两个后端结构体里都**没有** issued_by/created_by 两列（身份只来自服务端会话），
 * 复用新建类型会让调用方以为能改开单人/制单人而后端静默丢弃。
 */
export interface UpdateProductionRecipePayload {
  work_order_id?: number;
  dye_batch_id?: number;
  source_recipe_id?: number;
  lab_dip_resample_id?: number;
  customer_id?: number;
  color_no?: string;
  fabric_name?: string;
  fabric_spec?: string;
  fabric_width?: number;
  gram_weight?: number;
  fabric_weight?: number;
  equipment_no?: string;
  liquor_ratio?: string;
  bath_volume?: number;
  adjustment_factor?: number;
  recipe_detail?: RecipeMaterialItemInput[];
  total_dye_cost?: number;
  total_auxiliary_cost?: number;
  remarks?: string;
}

/**
 * 大货处方列表查询参数——严格对齐后端 production_recipe_handler.rs::ProductionRecipeListQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 * color_no 为色号（空即白坯）。
 */
export interface ProductionRecipeListQuery {
  page?: number;
  page_size?: number;
  work_order_id?: number;
  dye_batch_id?: number;
  customer_id?: number;
  color_no?: string;
  status?: string;
}

export function getProductionRecipeList(
  params?: ProductionRecipeListQuery
): Promise<ApiResponse<PaginatedResponse<ProductionRecipe>>> {
  return request.get('/production/production-recipes', { params });
}

export function getProductionRecipe(id: number): Promise<ApiResponse<ProductionRecipe>> {
  return request.get(`/production/production-recipes/${id}`);
}

export function createProductionRecipe(
  data: CreateProductionRecipePayload
): Promise<ApiResponse<ProductionRecipe>> {
  return request.post('/production/production-recipes', data);
}

export function updateProductionRecipe(
  id: number,
  data: UpdateProductionRecipePayload
): Promise<ApiResponse<ProductionRecipe>> {
  return request.put(`/production/production-recipes/${id}`, data);
}

export function deleteProductionRecipe(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/production-recipes/${id}`);
}

// 审批端点无请求体：审批人身份唯一来源是服务端会话（AuthContext.user_id），
// 后端 handlers/production_recipe_handler.rs::approve 不绑定 body 提取器，
// 前端不得再声明/发送 approved_by。
export function approveProductionRecipe(id: number): Promise<ApiResponse<ProductionRecipe>> {
  return request.post(`/production/production-recipes/${id}/approve`);
}

export function closeProductionRecipe(id: number): Promise<ApiResponse<ProductionRecipe>> {
  return request.post(`/production/production-recipes/${id}/close`);
}

export function cancelProductionRecipe(id: number): Promise<ApiResponse<ProductionRecipe>> {
  return request.post(`/production/production-recipes/${id}/cancel`);
}

export function calculateRecipeAmounts(
  data: CalculateAmountsPayload
): Promise<ApiResponse<RecipeMaterialItem[]>> {
  return request.post('/production/production-recipes/calculate', data);
}

export function listRecipeAdditions(id: number): Promise<ApiResponse<unknown[]>> {
  return request.get(`/production/production-recipes/${id}/additions`);
}

export function createRecipeAddition(
  id: number,
  data: {
    production_recipe_id: number;
    work_order_id?: number;
    dye_batch_id?: number;
    addition_reason?: string;
    addition_detail?: Array<{
      material_code: string;
      material_name: string;
      amount: number;
      unit: string;
      category: string;
    }>;
    total_cost?: number;
    remarks?: string;
  }
): Promise<ApiResponse<unknown>> {
  return request.post(`/production/production-recipes/${id}/additions`, data);
}

// 同上：加料审批人身份取会话，后端 handlers/production_recipe_handler.rs::approve_addition
// 不绑定 body 提取器，前端不再传 approved_by。
export function approveRecipeAddition(additionId: number): Promise<ApiResponse<unknown>> {
  return request.post(`/production/production-recipes/additions/${additionId}/approve`);
}

export function closeRecipeAddition(additionId: number): Promise<ApiResponse<unknown>> {
  return request.post(`/production/production-recipes/additions/${additionId}/close`);
}
