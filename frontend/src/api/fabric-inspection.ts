import { request } from './request';

export interface FabricInspection {
  id: number;
  status: string;
  [key: string]: unknown;
}

export function getFabricInspectionList(params?: Record<string, unknown>) {
  return request.get('/production/fabric-inspections', { params });
}

export function createFabricInspection(data: Record<string, unknown>) {
  return request.post('/production/fabric-inspections', data);
}

export function getFabricInspectionByNo(no: string) {
  return request.get(`/production/fabric-inspections/by-no/${no}`);
}

export function getFabricInspectionDetail(id: number) {
  return request.get(`/production/fabric-inspections/${id}`);
}

export function updateFabricInspection(id: number, data: Record<string, unknown>) {
  return request.put(`/production/fabric-inspections/${id}`, data);
}

export function deleteFabricInspection(id: number) {
  return request.delete(`/production/fabric-inspections/${id}`);
}

export function gradeFabricInspection(id: number, data: Record<string, unknown>) {
  return request.post(`/production/fabric-inspections/${id}/grade`, data);
}

export function closeFabricInspection(id: number) {
  return request.post(`/production/fabric-inspections/${id}/close`);
}

export function addFabricRoll(id: number, data: Record<string, unknown>) {
  return request.post(`/production/fabric-inspections/${id}/roll`, data);
}

export function createFabricDefect(data: Record<string, unknown>) {
  return request.post('/production/fabric-defects', data);
}

/** 按验布单查询疵点列表（GET /fabric-inspections/{inspectionId}/defects） */
export function listFabricDefectsByInspection(inspectionId: number) {
  return request.get(`/production/fabric-inspections/${inspectionId}/defects`);
}

export function getFabricDefect(id: number) {
  return request.get(`/production/fabric-defects/${id}`);
}

export function deleteFabricDefect(id: number) {
  return request.delete(`/production/fabric-defects/${id}`);
}

export const INSPECTION_STATUS_LABEL: Record<string, string> = {
  draft: '草稿',
  inspecting: '检验中',
  graded: '已定级',
  closed: '已关闭',
};
