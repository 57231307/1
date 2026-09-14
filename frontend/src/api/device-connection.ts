import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 设备连接状态
 * 后端在线判定：last_heartbeat_at 在超时窗口内为 online，超时为 timeout，其余为 offline
 */
export type DeviceConnectionStatus = 'online' | 'offline' | 'timeout';

/** 设备类型（RegisterDeviceRequest.device_type） */
export type DeviceType = 'pda' | 'industrial_terminal' | 'scanner' | 'other';

export interface DeviceConnection {
  id: number;
  device_id: string;
  device_name: string | null;
  device_type: DeviceType | string;
  user_id: number | null;
  username: string | null;
  workshop: string | null;
  ip_address: string | null;
  session_token: string | null;
  status: DeviceConnectionStatus | string;
  last_heartbeat_at: string;
  connected_at: string;
  disconnected_at: string | null;
  metadata: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
}

export interface RegisterDevicePayload {
  /** 设备唯一标识（PDA 序列号 / MAC / 自定义编号），必填 */
  device_id: string;
  device_name?: string;
  device_type?: DeviceType | string;
  user_id?: number;
  username?: string;
  workshop?: string;
  ip_address?: string;
  metadata?: Record<string, unknown>;
}

export interface DeviceHeartbeatPayload {
  user_id?: number;
  username?: string;
  workshop?: string;
  ip_address?: string;
  metadata?: Record<string, unknown>;
}

export interface DeviceConnectionListQuery {
  status?: DeviceConnectionStatus | string;
  device_type?: DeviceType | string;
  workshop?: string;
  user_id?: number;
  page?: number;
  page_size?: number;
}

export interface DeviceOnlineCount {
  online_count: number;
  workshop: string | null;
}

export interface CleanupTimeoutResult {
  timed_out_count: number;
}

/** 设备注册（首次或重新上线，POST /device-connections/register） */
export function registerDevice(
  data: RegisterDevicePayload
): Promise<ApiResponse<DeviceConnection>> {
  return request.post('/device-connections/register', data);
}

/** 设备列表（分页+过滤，GET /device-connections） */
export function getDeviceConnections(
  params?: DeviceConnectionListQuery
): Promise<ApiResponse<PaginatedResponse<DeviceConnection>>> {
  return request.get('/device-connections', { params });
}

/** 设备详情（GET /device-connections/{device_id}） */
export function getDeviceConnection(deviceId: string): Promise<ApiResponse<DeviceConnection>> {
  return request.get(`/device-connections/${deviceId}`);
}

/** 心跳上报（POST /device-connections/{device_id}/heartbeat） */
export function sendDeviceHeartbeat(
  deviceId: string,
  data?: DeviceHeartbeatPayload
): Promise<ApiResponse<DeviceConnection>> {
  return request.post(`/device-connections/${deviceId}/heartbeat`, data ?? {});
}

/** 主动下线（POST /device-connections/{device_id}/disconnect） */
export function disconnectDevice(deviceId: string): Promise<ApiResponse<DeviceConnection | null>> {
  return request.post(`/device-connections/${deviceId}/disconnect`);
}

/** 在线设备数（可按车间过滤，GET /device-connections/online/count） */
export function getOnlineDeviceCount(
  params?: { workshop?: string }
): Promise<ApiResponse<DeviceOnlineCount>> {
  return request.get('/device-connections/online/count', { params });
}

/** 手动触发超时清理（POST /device-connections/cleanup-timeout，timeout_secs 默认 300 秒） */
export function cleanupTimeoutDevices(
  timeoutSecs?: number
): Promise<ApiResponse<CleanupTimeoutResult>> {
  return request.post('/device-connections/cleanup-timeout', { timeout_secs: timeoutSecs });
}
