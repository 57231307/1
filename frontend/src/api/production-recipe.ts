import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

/**
 * 大货处方（production_recipe）状态
 * 后端状态机：draft→approved→closed；cancelled 仅草稿可作废
 */
export type ProductionRecipeStatus = 'draft' | 'approved' | 'closed' | 'cancelled';

export interface RecipeMaterialItem {
  material_code: string;
  material_name: string;
  concentration: number | null;
  unit: string;
  amount: number | string;
  category: string;
}

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
  fabric_width: string | number | null;
  gram_weight: string | number | null;
  fabric_weight: string | number;
  equipment_no: string | null;
  liquor_ratio: string;
  bath_volume: string | number | null;
  adjustment_factor: string | number | null;
  recipe_detail: RecipeMaterialItem[] | null;
  total_dye_cost: string | number | null;
  total_auxiliary_cost: string | number | null;
  status: ProductionRecipeStatus;
  approved_by: number | null;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

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
  /** 备布重量 kg（必填，用量计算依据） */
  fabric_weight: number;
  equipment_no?: string;
  /** 浴比如 1:8（必填） */
  liquor_ratio: string;
  bath_volume?: number;
  adjustment_factor?: number;
  recipe_detail?: RecipeMaterialItem[];
  total_dye_cost?: number;
  total_auxiliary_cost?: number;
  remarks?: string;
}

export interface CalculateAmountsPayload {
  /** 备布重量 kg */
  fabric_weight: number;
  /** 浴比如 1:8 */
  liquor_ratio: string;
  adjustment_factor?: number;
  /** 物料明细（需包含 concentration） */
  items: RecipeMaterialItem[];
}

export interface ProductionRecipeListQuery extends QueryParams {
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
  data: Partial<CreateProductionRecipePayload>
): Promise<ApiResponse<ProductionRecipe>> {
  return request.put(`/production/production-recipes/${id}`, data);
}

export function deleteProductionRecipe(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/production-recipes/${id}`);
}

export function approveProductionRecipe(
  id: number,
  data: { approved_by: number }
): Promise<ApiResponse<ProductionRecipe>> {
  return request.post(`/production/production-recipes/${id}/approve`, data);
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

export function approveRecipeAddition(
  additionId: number,
  data: { approved_by: number }
): Promise<ApiResponse<unknown>> {
  return request.post(`/production/production-recipes/additions/${additionId}/approve`, data);
}

export function closeRecipeAddition(additionId: number): Promise<ApiResponse<unknown>> {
  return request.post(`/production/production-recipes/additions/${additionId}/close`);
}
