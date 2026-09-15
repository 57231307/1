import { request } from './request';

export interface FabricInspection {
  id: number;
  status: string;
  [key: string]: unknown;
}

export function getFabricInspectionList(params?: Record<string, unknown>) {
  return request.get('/fabric-inspections', { params });
}

export function createFabricInspection(data: Record<string, unknown>) {
  return request.post('/fabric-inspections', data);
}

export function getFabricInspectionByNo(no: string) {
  return request.get(`/fabric-inspections/by-no/${no}`);
}

export function getFabricInspectionDetail(id: number) {
  return request.get(`/fabric-inspections/${id}`);
}

export function updateFabricInspection(id: number, data: Record<string, unknown>) {
  return request.put(`/fabric-inspections/${id}`, data);
}

export function deleteFabricInspection(id: number) {
  return request.delete(`/fabric-inspections/${id}`);
}

export function gradeFabricInspection(id: number, data: Record<string, unknown>) {
  return request.post(`/fabric-inspections/${id}/grade`, data);
}

export function closeFabricInspection(id: number) {
  return request.post(`/fabric-inspections/${id}/close`);
}

export function addFabricRoll(id: number, data: Record<string, unknown>) {
  return request.post(`/fabric-inspections/${id}/roll`, data);
}

export function createFabricDefect(data: Record<string, unknown>) {
  return request.post('/fabric-defects', data);
}

export function getFabricDefect(id: number) {
  return request.get(`/fabric-defects/${id}`);
}

export function deleteFabricDefect(id: number) {
  return request.delete(`/fabric-defects/${id}`);
}

export const INSPECTION_STATUS_LABEL: Record<string, string> = {
  draft: '草稿',
  inspecting: '检验中',
  graded: '已定级',
  closed: '已关闭',
};
