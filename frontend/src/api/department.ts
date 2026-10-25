import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Department {
  id: number;
  name: string;
  code: string;
  /** DB 可空列 parent_id：后端出参键恒存在，真实空值为 null */
  parent_id?: number | null;
  /** DB 可空列 manager_id：真实空值为 null */
  manager_id?: number | null;
  manager_name?: string | null;
  /** DB 可空列 description（TEXT）：真实空值为 null */
  description?: string | null;
  sort_order: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

/**
 * 部门树节点：出参载体 = 后端 services/department_service.rs 的 DepartmentTreeNode
 * （Serialize 结构体，无 rename，九键 id/name/code/description/parent_id/manager_name/
 * sort_order/is_active/children）。
 * 与 department::Model（列表/详情载体）是两个不同类型：树节点不携带
 * manager_id/created_at/updated_at，也不携带 Model 的 is_deleted，按 Model 形状消费树出参会在假字段上取值。
 * 同级顺序由后端显式定序（sort_order 升序、id 升序，根级与各层一致），前端不再自行排序。
 */
export interface DepartmentTreeNode {
  id: number;
  name: string;
  /** DB NOT NULL 列 code（部门编码，查重由后端 service 负责） */
  code: string;
  /** DB 可空列 description：后端出参键恒存在，真实空值为 null */
  description: string | null;
  /** DB 可空列 parent_id（自引用外键）：键恒存在，顶级部门为 null；父级行缺席时保持原值不改写 */
  parent_id: number | null;
  /**
   * 负责人姓名：非数据库列（Model 上 #[sea_orm(ignore)] 瞬态字段），
   * 由 service 按 manager_id 批量查 users.username 回填；未指派或回填失败为 null，禁止提交
   */
  manager_name: string | null;
  /** 排序号：后端出参按它定同级先后 */
  sort_order: number;
  /** 启用状态：树端点不过滤该列，停用部门同样出现在出参里 */
  is_active: boolean;
  /** 后端序列化为 Vec 且无 skip_serializing_if：键恒存在，叶子节点为空数组 */
  children: DepartmentTreeNode[];
}

export interface DepartmentCreateRequest {
  name: string;
  code: string;
  parent_id?: number;
  manager_id?: number;
  sort_order?: number;
}

// 字段集严格对齐后端 UpdateDepartmentRequest（handlers/department_handler.rs）。
// 三态语义（RFC 7386 JSON Merge Patch）：键缺席=保持原值、显式 null=清空为 NULL、有值=覆盖。
// - name/code/sort_order/is_active 为 NOT NULL 列：不开 null 清空，空则省略键（保持原值），
//   发送显式 null 会被后端判业务错误（400 + 外显文案）。
// - description/parent_id/manager_id 为 DB 可空列（m0001 DDL 核实）：清空须送 null，
//   禁止 `|| undefined` 省略键（省略=保持原值，正是本轮消灭的静默丢弃形态）。
//   parent_id 送 null = 脱离父级成为顶级部门；manager_id 送 null = 清除负责人。
// code 后端本批已支持（NOT NULL UNIQUE，查重由 service 负责），编辑对话框编码可改 ⇒
// 必须随 payload 提交，否则改动静默丢失。
export interface DepartmentUpdateRequest {
  name?: string;
  code?: string;
  description?: string | null;
  parent_id?: number | null;
  manager_id?: number | null;
  sort_order?: number;
  is_active?: boolean;
}

// 部门列表查询参数：字段集严格对齐后端 DepartmentListQuery（handlers/department_handler.rs）。
// 搜索键是 search（不是 keyword）；后端不读 order_by/order_dir/status 等通用键。
export interface DepartmentListQuery {
  page?: number;
  page_size?: number;
  parent_id?: number;
  search?: string;
}

// 后端 department_handler::list（define_crud_handlers 宏）返回 PaginatedResponse { items, total, page, page_size }
export function getDepartmentList(
  params?: DepartmentListQuery
): Promise<ApiResponse<{ items: Department[]; total: number; page: number; page_size: number }>> {
  return request.get('/departments', { params });
}

export function getDepartment(id: number): Promise<ApiResponse<Department>> {
  return request.get(`/departments/${id}`);
}

export function createDepartment(data: DepartmentCreateRequest): Promise<ApiResponse<Department>> {
  return request.post('/departments', data);
}

export function updateDepartment(
  id: number,
  data: DepartmentUpdateRequest
): Promise<ApiResponse<Department>> {
  return request.put(`/departments/${id}`, data);
}

export function deleteDepartment(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/departments/${id}`);
}

// 后端 department_handler::get_department_tree 出参 = ApiResponse<Vec<DepartmentTreeNode>>，
// data 为裸数组；节点键集合见 DepartmentTreeNode 声明处注释（与列表载体 department::Model 不同型）。
export function getDepartmentTree(): Promise<ApiResponse<DepartmentTreeNode[]>> {
  return request.get('/departments/tree');
}
