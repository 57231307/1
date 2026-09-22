import { request } from './request';
import type { ApiResponse } from '@/types/api';

// list_indicators -> EvaluationIndicatorQuery（snake_case wire，无 rename_all）
export interface EvaluationIndicatorQueryParams {
  page?: number;
  page_size?: number;
  category?: string;
  status?: string;
}

// list_evaluation_records / list_evaluations -> EvaluationRecordQuery（snake_case wire）
export interface EvaluationRecordQueryParams {
  page?: number;
  page_size?: number;
  supplier_id?: number;
  period?: string;
}

export interface EvaluationIndicator {
  id?: number;
  indicatorCode?: string;
  indicatorName?: string;
  category?: string;
  weight?: number;
  maxScore?: number;
  status?: string;
  description?: string;
  createdAt?: string;
  updatedAt?: string;
}

export interface EvaluationRecord {
  id?: number;
  supplierId?: number;
  supplierName?: string;
  evaluationDate?: string;
  period?: string;
  totalScore?: number;
  rating?: string;
  status?: string;
  evaluatorId?: number;
  evaluatorName?: string;
  remark?: string;
  createdAt?: string;
  updatedAt?: string;
}

export interface SupplierScore {
  supplierId?: number;
  supplierName?: string;
  totalScore?: number;
  rating?: string;
  rank?: number;
}

// 后端 CreateEvaluationIndicatorRequest 无 rename_all，wire 为 snake_case。
// 后端无 description 字段（评估方法为 evaluation_method），故不声明 description。
export interface CreateEvaluationIndicatorRequest {
  indicator_code: string;
  indicator_name: string;
  category: string;
  weight: number;
  max_score: number;
  evaluation_method?: string;
}

// 后端 SupplierEvaluationRequest 为「单指标打分」：supplier_id / evaluation_period /
// indicator_id / score 均必填，wire 为 snake_case。
export interface CreateEvaluationRequest {
  supplier_id: number;
  evaluation_period: string;
  indicator_id: number;
  score: number;
  remark?: string;
}

export function getEvaluationIndicatorList(
  params?: EvaluationIndicatorQueryParams
  // 后端 supplier_evaluation_handler::list_indicators 返回 ApiResponse<Vec<Model>> ⇒ 裸数组
): Promise<ApiResponse<EvaluationIndicator[]>> {
  return request.get('/purchase/supplier-evaluations/indicators', { params });
}

export function createIndicator(
  data: CreateEvaluationIndicatorRequest
): Promise<ApiResponse<EvaluationIndicator>> {
  return request.post('/purchase/supplier-evaluations/indicators', data);
}

export function getEvaluationRecordList(
  params?: EvaluationRecordQueryParams
  // 后端 list_evaluation_records / list_evaluations 均返回 ApiResponse<Vec<Model>> ⇒ 裸数组
): Promise<ApiResponse<EvaluationRecord[]>> {
  return request.get('/purchase/supplier-evaluations/records', { params });
}

export function getEvaluationRecord(id: number): Promise<ApiResponse<EvaluationRecord>> {
  return request.get(`/purchase/supplier-evaluations/records/${id}`);
}

export function createEvaluationRecord(
  data: CreateEvaluationRequest
): Promise<ApiResponse<EvaluationRecord>> {
  return request.post('/purchase/supplier-evaluations/records', data);
}

export function getSupplierScore(supplierId: number): Promise<ApiResponse<SupplierScore>> {
  return request.get(`/purchase/supplier-evaluations/suppliers/${supplierId}/score`);
}

export function getSupplierRankings(params?: {
  limit?: number;
}): Promise<ApiResponse<SupplierScore[]>> {
  return request.get('/purchase/supplier-evaluations/rankings', { params });
}

export function getEvaluationList(
  params?: EvaluationRecordQueryParams
  // 后端 list_evaluation_records / list_evaluations 均返回 ApiResponse<Vec<Model>> ⇒ 裸数组
): Promise<ApiResponse<EvaluationRecord[]>> {
  return request.get('/purchase/supplier-evaluations', { params });
}

export function getEvaluation(id: number): Promise<ApiResponse<EvaluationRecord>> {
  return request.get(`/purchase/supplier-evaluations/${id}`);
}

export function createEvaluation(
  data: CreateEvaluationRequest
): Promise<ApiResponse<EvaluationRecord>> {
  return request.post('/purchase/supplier-evaluations', data);
}

export function updateEvaluation(
  id: number,
  data: Partial<EvaluationRecord>
): Promise<ApiResponse<EvaluationRecord>> {
  return request.put(`/purchase/supplier-evaluations/${id}`, data);
}

export function deleteEvaluation(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/supplier-evaluations/${id}`);
}
