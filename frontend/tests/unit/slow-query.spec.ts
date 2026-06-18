/**
 * SlowQueryView 单元测试（P13 批 1 B-慢查询审计）
 * 覆盖：挂载首屏加载 / 筛选 / 刷新触发
 * 注意：本地不允许 npm run test:run，所有测试通过 git push 触发 CI 验证
 */
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'

// 模块级 mock：跨测试共享
const mockListSlowQueries = vi.fn()
const mockGetSlowQueryStats = vi.fn()
const mockRefreshSlowQueries = vi.fn()

vi.mock('@/api/slow-query', () => ({
  listSlowQueries: (...args: unknown[]) => mockListSlowQueries(...args),
  getSlowQueryStats: (...args: unknown[]) => mockGetSlowQueryStats(...args),
  refreshSlowQueries: (...args: unknown[]) => mockRefreshSlowQueries(...args),
}))

// 局部 mock element-plus：保留真实 export
vi.mock('element-plus', async (importOriginal) => {
  const actual = await importOriginal<typeof import('element-plus')>()
  return { ...actual }
})

// mock V2Table：避免依赖 el-table-v2 / ElAutoResizer 真实渲染
vi.mock('@/components/V2Table/index.vue', () => ({
  default: {
    name: 'V2Table',
    props: ['columns', 'data', 'loading', 'page', 'pageSize', 'total', 'rowKey', 'emptyText'],
    emits: ['page-change', 'size-change'],
    template: '<div class="v2-table-mock" />',
  },
}))

import SlowQueryView from '@/views/system/slow-query/index.vue'

const sampleItems = [
  {
    id: 1,
    query_text: 'SELECT * FROM users WHERE id = $1',
    execution_time_ms: 250.5,
    calls: 100,
    rows_examined: 50,
    database_name: 'bingxi_erp',
    tenant_id: 1,
    captured_at: '2026-06-18T08:00:00Z',
  },
  {
    id: 2,
    query_text: 'SELECT count(*) FROM orders',
    execution_time_ms: 500.0,
    calls: 25,
    rows_examined: 1000,
    database_name: 'bingxi_erp',
    tenant_id: 1,
    captured_at: '2026-06-18T08:05:00Z',
  },
]

const sampleStats = {
  top10: [
    {
      query_text: 'SELECT count(*) FROM orders',
      max_exec_time_ms: 500.0,
      total_calls: 100,
      avg_rows: 1000.0,
      sample_count: 4,
    },
    {
      query_text: 'SELECT * FROM users',
      max_exec_time_ms: 250.5,
      total_calls: 50,
      avg_rows: 50.0,
      sample_count: 2,
    },
  ],
  total_count: 2,
  time_range: '近 7 天',
}

describe('SlowQueryView（P13 批 1 B-慢查询审计）', () => {
  beforeEach(() => {
    mockListSlowQueries.mockReset()
    mockGetSlowQueryStats.mockReset()
    mockRefreshSlowQueries.mockReset()

    mockListSlowQueries.mockResolvedValue({
      items: sampleItems,
      total: 2,
      page: 1,
      page_size: 20,
    } as any)
    mockGetSlowQueryStats.mockResolvedValue(sampleStats as any)
    mockRefreshSlowQueries.mockResolvedValue({
      inserted: 3,
      message: '本次采集写入 3 条慢查询记录',
    } as any)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  /** 挂载即触发首屏加载（同时调用 list + stats） */
  it('挂载时自动加载首屏数据并发送分页参数', async () => {
    const wrapper = mount(SlowQueryView)
    await flushPromises()
    expect(mockListSlowQueries).toHaveBeenCalledTimes(1)
    expect(mockGetSlowQueryStats).toHaveBeenCalledTimes(1)
    const args = mockListSlowQueries.mock.calls[0][0]
    expect(args.page).toBe(1)
    expect(args.page_size).toBe(20)
    expect(wrapper.find('.slow-query-view').exists()).toBe(true)
  })

  /** 点击查询按钮：将筛选条件传入 API 并回到第一页 */
  it('点击查询按钮会把筛选条件传入 API', async () => {
    const wrapper = mount(SlowQueryView)
    await flushPromises()
    mockListSlowQueries.mockClear()

    const vm = wrapper.vm as any
    vm.filterForm.min_duration = 200
    vm.filterForm.keyword = 'users'
    vm.filterForm.dateRange = ['2026-06-18T00:00:00Z', '2026-06-18T23:59:59Z']
    await nextTick()

    await vm.handleQuery()
    await flushPromises()

    expect(mockListSlowQueries).toHaveBeenCalledTimes(1)
    const params = mockListSlowQueries.mock.calls[0][0]
    expect(params.min_duration).toBe(200)
    expect(params.keyword).toBe('users')
    expect(params.start_time).toBe('2026-06-18T00:00:00Z')
    expect(params.end_time).toBe('2026-06-18T23:59:59Z')
    expect(params.page).toBe(1)
  })

  /** 点击手动刷新按钮：调用 refreshSlowQueries 并重新加载 */
  it('点击手动刷新按钮触发 refreshSlowQueries 并重新加载', async () => {
    const wrapper = mount(SlowQueryView)
    await flushPromises()
    // 清掉首屏调用计数，便于精确断言后续调用
    mockListSlowQueries.mockClear()
    mockGetSlowQueryStats.mockClear()
    mockRefreshSlowQueries.mockClear()

    const vm = wrapper.vm as any
    await vm.handleRefresh()
    await flushPromises()

    // refresh 调用一次
    expect(mockRefreshSlowQueries).toHaveBeenCalledTimes(1)
    // refresh 后会重新加载 list + stats
    expect(mockListSlowQueries).toHaveBeenCalledTimes(1)
    expect(mockGetSlowQueryStats).toHaveBeenCalledTimes(1)
  })
})
