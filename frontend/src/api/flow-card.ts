import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 流转卡（production_flow_card）状态
 * 后端状态机：pending→scheduled→preparing→dyeing→dyed→inspecting→completed→shipped；terminated 为异常终止
 */
export type FlowCardStatus =
  | 'pending'
  | 'scheduled'
  | 'preparing'
  | 'dyeing'
  | 'dyed'
  | 'inspecting'
  | 'completed'
  | 'shipped'
  | 'terminated';

export interface FlowCard {
  id: number;
  card_no: string;
  barcode: string;
  production_order_id: number;
  dye_batch_id: number | null;
  dye_lot_no: string | null;
  process_route_id: number | null;
  customer_id: number | null;
  customer_name: string | null;
  order_no: string | null;
  product_id: number | null;
  product_name: string | null;
  color_no: string | null;
  dyeing_requirements: string | null;
  planned_fabric_weight: string | number | null;
  actual_fabric_weight: string | number | null;
  current_step_seq: number;
  status: FlowCardStatus;
  priority: number;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * 创建流转卡请求（对齐后端 CreateFlowCardRequest，flow_card_dto.rs:44）。
 * production_order_id 必填：工艺卡必须挂在具体生产订单上；其余为可选。
 */
export interface CreateFlowCardPayload {
  production_order_id: number;
  dye_batch_id?: number;
  dye_lot_no?: string;
  process_route_id?: number;
  product_id?: number;
  product_name?: string;
  color_no?: string;
  dyeing_requirements?: string;
  planned_fabric_weight?: number;
  priority?: number;
  remarks?: string;
}

/**
 * 流转卡列表查询参数——严格对齐后端 flow_card_handler.rs::FlowCardListQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 * 支持真实筛选：卡号 card_no、条码 barcode、缸号 dye_lot_no、生产订单、状态、客户。
 */
export interface FlowCardListQuery {
  page?: number;
  page_size?: number;
  card_no?: string;
  barcode?: string;
  dye_lot_no?: string;
  production_order_id?: number;
  status?: string;
  customer_id?: number;
}

export interface ScheduleFlowCardPayload {
  dye_batch_id?: number;
  dye_lot_no?: string;
}

/** 备布完成请求（对齐后端 CompletePreparingRequest，flow_card_handler.rs:239）：实际称重量 kg 必填 */
export interface CompletePreparingPayload {
  actual_fabric_weight: number;
}

export function getFlowCardList(
  params?: FlowCardListQuery
): Promise<ApiResponse<PaginatedResponse<FlowCard>>> {
  return request.get('/production/flow-cards', { params });
}

export function getFlowCard(id: number): Promise<ApiResponse<FlowCard>> {
  return request.get(`/production/flow-cards/${id}`);
}

export function createFlowCard(data: CreateFlowCardPayload): Promise<ApiResponse<FlowCard>> {
  return request.post('/production/flow-cards', data);
}

export function updateFlowCard(
  id: number,
  data: Partial<CreateFlowCardPayload>
): Promise<ApiResponse<FlowCard>> {
  return request.put(`/production/flow-cards/${id}`, data);
}

export function deleteFlowCard(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/flow-cards/${id}`);
}

export function scheduleFlowCard(
  id: number,
  data?: ScheduleFlowCardPayload
): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/schedule`, data ?? {});
}

export function startPreparing(id: number): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/start-preparing`);
}

export function completePreparing(
  id: number,
  data: CompletePreparingPayload
): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/complete-preparing`, data);
}

export function startDyeing(id: number): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/start-dyeing`);
}

export function completeDyeing(id: number): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/complete-dyeing`);
}

export function startInspecting(id: number): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/start-inspecting`);
}

export function completeFlowCard(id: number): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/complete`);
}

export function shipFlowCard(id: number): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/ship`);
}

export function terminateFlowCard(
  id: number,
  data?: { reason?: string }
): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/terminate`, data ?? {});
}

export function reactivateFlowCard(id: number): Promise<ApiResponse<FlowCard>> {
  return request.post(`/production/flow-cards/${id}/reactivate`);
}
