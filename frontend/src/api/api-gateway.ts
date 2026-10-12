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

/**
 * /api-gateway/endpoints GET 的查询参数。
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

export interface ApiStats {
  total_endpoints: number;
  active_endpoints: number;
  inactive_endpoints: number;
}

export function getApiStats(): Promise<ApiResponse<ApiStats>> {
  return request.get('/api-gateway/stats');
}
