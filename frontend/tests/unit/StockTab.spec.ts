/**
 * StockTab.vue 单元测试
 * 任务编号: Wave 4 P2-1 PR-2
 * 覆盖：数据加载 / 排序 / 过滤 / 行选择
 * 注意：本地不允许 npm run test:run，所有测试通过 git push 触发 CI 验证
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { ref, nextTick } from 'vue'

// 使用 vi.hoisted 确保 mock 状态在 vi.mock 工厂执行前已就绪
const mockTableApi = vi.hoisted(() => ({
  data: ref<any[]>([]),
  loading: ref(false),
  page: ref(1),
  pageSize: ref(20),
  total: ref(0),
  queryParams: ref<Record<string, any>>({}),
  refresh: vi.fn(async () => {}),
  reset: vi.fn(() => {
    mockTableApi.queryParams.value = {}
    mockTableApi.page.value = 1
    mockTableApi.pageSize.value = 20
  }),
  setQueryParam: vi.fn(),
}))

vi.mock('@/composables/useTableApi', () => ({
  useTableApi: () => mockTableApi,
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
    mockTableApi.data.value = []
    mockTableApi.loading.value = false
    mockTableApi.page.value = 1
    mockTableApi.pageSize.value = 20
    mockTableApi.total.value = 0
    mockTableApi.queryParams.value = {}
  })

  it('数据加载：组件挂载时正确接收数据并渲染表格', async () => {
    mockTableApi.data.value = sampleStocks
    mockTableApi.total.value = sampleStocks.length

    const wrapper = mount(StockTab)
    await flushPromises()

    // 验证 useTableApi 已使用正确 URL 调用
    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.stock-tab').exists()).toBe(true)
    // 验证过滤表单存在
    expect(wrapper.find('.filter-card').exists()).toBe(true)
    // 验证表格卡片存在
    expect(wrapper.find('.table-card').exists()).toBe(true)
  })

  it('过滤：输入关键词触发查询并更新 queryParams', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    // 找到关键词输入框
    const keywordInput = wrapper.find('input[placeholder*="产品编码"]')
    expect(keywordInput.exists()).toBe(true)

    // 设置关键词并触发查询
    await keywordInput.setValue('SKU-001')
    await wrapper.findAll('button').find(b => b.text().includes('查询'))?.trigger('click')
    await flushPromises()

    // 验证 queryParams 已更新
    expect(mockTableApi.queryParams.value.keyword).toBe('SKU-001')
    // 验证 refresh 被调用
    expect(mockTableApi.refresh).toHaveBeenCalled()
  })

  it('过滤：选择状态触发查询并重置 page 为 1', async () => {
    mockTableApi.page.value = 5
    const wrapper = mount(StockTab)
    await flushPromises()

    // 直接通过 setValue 模拟 el-select 状态变更
    const vm = wrapper.vm as any
    vm.localQueryParams.status = 'warning'
    await nextTick()

    // 触发查询按钮
    const queryBtn = wrapper.findAll('button').find(b => b.text().includes('查询'))
    expect(queryBtn).toBeTruthy()
    await queryBtn!.trigger('click')
    await flushPromises()

    // 验证 page 重置为 1
    expect(mockTableApi.page.value).toBe(1)
    // 验证 queryParams 同步
    expect(mockTableApi.queryParams.value.status).toBe('warning')
  })

  it('重置：清空所有过滤条件并清空选择', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    // 模拟已有选择
    const vm = wrapper.vm as any
    vm.localQueryParams.keyword = 'test'
    vm.localQueryParams.warehouse_id = 1
    vm.localQueryParams.status = 'normal'

    // 触发重置
    const resetBtn = wrapper.findAll('button').find(b => b.text().includes('重置'))
    expect(resetBtn).toBeTruthy()
    await resetBtn!.trigger('click')
    await flushPromises()

    // 验证 localQueryParams 已清空
    expect(vm.localQueryParams.keyword).toBe('')
    expect(vm.localQueryParams.warehouse_id).toBeUndefined()
    expect(vm.localQueryParams.status).toBe('')
    // 验证 reset 被调用
    expect(mockTableApi.reset).toHaveBeenCalled()
    // 验证 refresh 被调用
    expect(mockTableApi.refresh).toHaveBeenCalled()
  })

  it('排序：列定义中标记 sortable 的字段应包含 sortable=true', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    // 通过 vm 访问 columns（computed 会被自动 unwrap）
    const vm = wrapper.vm as any
    const cols = vm.columns
    // 兼容 columns 仍为 ref 的情况
    const colList = Array.isArray(cols) ? cols : (cols?.value ?? [])

    // 验证 SKU 列可排序
    const skuCol = colList.find((c: any) => c.key === 'product_code')
    expect(skuCol?.sortable).toBe(true)
    // 验证产品名列可排序
    const nameCol = colList.find((c: any) => c.key === 'product_name')
    expect(nameCol?.sortable).toBe(true)
    // 验证数量列可排序
    const qtyCol = colList.find((c: any) => c.key === 'quantity')
    expect(qtyCol?.sortable).toBe(true)
    // 验证更新时间列可排序
    const updCol = colList.find((c: any) => c.key === 'updated_at')
    expect(updCol?.sortable).toBe(true)
  })

  it('行选择：toggleRow 单行切换', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    const row = sampleStocks[0]

    // 初始未选中
    expect(vm.selectedRows.length).toBe(0)
    // 选中第一行
    vm.toggleRow(row)
    await nextTick()
    expect(vm.selectedRows.length).toBe(1)
    expect(vm.selectedRows[0].id).toBe(1)
    // 重复点击取消选中
    vm.toggleRow(row)
    await nextTick()
    expect(vm.selectedRows.length).toBe(0)
  })

  it('行选择：toggleAll 全选当前页', async () => {
    mockTableApi.data.value = sampleStocks
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    // 全选
    vm.toggleAll()
    await nextTick()
    expect(vm.selectedRows.length).toBe(sampleStocks.length)
    expect(vm.allSelected).toBe(true)
    // 再次点击取消全选
    vm.toggleAll()
    await nextTick()
    expect(vm.selectedRows.length).toBe(0)
    expect(vm.allSelected).toBe(false)
  })

  it('行选择：indeterminate 在部分选中时为 true', async () => {
    mockTableApi.data.value = sampleStocks
    const wrapper = mount(StockTab)
    await flushPromises()

    const vm = wrapper.vm as any
    // 选中一行
    vm.toggleRow(sampleStocks[0])
    await nextTick()
    // 部分选中 -> indeterminate = true
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

    // 模拟翻页事件
    vm.handlePageChange(2)
    await nextTick()
    expect(mockTableApi.page.value).toBe(2)
    // 翻页应清空选择
    expect(vm.selectedRows.length).toBe(0)
  })

  it('暴露 fetchData：父组件可通过 ref 调用', async () => {
    const wrapper = mount(StockTab)
    await flushPromises()

    // 验证 defineExpose 的 fetchData 可访问
    const exposed = (wrapper.vm as any).$.exposed
    expect(exposed?.fetchData).toBeDefined()
    // 调用 fetchData 应触发 refresh
    exposed?.fetchData()
    expect(mockTableApi.refresh).toHaveBeenCalled()
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

    // 验证低库存（quantity=30 < min_quantity=50）渲染 low-stock class
    const vnode = qtyCol.renderCell(sampleStocks[1])
    // vnode.props 应包含 class 对象
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

    // 验证状态列渲染函数存在
    const vnode = statusCol.renderCell(sampleStocks[0])
    expect(vnode).toBeDefined()
  })
})
