/**
 * API 类型定义
 */

/** 业务响应统一格式 */
export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
}

/** 通知数据载荷（与 WebSocket 一致） */
export interface NotificationPayload {
  id: number;
  title: string;
  content: string;
  category: string;
  priority: number;
  created_at: string;
}

/** 分页响应 */
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  size: number;
}

/** 用户信息 */
export interface User {
  id: number;
  username: string;
  tenant_id: number;
  email?: string;
  roles?: string[];
}
