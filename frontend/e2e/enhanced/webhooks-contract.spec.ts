// Webhook 与集成契约面 — enhanced/webhooks-contract
// 功能（业务规则面，逐端点对后端源码核实后编写）：
//   1) 集成配置族 CRUD 与状态跃迁：POST /webhooks/integrations 落库 events 恒为通配、
//      user_id 记本轮会话账号（backend/src/handlers/webhook_integration_handler.rs::
//      create_integration 构造段）；PUT 部分字段更新未提交列保持原值（同文件
//      update_integration 逐字段 Set 段）；DELETE 为软删（is_active=false，行保留，
//      backend/src/services/webhook_service.rs::delete_webhook），删除后列表按启用过滤排除、
//      执行状态仍可按 id 读回、发送通道按「已禁用」拒绝。
//   2) 行级越权（IDOR）：他人私有集成对非所有者 PUT 与 GET logs 均 403（FORBIDDEN、
//      脱敏「无权限」），且被拒更新零改写。
//   3) 执行状态读族 GET /webhooks/{id}/logs：出参逐列对应 webhooks 表 last_* 与配置列，
//      retry_count 与 max_retry_count 常量 5（backend/src/handlers/webhook_handler.rs::
//      get_webhook_logs 装配段）。
//   4) 出站零外呼纪律：所有测试行的 URL 一律指向 127.0.0.1 字面量，SSRF 卫兵在
//      backend/src/services/webhook_service.rs::trigger_webhook 的 validate_url 分支拦截，
//      拦截发生在 mark_sending 之后、真实网络请求之前，因此断言的绿只来自
//      「库内侧（SENDING 留痕、retry_count 不涨、目标响应体恒为隐藏常量）」，
//      绝不产生对外部系统的调用，也不伪造发送成功。
//   5) 重试族：无 last_payload 时拒绝且零状态漂移；有留痕 payload 时重投仍走 SSRF 拦截腿，
//      信封文案逐字「Webhook 重试已执行，但目标返回非成功状态」。
//   6) 消息发送前置校验：空内容与不支持消息类型在触达服务层之前 400（BAD_REQUEST 族），
//      幽灵 integration_id 在所有权门 404，禁用行在 ensure_webhook_active 门 400，全部断言
//      后续读回零落库；被拦截后的真实外发失败当前回 INTERNAL_ERROR 信封，属源码行为如实钉住，
//      外发面覆盖登记为残量。
//   7) 入站回调 POST /webhooks/integrations/callback：缺签名头 401、非法十六进制签名 401、
//      合法十六进制但错误签名 401（backend/src/utils/webhook_signature.rs::
//      verify_webhook_signature 三拒绝腿），入站校验发生在任何解析入库之前。
// 调用方：CI e2e 分片 36（extras enhanced+production）。
// 入参：apiCall、apiCallRaw、apiCallExpectFail 与 page.request 直连（回调需自定义签名头）；
//   URL 含 127.0.0.1 与不可达端口组合，保证即使卫兵失效也不可能连出真实外部系统。
// 传给谁：越权面用 loginInIsolatedContext 以种子角色 salesperson 独立会话发起。
// 存什么：webhooks 表的 name、url、events、is_active、user_id、last_triggered_at、
//   last_status、retry_count。
// 存哪里：表 webhooks。
// 残量登记（不许算已覆盖）：真实出站成功路径（需外部桩或产品裁定）；带正确密钥签名的
//   回调成功路径（webhook_secret 只在服务端，e2e 不可持有）。
import { test, expect } from '../diagnose-fixture';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getRoleCredential,
  loginInIsolatedContext,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
  API_BASE,
  API_PREFIX,
  type IsolatedAuthedSession,
} from '../flow/helpers';

interface IntegrationItem {
  id: number;
  name: string;
  platform: string;
  webhook_url: string;
  is_active: boolean;
  last_triggered_at: string | null;
  last_status: string | null;
  created_at: string;
}

interface WebhookLogsEntry {
  id: number;
  name: string;
  url: string;
  events: string;
  is_active: boolean;
  last_triggered_at: string | null;
  last_status: string | null;
  retry_count: number;
  max_retry_count: number;
}

/** 本轮专属的不可达出站目标（127.0.0.1 字面量必被 SSRF 卫兵拦截，绝无真实外呼） */
const LOOPBACK_URL = 'https://127.0.0.1:59999/e2e-ssrf-no-outbound';

/**
 * 功能：创建一个指向回环地址的集成配置行（创建腿不校验 URL，投递腿才校验）。
 * 调用方：本文件各用例。入参：page、唯一后缀 tag、可选 is_active（缺省服务端默认 true）。
 * 传给谁：返回 id 进 logs 读回与发送族入参。存什么：webhooks 一行。存哪里：表 webhooks。
 */
async function createLoopbackIntegration(
  page: import('@playwright/test').Page,
  tag: string,
  isActive?: boolean
): Promise<number> {
  const created = await apiCall<IntegrationItem>(page, 'POST', '/webhooks/integrations', {
    name: `契约集成${tag}`,
    platform: 'GENERIC',
    webhook_url: `${LOOPBACK_URL}?t=${tag}`,
    secret: `sec-${tag}`,
    ...(isActive === undefined ? {} : { is_active: isActive }),
  });
  const id = Number(created.data?.id);
  expect(id, `创建集成失败：${JSON.stringify(created.data)}`).toBeGreaterThan(0);
  expect(created.data!.platform, '创建响应必须回显提交的平台名').toBe('GENERIC');
  expect(created.message, '创建信封文案必须逐字').toBe('Webhook 集成创建成功');
  return id;
}

test.describe('Webhook 与集成 - 契约面与零外呼纪律', () => {
  test('集成 CRUD、启用门与 IDOR 越权面', async ({ page, browser }) => {
    const tag = genCode('WHA');
    const id = await createLoopbackIntegration(page, tag);
    try {
      // 初始执行状态：全空留痕、通配事件、retry 常量
      const logs = await apiCallRaw<WebhookLogsEntry>(page, 'GET', `/webhooks/${id}/logs`);
      expect(logs.id).toBe(id);
      expect(logs.name, 'logs 名称必须与落库行同源').toBe(`契约集成${tag}`);
      expect(logs.events, '服务端把 events 恒置为通配').toBe('*');
      expect(logs.is_active).toBe(true);
      expect(logs.last_status, '新建行不得有历史执行状态').toBeNull();
      expect(logs.last_triggered_at).toBeNull();
      expect(logs.retry_count).toBe(0);
      expect(logs.max_retry_count, '重试上限常量必须透出 5').toBe(5);

      // PUT 只改名称：URL 列保持原值（读回 url 仍等于创建值）
      const updated = await apiCall<IntegrationItem>(page, 'PUT', `/webhooks/integrations/${id}`, {
        name: `契约集成改名${tag}`,
      });
      expect(updated.message, '更新信封文案必须逐字').toBe('Webhook 集成更新成功');
      expect(updated.data!.platform, '更新响应平台为表外虚拟列恒 GENERIC').toBe('GENERIC');
      const afterPut = await apiCallRaw<WebhookLogsEntry>(page, 'GET', `/webhooks/${id}/logs`);
      expect(afterPut.name).toBe(`契约集成改名${tag}`);
      expect(afterPut.url, '未提交 url 时 URL 列不得改写').toBe(`${LOOPBACK_URL}?t=${tag}`);

      // 列表按启用过滤且按属主可见
      const list = await apiCallRaw<IntegrationItem[]>(page, 'GET', '/webhooks/integrations');
      expect(Array.isArray(list)).toBe(true);
      expect(
        list.some(r => r.id === id),
        '启用状态下列表必须含本行'
      ).toBe(true);

      // 停用后列表排除、发送通道按已禁用拒绝且零状态漂移
      await apiCall(page, 'PUT', `/webhooks/integrations/${id}`, { is_active: false });
      const listAfterDisable = await apiCallRaw<IntegrationItem[]>(
        page,
        'GET',
        '/webhooks/integrations'
      );
      expect(
        listAfterDisable.some(r => r.id === id),
        '列表只出启用行，停用后仍出现即过滤失效'
      ).toBe(false);
      const disabledSend = await apiCallExpectFail(
        page,
        'POST',
        '/webhooks/integrations/dingtalk/send',
        { integration_id: id, message_type: 'text', content: `停用探针${tag}` }
      );
      expect(
        disabledSend.status,
        `已禁用行发送必须 400，实际 ${JSON.stringify(disabledSend)}`
      ).toBe(400);
      expect(failureCode(disabledSend)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
      const logsAfterDisabledSend = await apiCallRaw<WebhookLogsEntry>(
        page,
        'GET',
        `/webhooks/${id}/logs`
      );
      expect(logsAfterDisabledSend.last_status, '已禁用拒绝发生在留痕写入之前').toBeNull();
      expect(logsAfterDisabledSend.last_triggered_at).toBeNull();

      // IDOR：salesperson 独立会话读与管理他人私有行均 403
      let intruder: IsolatedAuthedSession | undefined;
      try {
        const cred = getRoleCredential('salesperson');
        if (!cred) throw new Error('前置：salesperson 角色凭证缺失，属 setup 缺陷');
        intruder = await loginInIsolatedContext(browser, cred.username, cred.password);
        const idorPut = await apiCallExpectFail(
          intruder.page,
          'PUT',
          `/webhooks/integrations/${id}`,
          { name: '越权改名' }
        );
        expect(idorPut.status, `越权 PUT 必须 403，实际 ${JSON.stringify(idorPut)}`).toBe(403);
        expect(failureCode(idorPut)).toBe(APP_ERROR_CODES.FORBIDDEN);
        expect(idorPut.message, '权限文案必须为脱敏常量').toBe('无权限');
        const idorLogs = await apiCallExpectFail(intruder.page, 'GET', `/webhooks/${id}/logs`);
        expect(idorLogs.status, '越权读他人执行状态必须 403').toBe(403);
        const idorList = await apiCallRaw<IntegrationItem[]>(
          intruder.page,
          'GET',
          '/webhooks/integrations'
        );
        expect(
          idorList.some(r => r.id === id),
          '他人私有行不得混入越权会话列表'
        ).toBe(false);
        const afterIdor = await apiCallRaw<WebhookLogsEntry>(page, 'GET', `/webhooks/${id}/logs`);
        expect(afterIdor.name, '被拒的越权 PUT 不得改写名称列').toBe(`契约集成改名${tag}`);
      } finally {
        if (intruder) await intruder.context.close();
      }

      // 软删：列表排除、行保留可按 logs 读回 is_active=false
      const del = await apiCall<unknown>(
        page,
        'DELETE',
        `/webhooks/integrations/integration/${id}`
      );
      expect(del.message, '删除信封文案必须逐字').toBe('Webhook 集成已删除');
      const logsAfterDelete = await apiCallRaw<WebhookLogsEntry>(
        page,
        'GET',
        `/webhooks/${id}/logs`
      );
      expect(logsAfterDelete.is_active, '删除为软删语义：行保留、启用列翻转').toBe(false);
      const listAfterDelete = await apiCallRaw<IntegrationItem[]>(
        page,
        'GET',
        '/webhooks/integrations'
      );
      expect(
        listAfterDelete.some(r => r.id === id),
        '删除后列表必须排除本行'
      ).toBe(false);

      const ghost = 2_100_000_000 + (Date.now() % 1000);
      const ghostPut = await apiCallExpectFail(page, 'PUT', `/webhooks/integrations/${ghost}`, {
        name: '幽灵',
      });
      expect(ghostPut.status, '幽灵 id 更新必须 404').toBe(404);
      expect(failureCode(ghostPut)).toBe(APP_ERROR_CODES.NOT_FOUND);
      expect(ghostPut.message).toBe('资源未找到');
    } finally {
      await tryCleanup(
        page,
        'DELETE',
        `/webhooks/integrations/integration/${id}`,
        'webhook-integration'
      );
    }
  });

  test('测试与重试族 - 库内留痕对账且外呼被 SSRF 卫兵拦截', async ({ page }) => {
    const tag = genCode('WHB');
    const id = await createLoopbackIntegration(page, tag);
    try {
      // 无历史 payload 时重试先被拒，且零状态漂移
      const retryCold = await apiCallExpectFail(page, 'POST', `/webhooks/${id}/retry`);
      expect(retryCold.status, `冷重试必须 400，实际 ${JSON.stringify(retryCold)}`).toBe(400);
      expect(failureCode(retryCold)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
      const logsCold = await apiCallRaw<WebhookLogsEntry>(page, 'GET', `/webhooks/${id}/logs`);
      expect(logsCold.last_status, '冷重试不得写 SENDING 留痕').toBeNull();
      expect(logsCold.last_triggered_at).toBeNull();

      // 测试腿：卫兵拦截后回库内侧结果，不回显目标响应体
      const tested = await apiCall<{
        success: boolean;
        status_code: number | null;
        response_body: string | null;
        error: string | null;
      }>(page, 'POST', `/webhooks/${id}/test`);
      expect(tested.message, '测试信封文案必须逐字').toBe('测试消息已发送');
      expect(tested.data!.success, '回环目标必被拦截，投递结果必须如实 false').toBe(false);
      expect(tested.data!.status_code, '未发生真实出站，状态码必须为 null').toBeNull();
      expect(tested.data!.response_body, 'SSRF 缓解必须把响应体替换为隐藏常量').toBe(
        '出于安全原因，已隐藏响应内容'
      );
      expect(
        String(tested.data!.error),
        `错误必须点名 SSRF 拦截腿，实际 ${tested.data!.error}`
      ).toContain('SSRF 防护拦截');

      // 库内侧留痕：SENDING 在拦截之前写入、retry_count 不涨
      const logsAfterTest = await apiCallRaw<WebhookLogsEntry>(page, 'GET', `/webhooks/${id}/logs`);
      expect(logsAfterTest.last_status, '投递前置留痕必须落 SENDING').toBe('SENDING');
      expect(logsAfterTest.last_triggered_at, '触发时间必须落库非空').not.toBeNull();
      expect(logsAfterTest.retry_count, '拦截腿不得计入重试计数').toBe(0);

      // 已有 payload 留痕后重试：重投仍走拦截腿，信封逐字
      const retried = await apiCall<{ success: boolean; error: string | null }>(
        page,
        'POST',
        `/webhooks/${id}/retry`
      );
      expect(retried.message, '重试未成功信封必须逐字').toBe(
        'Webhook 重试已执行，但目标返回非成功状态'
      );
      expect(retried.data!.success).toBe(false);
      expect(String(retried.data!.error)).toContain('SSRF 防护拦截');

      // test-integration 入口与 test 同腿同形
      const integrationTest = await apiCall<{
        success: boolean;
        response_body: string | null;
        error: string | null;
      }>(page, 'POST', `/webhooks/integrations/test-integration/${id}`);
      expect(integrationTest.data!.success).toBe(false);
      expect(integrationTest.data!.response_body).toBe('出于安全原因，已隐藏响应内容');
      expect(String(integrationTest.data!.error)).toContain('SSRF 防护拦截');

      const ghost = 2_100_000_000 + (Date.now() % 1000);
      const ghostTest = await apiCallExpectFail(
        page,
        'POST',
        `/webhooks/integrations/test-integration/${ghost}`
      );
      expect(ghostTest.status, '幽灵 id 测试必须 404 而非被吞成 200').toBe(404);
      expect(failureCode(ghostTest)).toBe(APP_ERROR_CODES.NOT_FOUND);
      expect(ghostTest.message).toBe('资源未找到');
    } finally {
      await tryCleanup(
        page,
        'DELETE',
        `/webhooks/integrations/integration/${id}`,
        'webhook-integration'
      );
    }
  });

  test('消息发送族 - 前置校验 400 与幽灵 404 全部零投递', async ({ page }) => {
    const tag = genCode('WHC');
    const id = await createLoopbackIntegration(page, tag);
    try {
      const empty = await apiCallExpectFail(page, 'POST', '/webhooks/integrations/dingtalk/send', {
        integration_id: id,
        message_type: 'text',
        content: '',
      });
      expect(empty.status, `空内容必须 400，实际 ${JSON.stringify(empty)}`).toBe(400);
      expect(failureCode(empty)).toBe(APP_ERROR_CODES.BAD_REQUEST);
      expect(empty.message, 'BadRequest 族外显必须为脱敏常量').toBe('请求参数错误');

      const badType = await apiCallExpectFail(page, 'POST', '/webhooks/integrations/wechat/send', {
        integration_id: id,
        message_type: 'voice',
        content: `类型探针${tag}`,
      });
      expect(badType.status, '不支持的消息类型必须 400').toBe(400);
      expect(failureCode(badType)).toBe(APP_ERROR_CODES.BAD_REQUEST);

      const ghost = 2_100_000_000 + (Date.now() % 1000);
      const ghostSend = await apiCallExpectFail(
        page,
        'POST',
        '/webhooks/integrations/dingtalk/send',
        {
          integration_id: ghost,
          message_type: 'text',
          content: `幽灵探针${tag}`,
        }
      );
      expect(ghostSend.status, '幽灵 integration 必须 404').toBe(404);
      expect(failureCode(ghostSend)).toBe(APP_ERROR_CODES.NOT_FOUND);

      // 前置校验三连拒均不得触碰投递腿：logs 保持全空留痕
      const logsClean = await apiCallRaw<WebhookLogsEntry>(page, 'GET', `/webhooks/${id}/logs`);
      expect(logsClean.last_status, '三连拒全部先于 mark_sending，不得有任何留痕').toBeNull();
      expect(logsClean.last_triggered_at).toBeNull();
      expect(logsClean.retry_count).toBe(0);

      // 合法载荷走到投递腿：回环 URL 被卫兵拦截，投递失败当前以 INTERNAL_ERROR 信封上抛
      const sent = await apiCallExpectFail(page, 'POST', '/webhooks/integrations/dingtalk/send', {
        integration_id: id,
        message_type: 'markdown',
        title: `契约标题${tag}`,
        content: `正文${tag}`,
      });
      expect(sent.status, `拦截后的外发失败当前契约为 500，实际 ${JSON.stringify(sent)}`).toBe(500);
      expect(failureCode(sent)).toBe('INTERNAL_ERROR');
      expect(sent.message).toBe('服务器内部错误');
      const logsSent = await apiCallRaw<WebhookLogsEntry>(page, 'GET', `/webhooks/${id}/logs`);
      expect(logsSent.last_status, '投递腿必须先落 SENDING 留痕').toBe('SENDING');
      expect(logsSent.retry_count, '拦截路径不得计入重试次数').toBe(0);
    } finally {
      await tryCleanup(
        page,
        'DELETE',
        `/webhooks/integrations/integration/${id}`,
        'webhook-integration'
      );
    }
  });

  test('入站回调 - 三签名拒绝腿全 401 且发生在入库之前', async ({ page }) => {
    const probeBody = JSON.stringify({
      event_type: `e2e.probe.${genCode('WHD')}`,
      payload: { probe: true },
    });
    // 回调走 page.request 直连（需自定义签名头），每次发送前现取会话内活性 CSRF
    const freshCsrf = async (): Promise<string> =>
      (await page.context().cookies()).find(c => c.name === 'csrf_token')?.value ?? '';

    const noSig = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/webhooks/integrations/callback`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Requested-With': 'XMLHttpRequest',
          'X-CSRF-Token': await freshCsrf(),
        },
        data: probeBody,
      }
    );
    expect(noSig.status(), `缺签名头必须 401，实际 ${noSig.status()}`).toBe(401);

    const badHex = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/webhooks/integrations/callback`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Requested-With': 'XMLHttpRequest',
          'X-CSRF-Token': await freshCsrf(),
          'X-Webhook-Signature': 'sha256=not-hex',
        },
        data: probeBody,
      }
    );
    expect(badHex.status(), `非法十六进制签名必须 401，实际 ${badHex.status()}`).toBe(401);

    const wrongSig = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/webhooks/integrations/callback`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Requested-With': 'XMLHttpRequest',
          'X-CSRF-Token': await freshCsrf(),
          'X-Webhook-Signature': 'a'.repeat(64),
        },
        data: probeBody,
      }
    );
    expect(wrongSig.status(), `错误签名必须 401，实际 ${wrongSig.status()}`).toBe(401);
    const body = (await wrongSig.json()) as { code?: string; message?: string };
    expect(body.code, '回调拒绝机器码必须为 UNAUTHORIZED').toBe('UNAUTHORIZED');
    expect(body.message, '未认证文案必须为脱敏常量').toBe('未授权');
  });
});
