import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 工作中心产能（后端 GET /capacity/work-centers → Vec<WorkCenterCapacity>，
// models/dto/capacity_dto.rs::WorkCenterCapacity；handler capacity_handler.rs::list_work_centers
// 用 to_value 直接序列化，故键 = DTO 字段名，可空列标 | null）
export interface WorkCenter {
  id: number;
  code: string;
  name: string;
  work_center_type: string | null;
  daily_capacity: number;
  capacity_unit: string | null;
  // 工作中心运营态（active_status：ACTIVE/INACTIVE/…，见 models/status/general.rs），非负荷态
  status: string;
  shifts: ShiftInfo[];
}

export interface ShiftInfo {
  shift_name: string;
  start_time: string;
  end_time: string;
  capacity_ratio: number;
}

// 产能负荷项（后端 GET /capacity/load-analysis → Vec<CapacityLoadItem>，
// models/dto/capacity_dto.rs::CapacityLoadItem）。load_rate 已是百分比（服务侧 ×100，
// capacity_service.rs::build_capacity_load_item），status 取 IDLE/NORMAL/HIGH/OVERLOADED。
export interface CapacityLoadItem {
  work_center_id: number;
  work_center_code: string;
  work_center_name: string;
  daily_capacity: number;
  capacity_unit: string | null;
  planned_quantity: number;
  in_progress_quantity: number;
  total_demand: number;
  load_rate: number;
  status: string;
  gap_quantity: number;
  suggestions: BottleneckSuggestion[];
}

export interface BottleneckSuggestion {
  suggestion_type: string;
  description: string;
  suggested_quantity: number;
  priority: string;
}

// 产能表合并行：/capacity/work-centers（产能定义）+ /capacity/load-analysis（负荷度量）按
// id 关联。负荷列在无对应 load 记录（如工作中心非启用态、不在 load_analysis 结果内）时为 null，
// 属真实缺省而非缺键，由展示层按空处理。
export interface CapacityRow extends WorkCenter {
  total_demand: number | null;
  load_rate: number | null;
  load_status: string | null;
  bottleneck: boolean;
}

export interface CapacityTrend {
  date: string;
  planned_hours: number;
  actual_hours: number;
  capacity_hours: number;
}

export interface CapacitySummary {
  total_work_centers: number;
  normal_count: number;
  busy_count: number;
  overload_count: number;
  bottleneck_count: number;
  avg_load_rate: number;
}

// D14 Batch 5b：原 capacityApi.getSummary 转为风格 B 函数
export const getCapacitySummary = () =>
  request.get<ApiResponse<CapacitySummary>>('/production/capacity/summary');

// D14 Batch 5b：原 capacityApi.getTrend 转为风格 B 函数
export const getCapacityTrend = (params?: { days?: number; work_center_id?: number }) =>
  request.get<ApiResponse<CapacityTrend[]>>('/production/capacity/trend', { params });

// 后端 capacity_handler::list_work_centers 无 Query<T> 提取器，任何 page/page_size/status 都会被
// Axum 整体丢弃，故前端不再发送 params。其返回为 ApiResponse::success(to_value(Vec<WorkCenterCapacity>))，
// data 是**裸数组**（非 {list/items/data} 信封），故类型钉为 WorkCenter[]。
export const getWorkCenterList = () =>
  request.get<ApiResponse<WorkCenter[]>>('/production/capacity/work-centers');

// GET /capacity/load-analysis → Vec<CapacityLoadItem>（Query：date_from/date_to/work_center_id，
// 见 capacity_handler.rs::LoadAnalysisParams）。用于给产能表补负荷率/已用量/负荷态/瓶颈列。
export const getLoadAnalysis = (params?: {
  date_from?: string;
  date_to?: string;
  work_center_id?: number;
}) =>
  request.get<ApiResponse<CapacityLoadItem[]>>('/production/capacity/load-analysis', { params });

// GET /capacity/bottlenecks 实为 get_load_analysis（routes/production.rs:686），
// 返回 Vec<CapacityLoadItem>（非 WorkCenter[]）。瓶颈侧栏按负荷项渲染。
export const getCapacityBottlenecks = () =>
  request.get<ApiResponse<CapacityLoadItem[]>>('/production/capacity/bottlenecks');
