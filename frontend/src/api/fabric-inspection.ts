import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

/**
 * 验布记录状态（后端状态机）：pending → inspecting → graded → rolled → closed
 */
export type FabricInspectionStatus = 'pending' | 'inspecting' | 'graded' | 'rolled' | 'closed';

export interface FabricInspection {
  id: number;
  inspection_no: string;
  flow_card_id: number | null;
  dye_lot_no: string | null;
  product_id: number | null;
  product_name: string | null;
  color_no: string | null;
  inspection_date: string;
  inspector_id: number | null;
  inspector_name: string | null;
  machine_no: string | null;
  /** 评分制式：4point（四分制）/ 10point（十分制） */
  scoring_system: string;
  inspected_yards: string | number;
  fabric_width_inches: string | number | null;
  total_defect_points: number;
  points_per_100_sq_yards: string | number | null;
  grade: string | null;
  qualification_rate: string | number | null;
  total_rolls: number;
  total_roll_length: string | number;
  total_roll_weight: string | number;
  status: FabricInspectionStatus;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

export interface ListFabricInspectionsQuery extends QueryParams {
  inspection_no?: string;
  dye_lot_no?: string;
  flow_card_id?: number;
  product_id?: number;
  status?: string;
}

/** 创建验布记录（对齐后端 CreateInspectionRequest） */
export interface CreateFabricInspectionPayload {
  flow_card_id?: number;
  dye_lot_no?: string;
  product_id?: number;
  product_name?: string;
  color_no?: string;
  inspection_date: string;
  inspector_id?: number;
  inspector_name?: string;
  machine_no?: string;
  scoring_system?: string;
  fabric_width_inches?: number;
  remarks?: string;
}

/** 评级请求（inspecting → graded） */
export interface GradeFabricInspectionPayload {
  inspected_yards: number;
  qualification_rate?: number;
}

/** 打卷入库请求（graded → rolled） */
export interface RollFabricPayload {
  warehouse_id: number;
  roll_length: number;
  roll_weight?: number;
  roll_width?: number;
  roll_gram_weight?: number;
}

export interface FabricDefect {
  id: number;
  inspection_id: number;
  defect_type: string;
  position_yards: string | number;
  defect_length_inches: string | number;
  direction: string | null;
  is_hole: boolean;
  is_continuous: boolean;
  is_half_width: boolean;
  points: number;
  description: string | null;
  created_at: string;
}

/** 创建疵点明细（对齐后端 CreateDefectRequest） */
export interface CreateFabricDefectPayload {
  inspection_id: number;
  defect_type: string;
  position_yards: number;
  defect_length_inches: number;
  direction?: string;
  is_hole?: boolean;
  is_continuous?: boolean;
  is_half_width?: boolean;
  description?: string;
}

// ==================== 验布记录 ====================

export function getFabricInspections(
  params?: ListFabricInspectionsQuery
): Promise<ApiResponse<PaginatedResponse<FabricInspection>>> {
  return request.get('/production/fabric-inspections', { params });
}

export function createFabricInspection(
  data: CreateFabricInspectionPayload
): Promise<ApiResponse<FabricInspection>> {
  return request.post('/production/fabric-inspections', data);
}

export function getFabricInspection(id: number): Promise<ApiResponse<FabricInspection>> {
  return request.get(`/production/fabric-inspections/${id}`);
}

export function updateFabricInspection(
  id: number,
  data: Partial<CreateFabricInspectionPayload>
): Promise<ApiResponse<FabricInspection>> {
  return request.put(`/production/fabric-inspections/${id}`, data);
}

export function deleteFabricInspection(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/fabric-inspections/${id}`);
}

/** 开始验布（pending → inspecting） */
export function startFabricInspection(id: number): Promise<ApiResponse<FabricInspection>> {
  return request.post(`/production/fabric-inspections/${id}/start`);
}

/** 评级（inspecting → graded） */
export function gradeFabricInspection(
  id: number,
  data: GradeFabricInspectionPayload
): Promise<ApiResponse<FabricInspection>> {
  return request.post(`/production/fabric-inspections/${id}/grade`, data);
}

/** 打卷入库（graded → rolled） */
export function rollFabric(
  id: number,
  data: RollFabricPayload
): Promise<ApiResponse<FabricInspection>> {
  return request.post(`/production/fabric-inspections/${id}/roll`, data);
}

/** 关闭归档（rolled → closed） */
export function closeFabricInspection(id: number): Promise<ApiResponse<FabricInspection>> {
  return request.post(`/production/fabric-inspections/${id}/close`);
}

// ==================== 疵点明细 ====================

export function getFabricDefects(
  inspectionId: number
): Promise<ApiResponse<FabricDefect[]>> {
  return request.get(`/production/fabric-inspections/${inspectionId}/defects`);
}

export function createFabricDefect(
  data: CreateFabricDefectPayload
): Promise<ApiResponse<FabricDefect>> {
  return request.post('/production/fabric-defects', data);
}
