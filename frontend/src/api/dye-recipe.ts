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
  /** 配方正文列（后端 dye_recipe.chemical_formula，Option → 可空；建单/编辑表单以此键提交正文） */
  chemical_formula: string | null;
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

/**
 * 染色配方更新载荷 —— 逐字段对齐后端 UpdateDyeRecipeRequest
 * （services/dye_recipe_service.rs，三态语义 RFC 7386 JSON Merge Patch）：
 * 键缺席=保持原值、显式 null=清空为 NULL（仅声明 `| null` 的 DB 可空列）、有值=覆盖。
 * 可空列依据 system 域 DDL（system/mod.rs:113-131 补列全部可空）。
 * color_code 为建表 NOT NULL（system/m0003_add_dye_tables.rs:30），不声明 null——
 * 显式 null 会被后端 business_displayable 拒绝。此前用 Partial<DyeRecipe>（出参形状）
 * 冒充更新契约：含后端更新 DTO 不消费的 recipe_no/recipe_name/recipe_items 等键（serde 丢弃），
 * 且可空列无法表达显式 null。
 */
export interface DyeRecipeAuxiliary {
  name: string;
  amount: number;
  unit: string;
}

export interface DyeRecipeUpdatePayload {
  color_no?: string | null;
  color_code?: string;
  color_name?: string | null;
  fabric_type?: string | null;
  dye_type?: string | null;
  chemical_formula?: string | null;
  temperature?: number | null;
  time_minutes?: number | null;
  ph_value?: number | null;
  liquor_ratio?: number | null;
  auxiliaries?: DyeRecipeAuxiliary[] | null;
  status?: string | null;
  remarks?: string | null;
}

export function updateDyeRecipe(
  id: number,
  data: DyeRecipeUpdatePayload
): Promise<ApiResponse<DyeRecipe>> {
  return request.put(`/production/dye-recipes/${id}`, data);
}

export function deleteDyeRecipe(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/dye-recipes/${id}`);
}

// 审批端点无请求体：审批人身份唯一来源是服务端会话（AuthContext.user_id），
// 后端 handlers/dye_recipe_handler.rs::approve_recipe 不绑定 body 提取器，
// 前端不得再声明/发送 approved_by（否则触发 check-api-request 载荷失配）。
export function approveDyeRecipe(id: number): Promise<ApiResponse<void>> {
  return request.post(`/production/dye-recipes/${id}/approve`);
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
