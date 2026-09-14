import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

/**
 * 化学品主数据状态：active → inactive / discontinued（更新时校验）
 */
export type ChemicalStatus = 'active' | 'inactive' | 'discontinued';

export interface Chemical {
  id: number;
  chemical_code: string;
  chemical_name: string;
  chemical_name_en: string | null;
  chemical_type: string;
  category_id: number | null;
  dye_category: string | null;
  auxiliary_category: string | null;
  cas_number: string | null;
  specification: string | null;
  unit: string;
  standard_price: string | number;
  cost_price: string | number;
  ghs_classification: string | null;
  un_number: string | null;
  hazard_class: string | null;
  signal_word: string | null;
  msds_url: string | null;
  msds_version: string | null;
  shelf_life_days: number | null;
  storage_condition: string | null;
  storage_temperature: string | null;
  safety_stock: string | number;
  reorder_point: string | number;
  reorder_quantity: string | number;
  supplier_id: number | null;
  status: ChemicalStatus;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

export interface ListChemicalsQuery extends QueryParams {
  chemical_type?: string;
  category_id?: number;
  dye_category?: string;
  auxiliary_category?: string;
  supplier_id?: number;
  cas_number?: string;
  ghs_classification?: string;
}

/**
 * 创建化学品主数据负载（对齐后端 CreateChemicalMasterRequest，必填：编码/名称/类型）
 */
export interface CreateChemicalPayload {
  chemical_code: string;
  chemical_name: string;
  chemical_name_en?: string;
  chemical_type: string;
  category_id?: number;
  dye_category?: string;
  auxiliary_category?: string;
  cas_number?: string;
  specification?: string;
  unit?: string;
  standard_price?: number;
  cost_price?: number;
  ghs_classification?: string;
  un_number?: string;
  hazard_class?: string;
  signal_word?: string;
  msds_url?: string;
  msds_version?: string;
  shelf_life_days?: number;
  storage_condition?: string;
  storage_temperature?: string;
  safety_stock?: number;
  reorder_point?: number;
  reorder_quantity?: number;
  supplier_id?: number;
  remarks?: string;
}

export type UpdateChemicalPayload = Partial<CreateChemicalPayload>;

export function getChemicals(
  params?: ListChemicalsQuery
): Promise<ApiResponse<PaginatedResponse<Chemical>>> {
  return request.get('/chemicals', { params });
}

export function getChemicalByCode(code: string): Promise<ApiResponse<Chemical>> {
  return request.get(`/chemicals/by-code/${code}`);
}

export function getChemical(id: number): Promise<ApiResponse<Chemical>> {
  return request.get(`/chemicals/${id}`);
}

export function createChemical(data: CreateChemicalPayload): Promise<ApiResponse<Chemical>> {
  return request.post('/chemicals', data);
}

export function updateChemical(
  id: number,
  data: UpdateChemicalPayload
): Promise<ApiResponse<Chemical>> {
  return request.put(`/chemicals/${id}`, data);
}

export function deleteChemical(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/chemicals/${id}`);
}
