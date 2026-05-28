import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface DyeRecipe {
  id: number
  recipe_no: string
  recipe_name: string
  color_code: string
  color_name: string
  fabric_type: string
  version: number
  status: 'draft' | 'approved' | 'obsolete'
  recipe_items: RecipeItem[]
  process_parameters: Record<string, any>
  created_by: number
  created_by_name: string
  approved_by: number
  approved_by_name: string
  approved_at: string
  created_at: string
  updated_at: string
}

export interface RecipeItem {
  id: number
  recipe_id: number
  chemical_name: string
  chemical_code: string
  dosage: number
  dosage_unit: string
  sequence: number
  remark: string
}

export function listDyeRecipes(params?: QueryParams): Promise<ApiResponse<DyeRecipe[]>> {
  return request.get('/dye-recipes', { params })
}

export function getDyeRecipe(id: number): Promise<ApiResponse<DyeRecipe>> {
  return request.get(`/dye-recipes/${id}`)
}

export function createDyeRecipe(data: Partial<DyeRecipe>): Promise<ApiResponse<DyeRecipe>> {
  return request.post('/dye-recipes', data)
}

export function updateDyeRecipe(
  id: number,
  data: Partial<DyeRecipe>
): Promise<ApiResponse<DyeRecipe>> {
  return request.put(`/dye-recipes/${id}`, data)
}

export function deleteDyeRecipe(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/dye-recipes/${id}`)
}

export function approveDyeRecipe(id: number): Promise<ApiResponse<void>> {
  return request.post(`/dye-recipes/${id}/approve`)
}

export function createNewVersion(id: number): Promise<ApiResponse<DyeRecipe>> {
  return request.post(`/dye-recipes/${id}/version`)
}

export function getRecipesByColor(colorCode: string): Promise<ApiResponse<DyeRecipe[]>> {
  return request.get(`/dye-recipes/by-color/${colorCode}`)
}

export function getRecipeVersions(id: number): Promise<ApiResponse<DyeRecipe[]>> {
  return request.get(`/dye-recipes/${id}/versions`)
}
