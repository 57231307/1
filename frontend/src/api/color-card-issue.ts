// 色卡发放 API 客户端（V15 P0-F11）
//
// 拆分自 frontend/src/api/color-card.ts：将发放相关 API 独立到 api/ 目录
// 端点路径相对于 baseURL（/api/v1/erp），不要重复添加前缀
//
// 创建时间：2026-07-18（Batch 477 P0-F11）

import { request } from './request'
import type {
  IssueRecordInfo,
  ListIssuesQuery,
  CreateIssueDto,
  ReturnIssueDto,
  MarkLostDto,
  MarkDamagedDto,
  CancelIssueDto,
} from '@/types/colorCardIssue'
import type { PagedResponse } from './color-card'

// 发放记录详情
export function getIssue(recordId: number) {
  return request.get<{ data: IssueRecordInfo }>(`/color-cards/issues/${recordId}`)
}

// 发放记录列表
export function listIssues(params: ListIssuesQuery) {
  return request.get<{ data: PagedResponse<IssueRecordInfo> }>(
    '/color-cards/issues',
    { params },
  )
}

// 创建发放记录（V15 P0-F10：后端在事务内扣减色卡 issued_quantity）
export function issueColorCard(dto: CreateIssueDto) {
  return request.post<{ data: IssueRecordInfo }>('/color-cards/issues', dto)
}

// 归还色卡（V15 P0-F10：后端在事务内恢复色卡 issued_quantity）
export function returnIssue(recordId: number, dto: ReturnIssueDto) {
  return request.post<{ data: IssueRecordInfo }>(
    `/color-cards/issues/${recordId}/return`,
    dto,
  )
}

// 登记遗失（V15 P0-F10：后端在事务内扣减 issued_quantity + 色卡状态变 lost）
export function markIssueLost(recordId: number, dto: MarkLostDto) {
  return request.post<{ data: IssueRecordInfo }>(
    `/color-cards/issues/${recordId}/lost`,
    dto,
  )
}

// 标记损坏（V15 P0-F10：后端在事务内扣减 issued_quantity）
export function markIssueDamaged(recordId: number, dto: MarkDamagedDto) {
  return request.post<{ data: IssueRecordInfo }>(
    `/color-cards/issues/${recordId}/damaged`,
    dto,
  )
}

// 取消发放（V15 P0-F10：后端在事务内恢复 issued_quantity，等同从未发放）
export function cancelIssue(recordId: number, dto: CancelIssueDto) {
  return request.post<{ data: IssueRecordInfo }>(
    `/color-cards/issues/${recordId}/cancel`,
    dto,
  )
}
