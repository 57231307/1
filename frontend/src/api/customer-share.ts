import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Signature {
  id: number;
  status: string;
  [key: string]: unknown;
}

export function signContract(data: Record<string, unknown>) {
  return request.post('/contract-signatures/sign', data);
}

export function verifySignature(id: number) {
  return request.get(`/contract-signatures/${id}/verify`);
}

export function revokeSignature(id: number, data?: Record<string, unknown>) {
  return request.post(`/contract-signatures/${id}/revoke`, data ?? {});
}

export function listSignedContracts(params?: Record<string, unknown>) {
  return request.get('/contract-signatures', { params });
}

export function createCustomerShare(data: {
  customer_id: number;
  target_user_id: number;
  permission: string;
}) {
  return request.post('/customer-shares', data);
}

export function listCustomerShares(params?: Record<string, unknown>) {
  return request.get('/customer-shares', { params });
}

export function getSharesByCustomer(customerId: number) {
  return request.get('/customer-shares/by-customer', { params: { customer_id: customerId } });
}

export function getSharesByUser(userId: number) {
  return request.get('/customer-shares/by-user', { params: { user_id: userId } });
}

export function checkSharePermission(params: Record<string, unknown>) {
  return request.get<ApiResponse<Record<string, unknown>>>('/customer-shares/check', { params });
}

export function revokeShare(shareId: number, data?: Record<string, unknown>) {
  return request.post(`/customer-shares/revoke`, { share_id: shareId, ...(data ?? {}) });
}

export function expireOverdueShares() {
  return request.post('/customer-shares/expire-overdue');
}

export function addTeamMember(data: Record<string, unknown>) {
  return request.post('/customer-team-members', data);
}

export function removeTeamMember(memberId: number) {
  return request.delete(`/customer-team-members/${memberId}`);
}

export function listTeamMembers(customerId: number) {
  return request.get(`/customer-team-members/by-customer/${customerId}`);
}

export function listUserTeams(userId: number) {
  return request.get<ApiResponse<Record<string, unknown>[]>>('/customer-team-members/by-user', {
    params: { user_id: userId },
  });
}

export function isTeamMember(params: Record<string, unknown>) {
  return request.get<ApiResponse<{ is_member?: boolean }>>('/customer-team-members/check', {
    params,
  });
}
