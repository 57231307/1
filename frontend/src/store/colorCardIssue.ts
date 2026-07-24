// 色卡发放 Pinia store（V15 P0-F11）
//
// 全局发放记录状态（与 useColorCardIssue composable 互补）：
// - composable 用于局部组件状态
// - store 用于跨路由/跨组件共享（如顶部未归还数提示、库存快照）
//
// 创建时间：2026-07-18（Batch 477 P0-F11）

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getIssueList } from '@/api/color-card-issue'
import { getColorCardList } from '@/api/color-card'
import type { IssueRecordInfo, ListIssuesQuery } from '@/types/colorCardIssue'
import { logger } from '@/utils/logger'

export const useColorCardIssueStore = defineStore('colorCardIssue', () => {
  const records = ref<IssueRecordInfo[]>([])
  const loading = ref(false)
  const total = ref(0)

  // 当前发放中记录数（status = issued）
  const activeCount = computed(() =>
    records.value.filter((r) => r.status === 'issued').length,
  )

  // 加载发放记录列表
  const fetchRecords = async (query: ListIssuesQuery = {}): Promise<void> => {
    loading.value = true
    try {
      const res = await getIssueList({ page_size: 100, ...query })
      if (res.data) {
        records.value = res.data.items
        total.value = res.data.total
      }
    } catch (error) {
      logger.error('加载色卡发放记录失败:', error)
    } finally {
      loading.value = false
    }
  }

  // 刷新（按当前 query 重新拉取）
  const refresh = async (): Promise<void> => {
    await fetchRecords()
  }

  return {
    records,
    loading,
    total,
    activeCount,
    fetchRecords,
    refresh,
  }
})

// 色卡库存快照 store（V15 P0-F10 库存联动支持）
// 用于跨组件展示色卡当前库存状态（stock_quantity / issued_quantity）
export const useColorCardStockStore = defineStore('colorCardStock', () => {
  const stocks = ref<Array<{
    id: number
    card_no: string
    card_name: string
    stock_quantity: number
    issued_quantity: number
  }>>([])
  const loading = ref(false)

  const fetchStocks = async (): Promise<void> => {
    loading.value = true
    try {
      const res = await getColorCardList({ status: 'active', page_size: 200 })
      stocks.value = (res.data?.items || []).map((c) => ({
        id: c.id,
        card_no: c.card_no,
        card_name: c.card_name,
        stock_quantity: c.stock_quantity,
        issued_quantity: c.issued_quantity,
      }))
    } catch (error) {
      logger.error('加载色卡库存快照失败:', error)
    } finally {
      loading.value = false
    }
  }

  return {
    stocks,
    loading,
    fetchStocks,
  }
})
