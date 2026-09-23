import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

export interface Chemical {
  id: number;
  chemical_code: string;
  chemical_name: string;
  chemical_type: string;
  status?: string;
  [key: string]: unknown;
}

export interface CreateChemicalPayload {
  chemical_code: string;
  chemical_name: string;
  chemical_name_en?: string;
  chemical_type: string;
  category_id?: number;
  cas_number?: string;
  [key: string]: unknown;
}

export function getChemicalList(params?: Record<string, unknown>) {
  return request.get('/chemicals', { params });
}

export function createChemical(data: CreateChemicalPayload) {
  return request.post('/chemicals', data);
}

export function getChemicalByCode(code: string) {
  return request.get(`/chemicals/by-code/${code}`);
}

export function updateChemical(id: number, data: Partial<CreateChemicalPayload>) {
  return request.put(`/chemicals/${id}`, data);
}

export function deleteChemical(id: number) {
  return request.delete(`/chemicals/${id}`);
}

export function getChemicalLotList(params?: Record<string, unknown>) {
  return request.get<ApiResponse<PaginatedResponse<Record<string, unknown>>>>('/chemical-lots', {
    params,
  });
}

export function createChemicalLot(data: Record<string, unknown>) {
  return request.post('/chemical-lots', data);
}

export function getChemicalLotByNo(no: string) {
  return request.get(`/chemical-lots/by-no/${no}`);
}

export function updateChemicalLot(id: number, data: Record<string, unknown>) {
  return request.put(`/chemical-lots/${id}`, data);
}

export function deleteChemicalLot(id: number) {
  return request.delete(`/chemical-lots/${id}`);
}

export function getChemicalCategoryTree() {
  return request.get('/chemical-categories/tree');
}

export function getChemicalCategoryList() {
  return request.get<ApiResponse<PaginatedResponse<Record<string, unknown>>>>(
    '/chemical-categories'
  );
}

export function createChemicalCategory(data: Record<string, unknown>) {
  return request.post('/chemical-categories', data);
}

export function updateChemicalCategory(id: number, data: Record<string, unknown>) {
  return request.put(`/chemical-categories/${id}`, data);
}

export function deleteChemicalCategory(id: number) {
  return request.delete(`/chemical-categories/${id}`);
}
