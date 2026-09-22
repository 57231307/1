import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 通知列表查询参数：字段集严格对齐后端 NotificationListQuery（handlers/notification_handler.rs）。
// 类型过滤键是 snake_case 的 notification_type；status 后端仅识别 UNREAD/READ/PROCESSED。
export interface NotificationQueryParams {
  page?: number;
  page_size?: number;
  status?: string;
  notification_type?: string;
}

export interface Notification {
  id?: number;
  title?: string;
  content?: string;
  status?: string;
  notificationType?: string;
  businessType?: string;
  businessId?: number;
  createdAt?: string;
  readAt?: string;
}

export interface NotificationSetting {
  id?: number;
  user_id?: number;
  email_enabled: boolean;
  internal_enabled: boolean;
  order_notification_type: string;
  approval_notification_type: string;
  inventory_notification_type: string;
  purchase_notification_type: string;
  finance_notification_type: string;
  system_notification_type: string;
}

export interface BatchOperationRequest {
  ids: number[];
}

export interface UpdateSettingRequest {
  email_enabled?: boolean;
  internal_enabled?: boolean;
  order_notification_type?: string;
  approval_notification_type?: string;
  inventory_notification_type?: string;
  purchase_notification_type?: string;
  finance_notification_type?: string;
  system_notification_type?: string;
}

/** 系统公告发送请求（仅管理员） */
export interface CreateAnnouncementRequest {
  /** 目标用户 id 列表（非空，后端自动去重） */
  userIds: number[];
  /** 公告标题（1-100 字符） */
  title: string;
  /** 公告内容（1-2000 字符） */
  content: string;
}

export interface AnnouncementResult {
  deliveredCount: number;
}

// 后端 notification_handler::list_notifications 返回 ApiResponse<serde_json::Value>，
// 其 data 由 json!({ "list": notifications, "total": total }) 构造，真实列表键为 list（非 items）。
export function getNotificationList(
  params?: NotificationQueryParams
): Promise<ApiResponse<{ list: Notification[]; total: number }>> {
  return request.get('/notifications', { params });
}

export function getNotification(id: number): Promise<ApiResponse<Notification>> {
  return request.get(`/notifications/${id}`);
}

export function getUnreadCount(): Promise<ApiResponse<number>> {
  return request.get('/notifications/unread-count');
}

export function markAsRead(id: number): Promise<ApiResponse<void>> {
  return request.post(`/notifications/${id}/read`);
}

export function batchMarkAsRead(data: BatchOperationRequest): Promise<ApiResponse<void>> {
  return request.post('/notifications/batch-read', data);
}

export function markAllAsRead(): Promise<ApiResponse<void>> {
  return request.post('/notifications/read-all');
}

export function deleteNotification(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/notifications/${id}`);
}

export function getSettings(): Promise<ApiResponse<NotificationSetting>> {
  return request.get('/user-notification-settings');
}

export function updateSetting(
  data: UpdateSettingRequest
): Promise<ApiResponse<NotificationSetting>> {
  return request.put('/user-notification-settings', data);
}

/**
 * 发送系统公告（仅管理员）
 *
 * 向指定用户列表广播 SYSTEM 类型站内通知；后端按用户通知偏好过滤，
 * 关闭内部消息的用户不会收到。
 */
export function createAnnouncement(
  data: CreateAnnouncementRequest
): Promise<ApiResponse<AnnouncementResult>> {
  return request.post('/notifications/announcement', {
    user_ids: data.userIds,
    title: data.title,
    content: data.content,
  });
}

/** WebSocket 票据响应（v12 P1-4：一次性短时票据替代 URL query JWT） */
export interface WsTicketResponse {
  ticket: string;
  expires_in: number;
}

/**
 * 获取 WebSocket 连接票据
 *
 * v12 P1-4 修复：浏览器 WebSocket API 不支持自定义 header，
 * 原方案通过 URL query 传递 JWT 会导致 token 泄露到浏览器历史、
 * 服务器 access log、中间代理日志。
 *
 * 新方案：客户端先通过 HTTP POST（自动携带 httpOnly Cookie JWT）获取
 * 一次性短时票据（30 秒有效），再用票据建立 WebSocket 连接。
 * 票据一次性消费，即使泄露也无法复用。
 */
export function getWsTicket(): Promise<WsTicketResponse> {
  return request.post<WsTicketResponse>('/ws/ticket');
}
