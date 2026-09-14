import { request } from './request';
import type { ApiResponse, QueryParams } from '@/types/api';

/**
 * OA 公告（oa_announcement）状态
 * 后端状态机：DRAFT → PUBLISHED → ARCHIVED
 */
export type AnnouncementStatus = 'DRAFT' | 'PUBLISHED' | 'ARCHIVED';

/** 公告类型：NOTICE=通知，ANNOUNCEMENT=公告，NEWS=新闻 */
export type AnnouncementType = 'NOTICE' | 'ANNOUNCEMENT' | 'NEWS';

/** 可见性范围：ALL/DEPT/ROLE/CUSTOM */
export type VisibilityScope = 'ALL' | 'DEPT' | 'ROLE' | 'CUSTOM';

export interface Announcement {
  id: number;
  title: string;
  content: string;
  announcement_type: AnnouncementType | string;
  publish_date: string;
  effective_date: string;
  expiry_date: string | null;
  publisher_id: number;
  status: AnnouncementStatus;
  is_top: boolean;
  attachments: unknown;
  remarks: string | null;
  visibility_scope: VisibilityScope | string;
  /** DEPT: {department_ids}；ROLE: {role_ids}；CUSTOM: {user_ids} */
  visible_scope_config: {
    user_ids?: number[];
    department_ids?: number[];
    role_ids?: number[];
  } | null;
  created_at: string;
  updated_at: string;
}

export interface CreateAnnouncementPayload {
  title: string;
  content: string;
  announcement_type: AnnouncementType;
  /** 发布日期（YYYY-MM-DD） */
  publish_date: string;
  /** 生效日期（YYYY-MM-DD） */
  effective_date: string;
  expiry_date?: string;
  is_top?: boolean;
  attachments?: Record<string, unknown>[];
  remarks?: string;
  visibility_scope?: VisibilityScope;
  visible_scope_config?: Record<string, unknown>;
}

/** 草稿可全字段更新；PUBLISHED/ARCHIVED 仅允许更新 expiry_date/remarks/is_top */
export interface UpdateAnnouncementPayload {
  title?: string;
  content?: string;
  announcement_type?: AnnouncementType;
  publish_date?: string;
  effective_date?: string;
  expiry_date?: string;
  is_top?: boolean;
  attachments?: Record<string, unknown>[];
  remarks?: string;
  visibility_scope?: VisibilityScope;
  visible_scope_config?: Record<string, unknown>;
}

export interface AnnouncementListQuery extends QueryParams {
  status?: string;
  announcement_type?: string;
  is_top?: boolean;
}

export interface AnnouncementListResult {
  items: Announcement[];
  total: number;
}

export function getAnnouncementList(
  params?: AnnouncementListQuery
): Promise<ApiResponse<AnnouncementListResult>> {
  return request.get('/oa-announcements', { params });
}

export function getAnnouncement(id: number): Promise<ApiResponse<Announcement>> {
  return request.get(`/oa-announcements/${id}`);
}

export function createAnnouncement(
  data: CreateAnnouncementPayload
): Promise<ApiResponse<Announcement>> {
  return request.post('/oa-announcements', data);
}

export function updateAnnouncement(
  id: number,
  data: UpdateAnnouncementPayload
): Promise<ApiResponse<Announcement>> {
  return request.put(`/oa-announcements/${id}`, data);
}

/** 仅 DRAFT 状态可硬删除 */
export function deleteAnnouncement(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/oa-announcements/${id}`);
}

/** 发布（DRAFT → PUBLISHED） */
export function publishAnnouncement(id: number): Promise<ApiResponse<Announcement>> {
  return request.post(`/oa-announcements/${id}/publish`);
}

/** 撤回/归档（PUBLISHED → ARCHIVED） */
export function archiveAnnouncement(id: number): Promise<ApiResponse<Announcement>> {
  return request.post(`/oa-announcements/${id}/archive`);
}
