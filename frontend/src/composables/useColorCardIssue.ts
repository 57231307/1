// 色卡发放业务 composable（V15 P0-F11）
//
// 封装发放列表加载、发放/归还/遗失/损坏/取消等业务操作
// 供 ColorCardIssueForm.vue / ColorCardIssueDetail.vue / issues.vue 等组件复用
//
// 创建时间：2026-07-18（Batch 477 P0-F11）

import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getIssueList,
  issueColorCard,
  returnIssue,
  markIssueLost,
  markIssueDamaged,
  cancelIssue,
} from '@/api/color-card-issue'
import type {
  IssueRecordInfo,
  ListIssuesQuery,
  CreateIssueDto,
  ReturnIssueDto,
  MarkLostDto,
  MarkDamagedDto,
  CancelIssueDto,
  IssueStatusValue,
} from '@/types/colorCardIssue'

// 发放中状态：未归还的发放记录
const ACTIVE_STATUS: IssueStatusValue = 'issued'

export function useColorCardIssue() {
  const records = ref<IssueRecordInfo[]>([])
  const loading = ref(false)
  const actionLoading = ref(false)
  const total = ref(0)

  // 发放中记录（status = issued）
  const activeIssues = computed(() =>
    records.value.filter((r) => r.status === ACTIVE_STATUS),
  )

  // 历史记录（status != issued）
  const historyRecords = computed(() =>
    records.value.filter((r) => r.status !== ACTIVE_STATUS),
  )

  // 加载发放记录列表
  const loadRecords = async (query: ListIssuesQuery = {}): Promise<void> => {
    loading.value = true
    try {
      const res = await getIssueList({ page_size: 100, ...query })
      records.value = res.data?.items || []
      total.value = res.data?.total || 0
    } finally {
      loading.value = false
    }
  }

  // 发放色卡（V15 P0-F10：后端事务内扣减库存）
  const issue = async (dto: CreateIssueDto): Promise<IssueRecordInfo | null> => {
    actionLoading.value = true
    try {
      const res = await issueColorCard(dto)
      ElMessage.success('发放成功')
      return res.data || null
    } finally {
      actionLoading.value = false
    }
  }

  // 归还色卡（V15 P0-F10：后端事务内恢复库存）
  const returnCard = async (
    recordId: number,
    dto: ReturnIssueDto,
  ): Promise<IssueRecordInfo | null> => {
    actionLoading.value = true
    try {
      const res = await returnIssue(recordId, dto)
      ElMessage.success('归还成功')
      return res.data || null
    } finally {
      actionLoading.value = false
    }
  }

  // 登记遗失（V15 P0-F10：后端事务内扣减库存 + 色卡状态变 lost）
  const markLost = async (
    recordId: number,
    dto: MarkLostDto,
  ): Promise<IssueRecordInfo | null> => {
    actionLoading.value = true
    try {
      const res = await markIssueLost(recordId, dto)
      ElMessage.success('已登记遗失')
      return res.data || null
    } finally {
      actionLoading.value = false
    }
  }

  // 标记损坏（V15 P0-F10：后端事务内扣减库存）
  const markDamaged = async (
    recordId: number,
    dto: MarkDamagedDto,
  ): Promise<IssueRecordInfo | null> => {
    actionLoading.value = true
    try {
      const res = await markIssueDamaged(recordId, dto)
      ElMessage.success('已标记损坏')
      return res.data || null
    } finally {
      actionLoading.value = false
    }
  }

  // 取消发放（V15 P0-F10：后端事务内恢复库存，等同从未发放）
  const cancel = async (
    recordId: number,
    dto: CancelIssueDto,
  ): Promise<IssueRecordInfo | null> => {
    actionLoading.value = true
    try {
      const res = await cancelIssue(recordId, dto)
      ElMessage.success('已取消发放')
      return res.data || null
    } finally {
      actionLoading.value = false
    }
  }

  return {
    records,
    loading,
    actionLoading,
    total,
    activeIssues,
    historyRecords,
    loadRecords,
    issue,
    returnCard,
    markLost,
    markDamaged,
    cancel,
  }
}
