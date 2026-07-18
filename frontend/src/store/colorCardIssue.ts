/**
 * 色卡发放 - Pinia store（V15 P0-F11）
 *
 * 设计原则：单一数据源。所有发放记录与可发放色卡列表由 store 集中管理，
 * 组件 / composable 通过 store 订阅与更新，避免 props 透传地狱。
 *
 * V15 P0-F10 后端已加入 stock_quantity 字段 + 事务 + 行锁，
 * 前端只需调用 API 即可，stock 不足由后端返回错误。
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  listColorCards,
  listIssues,
  issueColorCard,
  returnIssue,
  markIssueLost,
  markIssueDamaged,
  cancelIssue,
  type IssueRecordInfo,
  type ColorCardListItem,
} from '@/api/color-card'
import type {
  IssueFormState,
  ReturnDialogState,
  LostDialogState,
  DamagedDialogState,
} from '@/types/colorCardIssue'

export const useColorCardIssueStore = defineStore('colorCardIssue', () => {
  // ============== State ==============
  /** 可发放色卡列表（status=active） */
  const availableCards = ref<ColorCardListItem[]>([])
  /** 发放记录全量列表（前端按 status 切片为发放中 / 历史） */
  const issueRecords = ref<IssueRecordInfo[]>([])
  /** 列表加载中 */
  const loading = ref(false)
  /** 操作进行中（发放 / 归还 / 遗失 / 损坏 / 取消） */
  const actionLoading = ref(false)

  // ============== Getters ==============
  /** 发放中：status=issued */
  const activeIssues = computed(() =>
    issueRecords.value.filter((r) => r.status === 'issued'),
  )
  /** 历史记录：非 issued（已归还/遗失/损坏/已取消） */
  const historyRecords = computed(() =>
    issueRecords.value.filter((r) => r.status !== 'issued'),
  )

  // ============== Actions ==============

  /** 加载可发放色卡列表 */
  async function loadCards(): Promise<void> {
    const res = await listColorCards({ status: 'active', page_size: 200 })
    availableCards.value = res.data?.items || []
  }

  /** 加载发放记录列表 */
  async function loadRecords(): Promise<void> {
    loading.value = true
    try {
      const res = await listIssues({ page_size: 100 })
      issueRecords.value = res.data?.items || []
    } finally {
      loading.value = false
    }
  }

  /** 执行发放（V15 P0-F10 后端会扣减 stock_quantity） */
  async function issue(form: IssueFormState): Promise<void> {
    actionLoading.value = true
    try {
      await issueColorCard({
        color_card_id: form.color_card_id!,
        customer_id: form.customer_id,
        issue_qty: form.issue_qty || 1,
        expected_return_date: form.expected_return_date || undefined,
        purpose: form.purpose || undefined,
        remark: form.remark || undefined,
        dye_lot_no: form.dye_lot_no || undefined,
      })
      // 发放成功后刷新记录 + 可发放色卡列表（库存变化）
      await Promise.all([loadRecords(), loadCards()])
    } finally {
      actionLoading.value = false
    }
  }

  /** 归还色卡（V15 P0-F10 后端会还原 stock_quantity） */
  async function returnRecord(recordId: number, dto: ReturnDialogState): Promise<void> {
    actionLoading.value = true
    try {
      await returnIssue(recordId, {
        actual_return_date: dto.actual_return_date || undefined,
        remark: dto.remark || undefined,
      })
      await Promise.all([loadRecords(), loadCards()])
    } finally {
      actionLoading.value = false
    }
  }

  /** 登记遗失（V15 P0-F10 后端不还原 stock_quantity，色卡视为消耗） */
  async function markLost(recordId: number, dto: LostDialogState): Promise<void> {
    actionLoading.value = true
    try {
      await markIssueLost(recordId, {
        compensation_amount: dto.compensation_amount,
        remark: dto.remark || undefined,
      })
      await Promise.all([loadRecords(), loadCards()])
    } finally {
      actionLoading.value = false
    }
  }

  /** 标记损坏（V15 P0-F10 后端不还原 stock_quantity，色卡视为消耗） */
  async function markDamaged(recordId: number, dto: DamagedDialogState): Promise<void> {
    actionLoading.value = true
    try {
      await markIssueDamaged(recordId, {
        compensation_amount: dto.compensation_amount || undefined,
        remark: dto.remark || undefined,
      })
      await Promise.all([loadRecords(), loadCards()])
    } finally {
      actionLoading.value = false
    }
  }

  /** 取消发放（V15 P0-F10 后端会还原 stock_quantity） */
  async function cancelRecord(recordId: number, remark: string): Promise<void> {
    actionLoading.value = true
    try {
      await cancelIssue(recordId, {
        remark: remark || undefined,
      })
      await Promise.all([loadRecords(), loadCards()])
    } finally {
      actionLoading.value = false
    }
  }

  return {
    // state
    availableCards,
    issueRecords,
    loading,
    actionLoading,
    // getters
    activeIssues,
    historyRecords,
    // actions
    loadCards,
    loadRecords,
    issue,
    returnRecord,
    markLost,
    markDamaged,
    cancelRecord,
  }
})
