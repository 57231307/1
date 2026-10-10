import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface FabricInspection {
  id: number;
  status: string;
  [key: string]: unknown;
}

/**
 * 打卷入库入参：与后端 `RollFabricRequest` 逐字段一致
 * （backend/src/services/fabric_inspection_service.rs:201-209，键名不变）。
 * 裁定 §3：roll_weight/roll_width/roll_gram_weight 为实测值必填——后端类型虽写作
 * `Option<Decimal>`，但 `#[validate(required)]` + service 入口 `req.validate()` 强制
 * 缺失即 400 VALIDATION_ERROR（外显「打卷入库必须填写××」），故前端不标 `?`。
 * 取值范围口径 = `validate_roll_preconditions`（:592-609）：四数值均须 >0，
 * 后端无上限/精度约束，前端校验不多不少（不额外收紧）。
 * Decimal 序列化为字符串是出参侧；入参按 JSON number 提交（serde Decimal 接受数字）。
 */
export interface RollFabricPayload {
  warehouse_id: number;
  roll_length: number;
  roll_weight: number;
  roll_width: number;
  roll_gram_weight: number;
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

export function startFabricInspection(id: number) {
  return request.post(`/production/fabric-inspections/${id}/start`);
}

export function closeFabricInspection(id: number) {
  return request.post(`/production/fabric-inspections/${id}/close`);
}

/**
 * 打卷入库（graded → rolled，生成 inventory_piece 染色匹）：后端 roll_fabric
 * 返回更新后的验布记录（InspectionModel，backend/src/services/fabric_inspection_service.rs:552-579）。
 * 三实测值必填口径见 RollFabricPayload；提交前必须先过对话框校验（rules 与后端 DTO 同口径）。
 */
export function addFabricRoll(id: number, data: RollFabricPayload) {
  return request.post<ApiResponse<FabricInspection>>(
    `/production/fabric-inspections/${id}/roll`,
    data
  );
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
  pending: '待验布',
  inspecting: '检验中',
  graded: '已定级',
  rolled: '已打卷',
  closed: '已关闭',
};
