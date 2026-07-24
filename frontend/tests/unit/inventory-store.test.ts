import { describe, it, expect, vi, beforeEach } from 'vitest'

// D14 Batch 5b：原 inventoryApi 对象已转风格 B 函数
vi.mock('@/api/inventory', () => ({
  getStockList: vi.fn(),
  getStockAlertList: vi.fn(),
  createStockAdjustment: vi.fn(),
}))

// Use real Pinia for store tests
vi.mock('pinia', async (importOriginal) => {
  const actual = await importOriginal<typeof import('pinia')>()
  return actual
})

import { setActivePinia, createPinia } from 'pinia'
import { useInventoryStore } from '@/store/inventory'
import { getStockList, getStockAlertList, createStockAdjustment } from '@/api/inventory'
// P2-18 修复（批次 86 v2 复审）：清理 6 处 as any，改为显式类型断言
import type { ApiResponse } from '@/types/api'
import type { InventoryStock, StockAlert } from '@/api/inventory'

// 测试用响应类型别名（提升可读性）
type StockListResponse = ApiResponse<{ list: InventoryStock[]; total: number }>
type StockAlertsResponse = ApiResponse<StockAlert[]>
type AdjustmentResponse = ApiResponse<{ id: number; adjustment_no: string }>

describe('Inventory Store 测试', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('应该有正确的初始状态', () => {
    const store = useInventoryStore()
    expect(store.stocks).toEqual([])
    expect(store.alerts).toEqual([])
    expect(store.total).toBe(0)
    expect(store.loading).toBe(false)
  })

  it('fetchStocks 应该获取库存列表', async () => {
    const mockStocks = [
      { id: 1, product_name: '面料A', quantity: 100 },
      { id: 2, product_name: '面料B', quantity: 200 },
    ]
    vi.mocked(getStockList).mockResolvedValue({
      data: { list: mockStocks, total: 2 },
    } as unknown as StockListResponse)

    const store = useInventoryStore()
    await store.fetchStocks()

    expect(getStockList).toHaveBeenCalled()
    expect(store.stocks).toEqual(mockStocks)
    expect(store.total).toBe(2)
    expect(store.loading).toBe(false)
  })

  it('fetchStocks 应该处理错误', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.mocked(getStockList).mockRejectedValue(new Error('Network error'))

    const store = useInventoryStore()
    await store.fetchStocks()

    expect(store.stocks).toEqual([])
    expect(store.loading).toBe(false)
    expect(consoleSpy).toHaveBeenCalled()
    consoleSpy.mockRestore()
  })

  it('fetchStocks 应该设置 loading 状态', async () => {
    // P2-18 修复：promise 类型显式声明，避免 as any
    let resolvePromise: (value: StockListResponse) => void
    const promise = new Promise<StockListResponse>((resolve) => {
      resolvePromise = resolve
    })
    vi.mocked(getStockList).mockReturnValue(promise)

    const store = useInventoryStore()
    const fetchPromise = store.fetchStocks()

    expect(store.loading).toBe(true)

    resolvePromise!({ code: 0, message: 'ok', data: { list: [], total: 0 } })
    await fetchPromise

    expect(store.loading).toBe(false)
  })

  it('fetchAlerts 应该获取库存告警', async () => {
    const mockAlerts = [{ id: 1, product_name: '面料A', alert_type: 'low_stock' }]
    vi.mocked(getStockAlertList).mockResolvedValue({
      data: mockAlerts,
    } as unknown as StockAlertsResponse)

    const store = useInventoryStore()
    await store.fetchAlerts()

    expect(getStockAlertList).toHaveBeenCalled()
    expect(store.alerts).toEqual(mockAlerts)
  })

  it('fetchAlerts 应该处理错误', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.mocked(getStockAlertList).mockRejectedValue(new Error('Failed'))

    const store = useInventoryStore()
    await store.fetchAlerts()

    expect(store.alerts).toEqual([])
    expect(consoleSpy).toHaveBeenCalled()
    consoleSpy.mockRestore()
  })

  it('createAdjustment 应该创建调整并刷新列表', async () => {
    vi.mocked(createStockAdjustment).mockResolvedValue(
      {} as unknown as AdjustmentResponse
    )
    vi.mocked(getStockList).mockResolvedValue({
      data: { list: [{ id: 1 }], total: 1 },
    } as unknown as StockListResponse)

    const store = useInventoryStore()
    // P2-11a 修复（批次 83 v1 复审）：夹具对齐 StockAdjustmentData 契约
    const adjustmentData = {
      warehouse_id: 1,
      product_id: 1,
      adjustment_quantity: 10,
      adjustment_type: 'increase' as const,
      reason: '测试调整',
    }
    const result = await store.createAdjustment(adjustmentData)

    expect(createStockAdjustment).toHaveBeenCalledWith(adjustmentData)
    expect(getStockList).toHaveBeenCalled()
    expect(result).toBe(true)
  })

  it('createAdjustment 应该在失败时返回 false', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.mocked(createStockAdjustment).mockRejectedValue(new Error('Failed'))

    const store = useInventoryStore()
    // P2-11a 修复（批次 83 v1 复审）：夹具对齐 StockAdjustmentData 契约
    const result = await store.createAdjustment({
      warehouse_id: 1,
      product_id: 1,
      adjustment_quantity: 10,
      adjustment_type: 'increase',
      reason: '测试调整',
    })

    expect(result).toBe(false)
    expect(consoleSpy).toHaveBeenCalled()
    consoleSpy.mockRestore()
  })
})
