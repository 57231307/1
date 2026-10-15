/**
 * AuditLog 视图单元测试（P13 批 1 P3-2）
 * 覆盖：筛选交互 / 详情打开 / 导出触发 / 表格分页
 * 注意：本地不允许 npm run test:run，所有测试通过 git push 触发 CI 验证
 *
 * 批次 267：view 接入 useTableApi 后，mock 从 @/api/audit 改为 @/api/request
 * （useTableApi 内部调用 request.get(url, {params})）
 */
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { nextTick } from 'vue';

// i18n 由 tests/setup.ts 全局注入（config.global.plugins）

// 模块级 refs：跨测试共享 mock 状态
const mockRequestGet = vi.fn();
const mockRequestPost = vi.fn();
const mockGetAuditLog = vi.fn();

// 批次 267：mock @/api/request（useTableApi 内部调用 request.get；
// 敏感导出改走审批申请后，导出按钮经 createApproval 调用 request.post）
// 返回 ApiResponse 包装结构 { code, message, data: { items, total } }
vi.mock('@/api/request', () => ({
  request: {
    get: (...args: unknown[]) => mockRequestGet(...args),
    post: (...args: unknown[]) => mockRequestPost(...args),
  },
}));

// 保留 getAuditLog mock（详情接口仍由 @/api/audit 提供）
vi.mock('@/api/audit', () => ({
  getAuditLog: (...args: unknown[]) => mockGetAuditLog(...args),
}));

// 局部 mock element-plus：保留真实 export（避免 ElMessage 等子组件的运行时错误）
vi.mock('element-plus', async importOriginal => {
  const actual = await importOriginal<typeof import('element-plus')>();
  return { ...actual };
});

// mock V2Table：避免依赖 el-table-v2 / ElAutoResizer 真实渲染
vi.mock('@/components/V2Table/index.vue', () => ({
  default: {
    name: 'V2Table',
    props: ['columns', 'data', 'loading', 'page', 'pageSize', 'total', 'rowKey', 'emptyText'],
    emits: ['page-change', 'size-change', 'row-click'],
    template: '<div class="v2-table-mock" @click="$emit(\'row-click\', data[0])" />',
  },
}));

import AuditLogView from '@/views/system/audit-log/index.vue';

const sampleLogs = [
  {
    id: 1,
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
];

describe('AuditLogView（P13 批 1 P3-2）', () => {
  beforeEach(() => {
    mockRequestGet.mockReset();
    mockRequestPost.mockReset();
    mockGetAuditLog.mockReset();

    // 批次 267：mock request.get 返回 ApiResponse 包装结构
    mockRequestGet.mockResolvedValue({
      code: 200,
      message: 'success',
      data: {
        items: sampleLogs,
        total: 2,
        page: 1,
        page_size: 20,
      },
    });
    mockGetAuditLog.mockResolvedValue({
      ...sampleLogs[0],
      before_snapshot: { amount: 100 },
      after_snapshot: { amount: 200 },
    });
    // createApproval（request.post）成功信封：新提交落库 pending、无令牌
    mockRequestPost.mockResolvedValue({
      code: 200,
      message: 'success',
      data: { id: 99, status: 'pending', download_token: null },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  /** 挂载即触发首屏加载，参数包含 page/page_size */
  it('挂载时自动加载首屏数据并发送分页参数', async () => {
    const wrapper = mount(AuditLogView, {
      global: {},
    });
    await flushPromises();
    expect(mockRequestGet).toHaveBeenCalledTimes(1);
    // request.get(url, { params }) 第二参数的 params 含 page/page_size
    const params = mockRequestGet.mock.calls[0][1].params;
    expect(params.page).toBe(1);
    expect(params.page_size).toBe(20);
    expect(wrapper.find('.audit-log-view').exists()).toBe(true);
  });

  /** 点击查询按钮：将筛选条件传入 API 并回到第一页 */
  it('点击查询按钮会把筛选条件传入 API', async () => {
    const wrapper = mount(AuditLogView, {
      global: {},
    });
    await flushPromises();
    mockRequestGet.mockClear();

    // 模拟设置筛选值
    const vm = wrapper.vm as any;
    vm.filterForm.operation_type = 'UPDATE';
    vm.filterForm.severity = 'WARN';
    vm.filterForm.resource_type = 'user';
    vm.filterForm.request_id = 'trace-2';
    vm.filterForm.keyword = '密码';
    await nextTick();

    // 触发查询
    await vm.handleQuery();
    await flushPromises();

    expect(mockRequestGet).toHaveBeenCalledTimes(1);
    const params = mockRequestGet.mock.calls[0][1].params;
    expect(params.operation_type).toBe('UPDATE');
    expect(params.severity).toBe('WARN');
    expect(params.resource_type).toBe('user');
    expect(params.request_id).toBe('trace-2');
    expect(params.keyword).toBe('密码');
  });

  /** 点击详情按钮：调用 getAuditLog 并打开抽屉 */
  it('点击详情按钮调用 getAuditLog 并展示抽屉内容', async () => {
    const wrapper = mount(AuditLogView, {
      global: {},
    });
    await flushPromises();

    const vm = wrapper.vm as any;
    await vm.handleViewDetail(sampleLogs[0]);
    await flushPromises();

    expect(mockGetAuditLog).toHaveBeenCalledWith(1);
    expect(vm.currentDetail).toBeTruthy();
    expect(vm.currentDetail.before_snapshot).toEqual({ amount: 100 });
    expect(vm.detailVisible).toBe(true);
  });

  /** 点击导出按钮：敏感导出改走审批门，应 POST /export-approvals 提交申请 */
  it('点击导出按钮提交导出审批申请而非直连下载', async () => {
    const wrapper = mount(AuditLogView, {
      global: {},
    });
    await flushPromises();
    mockRequestPost.mockClear();

    const vm = wrapper.vm as any;
    await vm.handleExport();
    await flushPromises();

    expect(mockRequestPost).toHaveBeenCalledTimes(1);
    const [url, body] = mockRequestPost.mock.calls[0];
    expect(url).toBe('/export-approvals');
    // 申请载荷：资源类型钉死 audit_log，成品格式 xlsx（后端仅接受 xlsx/pdf），
    // export_params 为注册表白名单收敛后的筛选对象
    expect(body).toMatchObject({
      resource_type: 'audit_log',
      file_format: 'xlsx',
      export_params: expect.any(Object),
    });
  });

  /** 分页变化：将 page 传给 API */
  it('分页变化时把 page / page_size 传给 API', async () => {
    const wrapper = mount(AuditLogView, {
      global: {},
    });
    await flushPromises();
    mockRequestGet.mockClear();

    const vm = wrapper.vm as any;
    await vm.handlePageChange(3);
    await flushPromises();
    expect(mockRequestGet.mock.calls[0][1].params.page).toBe(3);

    await vm.handleSizeChange(50);
    await flushPromises();
    expect(mockRequestGet.mock.calls[1][1].params.page_size).toBe(50);
    expect(mockRequestGet.mock.calls[1][1].params.page).toBe(1);
  });
});
