import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  getCtx,
  ensureTestEntities,
} from './helpers';

/**
 * P5.11b 四套专用审批流 + BPM 引擎全流程
 *
 * 1. 坏账核销（writeoffs）双级审批：finance-approve → general-manager-approve
 * 2. 资金转账（fund-management/transfers）审批
 * 3. 角色变更（role-change-approvals）审批
 * 4. BPM 引擎：任务查询 → 待办 → 领取/审批 → 审批链
 *
 * 每套流在测试内真实创建前置单据并按自建对象 id 驱动审批链（禁 test.skip 静默跳过）；
 * 5xx 永远真失败。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
const TS = Date.now().toString().slice(-8);

test.describe('P5.11b 专用审批流', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('坏账核销双级审批链', async ({ page }) => {
    // 原实现：GET 列表 find(w.status 含 pending) 为空即 test.skip() → CI 永跳（两处缺陷叠加）。
    // 现真实串一条前置链：建客户 → 建应收单 → 审核应收单 → 建核销申请，断言核销 id 落地。
    //
    // 【当前后端缺陷，本用例预期硬红】：create_writeoff 的审核门 bad_debt_service.rs:449
    // 比较 `approval_status != "approved"`（小写），而应收单审核后 approve() 写入的是
    // STATUS_APPROVED="APPROVED"（大写，status/general.rs:22）→ 词表大小写永不相等 →
    // 对已审核应收单发起核销恒被拒，拿不到 pending 核销单。这正是 skip 掩盖的真缺陷。
    // 后端修复大小写后本用例转绿（届时核销审批链 finance/GM 步 <500 断言才真正生效）。
    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `41b核销客户${TS}`,
      customer_type: 'retail',
    });
    const customerId = cust?.data?.id;
    expect(customerId, `建核销客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    const inv = await apiCall<{ id?: number }>(page, 'POST', '/ar/invoices', {
      customer_id: customerId,
      invoice_amount: 1000,
      invoice_date: new Date().toISOString().slice(0, 10),
    });
    const invoiceId = inv?.data?.id;
    expect(invoiceId, `建应收单失败：${JSON.stringify(inv)}`).toBeTruthy();

    await apiCall(page, 'POST', `/ar/invoices/${invoiceId}/approve`);

    // apiCall 对 4xx/5xx 抛错并回显后端真实 message（含 "未审核通过（当前 approval_status=APPROVED）"），
    // 直接把词表大小写不一致的现场打到红名堆栈，无需再 try/catch。
    const wo = await apiCall<{ id?: number }>(page, 'POST', '/bad-debts/writeoffs', {
      customer_id: customerId,
      ar_invoice_id: invoiceId,
      writeoff_amount: 100,
      reason: 'E2E 41b 坏账核销',
    });
    const woId = wo?.data?.id;
    expect(woId, `建坏账核销申请应返回 id，实际：${JSON.stringify(wo)}`).toBeTruthy();

    // 一/二级审批：审批人=申请人 → SelfApprovalForbidden（4xx 业务拒绝，链可达），非 5xx。
    const l1 = await apiCallExpectFail(
      page,
      'POST',
      `/bad-debts/writeoffs/${woId}/finance-approve`,
      { comment: 'E2E 财务一级审批（预期自审批被拒）' }
    );
    expect(l1.status, `一级审批端点应可访问（<500），实际 ${l1.status}`).toBeLessThan(500);
    const l2 = await apiCallExpectFail(
      page,
      'POST',
      `/bad-debts/writeoffs/${woId}/general-manager-approve`,
      { comment: 'E2E 总经理二级审批' }
    );
    expect(l2.status, `二级审批端点应可访问（<500），实际 ${l2.status}`).toBeLessThan(500);
  });

  test('资金转账审批链', async ({ page }) => {
    // 原实现：GET /fund-management/transfers 读 data.items find status 'pending' → test.skip()，
    // CI 永跳。三重缺陷：list_transfer_records 返回 ApiResponse<Vec>（data 是裸数组，无 items）；
    // 模型 status 为大写 "PENDING"（'pending'.includes 大小写不符）；空库本就无记录。
    // 改造：真实建两个资金账户 → 充值 → 发起小额调拨（落库并返回记录 id）→ 断言 id 落地，
    // 再对审批端点做可达断言（<500：小额自动通过，审批端点回业务拒绝属正常）。
    const from = await apiCall<{ id?: number }>(page, 'POST', '/fund-management/accounts', {
      account_name: `41b转出账户${TS}`,
      account_no: `41bF${TS}`,
      account_type: 'bank',
      currency: 'CNY',
    });
    const fromId = from?.data?.id;
    expect(fromId, `建转出账户失败：${JSON.stringify(from)}`).toBeTruthy();
    const to = await apiCall<{ id?: number }>(page, 'POST', '/fund-management/accounts', {
      account_name: `41b转入账户${TS}`,
      account_no: `41bT${TS}`,
      account_type: 'bank',
      currency: 'CNY',
    });
    const toId = to?.data?.id;
    expect(toId, `建转入账户失败：${JSON.stringify(to)}`).toBeTruthy();

    await apiCall(page, 'POST', `/fund-management/accounts/${fromId}/deposit`, { amount: '10000' });
    const tr = await apiCall<{ id?: number }>(page, 'POST', '/fund-management/transfer', {
      from_account_id: fromId,
      to_account_id: toId,
      amount: 100,
      reason: 'E2E 41b 转账',
    });
    const trId = tr?.data?.id;
    expect(trId, `发起资金调拨应返回记录 id，实际：${JSON.stringify(tr)}`).toBeTruthy();

    const appr = await apiCallExpectFail(
      page,
      'POST',
      `/fund-management/transfers/${trId}/approve`,
      { comments: 'E2E 转账审批' }
    );
    expect(appr.status, `转账审批端点应可访问（<500），实际 ${appr.status}`).toBeLessThan(500);
  });

  test('角色变更审批链', async ({ page }) => {
    // 原实现：GET /role-change-approvals 读 data.items find status 'pending' → test.skip()，空库永跳。
    // 改造：真实建一条敏感角色变更申请（装置同 53-approval-depth-full）→ 断言 id 落地 →
    // 触发 approve-l1（申请人自审 → 防自审批 4xx，链可达）→ cancel 收尾。
    const roles = await apiCallRaw<{ roles?: Array<{ id: number; code: string }> }>(
      page,
      'GET',
      '/roles'
    );
    const roleList = roles?.roles ?? [];
    const sensitive = roleList.find(r =>
      ['admin', 'super_admin', 'finance', 'finance_admin'].includes(r.code)
    );
    expect(
      sensitive,
      `角色表缺敏感角色（admin/super_admin/finance/finance_admin），现有：${roleList.map(r => r.code).join(',')}`
    ).toBeTruthy();
    const me = await apiCallRaw<{ id?: number }>(page, 'GET', '/auth/me');
    const myId = me?.id;
    expect(myId, `/auth/me 未返回当前用户 id：${JSON.stringify(me)}`).toBeTruthy();

    const created = await apiCall<{ id?: number }>(page, 'POST', '/role-change-approvals', {
      change_type: 'assign_role',
      target_user_id: myId,
      target_role_id: sensitive.id,
      target_role_code: sensitive.code,
    });
    const apId = created?.data?.id;
    expect(apId, `建角色变更申请应返回 id，实际：${JSON.stringify(created)}`).toBeTruthy();
    try {
      const appr = await apiCallExpectFail(
        page,
        'POST',
        `/role-change-approvals/${apId}/approve-l1`,
        { comments: 'E2E 角色变更审批（预期自审批被拒）' }
      );
      expect(appr.status, `L1 审批端点应可访问（<500），实际 ${appr.status}`).toBeLessThan(500);
    } finally {
      // 审批记录无 DELETE 路由（405），只能走 /cancel 收尾，避免污染库。
      await tryCleanup(
        page,
        'POST',
        `/role-change-approvals/${apId}/cancel`,
        '[41b-角色变更] cancel'
      );
    }
  });
});

test.describe('P5.11c BPM 引擎全流程', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    // 确保 ensureTestEntities 建好 sales_order_approval 流程定义（含可解析的 user_task 首节点），
    // 供“流程实例审批链查询”用例真实启动实例。
    await ensureTestEntities(page);
  });

  test('任务查询 → 待办 → 审批链', async ({ page }) => {
    // 1. 任务列表
    const tasks = await page.request.get(`${API_BASE}${API_PREFIX}/bpm/tasks?page=1&page_size=10`);
    expect(tasks.status()).toBeLessThan(500);

    // 2. 待办任务
    const pending = await page.request.get(
      `${API_BASE}${API_PREFIX}/bpm/tasks/pending?page=1&page_size=10`
    );
    expect(pending.status()).toBeLessThan(500);

    // 3. 已办任务
    const completed = await page.request.get(
      `${API_BASE}${API_PREFIX}/bpm/tasks/completed?page=1&page_size=10`
    );
    expect(completed.status()).toBeLessThan(500);
  });

  test('流程实例审批链查询', async ({ page }) => {
    // 原实现：GET /bpm/monitor/instances 为空即 test.skip()——CI 空库无运行中实例 → 永跳。
    // 改造：用 ensureTestEntities 建好的 sales_order_approval 定义（含可解析的 user_task 首节点）
    // 真实启动一条流程实例，断言 instance_id 落地，再对该实例审批链端点做可达断言（<500）。
    const me = await apiCallRaw<{ id?: number }>(page, 'GET', '/auth/me');
    const initiatorId = me?.id;
    expect(initiatorId, `/auth/me 未返回当前用户 id：${JSON.stringify(me)}`).toBeTruthy();

    const started = await apiCall<{ instance_id?: number }>(page, 'POST', '/bpm/process/start', {
      process_key: 'sales_order_approval',
      business_type: 'sales_order',
      business_id: getCtx().salesOrderId ?? 1,
      title: 'E2E 41b 流程实例',
      initiator_id: initiatorId,
      initiator_name: 'e2e',
    });
    const instanceId = started?.data?.instance_id;
    expect(
      instanceId,
      `启动流程实例应返回 instance_id，实际：${JSON.stringify(started)}`
    ).toBeTruthy();

    const chain = await apiCallExpectFail(
      page,
      'GET',
      `/bpm/instances/${instanceId}/approval-chain`
    );
    expect(chain.status, `审批链端点应可访问（<500），实际 ${chain.status}`).toBeLessThan(500);
  });

  test('BPM 定义与模板列表', async ({ page }) => {
    for (const ep of ['/bpm/definitions', '/bpm/templates']) {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}${ep}?page=1&page_size=5`);
      expect(resp.status()).toBeLessThan(500);
    }
  });
});
