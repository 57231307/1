import { request } from './request';
import type { ApiResponse, QueryParams } from '@/types/api';

export interface OaAnnouncement {
  id?: number;
  title: string;
  content: string;
  announcement_type: string;
  publish_date: string;
  effective_date: string;
  expiry_date?: string | null;
  publisher_id?: number;
  status: string;
  is_top: boolean;
  attachments?: unknown;
  remarks?: string | null;
  visibility_scope: string;
  visible_scope_config?: {
    user_ids?: number[];
    department_ids?: number[];
    role_ids?: number[];
  } | null;
  created_at?: string;
  updated_at?: string;
  notified_count?: number;
}

export interface CreateOaAnnouncementRequest {
  title: string;
  content: string;
  announcement_type: string;
  publish_date: string;
  effective_date: string;
  expiry_date?: string;
  remarks?: string;
  is_top?: boolean;
  visibility_scope: string;
  visible_scope_config?: unknown;
}

export interface UpdateOaAnnouncementRequest {
  title?: string;
  content?: string;
  announcement_type?: string;
  expiry_date?: string;
  remarks?: string;
  is_top?: boolean;
  visibility_scope?: string;
  visible_scope_config?: unknown;
}

export interface OaAnnouncementQuery extends QueryParams {
  status?: string;
  announcement_type?: string;
  is_top?: boolean;
}

export function listOaAnnouncements(
  params?: OaAnnouncementQuery
): Promise<ApiResponse<{ items: OaAnnouncement[]; total: number }>> {
  return request.get('/oa-announcements/', { params });
}

export function getOaAnnouncement(id: number): Promise<ApiResponse<OaAnnouncement>> {
  return request.get(`/oa-announcements/${id}`);
}

export function createOaAnnouncement(
  data: CreateOaAnnouncementRequest
): Promise<ApiResponse<OaAnnouncement>> {
  return request.post('/oa-announcements/', data);
}

export function updateOaAnnouncement(
  id: number,
  data: UpdateOaAnnouncementRequest
): Promise<ApiResponse<OaAnnouncement>> {
  return request.put(`/oa-announcements/${id}`, data);
}

export function deleteOaAnnouncement(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/oa-announcements/${id}`);
}

export function publishOaAnnouncement(id: number): Promise<ApiResponse<OaAnnouncement>> {
  return request.post(`/oa-announcements/${id}/publish`);
}

export function archiveOaAnnouncement(id: number): Promise<ApiResponse<OaAnnouncement>> {
  return request.post(`/oa-announcements/${id}/archive`);
}
