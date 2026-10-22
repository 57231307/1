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

// mock @/api/request（useTableApi 内部调用 request.get；
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

// V2Table 渲染桩：按传入 columns 渲染表头文案，按 data 逐格渲染该列取值
// （formatter 与 renderCell 的产物都收敛为文本，未配置两者的列按该列 key 对应的行字段取值）。
// 此前的桩是空 div，既不渲染表头也不渲染单元格，任何列绑定错误都不会让断言失败，
// 等于把本页新加的资源名列与其余全部列的渲染验证整体关掉，故改为可承载断言的桩。
vi.mock('@/components/V2Table/index.vue', () => ({
  default: {
    name: 'V2Table',
    props: ['columns', 'data', 'loading', 'page', 'pageSize', 'total', 'rowKey', 'emptyText'],
    emits: ['page-change', 'size-change', 'row-click'],
    computed: {
      visibleColumns(): unknown[] {
        return (this.columns || []).filter((col: { hidden?: boolean }) => !col.hidden);
      },
    },
    methods: {
      // renderCell 返回组件 vnode，其文案藏在插槽函数里，需调用后才落到文本
      nodeText(node: unknown): string {
        if (node === null || node === undefined || node === false) return '';
        if (Array.isArray(node)) return node.map(item => this.nodeText(item)).join('');
        if (typeof node === 'function') return this.nodeText((node as () => unknown)());
        if (typeof node === 'object') {
          const obj = node as { default?: () => unknown; children?: unknown };
          if (typeof obj.default === 'function') return this.nodeText(obj.default());
          if ('children' in obj) return this.nodeText(obj.children);
          return '';
        }
        return String(node);
      },
      cellText(
        col: {
          key: string;
          formatter?: (row: unknown) => unknown;
          renderCell?: (row: unknown) => unknown;
        },
        row: Record<string, unknown>
      ): string {
        if (typeof col.formatter === 'function') return this.nodeText(col.formatter(row));
        if (typeof col.renderCell === 'function') return this.nodeText(col.renderCell(row));
        const value = row == null ? '' : row[col.key];
        return value === null || value === undefined ? '' : String(value);
      },
    },
    template: `
      <div class="v2-table-mock">
        <div class="v2-table-mock__head">
          <span
            v-for="col in visibleColumns"
            :key="col.key"
            class="v2-table-mock__head-cell"
            :data-col="col.key"
            >{{ col.title }}</span
          >
        </div>
        <div v-if="!data || data.length === 0" class="v2-table-mock__empty">{{ emptyText }}</div>
        <div
          v-for="(row, rowIndex) in data"
          :key="rowIndex"
          class="v2-table-mock__row"
          @click="$emit('row-click', row)"
        >
          <span
            v-for="col in visibleColumns"
            :key="col.key"
            class="v2-table-mock__cell"
            :data-col="col.key"
            >{{ cellText(col, row) }}</span
          >
        </div>
      </div>
    `,
  },
}));

import AuditLogView from '@/views/system/audit-log/index.vue';

// 详情抽屉及其内容组件在本测试环境未全局注册（未解析的组件不会渲染插槽内容，
// 抽屉里的字段值与快照 JSON 因此拿不到文本）。这里按需注册透传桩，
// 使详情断言落在真实渲染文本上，而不是只断组件实例状态。
const detailStubs = {
  'el-drawer': {
    name: 'ElDrawer',
    props: ['modelValue', 'title', 'size', 'direction', 'destroyOnClose'],
    template: '<div class="el-drawer-mock"><slot /></div>',
  },
  'el-descriptions': {
    name: 'ElDescriptions',
    props: ['column', 'border'],
    template: '<div class="el-descriptions-mock"><slot /></div>',
  },
  'el-descriptions-item': {
    name: 'ElDescriptionsItem',
    props: ['label', 'span'],
    template:
      '<div class="desc-item" :data-label="label"><span class="desc-item__label">{{ label }}</span><span class="desc-item__value"><slot /></span></div>',
  },
  'el-tag': {
    name: 'ElTag',
    props: ['type', 'size'],
    template: '<span class="el-tag-mock"><slot /></span>',
  },
  'el-text': {
    name: 'ElText',
    props: ['lineClamp'],
    template: '<span class="el-text-mock"><slot /></span>',
  },
};

function mountAuditLog() {
  return mount(AuditLogView, { global: { stubs: detailStubs } });
}

/** 表头文案清单（按渲染顺序，桩已过滤 hidden 列） */
function headerTexts(wrapper: ReturnType<typeof mountAuditLog>): string[] {
  return wrapper
    .findAll('.v2-table-mock__head-cell')
    .map(node => node.text().trim())
    .filter(text => text.length > 0);
}

/** 按列 key 取指定数据行的实际渲染文本 */
function cellText(
  wrapper: ReturnType<typeof mountAuditLog>,
  rowIndex: number,
  colKey: string
): string {
  const rows = wrapper.findAll('.v2-table-mock__row');
  if (rows.length <= rowIndex) {
    throw new Error(`表格只渲染出 ${rows.length} 行，取不到第 ${rowIndex + 1} 行`);
  }
  const cell = rows[rowIndex].find(`.v2-table-mock__cell[data-col="${colKey}"]`);
  if (!cell.exists()) {
    throw new Error(
      `第 ${rowIndex + 1} 行未渲染列 ${colKey}，实际表头=${JSON.stringify(headerTexts(wrapper))}`
    );
  }
  return cell.text().trim();
}

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
    const wrapper = mountAuditLog();
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
    const wrapper = mountAuditLog();
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

  /** 点击详情按钮：调用 getAuditLog 并把 before/after 快照真实渲染进抽屉文本 */
  it('点击详情按钮调用 getAuditLog 并展示抽屉内容', async () => {
    const wrapper = mountAuditLog();
    await flushPromises();

    const vm = wrapper.vm as any;
    await vm.handleViewDetail(sampleLogs[0]);
    await flushPromises();

    expect(mockGetAuditLog).toHaveBeenCalledWith(1);
    expect(vm.currentDetail).toBeTruthy();
    expect(vm.currentDetail.before_snapshot).toEqual({ amount: 100 });
    expect(vm.detailVisible).toBe(true);

    // 抽屉内两段快照必须把 JSON 内容渲染成可读文本，只断 currentDetail 状态不等于页面上看得见
    const blocks = wrapper.findAll('.snapshot-block');
    expect(blocks).toHaveLength(2);
    expect(blocks[0].text()).toContain('"amount": 100');
    expect(blocks[1].text()).toContain('"amount": 200');
    // 快照小标题与详情字段同样按实际文案出值
    const titles = wrapper.findAll('.snapshot-title').map(node => node.text());
    expect(titles[0]).toContain('变更前快照');
    expect(titles[1]).toContain('变更后快照');
    expect(wrapper.find('.desc-item[data-label="资源名称"] .desc-item__value').text()).toBe(
      'alice'
    );
  });

  /** 行点击：桩发出的 row-click 走页面 handleRowClick，按该行主键拉起详情 */
  it('点击表格行按该行主键拉取详情并打开抽屉', async () => {
    const wrapper = mountAuditLog();
    await flushPromises();
    mockGetAuditLog.mockClear();

    await wrapper.findAll('.v2-table-mock__row')[1].trigger('click');
    await flushPromises();

    // 第二行的主键是 2（行数据 id），点击错行或整行未接上事件都会在这里暴露
    expect(mockGetAuditLog).toHaveBeenCalledWith(2);
    expect((wrapper.vm as any).detailVisible).toBe(true);
    expect(wrapper.findAll('.snapshot-block')[0].text()).toContain('"amount": 100');
  });

  /** 表头文案与逐格实际出值：列绑定错（含绑到后端不返回的键）会渲染成空文本或错值 */
  it('表格逐列渲染表头文案与行数据实际出值', async () => {
    const wrapper = mountAuditLog();
    await flushPromises();

    const headers = headerTexts(wrapper);
    // 本页新加的资源名列与既有列一同进入表头，缺一即列定义被改坏
    for (const title of [
      'ID',
      '操作时间',
      '操作类型',
      '级别',
      '操作人',
      '资源类型',
      '资源 ID',
      '资源名称',
      '客户端 IP',
      '请求追踪',
      '描述',
      '操作',
    ]) {
      expect(headers, `表头缺少列 ${title}，实际表头=${JSON.stringify(headers)}`).toContain(title);
    }

    // 逐列断第一行的实际渲染文本：formatter 与 renderCell 的产物都必须是文案而非空白
    expect(cellText(wrapper, 0, 'id')).toBe('1');
    expect(cellText(wrapper, 0, 'operation_type')).toBe('登录');
    expect(cellText(wrapper, 0, 'severity')).toBe('信息');
    expect(cellText(wrapper, 0, 'username')).toBe('alice');
    expect(cellText(wrapper, 0, 'resource_type')).toBe('auth');
    expect(cellText(wrapper, 0, 'resource_id')).toBe('7');
    expect(cellText(wrapper, 0, 'ip_address')).toBe('10.0.0.1');
    expect(cellText(wrapper, 0, 'request_id')).toBe('trace-1');
    expect(cellText(wrapper, 0, 'description')).toBe('用户登录成功');
    expect(cellText(wrapper, 0, 'actions')).toBe('详情');
    // 操作时间列走页面的 formatDateTime，输出随运行时区变化，故断格式化后的日期时间形态而非绝对时刻
    expect(cellText(wrapper, 0, 'created_at')).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);

    // 第二行换值，排除整列取首行或写死文案
    expect(cellText(wrapper, 1, 'operation_type')).toBe('更新');
    expect(cellText(wrapper, 1, 'severity')).toBe('警告');
    expect(cellText(wrapper, 1, 'resource_type')).toBe('user');
    expect(cellText(wrapper, 1, 'description')).toBe('修改密码失败');
  });

  /** 资源名列取的是行数据 resource_name，不得与 resource_id 混绑 */
  it('资源名列渲染行数据的 resource_name 而非 resource_id', async () => {
    const wrapper = mountAuditLog();
    await flushPromises();

    // 夹具里两行的 resource_name 与 resource_id 不同值（alice 对 7，bob 对 8），
    // 绑错键时下面两条断言必然一红，不接受只断列对象里存在该 key
    expect(cellText(wrapper, 0, 'resource_name')).toBe('alice');
    expect(cellText(wrapper, 1, 'resource_name')).toBe('bob');
    expect(cellText(wrapper, 0, 'resource_id')).toBe('7');
    expect(cellText(wrapper, 1, 'resource_id')).toBe('8');
  });

  /** 点击导出按钮：敏感导出改走审批门，应 POST /export-approvals 提交申请 */
  it('点击导出按钮提交导出审批申请而非直连下载', async () => {
    const wrapper = mountAuditLog();
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
    const wrapper = mountAuditLog();
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
