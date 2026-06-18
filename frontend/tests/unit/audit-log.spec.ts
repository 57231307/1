/**
 * AuditLog 视图单元测试（P13 批 1 P3-2）
 * 覆盖：筛选交互 / 详情打开 / 导出触发 / 表格分页
 * 注意：本地不允许 npm run test:run，所有测试通过 git push 触发 CI 验证
 */
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick, ref } from 'vue'

// 模块级 refs：跨测试共享 mock 状态
const mockListAuditLogs = vi.fn()
const mockGetAuditLog = vi.fn()
const mockExportAuditLogs = vi.fn()

vi.mock('@/api/audit', () => ({
  listAuditLogs: (...args: unknown[]) => mockListAuditLogs(...args),
  getAuditLog: (...args: unknown[]) => mockGetAuditLog(...args),
  exportAuditLogs: (...args: unknown[]) => mockExportAuditLogs(...args),
}))

// 局部 mock element-plus：保留真实 export（避免 ElMessage 等子组件的运行时错误）
vi.mock('element-plus', async (importOriginal) => {
  const actual = await importOriginal<typeof import('element-plus')>()
  return { ...actual }
})

// mock V2Table：避免依赖 el-table-v2 / ElAutoResizer 真实渲染
vi.mock('@/components/V2Table/index.vue', () => ({
  default: {
    name: 'V2Table',
    props: ['columns', 'data', 'loading', 'page', 'pageSize', 'total', 'rowKey', 'emptyText'],
    emits: ['page-change', 'size-change', 'row-click'],
    template: '<div class="v2-table-mock" @click="$emit(\'row-click\', data[0])" />',
  },
}))

import AuditLogView from '@/views/system/audit-log/index.vue'

const sampleLogs = [
  {
    id: 1,
    tenant_id: 1,
    user_id: 7,
    username: 'alice',
    operation_type: 'LOGIN',
    severity: 'INFO',
    resource_type: 'auth',
    resource_id: '7',
    resource_name: 'alice',
    description: '用户登录成功',
    ip_address: '10.0.0.1',
    user_agent: 'Mozilla/5.0',
    request_id: 'trace-1',
    request_method: 'POST',
    request_path: '/api/v1/erp/auth/login',
    created_at: '2026-06-18T08:00:00Z',
  },
  {
    id: 2,
    tenant_id: 1,
    user_id: 8,
    username: 'bob',
    operation_type: 'UPDATE',
    severity: 'WARN',
    resource_type: 'user',
    resource_id: '8',
    resource_name: 'bob',
    description: '修改密码失败',
    ip_address: '10.0.0.2',
    user_agent: 'Mozilla/5.0',
    request_id: 'trace-2',
    request_method: 'PUT',
    request_path: '/api/v1/erp/users/change-password',
    created_at: '2026-06-18T08:05:00Z',
  },
]

describe('AuditLogView（P13 批 1 P3-2）', () => {
  beforeEach(() => {
    mockListAuditLogs.mockReset()
    mockGetAuditLog.mockReset()
    mockExportAuditLogs.mockReset()

    mockListAuditLogs.mockResolvedValue({
      items: sampleLogs,
      total: 2,
      page: 1,
      page_size: 20,
    } as any)
    mockGetAuditLog.mockResolvedValue({
      ...sampleLogs[0],
      before_snapshot: { amount: 100 },
      after_snapshot: { amount: 200 },
    })
    mockExportAuditLogs.mockResolvedValue(new Blob(['id,name\n1,test'], { type: 'text/csv' }))
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  /** 挂载即触发首屏加载，参数包含 page/page_size */
  it('挂载时自动加载首屏数据并发送分页参数', async () => {
    const wrapper = mount(AuditLogView)
    await flushPromises()
    expect(mockListAuditLogs).toHaveBeenCalledTimes(1)
    const args = mockListAuditLogs.mock.calls[0][0]
    expect(args.page).toBe(1)
    expect(args.page_size).toBe(20)
    expect(wrapper.find('.audit-log-view').exists()).toBe(true)
  })

  /** 点击查询按钮：将筛选条件传入 API 并回到第一页 */
  it('点击查询按钮会把筛选条件传入 API', async () => {
    const wrapper = mount(AuditLogView)
    await flushPromises()
    mockListAuditLogs.mockClear()

    // 模拟设置筛选值
    const vm = wrapper.vm as any
    vm.filterForm.operation_type = 'UPDATE'
    vm.filterForm.severity = 'WARN'
    vm.filterForm.resource_type = 'user'
    vm.filterForm.request_id = 'trace-2'
    vm.filterForm.keyword = '密码'
    await nextTick()

    // 触发查询
    await vm.handleQuery()
    await flushPromises()

    expect(mockListAuditLogs).toHaveBeenCalledTimes(1)
    const params = mockListAuditLogs.mock.calls[0][0]
    expect(params.operation_type).toBe('UPDATE')
    expect(params.severity).toBe('WARN')
    expect(params.resource_type).toBe('user')
    expect(params.request_id).toBe('trace-2')
    expect(params.keyword).toBe('密码')
  })

  /** 点击详情按钮：调用 getAuditLog 并打开抽屉 */
  it('点击详情按钮调用 getAuditLog 并展示抽屉内容', async () => {
    const wrapper = mount(AuditLogView)
    await flushPromises()

    const vm = wrapper.vm as any
    await vm.handleViewDetail(sampleLogs[0])
    await flushPromises()

    expect(mockGetAuditLog).toHaveBeenCalledWith(1)
    expect(vm.currentDetail).toBeTruthy()
    expect(vm.currentDetail.before_snapshot).toEqual({ amount: 100 })
    expect(vm.detailVisible).toBe(true)
  })

  /** 点击导出按钮：调用 exportAuditLogs 并触发浏览器下载 */
  it('点击导出按钮调用 exportAuditLogs 并触发下载', async () => {
    // mock DOM 操作
    const createUrlSpy = vi.fn().mockReturnValue('blob:mock-url')
    const revokeUrlSpy = vi.fn()
    const clickSpy = vi.fn()
    const originalCreate = URL.createObjectURL
    const originalRevoke = URL.revokeObjectURL
    URL.createObjectURL = createUrlSpy as any
    URL.revokeObjectURL = revokeUrlSpy as any

    // mock 临时 a 元素的 click
    const origCreateElement = document.createElement.bind(document)
    const createElementSpy = vi.spyOn(document, 'createElement').mockImplementation(((tag: string) => {
      const el = origCreateElement(tag)
      if (tag === 'a') {
        el.click = clickSpy
      }
      return el
    }) as any)

    const wrapper = mount(AuditLogView)
    await flushPromises()

    const vm = wrapper.vm as any
    await vm.handleExport()
    await flushPromises()

    expect(mockExportAuditLogs).toHaveBeenCalledTimes(1)
    expect(createUrlSpy).toHaveBeenCalled()
    expect(clickSpy).toHaveBeenCalled()
    expect(revokeUrlSpy).toHaveBeenCalled()

    // 清理 mock
    URL.createObjectURL = originalCreate
    URL.revokeObjectURL = originalRevoke
    createElementSpy.mockRestore()
  })

  /** 分页变化：将 page 传给 API */
  it('分页变化时把 page / page_size 传给 API', async () => {
    const wrapper = mount(AuditLogView)
    await flushPromises()
    mockListAuditLogs.mockClear()

    const vm = wrapper.vm as any
    await vm.handlePageChange(3)
    await flushPromises()
    expect(mockListAuditLogs.mock.calls[0][0].page).toBe(3)

    await vm.handleSizeChange(50)
    await flushPromises()
    expect(mockListAuditLogs.mock.calls[1][0].page_size).toBe(50)
    expect(mockListAuditLogs.mock.calls[1][0].page).toBe(1)
  })
})
