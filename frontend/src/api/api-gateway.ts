import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface ApiEndpoint {
  id: number;
  path: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  description: string;
  module: string;
  status: 'active' | 'inactive' | 'deprecated';
  rate_limit: number;
  timeout: number;
  authentication: boolean;
  authorization: string[];
  request_schema: Record<string, unknown>;
  response_schema: Record<string, unknown>;
  // 后端 endpoint_to_json 恒定回传（api_gateway_handler.rs:129-151）：version 缺省 "v1"，
  // deprecated_at/sunset_at 为可空，deprecation_note 缺省空串
  version: string;
  deprecated_at: string | null;
  sunset_at: string | null;
  deprecation_note: string;
  created_at: string;
  updated_at: string;
}

/**
 * POST /api-gateway/endpoints 载荷（唯一真相：api_gateway_handler::UpsertApiEndpointRequest）。
 * path/method 后端 handler 显式必填（缺任一 → 400 validation），其余可选；
 * rate_limit 后端校验 0-10000（越界 400）。
 */
export interface CreateApiEndpointRequest {
  path: string;
  method: ApiEndpoint['method'];
  description?: string;
  module?: string;
  status?: ApiEndpoint['status'];
  rate_limit?: number;
  timeout?: number;
  authentication?: boolean;
  authorization?: string[];
  request_schema?: Record<string, unknown>;
  response_schema?: Record<string, unknown>;
  version?: string;
  deprecated_at?: string;
  sunset_at?: string;
  deprecation_note?: string;
}

/**
 * PUT /api-gateway/endpoints/{id} 载荷：与创建共用 Upsert DTO，全部可选，
 * 未提供的键后端保持原值（逐键 if let Some 更新）。
 */
export interface UpdateApiEndpointRequest {
  path?: string;
  method?: ApiEndpoint['method'];
  description?: string;
  module?: string;
  status?: ApiEndpoint['status'];
  rate_limit?: number;
  timeout?: number;
  authentication?: boolean;
  authorization?: string[];
  request_schema?: Record<string, unknown>;
  response_schema?: Record<string, unknown>;
  version?: string;
  deprecated_at?: string;
  sunset_at?: string;
  deprecation_note?: string;
}

export interface ApiLog {
  id: number;
  endpoint_id: number;
  endpoint_path: string;
  method: string;
  request_body: string;
  response_body: string;
  status_code: number;
  response_time: number;
  ip_address: string;
  user_agent: string;
  user_id: number;
  user_name: string;
  created_at: string;
}

export interface ApiKey {
  id: number;
  key_name: string;
  api_key: string;
  description: string;
  permissions: string[];
  rate_limit: number;
  expires_at: string;
  status: 'active' | 'inactive' | 'expired';
  created_by: number;
  created_by_name: string;
  created_at: string;
  last_used_at: string;
}

/**
 * API 网关三个列表端点（/api-gateway/endpoints|logs|keys GET）共用的查询参数。
 * 对应后端 api_gateway_handler::ApiGwQuery（backend/src/handlers/api_gateway_handler.rs:62），
 * 无 rename_all → snake_case，全 Option → 可选。
 * 后端不读 order_by/order_dir/supplier_name/... （原 QueryParams 键被 Axum 静默丢弃）。
 */
export interface ApiGwQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  status?: string;
  method?: string;
}

export function getApiEndpointList(params?: ApiGwQueryParams): Promise<ApiResponse<ApiEndpoint[]>> {
  return request.get('/api-gateway/endpoints', { params });
}

export function getApiEndpoint(id: number): Promise<ApiResponse<ApiEndpoint>> {
  return request.get(`/api-gateway/endpoints/${id}`);
}

export function createApiEndpoint(
  data: CreateApiEndpointRequest
): Promise<ApiResponse<ApiEndpoint>> {
  return request.post('/api-gateway/endpoints', data);
}

export function updateApiEndpoint(
  id: number,
  data: UpdateApiEndpointRequest
): Promise<ApiResponse<ApiEndpoint>> {
  return request.put(`/api-gateway/endpoints/${id}`, data);
}

export function deleteApiEndpoint(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api-gateway/endpoints/${id}`);
}

export function getApiLogList(params?: ApiGwQueryParams): Promise<ApiResponse<ApiLog[]>> {
  return request.get('/api-gateway/logs', { params });
}

export function getApiLog(id: number): Promise<ApiResponse<ApiLog>> {
  return request.get(`/api-gateway/logs/${id}`);
}

export function getApiKeyList(params?: ApiGwQueryParams): Promise<ApiResponse<ApiKey[]>> {
  return request.get('/api-gateway/keys', { params });
}

export function getApiKey(id: number): Promise<ApiResponse<ApiKey>> {
  return request.get(`/api-gateway/keys/${id}`);
}

/**
 * POST /api-gateway/keys 载荷（唯一真相：api_gateway_handler::CreateApiKeyGwRequest）。
 * key_name 为非 Option 必填（缺失 serde 422）；后端创建 DTO 无 description/status 字段
 * （description 仅更新 DTO 有，创建传了会被静默丢弃——后端缺口已登记串行清单）。
 * expires_at 为 ISO 8601 字符串；空值必须省略该键（后端按解析失败处理会清掉有效期语义）。
 */
export interface CreateApiKeyRequest {
  key_name: string;
  permissions?: string[];
  rate_limit?: number;
  expires_at?: string;
}

/**
 * PUT /api-gateway/keys/{id} 载荷（唯一真相：UpdateApiKeyGwRequest，全字段 Option）；
 * status 后端仅识别 'active'（其余值 → inactive）。
 */
export interface UpdateApiKeyRequest {
  key_name?: string;
  description?: string;
  permissions?: string[];
  rate_limit?: number;
  expires_at?: string;
  status?: ApiKey['status'];
}

export function createApiKey(data: CreateApiKeyRequest): Promise<ApiResponse<ApiKey>> {
  return request.post('/api-gateway/keys', data);
}

export function updateApiKey(id: number, data: UpdateApiKeyRequest): Promise<ApiResponse<ApiKey>> {
  return request.put(`/api-gateway/keys/${id}`, data);
}

export function deleteApiKey(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api-gateway/keys/${id}`);
}

export function regenerateApiKey(id: number): Promise<ApiResponse<ApiKey>> {
  return request.post(`/api-gateway/keys/${id}/regenerate`);
}

// P2-16 修复（批次 86 v2 复审）：getApiStats ApiResponse<any> → 显式接口

/** API 网关统计 */
export interface ApiStats {
  total_endpoints: number;
  active_endpoints: number;
  inactive_endpoints: number;
  total_keys: number;
  active_keys: number;
  total_requests: number;
  total_errors: number;
  avg_response_time_ms: number;
  [key: string]: unknown;
}

export function getApiStats(): Promise<ApiResponse<ApiStats>> {
  return request.get('/api-gateway/stats');
}
