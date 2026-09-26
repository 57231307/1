import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 染色配方状态词表（唯一真相源：backend/src/models/status/quality_dyeing.rs::dye_recipe，
 * 与迁移 v15 的 CHECK "chk_dye_recipe_status" 取值集逐项相等；中文只出现在 i18n 展示层）
 */
export const DYE_RECIPE_STATUS = {
  DRAFT: 'draft',
  PENDING_APPROVAL: 'pending_approval',
  APPROVED: 'approved',
  DISABLED: 'disabled',
} as const;

export type DyeRecipeStatus = (typeof DYE_RECIPE_STATUS)[keyof typeof DYE_RECIPE_STATUS];

export interface DyeRecipe {
  id: number;
  recipe_no: string;
  recipe_name: string;
  color_code: string;
  color_name: string;
  fabric_type: string;
  version: number;
  status: DyeRecipeStatus;
  recipe_items: RecipeItem[];
  process_parameters: Record<string, unknown>;
  created_by: number;
  created_by_name: string;
  approved_by: number;
  approved_by_name: string;
  approved_at: string;
  created_at: string;
  updated_at: string;
}

export interface RecipeItem {
  id: number;
  recipe_id: number;
  chemical_name: string;
  chemical_code: string;
  dosage: number;
  dosage_unit: string;
  sequence: number;
  remark: string;
}

/**
 * 染色配方列表查询参数——严格对齐后端 dye_recipe_handler.rs::DyeRecipeListQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 * 色号字段后端为 color_code（不是 color_no），status 取 DYE_RECIPE_STATUS 词表。
 * download_token 为敏感导出审批令牌（页面暂未暴露，故不主动传）。
 */
export interface DyeRecipeListParams {
  page?: number;
  page_size?: number;
  recipe_no?: string;
  color_code?: string;
  color_name?: string;
  dye_type?: string;
  status?: string;
  download_token?: string;
}

export function getDyeRecipeList(
  params?: DyeRecipeListParams
): Promise<ApiResponse<{ items: DyeRecipe[]; total: number; page: number; page_size: number }>> {
  return request.get('/production/dye-recipes', { params });
}

export function getDyeRecipe(id: number): Promise<ApiResponse<DyeRecipe>> {
  return request.get(`/production/dye-recipes/${id}`);
}

export function createDyeRecipe(data: Partial<DyeRecipe>): Promise<ApiResponse<DyeRecipe>> {
  return request.post('/production/dye-recipes', data);
}

export function updateDyeRecipe(
  id: number,
  data: Partial<DyeRecipe>
): Promise<ApiResponse<DyeRecipe>> {
  return request.put(`/production/dye-recipes/${id}`, data);
}

export function deleteDyeRecipe(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/dye-recipes/${id}`);
}

/// 审批请求体：后端 approve_recipe 要求 approved_by（真实登录用户 ID，禁止伪造/默认值）
export interface ApproveDyeRecipeRequest {
  approved_by: number;
}

export function approveDyeRecipe(
  id: number,
  data: ApproveDyeRecipeRequest
): Promise<ApiResponse<void>> {
  return request.post(`/production/dye-recipes/${id}/approve`, data);
}

export function submitDyeRecipe(id: number): Promise<ApiResponse<void>> {
  return request.post(`/production/dye-recipes/${id}/submit`);
}

export function createNewVersion(id: number): Promise<ApiResponse<DyeRecipe>> {
  return request.post(`/production/dye-recipes/${id}/version`);
}

export function getRecipesByColor(colorCode: string): Promise<ApiResponse<DyeRecipe[]>> {
  return request.get(`/production/dye-recipes/by-color/${colorCode}`);
}

export function getRecipeVersions(id: number): Promise<ApiResponse<DyeRecipe[]>> {
  return request.get(`/production/dye-recipes/${id}/versions`);
}

export function exportDyeRecipes(params?: DyeRecipeListParams): Promise<Blob> {
  return request.get('/production/dye-recipes/export', { params, responseType: 'blob' });
}
