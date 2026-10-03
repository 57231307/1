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

/**
 * API 密钥出参（唯一真相：api_gateway_handler::key_to_json，
 * 数据源为 services/api_key_service::ApiKeyWithCreator —— api_keys 全列 +
 * LEFT JOIN users 派生列）。
 *
 * 可空列一律 `string | null`，null 是后端列真值，不是"取不到值的空白"：
 * - `description`：api_keys.description 可空列，null = 未填/已清空；
 * - `expires_at`：null = 永不过期（列 NULL），与脏值可区分；
 * - `created_by_name`：读侧 `column_as(users.username) + LeftJoin` 富化的真实用户名，
 *   创建者用户行缺失（悬挂 created_by）时为 null；
 * - `last_used_at`：null = 从未使用。
 */
export interface ApiKey {
  id: number;
  key_name: string;
  api_key: string;
  description: string | null;
  permissions: string[];
  rate_limit: number;
  expires_at: string | null;
  status: 'active' | 'inactive' | 'expired';
  created_by: number;
  created_by_name: string | null;
  created_at: string;
  last_used_at: string | null;
}

/**
 * API 网关三个列表端点（/api-gateway/endpoints|logs|keys GET）共用的查询参数。
 * 对应后端 api_gateway_handler::ApiGwQuery（backend/src/handlers/api_gateway_handler.rs:62），
 * 无 rename_all → snake_case，全 Option → 可选。
 * 后端不读 order_by/order_dir/supplier_name/... （ApiGwQuery 之外的未知键不参与反序列化，被忽略）。
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
 * key_name 必填（缺失 serde 422）；`description` 与 `expires_at` 真实落库
 * （description → api_keys.description 可空列；expires_at → 绝对到期时刻，不再反算天数）。
 * 未采集时可省略该键或显式送 null（两者都落 NULL）；`expires_at` 有值但格式非法 →
 * 400 VALIDATION_ERROR 并外显原因，不会被当成"永不过期"。
 */
export interface CreateApiKeyRequest {
  key_name: string;
  description?: string | null;
  permissions?: string[];
  rate_limit?: number;
  expires_at?: string | null;
}

/**
 * PUT /api-gateway/keys/{id} 载荷（唯一真相：UpdateApiKeyGwRequest）。
 * `description` / `expires_at` 为显式三态（后端 double_option 适配器 + RFC 7386 口径）：
 * - 省略该键 = 保持原值（如「启用/停用」只送 status）；
 * - 显式 `null` = 清空：description → NULL，expires_at → 永不过期；
 * - 有值 = 覆盖；`expires_at` 须为 ISO 8601，格式非法 → 400 VALIDATION_ERROR（外显原因）。
 * status 后端仅识别 'active'（其余值 → inactive）。
 */
export interface UpdateApiKeyRequest {
  key_name?: string;
  description?: string | null;
  permissions?: string[];
  rate_limit?: number;
  expires_at?: string | null;
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
