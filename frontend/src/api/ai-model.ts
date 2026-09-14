import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 模型版本状态（后端 validate_model_status）
 * 状态机：draft →（approval approved 后）active → retired → archived
 */
export type ModelVersionStatus = 'draft' | 'active' | 'retired' | 'archived';

/** 模型审批状态（validate_approval_status） */
export type ModelApprovalStatus = 'pending' | 'approved' | 'rejected';

/** AI 决策类型（validate_decision_type） */
export type AiDecisionType =
  | 'process_optimization'
  | 'quality_prediction'
  | 'sales_forecast'
  | 'inventory_optimization'
  | 'anomaly_detection'
  | 'recommendation';

export interface AiModelVersion {
  id: number;
  model_name: string;
  version: string;
  algorithm: string;
  parameters_json: Record<string, unknown> | null;
  training_date: string | null;
  training_dataset_size: number | null;
  accuracy_metrics_json: Record<string, unknown> | null;
  status: ModelVersionStatus | string;
  changed_by: number | null;
  change_reason: string | null;
  approval_status: ModelApprovalStatus | string;
  approved_by: number | null;
  approved_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateAiModelVersionPayload {
  model_name: string;
  version: string;
  algorithm: string;
  parameters_json?: Record<string, unknown>;
  training_date?: string;
  training_dataset_size?: number;
  accuracy_metrics_json?: Record<string, unknown>;
  change_reason?: string;
  changed_by?: number;
}

export interface ApproveAiModelVersionPayload {
  approved_by: number;
  /** pending / approved / rejected */
  approval_status: ModelApprovalStatus | string;
  comments?: string;
}

export interface ChangeAiModelStatusPayload {
  /** draft / active / retired / archived */
  new_status: ModelVersionStatus | string;
  changed_by?: number;
  change_reason?: string;
}

export interface AiModelEvaluation {
  id: number;
  model_version_id: number;
  evaluation_date: string;
  accuracy: string | number | null;
  precision: string | number | null;
  recall: string | number | null;
  f1_score: string | number | null;
  sample_count: number;
  evaluation_report: string | null;
  created_at: string;
}

export interface CreateAiModelEvaluationPayload {
  model_version_id: number;
  accuracy?: number;
  precision?: number;
  recall?: number;
  f1_score?: number;
  sample_count: number;
  evaluation_report?: string;
}

export interface AiDecisionLog {
  id: number;
  decision_type: AiDecisionType | string;
  model_version_id: number | null;
  input_json: Record<string, unknown> | null;
  output_json: Record<string, unknown> | null;
  user_id: number | null;
  ip_address: string | null;
  latency_ms: number | null;
  confidence: string | number | null;
  source: string | null;
  degraded: boolean;
  sensitivity_level: string;
  operation_category: string;
  created_at: string;
}

export interface CreateAiDecisionLogPayload {
  decision_type: AiDecisionType | string;
  model_version_id?: number;
  input_json?: Record<string, unknown>;
  output_json?: Record<string, unknown>;
  user_id?: number;
  ip_address?: string;
  latency_ms?: number;
  confidence?: number;
  source?: string;
  degraded?: boolean;
}

export interface AiDecisionLogQuery {
  page?: number;
  page_size?: number;
  decision_type?: AiDecisionType | string;
  user_id?: number;
}

// ================ 模型版本 ================

/** 模型版本列表（GET /ai-models/versions，可选按 model_name 过滤） */
export function getAiModelVersions(
  params?: { model_name?: string }
): Promise<ApiResponse<AiModelVersion[]>> {
  return request.get('/ai-models/versions', { params });
}

/** 注册模型版本（POST /ai-models/versions） */
export function createAiModelVersion(
  data: CreateAiModelVersionPayload
): Promise<ApiResponse<AiModelVersion>> {
  return request.post('/ai-models/versions', data);
}

/** 获取当前生效版本（status=active，GET /ai-models/versions/active/{model_name}） */
export function getActiveAiModelVersion(
  modelName: string
): Promise<ApiResponse<AiModelVersion | null>> {
  return request.get(`/ai-models/versions/active/${modelName}`);
}

/** 审批模型版本（POST /ai-models/versions/{id}/approve） */
export function approveAiModelVersion(
  versionId: number,
  data: ApproveAiModelVersionPayload
): Promise<ApiResponse<AiModelVersion>> {
  return request.post(`/ai-models/versions/${versionId}/approve`, data);
}

/** 变更模型状态（启用/停用，POST /ai-models/versions/{id}/status） */
export function changeAiModelStatus(
  versionId: number,
  data: ChangeAiModelStatusPayload
): Promise<ApiResponse<AiModelVersion>> {
  return request.post(`/ai-models/versions/${versionId}/status`, data);
}

// ================ 模型评估 ================

/** 创建模型评估（POST /ai-models/evaluations） */
export function createAiModelEvaluation(
  data: CreateAiModelEvaluationPayload
): Promise<ApiResponse<AiModelEvaluation>> {
  return request.post('/ai-models/evaluations', data);
}

/** 模型评估列表（GET /ai-models/evaluations/{model_version_id}） */
export function getAiModelEvaluations(
  modelVersionId: number
): Promise<ApiResponse<AiModelEvaluation[]>> {
  return request.get(`/ai-models/evaluations/${modelVersionId}`);
}

/** 检测模型漂移（GET /ai-models/evaluations/{model_version_id}/drift） */
export function detectAiModelDrift(
  modelVersionId: number
): Promise<ApiResponse<{
  has_drift: boolean;
  expected_accuracy: number | null;
  actual_accuracy: number | null;
  score: number | null;
}>> {
  return request.get(`/ai-models/evaluations/${modelVersionId}/drift`);
}

// ================ 调用统计（决策日志 / 准确率报告） ================

/** AI 决策日志列表（分页，GET /ai-models/decisions） */
export function getAiDecisionLogs(
  params?: AiDecisionLogQuery
): Promise<ApiResponse<PaginatedResponse<AiDecisionLog>>> {
  return request.get('/ai-models/decisions', { params });
}

/** 记录 AI 决策日志（POST /ai-models/decisions） */
export function logAiDecision(
  data: CreateAiDecisionLogPayload
): Promise<ApiResponse<AiDecisionLog>> {
  return request.post('/ai-models/decisions', data);
}

/** AI 质量月度对账（GET /ai-models/reconcile，report_period 如 2026-09） */
export function reconcileAiQuality(
  params?: { report_period?: string }
): Promise<ApiResponse<unknown>> {
  return request.get('/ai-models/reconcile', { params });
}

/** 准确率报告列表（GET /ai-models/accuracy-reports） */
export function getAiAccuracyReports(params?: { limit?: number }): Promise<ApiResponse<unknown[]>> {
  return request.get('/ai-models/accuracy-reports', { params });
}
