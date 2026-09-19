import { request } from './request';

export interface BadDebt {
  id: number;
  status: string;
  [key: string]: unknown;
}

export interface CollectionTask {
  id: number;
  status: string;
  [key: string]: unknown;
}

// 坏账计提
export function runProvision(data: Record<string, unknown>) {
  return request.post('/bad-debts/run-provision', data);
}

export function getBadDebtList(params?: Record<string, unknown>) {
  return request.get('/bad-debts', { params });
}

export function getBadDebtDetail(id: number) {
  return request.get(`/bad-debts/${id}`);
}

export function confirmBadDebt(id: number) {
  return request.post(`/bad-debts/${id}/confirm`);
}

export function reverseBadDebt(id: number) {
  return request.post(`/bad-debts/${id}/reverse`);
}

// 催收任务
export function getCollectionTaskList(params?: Record<string, unknown>) {
  return request.get('/collection-tasks', { params });
}

export function getCollectionTaskDetail(id: number) {
  return request.get(`/collection-tasks/${id}`);
}

export function reassignCollectionTask(id: number, data: Record<string, unknown>) {
  return request.post(`/collection-tasks/${id}/reassign`, data);
}

export function cancelCollectionTask(id: number) {
  return request.post(`/collection-tasks/${id}/cancel`);
}

export const BAD_DEBT_STATUS_LABEL: Record<string, string> = {
  pending: '待确认',
  confirmed: '已确认',
  reversed: '已冲销',
};

export const COLLECTION_TASK_STATUS_LABEL: Record<string, string> = {
  open: '进行中',
  reassigning: '转派中',
  cancelled: '已取消',
  completed: '已完成',
};
