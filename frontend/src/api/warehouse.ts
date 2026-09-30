import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

export interface Warehouse {
  id: number;
  /** warehouses.warehouse_code（NOT NULL） */
  warehouse_code: string;
  /** warehouses.name 列，出参经后端 #[serde(rename = "warehouse_name")] 键为 warehouse_name（NOT NULL） */
  warehouse_name: string;
  /** warehouses.warehouse_type（Option，NULL=不校验仓型） */
  warehouse_type?: string;
  address?: string;
  city?: string;
  province?: string;
  country?: string;
  postal_code?: string;
  phone?: string;
  email?: string;
  /** warehouses.contact_person（Option，联系人） */
  contact_person?: string;
  /** warehouses.manager_id（Option<i32>） */
  manager_id?: number;
  /** warehouses.capacity（Option<i32>） */
  capacity?: number;
  /** warehouses.is_default（NOT NULL，全局唯一默认仓由后端保证） */
  is_default: boolean;
  /** warehouses.is_active（NOT NULL bool）：启用状态读侧以此为准 */
  is_active: boolean;
  /** warehouses.notes（Option，备注） */
  notes?: string;
  created_at: string;
  updated_at: string;
}

export interface WarehouseLocation {
  id: number;
  warehouse_id: number;
  location_code: string;
  /** warehouse_locations.location_type（Option<String>，NULL→null） */
  location_type: string | null;
  /**
   * max_weight/max_height 为 Option<rust_decimal>，出参序列化为字符串（如 "1250.00"）、NULL→null；
   * 绑定 el-input-number / 运算前必须 Number() 归一，禁止当 number 直接算。
   * 入参方向（Create/UpdateLocationRequest）为 f64，以 JSON number 提交。
   */
  max_weight: string | null;
  max_height: string | null;
  /** Option<bool>，NULL→null */
  is_batch_managed: boolean | null;
  is_color_managed: boolean | null;
  created_at: string;
  updated_at: string;
}

/**
 * 创建仓库载荷 —— 逐字段对齐 handlers/warehouse_handler.rs::CreateWarehouseRequest。
 * 后端字段名为 name/code（create 侧虽有 #[serde(alias=warehouse_name/warehouse_code)]，
 * 别名不算契约字段名，前端统一按后端字段名提交）；全部 Option，空串一律省略键
 * （name/code 带 length(min=1) 校验，Some("") 会被 validator 判 422）。
 * status/is_active 不在创建契约：新建恒启用（service 固定 is_active=true），启用状态仅在更新时改。
 */
export interface CreateWarehousePayload {
  name?: string;
  code?: string;
  address?: string;
  /** manager 为字符串入参（后端 parse::<i32> 为 manager_id，解析失败 400） */
  manager?: string;
  phone?: string;
  contact_person?: string;
  is_default?: boolean;
  capacity?: number;
  /** description 建单时写入 notes 列 */
  description?: string;
  warehouse_type?: string;
}

/**
 * 更新仓库载荷 —— 逐字段对齐 handlers/warehouse_handler.rs::UpdateWarehouseRequest。
 * 更新侧 name 无 alias（此前视图整表回传 warehouse_name ⇒ serde 不识别 ⇒ 仓库改名永远不生效）；
 * code/notes 不在更新契约（编码不可改、描述仅建单落 notes）；status(active/inactive) 映射 is_active。
 */
export interface UpdateWarehousePayload {
  name?: string;
  address?: string;
  manager?: string;
  phone?: string;
  contact_person?: string;
  is_default?: boolean;
  capacity?: number;
  status?: string;
  warehouse_type?: string;
}

/**
 * 创建库位载荷 —— 对齐 CreateLocationRequest。
 * warehouse_id/location_code 为非 Option 必填（不得标 ?）；
 * is_batch_managed/is_color_managed 不在创建契约（handler 固定 Set(Some(true))，表单传了也会被丢弃）。
 */
export interface CreateWarehouseLocationPayload {
  warehouse_id: number;
  location_code: string;
  location_type?: string;
  max_weight?: number;
  max_height?: number;
}

/**
 * 更新库位载荷 —— 对齐 UpdateLocationRequest（全部 Option；warehouse_id 不可改，不在契约）。
 */
export interface UpdateWarehouseLocationPayload {
  location_code?: string;
  location_type?: string;
  max_weight?: number;
  max_height?: number;
  is_batch_managed?: boolean;
  is_color_managed?: boolean;
}

export interface WarehouseQueryParams {
  page?: number;
  page_size?: number;
  /** 后端按 name / warehouse_code 模糊匹配，参数名为 search */
  search?: string;
  warehouse_type?: string;
  status?: string;
}

// D14 Batch 5b：原 warehouseApi.list 转为风格 B 函数
export const getWarehouseList = (params?: WarehouseQueryParams) =>
  request.get<ApiResponse<{ items: Warehouse[]; total: number }>>('/warehouses', { params });

// D14 Batch 5b：原 warehouseApi.getById 转为风格 B 函数
export const getWarehouseById = (id: number) =>
  request.get<ApiResponse<Warehouse>>(`/warehouses/${id}`);

// D14 Batch 5b：原 warehouseApi.create 转为风格 B 函数
export const createWarehouse = (data: CreateWarehousePayload) =>
  request.post<ApiResponse<Warehouse>>('/warehouses', data);

// D14 Batch 5b：原 warehouseApi.update 转为风格 B 函数
export const updateWarehouse = (id: number, data: UpdateWarehousePayload) =>
  request.put<ApiResponse<Warehouse>>(`/warehouses/${id}`, data);

// D14 Batch 5b：原 warehouseApi.delete 转为风格 B 函数
export const deleteWarehouse = (id: number) =>
  request.delete<ApiResponse<null>>(`/warehouses/${id}`);

// D14 Batch 5b：原 warehouseApi.getLocations 转为风格 B 函数
// 后端 /warehouses/locations 出参是分页对象（{ items, total, page, page_size }），
// page_size 上限 100，故按上限取首页
export const getWarehouseLocationList = (warehouseId: number) =>
  request.get<ApiResponse<PaginatedResponse<WarehouseLocation>>>('/warehouses/locations', {
    params: { warehouse_id: warehouseId, page_size: 100 },
  });

// D14 Batch 5b：原 warehouseApi.createLocation 转为风格 B 函数
export const createWarehouseLocation = (data: CreateWarehouseLocationPayload) =>
  request.post<ApiResponse<WarehouseLocation>>('/warehouses/locations', data);

// D14 Batch 5b：原 warehouseApi.updateLocation 转为风格 B 函数
export const updateWarehouseLocation = (id: number, data: UpdateWarehouseLocationPayload) =>
  request.put<ApiResponse<WarehouseLocation>>(`/warehouses/locations/${id}`, data);

// D14 Batch 5b：原 warehouseApi.deleteLocation 转为风格 B 函数
export const deleteWarehouseLocation = (id: number) =>
  request.delete<ApiResponse<null>>(`/warehouses/locations/${id}`);

// D14 Batch 5b：原 warehouseApi.getLocation 转为风格 B 函数
export const getWarehouseLocation = (id: number) =>
  request.get<ApiResponse<WarehouseLocation>>(`/warehouses/locations/${id}`);
