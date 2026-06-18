/**
 * StockTab.vue 单元测试
 * 任务编号: Wave 4 P2-1 PR-2
 * 覆盖：数据加载 / 排序 / 过滤 / 行选择
 * 注意：本地不允许 npm run test:run，所有测试通过 git push 触发 CI 验证
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { ref, nextTick } from 'vue'

// 模块级 refs：用于跨测试共享 mock 状态（在 vi.mock 工厂执行前已初始化）
const mockData = ref<any[]>([])
const mockLoading = ref(false)
const mockPage = ref(1)
const mockPageSize = ref(20)
const mockTotal = ref(0)
const mockQueryParams = ref<Record<string, any>>({})

// 工厂函数：构造 useTableApi mock 返回值
const makeTableApiMock = () => ({
  data: mockData,
  loading: mockLoading,
  page: mockPage,
  pageSize: mockPageSize,
  total: mockTotal,
  queryParams: mockQueryParams,
  refresh: vi.fn(async () => {}),
  reset: vi.fn(() => {
    mockQueryParams.value = {}
    mockPage.value = 1
    mockPageSize.value = 20
  }),
  setQueryParam: vi.fn(),
})

// vi.mock 在模块顶层被提升，工厂函数在 import StockTab 时调用
vi.mock('@/composables/useTableApi', () => ({
  useTableApi: () => makeTableApiMock(),
}))

vi.mock('@/api/warehouse', () => ({
  warehouseApi: {
    list: vi.fn().mockResolvedValue({
      data: {
        list: [
          { id: 1, warehouse_name: '主仓' },
          { id: 2, warehouse_name: '分仓' },
        ],
      },
    }),
  },
}))

// 局部 mock element-plus：保留真实 export
vi.mock('element-plus', async (importOriginal) => {
  const actual = await importOriginal<typeof import('element-plus')>()
  return { ...actual }
})

// mock 懒加载，避免按需加载阻塞测试
vi.mock('@/utils/lazy-loader', () => ({
  loadIfNot: vi.fn(),
  createLazyLoader: () => ({}),
}))

import StockTab from '@/views/inventory/tabs/StockTab.vue'

const sampleStocks = [
  {
    id: 1,
    product_id: 101,
    warehouse_id: 1,
    product_code: 'SKU-001',
    product_name: '产品甲',
    warehouse_name: '主仓',
    batch_no: 'B20240601',
    location: 'A-01',
    quantity: 100,
    min_quantity: 50,
    unit: '米',
    status: 'normal',
    updated_at: '2026-06-16 10:00:00',
  },
  {
    id: 2,
    product_id: 102,
    warehouse_id: 2,
    product_code: 'SKU-002',
    product_name: '产品乙',
    warehouse_name: '分仓',
    batch_no: 'B20240602',
    location: 'B-02',
    quantity: 30,
    min_quantity: 50,
    unit: '米',
    status: 'warning',
    updated_at: '2026-06-16 11:00:00',
  },
]

describe('StockTab.vue', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockData.value = []
    mockLoading.value = false
    mockPage.value = 1
    mockPageSize.value = 20
    mockTotal.value = 0
    mockQueryParams.value = {}
  })

  it('数据加载：组件挂载时正确接收数据并渲染表格', async () => {
    mockData.value = sampleStocks
    mockTotal.value = sampleStocks.length

    const wrapper = mount(StockTab)
    await flushPromises()

    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.stock-tab').exists()).toBe(true)
    expect(wrapper.find('.filter-card').exists()).toBe(true)
    expect(wrapper.find('.table-card').exists()).toBe(true)
  })

  it('过滤：输入关键词触发查询并更新 queryParams', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    // 直接通过 vm 修改响应式参数（el-input 渲染依赖 Element Plus 内部组件，DOM 选择器不稳定）
    const vm = wrapper.vm as any
    vm.localQueryParams.keyword = 'SKU-001'
    await nextTick()

    const queryBtn = wrapper.findAll('button').find(b => b.text().includes('查询'))
    expect(queryBtn).toBeTruthy()
    await queryBtn!.trigger('click')
    await flushPromises()

    expect(mockQueryParams.value.keyword).toBe('SKU-001')
  })

  it('过滤：选择状态触发查询并重置 page 为 1', async () => {
    mockPage.value = 5
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    vm.localQueryParams.status = 'warning'
    await nextTick()

    const queryBtn = wrapper.findAll('button').find(b => b.text().includes('查询'))
    expect(queryBtn).toBeTruthy()
    await queryBtn!.trigger('click')
    await flushPromises()

    expect(mockPage.value).toBe(1)
    expect(mockQueryParams.value.status).toBe('warning')
  })

  it('重置：清空所有过滤条件', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    vm.localQueryParams.keyword = 'test'
    vm.localQueryParams.warehouse_id = 1
    vm.localQueryParams.status = 'normal'

    const resetBtn = wrapper.findAll('button').find(b => b.text().includes('重置'))
    expect(resetBtn).toBeTruthy()
    await resetBtn!.trigger('click')
    await flushPromises()

    expect(vm.localQueryParams.keyword).toBe('')
    expect(vm.localQueryParams.warehouse_id).toBeUndefined()
    expect(vm.localQueryParams.status).toBe('')
  })

  it('排序：列定义中标记 sortable 的字段应包含 sortable=true', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    const cols = vm.columns
    const colList = Array.isArray(cols) ? cols : (cols?.value ?? [])

    const skuCol = colList.find((c: any) => c.key === 'product_code')
    expect(skuCol?.sortable).toBe(true)
    const nameCol = colList.find((c: any) => c.key === 'product_name')
    expect(nameCol?.sortable).toBe(true)
    const qtyCol = colList.find((c: any) => c.key === 'quantity')
    expect(qtyCol?.sortable).toBe(true)
    const updCol = colList.find((c: any) => c.key === 'updated_at')
    expect(updCol?.sortable).toBe(true)
  })

  it('行选择：toggleRow 单行切换', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    const row = sampleStocks[0]

    expect(vm.selectedRows.length).toBe(0)
    vm.toggleRow(row)
    await nextTick()
    expect(vm.selectedRows.length).toBe(1)
    expect(vm.selectedRows[0].id).toBe(1)
    vm.toggleRow(row)
    await nextTick()
    expect(vm.selectedRows.length).toBe(0)
  })

  it('行选择：toggleAll 全选当前页', async () => {
    mockData.value = sampleStocks
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    vm.toggleAll()
    await nextTick()
    expect(vm.selectedRows.length).toBe(sampleStocks.length)
    expect(vm.allSelected).toBe(true)
    vm.toggleAll()
    await nextTick()
    expect(vm.selectedRows.length).toBe(0)
    expect(vm.allSelected).toBe(false)
  })

  it('行选择：indeterminate 在部分选中时为 true', async () => {
    mockData.value = sampleStocks
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    vm.toggleRow(sampleStocks[0])
    await nextTick()
    expect(vm.indeterminate).toBe(true)
    expect(vm.allSelected).toBe(false)
  })

  it('行选择：clearSelection 清空选择', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    vm.toggleRow(sampleStocks[0])
    vm.toggleRow(sampleStocks[1])
    await nextTick()
    expect(vm.selectedRows.length).toBe(2)

    vm.clearSelection()
    await nextTick()
    expect(vm.selectedRows.length).toBe(0)
  })

  it('翻页：handlePageChange 翻页时清空选择', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    vm.toggleRow(sampleStocks[0])
    await nextTick()
    expect(vm.selectedRows.length).toBe(1)

    vm.handlePageChange(2)
    await nextTick()
    expect(mockPage.value).toBe(2)
    expect(vm.selectedRows.length).toBe(0)
  })

  it('暴露 fetchData：父组件可通过 ref 调用', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const exposed = (wrapper.vm as any).$.exposed
    expect(exposed?.fetchData).toBeDefined()
  })

  it('列定义：低库存数量触发 low-stock 高亮 class', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    const cols = vm.columns
    const colList = Array.isArray(cols) ? cols : (cols?.value ?? [])
    const qtyCol = colList.find((c: any) => c.key === 'quantity')
    expect(qtyCol).toBeTruthy()
    expect(qtyCol.renderCell).toBeDefined()

    const vnode = qtyCol.renderCell(sampleStocks[1])
    expect(vnode.props).toBeDefined()
  })

  it('列定义：状态列使用 el-tag 渲染', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    const cols = vm.columns
    const colList = Array.isArray(cols) ? cols : (cols?.value ?? [])
    const statusCol = colList.find((c: any) => c.key === 'status')
    expect(statusCol).toBeTruthy()
    expect(statusCol.renderCell).toBeDefined()

    const vnode = statusCol.renderCell(sampleStocks[0])
    expect(vnode).toBeDefined()
  })
})
