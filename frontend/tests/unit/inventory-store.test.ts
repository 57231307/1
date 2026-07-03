import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock the inventory API before imports
vi.mock('@/api/inventory', () => ({
  inventoryApi: {
    getStockList: vi.fn(),
    getStockAlerts: vi.fn(),
    createStockAdjustment: vi.fn(),
  },
}))

// Use real Pinia for store tests
vi.mock('pinia', async (importOriginal) => {
  const actual = await importOriginal<typeof import('pinia')>()
  return actual
})

import { setActivePinia, createPinia } from 'pinia'
import { useInventoryStore } from '@/store/inventory'
import { inventoryApi } from '@/api/inventory'

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
    vi.mocked(inventoryApi.getStockList).mockResolvedValue({
      data: { list: mockStocks, total: 2 },
    } as any)

    const store = useInventoryStore()
    await store.fetchStocks()

    expect(inventoryApi.getStockList).toHaveBeenCalled()
    expect(store.stocks).toEqual(mockStocks)
    expect(store.total).toBe(2)
    expect(store.loading).toBe(false)
  })

  it('fetchStocks 应该处理错误', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.mocked(inventoryApi.getStockList).mockRejectedValue(new Error('Network error'))

    const store = useInventoryStore()
    await store.fetchStocks()

    expect(store.stocks).toEqual([])
    expect(store.loading).toBe(false)
    expect(consoleSpy).toHaveBeenCalled()
    consoleSpy.mockRestore()
  })

  it('fetchStocks 应该设置 loading 状态', async () => {
    let resolvePromise: (value: any) => void
    const promise = new Promise((resolve) => {
      resolvePromise = resolve
    })
    vi.mocked(inventoryApi.getStockList).mockReturnValue(promise as any)

    const store = useInventoryStore()
    const fetchPromise = store.fetchStocks()

    expect(store.loading).toBe(true)

    resolvePromise!({ data: { list: [], total: 0 } })
    await fetchPromise

    expect(store.loading).toBe(false)
  })

  it('fetchAlerts 应该获取库存告警', async () => {
    const mockAlerts = [{ id: 1, product_name: '面料A', alert_type: 'low_stock' }]
    vi.mocked(inventoryApi.getStockAlerts).mockResolvedValue({
      data: mockAlerts,
    } as any)

    const store = useInventoryStore()
    await store.fetchAlerts()

    expect(inventoryApi.getStockAlerts).toHaveBeenCalled()
    expect(store.alerts).toEqual(mockAlerts)
  })

  it('fetchAlerts 应该处理错误', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.mocked(inventoryApi.getStockAlerts).mockRejectedValue(new Error('Failed'))

    const store = useInventoryStore()
    await store.fetchAlerts()

    expect(store.alerts).toEqual([])
    expect(consoleSpy).toHaveBeenCalled()
    consoleSpy.mockRestore()
  })

  it('createAdjustment 应该创建调整并刷新列表', async () => {
    vi.mocked(inventoryApi.createStockAdjustment).mockResolvedValue({} as any)
    vi.mocked(inventoryApi.getStockList).mockResolvedValue({
      data: { list: [{ id: 1 }], total: 1 },
    } as any)

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

    expect(inventoryApi.createStockAdjustment).toHaveBeenCalledWith(adjustmentData)
    expect(inventoryApi.getStockList).toHaveBeenCalled()
    expect(result).toBe(true)
  })

  it('createAdjustment 应该在失败时返回 false', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.mocked(inventoryApi.createStockAdjustment).mockRejectedValue(new Error('Failed'))

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
