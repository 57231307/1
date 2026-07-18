/**
 * useColorCardIssue - 色卡发放业务 composable（V15 P0-F11）
 *
 * 设计原则：将 issues.vue 的业务逻辑（加载列表 / 发放 / 归还 / 遗失 / 损坏 / 取消）
 * 抽离到 composable，配合 Pinia store 形成状态层 + 业务层分离。
 *
 * 关键差异（与原 issues.vue 内联实现对比）：
 *   - 状态：迁至 store/colorCardIssue.ts（跨组件共享）
 *   - 业务：迁至本 composable（组合式 API 封装）
 *   - UI 状态（对话框开关）：仍由调用方组件持有，本 composable 不托管
 */

import { storeToRefs } from 'pinia'
import { useColorCardIssueStore } from '@/store/colorCardIssue'
import type {
  IssueFormState,
  ReturnDialogState,
  LostDialogState,
  DamagedDialogState,
} from '@/types/colorCardIssue'

export function useColorCardIssue() {
  const store = useColorCardIssueStore()
  const {
    availableCards,
    issueRecords,
    activeIssues,
    historyRecords,
    loading,
    actionLoading,
  } = storeToRefs(store)

  /** 初始化：并行加载可发放色卡 + 发放记录 */
  async function init(): Promise<void> {
    await Promise.all([store.loadCards(), store.loadRecords()])
  }

  /** 刷新记录（不发起新加载色卡） */
  async function refreshRecords(): Promise<void> {
    await store.loadRecords()
  }

  /** 发放 */
  async function handleIssue(form: IssueFormState): Promise<boolean> {
    try {
      await store.issue(form)
      return true
    } catch (err) {
      console.error('[useColorCardIssue] 发放失败', err)
      return false
    }
  }

  /** 归还 */
  async function handleReturn(recordId: number, dto: ReturnDialogState): Promise<boolean> {
    try {
      await store.returnRecord(recordId, dto)
      return true
    } catch (err) {
      console.error('[useColorCardIssue] 归还失败', err)
      return false
    }
  }

  /** 遗失 */
  async function handleMarkLost(recordId: number, dto: LostDialogState): Promise<boolean> {
    try {
      await store.markLost(recordId, dto)
      return true
    } catch (err) {
      console.error('[useColorCardIssue] 登记遗失失败', err)
      return false
    }
  }

  /** 损坏 */
  async function handleMarkDamaged(recordId: number, dto: DamagedDialogState): Promise<boolean> {
    try {
      await store.markDamaged(recordId, dto)
      return true
    } catch (err) {
      console.error('[useColorCardIssue] 标记损坏失败', err)
      return false
    }
  }

  /** 取消 */
  async function handleCancel(recordId: number, remark: string): Promise<boolean> {
    try {
      await store.cancelRecord(recordId, remark)
      return true
    } catch (err) {
      console.error('[useColorCardIssue] 取消发放失败', err)
      return false
    }
  }

  return {
    // refs（state + getters）
    availableCards,
    issueRecords,
    activeIssues,
    historyRecords,
    loading,
    actionLoading,
    // actions
    init,
    refreshRecords,
    handleIssue,
    handleReturn,
    handleMarkLost,
    handleMarkDamaged,
    handleCancel,
  }
}
