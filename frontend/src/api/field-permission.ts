import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 字段权限——列表/详情出参。
 * 唯一真相：backend/src/handlers/field_permission_handler.rs::FieldPermissionResponse。
 * id/role_id/resource_type/field_name/can_read/can_write/mask_strategy/is_enabled/
 * created_at/updated_at 后端 NOT NULL，不标可空；role_name 为 LEFT JOIN roles.name 富化，
 * 无匹配角色（或详情端点）时恒 null。
 */
export interface FieldPermissionRow {
  id: number;
  role_id: number;
  resource_type: string;
  field_name: string;
  can_read: boolean;
  can_write: boolean;
  mask_strategy: string;
  is_enabled: boolean;
  created_at: string;
  updated_at: string;
  role_name: string | null;
}

/**
 * POST 载荷（唯一真相：CreateFieldPermissionPayload，六键 snake_case）。
 * role_id/resource_type/field_name/can_read/can_write 后端必填；
 * mask_strategy 为 Option，缺省后端落 "NONE"。取值域见后端消费侧：
 * field_permission_service 判 `== "MASK"` 置 "***"，其余值按 "NONE" 语义使用。
 */
export interface FieldPermissionCreateRequest {
  role_id: number;
  resource_type: string;
  field_name: string;
  can_read: boolean;
  can_write: boolean;
  mask_strategy?: string;
}

/** PUT 载荷（唯一真相：UpdateFieldPermissionPayload，全键 Option，只提交要改的键） */
export interface FieldPermissionUpdateRequest {
  can_read?: boolean;
  can_write?: boolean;
  mask_strategy?: string;
  is_enabled?: boolean;
}

/** 列表查询参数（后端 FieldPermissionQuery，两键均过滤可选） */
export interface FieldPermissionListQuery {
  resource_type?: string;
  role_id?: number;
}

/** GET /field-permissions——后端出参 `Json<Vec<FieldPermissionResponse>>`（裸数组载荷） */
export function getFieldPermissionList(
  params?: FieldPermissionListQuery
): Promise<ApiResponse<FieldPermissionRow[]>> {
  return request.get('/field-permissions', { params });
}

/** POST /field-permissions——服务端事务内查重（role_id+resource_type+field_name 唯一），重复 400 点名 */
export function createFieldPermission(
  data: FieldPermissionCreateRequest
): Promise<ApiResponse<FieldPermissionRow>> {
  return request.post('/field-permissions', data);
}

/** PUT /field-permissions/{id}——局部更新（read/write/mask/is_enabled） */
export function updateFieldPermission(
  id: number,
  data: FieldPermissionUpdateRequest
): Promise<ApiResponse<FieldPermissionRow>> {
  return request.put(`/field-permissions/${id}`, data);
}

/** DELETE /field-permissions/{id}——软删（置 is_enabled=false），行仍可由列表过滤感知 */
export function deleteFieldPermission(id: number): Promise<ApiResponse<unknown>> {
  return request.delete(`/field-permissions/${id}`);
}
