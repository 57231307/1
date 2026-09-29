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
  /**
   * 表单/筛选专用字段，非 warehouses 列、不出现在列表/详情响应中：
   * 建单时 description 写入 notes 列（见 CreateWarehouseRequest / warehouse_service.create）。
   */
  description?: string;
  /**
   * 表单/筛选专用字段，非 warehouses 列：更新时 status（active/inactive）映射到 is_active
   * （见 UpdateWarehouseRequest / warehouse_service.update），响应不含该键，读侧用 is_active。
   */
  status?: string;
}

export interface WarehouseLocation {
  id: number;
  warehouse_id: number;
  location_code: string;
  location_type?: string;
  max_weight?: number;
  max_height?: number;
  is_batch_managed?: boolean;
  is_color_managed?: boolean;
  created_at?: string;
  updated_at?: string;
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
export const createWarehouse = (data: Partial<Warehouse>) =>
  request.post<ApiResponse<Warehouse>>('/warehouses', data);

// D14 Batch 5b：原 warehouseApi.update 转为风格 B 函数
export const updateWarehouse = (id: number, data: Partial<Warehouse>) =>
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
export const createWarehouseLocation = (data: Partial<WarehouseLocation>) =>
  request.post<ApiResponse<WarehouseLocation>>('/warehouses/locations', data);

// D14 Batch 5b：原 warehouseApi.updateLocation 转为风格 B 函数
export const updateWarehouseLocation = (id: number, data: Partial<WarehouseLocation>) =>
  request.put<ApiResponse<WarehouseLocation>>(`/warehouses/locations/${id}`, data);

// D14 Batch 5b：原 warehouseApi.deleteLocation 转为风格 B 函数
export const deleteWarehouseLocation = (id: number) =>
  request.delete<ApiResponse<null>>(`/warehouses/locations/${id}`);

// D14 Batch 5b：原 warehouseApi.getLocation 转为风格 B 函数
export const getWarehouseLocation = (id: number) =>
  request.get<ApiResponse<WarehouseLocation>>(`/warehouses/locations/${id}`);
