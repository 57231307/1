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

/**
 * 评估指标出参 = `supplier_evaluation_indicator::Model`
 * （`backend/src/models/supplier_evaluation_indicator.rs`，list_indicators 直接返回裸数组）。
 * `evaluation_method` 是库内唯一的说明列——界面上"描述"列曾绑到不存在的 description。
 */
export interface EvaluationIndicator {
  id: number;
  indicator_code: string;
  indicator_name: string;
  category: string;
  weight: number;
  max_score: number;
  evaluation_method?: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

/**
 * 评估记录出参 = `EvaluationRecordView`
 * （`backend/src/services/supplier_evaluation_service.rs`，records/evaluations 及其 by-id 详情端点
 *  返回，`list_evaluation_records`/`list_evaluations`/`get_evaluation_record`/`get_evaluation` 富化视图）。
 *
 * 本表按「供应商 × 周期 × 单指标」逐行存储，一行即一次单指标评分，
 * 只有该条的 score / weighted_score，不存在跨指标聚合列（total_score / rating / status）——
 * 供应商级汇总请走 /rankings 或 /suppliers/{id}/score（SupplierScore），不要在前端自行聚合。
 * supplier_name / indicator_name / evaluator_name 由后端 LEFT JOIN 富化，外键缺失时为 null。
 * score / weighted_score 为 rust_decimal 序列化的 JSON 字符串（后端只开 serde 特性），
 * 不得建模为 number；需要参与运算时须在明确边界处 Number() 转换并注释。
 */
export interface EvaluationRecord {
  id: number;
  supplier_id: number;
  evaluation_period: string;
  indicator_id: number;
  score: string;
  max_score: number | null;
  weighted_score: string | null;
  evaluator_id: number | null;
  evaluation_date: string | null;
  remark: string | null;
  created_at: string;
  // 后端 LEFT JOIN 富化列（外键为 NULL 或关联缺失时为 null）
  supplier_name: string | null;
  indicator_name: string | null;
  evaluator_name: string | null;
}

/**
 * 供应商评分出参 = `SupplierScoreResponse`
 * （`backend/src/services/supplier_evaluation_service.rs`，score/rankings 端点返回）。
 * 真实字段为 average_score / total_records / rating / latest_evaluation_date；
 * `rank` 为排名榜前端按返回顺序本地计算（index+1），非后端字段。
 */
export interface SupplierScore {
  supplier_id: number;
  average_score: number;
  total_records: number;
  rating: string;
  latest_evaluation_date?: string | null;
  supplier_name?: string | null;
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

// 后端 UpdateEvaluationDto（supplier_evaluation_handler.rs:31）仅接受 score / remark。
export interface UpdateEvaluationRequest {
  score?: number;
  remark?: string;
}

export function getEvaluationIndicatorList(
  params?: EvaluationIndicatorQueryParams
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
  data: UpdateEvaluationRequest
): Promise<ApiResponse<EvaluationRecord>> {
  return request.put(`/purchase/supplier-evaluations/${id}`, data);
}

export function deleteEvaluation(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/supplier-evaluations/${id}`);
}
