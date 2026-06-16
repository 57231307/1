/**
 * useTableApi - 通用表格数据 composable
 * 任务编号: Wave 4 P2-1 PR-1
 * 关联 spec: docs/superpowers/specs/2026-06-16-wave4-p2-1-design.md 第四章
 */
import { ref, computed, watch } from 'vue'
import type { Ref } from 'vue'
import { request } from '@/api/request'

export interface UseTableApiOptions<T = any> {
  url: string
  defaultParams?: Record<string, any>
  defaultPageSize?: number
  pageKey?: string
  pageSizeKey?: string
  totalKey?: string
  listKey?: string
  retryCount?: number
  retryDelay?: number
  onError?: (err: any) => void
}

export interface UseTableApiReturn<T = any> {
  data: Ref<T[]>
  total: Ref<number>
  loading: Ref<boolean>
  page: Ref<number>
  pageSize: Ref<number>
  queryParams: Ref<Record<string, any>>
  refresh: () => Promise<void>
  reset: () => void
  setQueryParam: (key: string, value: any) => void
}

/**
 * 通用表格数据获取 composable
 * 支持分页 / 筛选 / 排序 / loading / 错误重试
 */
export function useTableApi<T = any>(
  optionsOrUrl: UseTableApiOptions<T> | string
): UseTableApiReturn<T> {
  const options: UseTableApiOptions<T> = typeof optionsOrUrl === 'string'
    ? { url: optionsOrUrl }
    : optionsOrUrl

  const {
    url,
    defaultParams = {},
    defaultPageSize = 20,
    pageKey = 'page',
    pageSizeKey = 'page_size',
    totalKey = 'total',
    listKey = 'list',
    retryCount = 2,
    retryDelay = 1000,
    onError,
  } = options

  const data = ref<T[]>([]) as Ref<T[]>
  const total = ref(0)
  const loading = ref(false)
  const page = ref(1)
  const pageSize = ref(defaultPageSize)
  const queryParams = ref<Record<string, any>>({ ...defaultParams })

  /**
   * 从响应中探测 list 和 total 字段
   */
  const detectList = (payload: any): T[] => {
    if (Array.isArray(payload)) return payload
    if (Array.isArray(payload?.list)) return payload.list
    if (Array.isArray(payload?.items)) return payload.items
    if (Array.isArray(payload?.data)) return payload.data
    if (Array.isArray(payload?.results)) return payload.results
    return []
  }

  const detectTotal = (payload: any): number => {
    if (typeof payload?.[totalKey] === 'number') return payload[totalKey]
    if (typeof payload?.total === 'number') return payload.total
    if (typeof payload?.count === 'number') return payload.count
    return 0
  }

  /**
   * 核心请求函数（带重试）
   */
  const fetchData = async (attempt = 0): Promise<void> => {
    loading.value = true
    try {
      const params: Record<string, any> = {
        ...queryParams.value,
        [pageKey]: page.value,
        [pageSizeKey]: pageSize.value,
      }
      const res = await request.get(url, { params })
      const payload = res.data?.data ?? res.data
      data.value = detectList(payload) as T[]
      total.value = detectTotal(payload)
    } catch (err) {
      if (attempt < retryCount) {
        await new Promise(r => setTimeout(r, retryDelay))
        return fetchData(attempt + 1)
      }
      onError?.(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const refresh = async (): Promise<void> => {
    await fetchData()
  }

  const reset = (): void => {
    queryParams.value = { ...defaultParams }
    page.value = 1
    pageSize.value = defaultPageSize
  }

  const setQueryParam = (key: string, value: any): void => {
    queryParams.value = { ...queryParams.value, [key]: value }
  }

  // 监听分页变化自动加载
  watch([page, pageSize], () => {
    fetchData()
  })

  // 初始加载
  fetchData()

  return {
    data,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh,
    reset,
    setQueryParam,
  }
}
