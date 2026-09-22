import { request } from './request';

export interface FabricInspection {
  id: number;
  status: string;
  [key: string]: unknown;
}

export function getFabricInspectionList(params?: Record<string, unknown>) {
  return request.get('/production/fabric-inspections', { params });
}

/** 建单入参：与后端 CreateInspectionRequest 一一对应（inspection_date 为必填，缺它即 422） */
export interface CreateFabricInspectionPayload {
  /** 验布日期 YYYY-MM-DD（后端 NaiveDate，必填） */
  inspection_date: string;
  dye_lot_no?: string;
  product_id?: number;
  product_name?: string;
  color_no?: string;
  inspector_id?: number;
  inspector_name?: string;
  machine_no?: string;
  /** 评分制式：four_point / ten_point，见 constants/fabric-scoring */
  scoring_system?: string;
  fabric_width_inches?: number;
  remarks?: string;
}

/** 更新入参：后端 UpdateInspectionRequest 只接受这五个字段（缸号/色号/日期建单后不可改） */
export interface UpdateFabricInspectionPayload {
  inspector_id?: number;
  inspector_name?: string;
  machine_no?: string;
  scoring_system?: string;
  fabric_width_inches?: number;
  remarks?: string;
}

export function createFabricInspection(data: CreateFabricInspectionPayload) {
  return request.post('/production/fabric-inspections', data);
}

export function getFabricInspectionByNo(no: string) {
  return request.get(`/production/fabric-inspections/by-no/${no}`);
}

export function getFabricInspectionDetail(id: number) {
  return request.get(`/production/fabric-inspections/${id}`);
}

export function updateFabricInspection(id: number, data: UpdateFabricInspectionPayload) {
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
