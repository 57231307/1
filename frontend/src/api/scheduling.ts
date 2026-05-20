import { request } from './request'
import type { ApiResponse } from './request'

export interface ScheduleTask {
  id: number
  order_no: string
  product_name: string
  work_center_id: number
  work_center_name: string
  quantity: number
  start_time: string
  end_time: string
  duration_hours: number
  status: 'pending' | 'scheduled' | 'running' | 'completed' | 'conflict'
  priority: number
  has_conflict: boolean
  conflict_details?: string
}

export interface WorkCenterGantt {
  id: number
  name: string
  code: string
  tasks: ScheduleTask[]
}

export interface GanttData {
  work_centers: WorkCenterGantt[]
  date_range: {
    start: string
    end: string
  }
  total_tasks: number
  conflict_count: number
}

export interface SchedulingParams {
  start_date: string
  end_date: string
  work_center_ids?: number[]
  priority_mode: 'fifo' | 'priority' | 'due_date'
  optimization_target: 'min_idle' | 'min_delay' | 'balance_load'
}

export interface ConflictItem {
  id: number
  task_id_1: number
  task_id_2: number
  work_center_id: number
  work_center_name: string
  overlap_start: string
  overlap_end: string
  order_no_1: string
  order_no_2: string
  severity: 'warning' | 'error'
  suggestion: string
}

export interface ScheduleResult {
  success: boolean
  scheduled_count: number
  conflict_count: number
  message: string
  tasks: ScheduleTask[]
  conflicts: ConflictItem[]
}

export const schedulingApi = {
  autoSchedule: (params: SchedulingParams) =>
    request.post<ApiResponse<ScheduleResult>>('/scheduling/auto-schedule', params),

  getGanttData: (params?: { start_date?: string; end_date?: string; work_center_ids?: number[] }) =>
    request.get<ApiResponse<GanttData>>('/scheduling/gantt', { params }),

  detectConflicts: (params?: { start_date?: string; end_date?: string }) =>
    request.get<ApiResponse<ConflictItem[]>>('/scheduling/conflicts', { params }),

  adjustTask: (taskId: number, data: { start_time: string; end_time: string; work_center_id?: number }) =>
    request.put<ApiResponse<ScheduleTask>>(`/scheduling/tasks/${taskId}/adjust`, data),

  getScheduleTasks: (params?: { page?: number; page_size?: number; status?: string; work_center_id?: number }) =>
    request.get<ApiResponse<{ list: ScheduleTask[]; total: number }>>('/scheduling/tasks', { params }),
}
