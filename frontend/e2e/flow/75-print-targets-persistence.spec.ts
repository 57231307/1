import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw, apiCallExpectFail, BASE_URL } from './helpers';

/**
 * Wave E P6 打印目标持久化与校验锁，兼 BUG-002 假成功族回归
 *
 * 覆盖三件事：
 * - 打印目标经 print-targets CRUD 真落 Postgres（新建后刷新读回同值，非内存影子存储）；
 *   kind 切换时后端把不适用的旧连接列重置为 NULL，行内容与建表 CHECK 语义一致。
 * - 非法入参 fail-visible 400（缺连接字段、非法 kind、重复编码、池无成员），不撞 DB CHECK 变 500。
 * - 打印模板新建后读回持久（BUG-002 原症状族：曾提示成功但刷新即丢，已由 SeaORM 化闭环）。
 *
 * 断言以 API 级为主（选择器不依赖 UI 结构，避免脆弱），末尾一条轻量校验页可达且健康。
 */

const suffix = Date.now().toString().slice(-6);

async function readBackTarget(
  page: import('@playwright/test').Page,
  id: number
): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `/print-targets/${id}`);
}

test.describe('打印目标持久化与校验', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('新建后刷新读回同值，kind 切换重置旧连接列', async ({ page }) => {
    const code = `PT-NET-${suffix}`;
    let createdId = 0;
    try {
      const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/print-targets', {
        target_code: code,
        display_name: `e2e-net-${suffix}`,
        kind: 'network',
        host: '127.0.0.1',
        port: 9100,
        default_copies: 2,
        enabled: true,
      });
      createdId = Number(created?.id);
      expect(createdId, `新建应返回带主键的行，实际：${JSON.stringify(created)}`).toBeGreaterThan(0);
      expect(created.kind).toBe('network');
      expect(created.host).toBe('127.0.0.1');
      expect(created.port).toBe(9100);
      expect(created.queue_name).toBeNull();

      // 读回（模拟刷新重进列表）：列表须含该编码，详情字段一致
      const list = await apiCallRaw<Array<Record<string, unknown>>>(page, 'GET', '/print-targets');
      expect(Array.isArray(list), `列表 data 应为数组：${JSON.stringify(list).slice(0, 120)}`).toBe(
        true
      );
      expect(
        list.some(r => r.target_code === code),
        `刷新读回列表应含 ${code}，证明已落库非内存态`
      ).toBe(true);

      // kind 切换 network→ipp：只提交新 kind 字段，旧 host/port 应被后端重置为 NULL
      await apiCallRaw(page, 'PUT', `/print-targets/${createdId}`, {
        kind: 'ipp',
        endpoint_url: 'http://127.0.0.1:631/print/x',
      });
      const after = await readBackTarget(page, createdId);
      expect(after.kind).toBe('ipp');
      expect(typeof after.endpoint_url).toBe('string');
      expect(after.host, `切换到 ipp 后 host 应被清空：${JSON.stringify(after)}`).toBeNull();
      expect(after.port, `切换到 ipp 后 port 应被清空：${JSON.stringify(after)}`).toBeNull();
    } finally {
      if (createdId) await apiCallRaw(page, 'DELETE', `/print-targets/${createdId}`);
    }
  });

  test('非法入参 fail-visible 400，不落 500', async ({ page }) => {
    // 缺 cups 必需 queue_name
    const missing = await apiCallExpectFail(page, 'POST', '/print-targets', {
      target_code: `PT-CUPS-${suffix}`,
      display_name: `e2e-cups-${suffix}`,
      kind: 'cups',
    });
    expect(missing.status, `缺连接字段应 400，实际 ${missing.status}`).toBe(400);

    // 非法 kind 白名单外
    const badKind = await apiCallExpectFail(page, 'POST', '/print-targets', {
      target_code: `PT-BAD-${suffix}`,
      display_name: `e2e-bad-${suffix}`,
      kind: 'serial_port',
      queue_name: 'lp0',
    });
    expect(badKind.status, `非法 kind 应 400，实际 ${badKind.status}`).toBe(400);

    // 池行无成员
    const emptyPool = await apiCallExpectFail(page, 'POST', '/print-targets', {
      target_code: `PT-POOL-${suffix}`,
      display_name: `e2e-pool-${suffix}`,
      kind: 'cups',
      is_pool: true,
      pool_members: [],
    });
    expect(emptyPool.status, `池无成员应 400，实际 ${emptyPool.status}`).toBe(400);

    // 重复编码：先建一个再建同号
    const dupCode = `PT-DUP-${suffix}`;
    let firstId = 0;
    try {
      const ok = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/print-targets', {
        target_code: dupCode,
        display_name: `e2e-dup-${suffix}`,
        kind: 'system_command',
        command_template: 'echo {file}',
      });
      firstId = Number(ok?.id);
      const dup = await apiCallExpectFail(page, 'POST', '/print-targets', {
        target_code: dupCode,
        display_name: `e2e-dup2-${suffix}`,
        kind: 'system_command',
        command_template: 'echo {file}',
      });
      expect(dup.status, `重复编码应 400，实际 ${dup.status}`).toBe(400);
    } finally {
      if (firstId) await apiCallRaw(page, 'DELETE', `/print-targets/${firstId}`);
    }
  });

  test('打印模板新建后读回持久（BUG-002 模板族）', async ({ page }) => {
    const tcode = `TPL-E2E-${suffix}`;
    let tid = 0;
    try {
      const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/print-templates', {
        template_name: `e2e-tpl-${suffix}`,
        template_code: tcode,
        // doc_type 是"单据→模板"查找键，后端已改为必填并校验取值集：
        // 缺它或写错它的模板永远不会被打印链路命中，故新建必须显式带上
        doc_type: 'sales_order',
        module: 'logistics',
        type: 'custom',
      });
      tid = Number(created?.id);
      expect(tid, `模板新建应返回主键：${JSON.stringify(created)}`).toBeGreaterThan(0);
      expect(
        String(created?.doc_type ?? ''),
        `模板出参应回显单据类型查找键，实得：${JSON.stringify(created).slice(0, 200)}`
      ).toBe('sales_order');

      const list = await apiCallRaw<Array<Record<string, unknown>>>(page, 'GET', '/print-templates');
      const mine = list.find(r => r.template_code === tcode);
      expect(mine, `刷新读回列表应含模板 ${tcode}，证明落库非内存影子存储`).toBeTruthy();
      expect(
        String(mine?.doc_type ?? ''),
        '读回的模板必须保留 doc_type（列表与详情同口径）'
      ).toBe('sales_order');
    } finally {
      if (tid) await apiCallRaw(page, 'DELETE', `/print-templates/${tid}`);
    }
  });

  test('目标仍被启用模板列为缺省目标 → 删除被拒 400 并点名编码，停用后方可删', async ({ page }) => {
    const tcode = `E2E-REF-${suffix}`;
    const tplCode = `TPL-E2E-REF-${suffix}`;
    let targetId = 0;
    let templateId = 0;
    try {
      const target = await apiCallRaw<{ id?: number }>(page, 'POST', '/print-targets', {
        target_code: tcode,
        display_name: `e2e被引用目标${tcode}`,
        kind: 'network',
        host: '127.0.0.1',
        port: 9100,
        enabled: true,
      });
      targetId = Number(target?.id);
      expect(targetId, `打印目标应真实落库：${JSON.stringify(target)}`).toBeGreaterThan(0);

      const tpl = await apiCallRaw<{ id?: number }>(page, 'POST', '/print-templates', {
        template_name: `e2e引用目标模板${tplCode}`,
        template_code: tplCode,
        doc_type: 'sales_order',
        module: 'sales',
        type: 'order',
        default_target_name: tcode,
        content: '<div>{order_no}</div>',
        is_default: false,
        status: 'active',
      });
      templateId = Number(tpl?.id);
      expect(templateId, `模板应真实落库：${JSON.stringify(tpl)}`).toBeGreaterThan(0);

      const fail = await apiCallExpectFail(page, 'DELETE', `/print-targets/${targetId}`);
      expect(
        fail.status,
        `被启用模板引用的目标删除必须可见拒绝，实得：${JSON.stringify(fail)}`
      ).toBe(400);
      expect(
        String(fail.message ?? ''),
        `拒绝文案须点名被引用的目标编码，实得：${JSON.stringify(fail)}`
      ).toContain(tcode);

      // 引用门只拦"启用中"的模板：停用同一模板后目标才允许删除
      await apiCallRaw(page, 'PUT', `/print-templates/${templateId}`, { status: 'inactive' });
      const deletedId = targetId;
      await apiCallRaw(page, 'DELETE', `/print-targets/${deletedId}`);
      targetId = 0;
      const gone = await apiCallExpectFail(page, 'GET', `/print-targets/${deletedId}`);
      expect(gone.status, `删除后详情应读不到，实得：${JSON.stringify(gone)}`).toBe(404);
    } finally {
      if (templateId) await apiCallRaw(page, 'DELETE', `/print-templates/${templateId}`);
      if (targetId) await apiCallRaw(page, 'DELETE', `/print-targets/${targetId}`);
    }
  });

  test('打印目标页可达且健康', async ({ page }) => {
    await page.goto(`${BASE_URL}/print-targets`);
    expect(page.url(), '超管应能进入打印目标页，不应被重定向到登录').not.toContain('/login');
    await expect(page.locator('.page-title')).toBeVisible();
    await expect(page.locator('.el-table').first()).toBeVisible();
  });
});
