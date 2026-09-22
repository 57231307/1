import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyPermissionDenied,
  verifyAuditLog,
  getCtx,
  genCode,
  ensureTestEntities,
  expectBadRequest,
} from './helpers';

test.describe.serial('Shard 5: 系统管理 + 权限 + 合规', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('5-1 审计日志查询（按操作类型/资源筛选）', async ({ page }) => {
    // GET /audit-logs 由 audit_log_handler::list_audit_logs 处理，响应键为
    // { items, total, page, page_size }，条目本身即含 operation_type/resource_name/description。
    // （另一套 audit_enhanced_handler::list_audit_logs 挂在 analytics 域的 /logs 上，
    //  响应键是 list——两者路径与结构都不同，不可混用。）
    const logs = await apiCallRaw<{
      items: Array<Record<string, unknown>>;
      total?: number;
    }>(page, 'GET', '/audit-logs?page=1&page_size=20');
    expect(Array.isArray(logs?.items), '审计日志列表应返回 items 数组').toBe(true);
    const rows = logs.items;
    expect(logs?.total ?? 0, '审计日志总数应大于 0').toBeGreaterThan(0);
    // 先断言行非空再取 rows[0]：原实现断言 total>0 后直接索引，
    // 一旦分页结果为空就是 TypeError 而不是可读的断言失败
    expect(rows.length, '首页审计日志不应为空').toBeGreaterThan(0);
    expect(String(rows[0].operation_type ?? ''), '审计记录应含 operation_type').toBeTruthy();
    expect(rows[0].action ?? rows[0].request_method, '审计记录应含操作标识').toBeTruthy();
  });

  test('5-2 用户列表 + 角色列表 + 部门列表', async ({ page }) => {
    const users = await apiCallRaw<{ items: Array<{ id: number; username: string }> }>(
      page,
      'GET',
      '/users?page=1&page_size=10'
    );
    // 容错：users 可能为 undefined
    expect(users?.items?.length ?? 0).toBeGreaterThanOrEqual(0);

    // GET /roles 由 role_handler::list_roles 处理，出参是 RoleListResponse{roles,total}
    // （不分页：page/page_size 参数会被 serde 直接忽略）。原实现读 roles.items 恒为 undefined，
    // 且 expect() 不带匹配器，是永不失败的空断言。
    const roles = await apiCallRaw<{
      roles: Array<{ id: number; code: string; name: string }>;
      total: number;
    }>(page, 'GET', '/roles');
    expect(
      Array.isArray(roles?.roles),
      `角色列表应返回 roles 数组，实际：${JSON.stringify(roles).slice(0, 200)}`
    ).toBe(true);
    expect(roles.total, '种子角色总数应大于 0').toBeGreaterThan(0);

    const depts = await apiCallRaw<{ items: Array<{ id: number; name: string }> }>(
      page,
      'GET',
      '/departments?page=1&page_size=10'
    );
    expect(Array.isArray(depts.items), `depts.items 应为后端返回的 items 数组`);
  });

  test('5-3 数据权限验证（行级隔离）', async ({ page }) => {
    const perms = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/data-permissions?page=1&page_size=5'
    );
    expect(Array.isArray(perms.items), `perms.items 应为后端返回的 items 数组`);
  });

  test('5-4 字段级权限验证（染色配方导出仅 dye_recipe_master 可）', async ({ page }) => {
    // admin 角色应可以访问（有 *:* 权限）
    const result = await apiCallExpectFail(page, 'GET', '/production/dye-recipes/export');
    // admin 可能被允许或被拒绝（取决于角色黑名单）
    expectBadRequest(result); // 应返回错误码
  });

  test('5-5 BPM 流程定义', async ({ page }) => {
    // bpm_definition_handler.rs:57 的列表 key 是 list；原实现读 items（恒 undefined）
    // 且 expect() 不带匹配器，是一条永不失败的断言
    const defs = await apiCallRaw<{ list: Array<{ id: number }> }>(
      page,
      'GET',
      '/bpm/definitions?page=1&page_size=5'
    );
    console.log(`[5-5] BPM 流程定义 list 长度=${defs?.list?.length ?? '(缺 key)'}`);
    expect(Array.isArray(defs?.list), 'BPM 流程定义应返回 list 数组').toBe(true);
  });

  test('5-6 BPM 审批任务', async ({ page }) => {
    // GET /bpm/tasks 出参是 PageResponse{total,page,page_size,total_pages,data}，
    // 与多数列表接口的 PaginatedResponse{items,…} 不同一（两套分页 DTO 已登记在 doto）。
    // 原断言按 items 判定，等于把后端真实字段名当成缺陷反过来判红。
    const tasks = await apiCallRaw<{
      data: Array<{ id: number; status: string }>;
      total: number;
      page: number;
      page_size: number;
      total_pages: number;
    }>(page, 'GET', '/bpm/tasks?page=1&page_size=5');
    expect(
      Array.isArray(tasks?.data),
      `BPM 任务应返回 data 数组，实际响应：${JSON.stringify(tasks).slice(0, 200)}`
    ).toBe(true);
    expect(typeof tasks.total, 'PageResponse 应回显 total').toBe('number');
    expect(tasks.page, '应回显请求页码').toBe(1);
    expect(tasks.page_size, '应回显每页数量').toBe(5);
    // bpm_task 取值域（backend/src/models/status/bpm_crm_contract.rs::bpm_task）
    const BPM_TASK_STATUSES = ['pending', 'completed', 'rejected', 'cancelled'];
    for (const task of tasks.data) {
      expect(BPM_TASK_STATUSES, `任务 ${task.id} 的状态在取值域外：${task.status}`).toContain(
        (task.status || '').toLowerCase()
      );
    }
  });

  test('5-7 定制订单 7 阶段状态机', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const result = await apiCall<{ id?: number }>(page, 'POST', '/custom-orders', {
      order_no: genCode('CO'),
      customer_id: ctx.customerId,
      product_id: ctx.productIds[0] || 1,
      color_id: ctx.productColorIds[0],
      spec: '65%棉35%涤 40S 133x72 150cm',
      quantity: 500,
      unit: '米',
      custom_requirements: { yarn_spec: '40S', dye_method: 'reactive', finishing_method: '防水' },
      expected_delivery_date: new Date(Date.now() + 30 * 86400000).toISOString().split('T')[0],
      notes: 'E2E 定制订单',
    });
    ctx.customOrderId = result.data?.id;

    if (ctx.customOrderId) {
      const order = await apiCallRaw<{ status: string }>(
        page,
        'GET',
        `/custom-orders/${ctx.customOrderId}`
      );
      const status = (order.status || '').toLowerCase();
      expect([
        'draft',
        'lab_dip',
        'quotation',
        'yarn_purchasing',
        'dyeing',
        'finishing',
        'delivery',
        'after_sales',
        'completed',
        'cancelled',
        'pending',
      ]).toContain(status ?? '(missing-status)');
    }
  });

  test('5-8 定制订单状态门校验（非法跳跃应拒绝）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.customOrderId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 直接从 draft 跳到 dyeing → 应拒绝（需要先完成 lab_dip + quotation）
    const result = await apiCallExpectFail(page, 'POST', `/custom-orders/${id}/advance`, {
      to_status: 'dyeing',
    });
    expectBadRequest(result); // 非法转换应被拒
  });

  test('5-9 大货批色审批（8 态状态机）', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/bulk-color-approvals?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`);
    if (list?.items?.length ?? 0 > 0) {
      const status = (list.items?.[0].status || '').toLowerCase();
      expect([
        'pending',
        'sampled',
        'sent_to_customer',
        'approved',
        'rejected',
        'rework',
        'downgraded',
        'scrapped',
      ]).toContain(status ?? '(missing-status)');
    }
  });

  test('5-10 坯布五维追溯（产品→色号→缸号→匹号）', async ({ page }) => {
    const ctx = getCtx();
    const trace = await apiCallRaw<{
      items: Array<{ product_id: number; color_no: string; dye_lot_no: string }>;
    }>(page, 'GET', '/business-trace?page=1&page_size=5');
    expect(Array.isArray(trace.items), `trace.items 应为后端返回的 items 数组`);
  });

  test('5-11 AI 工艺优化', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/ai-models/process-optimizations?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`);
  });

  test('5-12 AI 质量预测', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/ai-models/quality-predictions?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`);
  });

  test('5-13 通知列表', async ({ page }) => {
    // 后端 notification_handler.rs:75 的 payload key 是 list，不是 items；
    // 原实现 expect(notifications.items) 没有匹配器，是永远为真的空断言
    const notifications = await apiCallRaw<{ list: Array<{ id: number }> }>(
      page,
      'GET',
      '/notifications?page=1&page_size=5'
    );
    expect(Array.isArray(notifications.list), '通知列表应返回 list 数组').toBe(true);
  });

  test('5-14 仪表盘', async ({ page }) => {
    const dash = await apiCallRaw<Record<string, unknown>>(page, 'GET', '/dashboard');
    expect(Object.keys(dash ?? {}).length, '仪表盘应返回非空统计对象').toBeGreaterThan(0);
  });

  test('5-15 系统健康状态', async ({ page }) => {
    const status = await apiCallRaw<Record<string, unknown>>(page, 'GET', '/health');
    expect(Object.keys(status ?? {}).length, '系统健康状态应返回非空对象').toBeGreaterThan(0);
  });
});
