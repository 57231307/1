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
  API_BASE,
  API_PREFIX,
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
    // /audit-logs 的条目 DTO(AuditLogListItem) 无 action 字段、request_method 可空
    // （系统/后台审计事件无 HTTP 方法），"操作标识"由 operation_type 承担（上一行已断言）。
    // 这里改断言主键 id：它是 DTO 唯一保证非空的标识，能真实证明列表回读到了持久化记录。
    expect(Number(rows[0].id), '审计记录应含正整数主键 id').toBeGreaterThan(0);
  });

  test('5-2 用户列表 + 角色列表 + 部门列表', async ({ page }) => {
    const users = await apiCallRaw<{
      users: Array<{ id: number; username: string }>;
      total: number;
    }>(page, 'GET', '/users?page=1&page_size=10');
    // GET /users 出参是 UserListResponse{users,total,page,page_size}（不是 items），
    // 原写法 `expect(users?.items?.length ?? 0).toBeGreaterThanOrEqual(0)` 是恒真断言，
    // 键名写错也永远绿。
    expect(
      Array.isArray(users?.users),
      `用户列表应返回 users 数组，实际：${JSON.stringify(users).slice(0, 200)}`
    ).toBe(true);
    expect(
      Number(users.total) >= users.users.length,
      `total(${users.total}) 不应小于本页行数(${users.users.length})`
    ).toBe(true);
    for (const u of users.users.slice(0, 5)) {
      expect(Number(u.id), `用户行缺少 id：${JSON.stringify(u)}`).toBeGreaterThan(0);
      expect(String(u.username ?? ''), `用户行缺少 username：${JSON.stringify(u)}`).not.toBe('');
    }

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

    // GET /departments → department_service::list 返回 PaginatedResponse{items,total,page,page_size}。
    // 原实现 expect(Array.isArray(depts.items)) 不带匹配器，是永不失败的空断言。
    const depts = await apiCallRaw<{
      items: Array<{ id: number; name: string; code?: string; parent_id?: number | null }>;
      total: number;
    }>(page, 'GET', '/departments?page=1&page_size=10');
    expect(
      Array.isArray(depts?.items),
      `部门列表应返回 items 数组，实际：${JSON.stringify(depts).slice(0, 200)}`
    ).toBe(true);
    for (const d of depts.items) {
      expect(Number(d.id), `部门行缺少 id：${JSON.stringify(d)}`).toBeGreaterThan(0);
      expect(String(d.name ?? ''), `部门行缺少 name：${JSON.stringify(d)}`).not.toBe('');
    }
  });

  test('5-3 数据权限验证（行级隔离）', async ({ page }) => {
    // GET /data-permissions 出参是裸数组 Vec<DataPermissionResponse>（无分页信封）
    const perms = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/data-permissions?page=1&page_size=5'
    );
    expect(
      Array.isArray(perms),
      `数据权限列表应返回数组，实际：${JSON.stringify(perms).slice(0, 200)}`
    ).toBe(true);
    for (const row of perms) {
      expect(
        String(row.resource_type ?? ''),
        `数据权限行缺少 resource_type：${JSON.stringify(row)}`
      ).not.toBe('');
    }
  });

  test('5-4 字段级权限验证（染色配方导出仅 dye_recipe_master 可）', async ({ page }) => {
    // admin 角色应可以访问（有 *:* 权限）
    const result = await apiCallExpectFail(page, 'GET', '/production/dye-recipes/export');
    // admin 可能被允许或被拒绝（取决于角色黑名单）
    expectBadRequest(result); // 应返回错误码
  });

  test('5-5 BPM 流程定义', async ({ page }) => {
    // 列表 key 是 items：bpm_definition_handler.rs:55-57 用 json!({"items": items, ...}) 手拼。
    // （此处类型曾误写成 { list }，与断言读的 items 相互矛盾，tsc 直接判错）
    const defs = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/bpm/definitions?page=1&page_size=5'
    );
    console.log(`[5-5] BPM 流程定义 items 长度=${defs?.items?.length ?? '(缺 key)'}`);
    expect(Array.isArray(defs?.items), 'BPM 流程定义应返回 items 数组').toBe(true);
  });

  test('5-6 BPM 审批任务', async ({ page }) => {
    // 分页出参统一为 PaginatedResponse{items,total,page,page_size}（utils/response.rs:34），
    // 断言按该形状判定。
    const tasks = await apiCallRaw<{
      items: Array<{ id: number; status: string }>;
      total: number;
      page: number;
      page_size: number;
    }>(page, 'GET', '/bpm/tasks?page=1&page_size=5');
    expect(
      Array.isArray(tasks?.items),
      `BPM 任务应返回 items 数组，实际响应：${JSON.stringify(tasks).slice(0, 200)}`
    ).toBe(true);
    expect(typeof tasks.total, '应回显 total').toBe('number');
    expect(tasks.page, '应回显请求页码').toBe(1);
    expect(tasks.page_size, '应回显每页数量').toBe(5);
    // bpm_task 取值域（backend/src/models/status/bpm_crm_contract.rs::bpm_task）
    const BPM_TASK_STATUSES = ['pending', 'completed', 'rejected', 'cancelled'];
    for (const task of tasks.items) {
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
    expect(ctx.customOrderId, '5-7 定制订单创建未返回 id，7 阶段状态机断言无法执行').toBeTruthy();

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
  });

  test('5-8 定制订单状态门校验（非法跳跃应拒绝）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.customOrderId;
    expect(
      id,
      '5-7/ensureTestEntities 未建出定制订单（ctx.customOrderId 缺失），本用例前置失败'
    ).toBeTruthy();

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
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`).toBe(true);
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

  test('5-10 坯布五维追溯（按供应商+批号正向追链）', async ({ page }) => {
    const ctx = getCtx();
    // 追溯域没有"列表"端点：routes/analytics.rs business_trace() 只注册了
    // /five-dimension/{id}、/forward、/backward、/snapshot/{id}，
    // 原用例发的 GET /business-trace?page=… 是不存在的路径（出参键 items 也是猜的）。
    // 正向追链出参是 TraceListResponse{traces,total}。
    const batchNo = `E2E-TRC-${Date.now().toString().slice(-8)}`;
    const trace = await apiCallRaw<{ traces: Array<Record<string, unknown>>; total: number }>(
      page,
      'GET',
      `/business-trace/forward?supplier_id=${ctx.supplierId ?? 0}&batch_no=${encodeURIComponent(batchNo)}`
    );
    expect(
      Array.isArray(trace?.traces),
      `正向追溯应返回 traces 数组，实际：${JSON.stringify(trace).slice(0, 200)}`
    ).toBe(true);
    expect(
      Number(trace.total) >= trace.traces.length,
      `total(${trace.total}) 不应小于返回链数(${trace.traces.length})`
    ).toBe(true);
    for (const chain of trace.traces) {
      expect(String(chain.batch_no ?? ''), `追溯链缺少 batch_no：${JSON.stringify(chain)}`).toBe(
        batchNo
      );
    }
  });

  test('5-11 AI 工艺优化', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/ai/process-optimizations?page=1&page_size=5'
    );
    // 真实路径是 /api/v1/erp/ai/process-optimizations（routes/system.rs:366），
    // 原用例写的 /ai-models/... 属另一组路由前缀，不存在该端点。
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`).toBe(true);
  });

  test('5-12 AI 质量预测', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/ai/quality-predictions?page=1&page_size=5'
    );
    // 同上：/api/v1/erp/ai/quality-predictions（routes/system.rs:399）
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`).toBe(true);
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
    // routes/system.rs:39 dashboard() 仅注册子路径，无裸 /dashboard；
    // /dashboard/overview 对应聚合统计（dashboard_handler::get_dashboard_overview）。
    const dash = await apiCallRaw<Record<string, unknown>>(page, 'GET', '/dashboard/overview');
    expect(Object.keys(dash ?? {}).length, '仪表盘应返回非空统计对象').toBeGreaterThan(0);
  });

  test('5-15 系统健康状态', async ({ page }) => {
    // /health handler 刻意返回裸 Json(HealthStatus)，无 ApiResponse 信封（无 code 字段），
    // 故用 page.request 直调绕过信封校验。
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/health`);
    expect(resp.ok()).toBe(true);
    const body = await resp.json();
    expect(typeof body.status).toBe('string');
    expect(body.status).toBe('healthy');
  });
});
