import { test, expect, type Page } from '../diagnose-fixture';
import {
  API_BASE,
  API_PREFIX,
  APP_ERROR_CODES,
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
} from './helpers';

/**
 * 124 webhooks 族十四端点的真实契约（核心面 CRUD/test/retry/logs + 集成面 CRUD/test-integration/callback 签名门与发送面校验腿）
 *
 * 守的是什么：webhook 行的落库回读、软删语义、执行状态持久化、SSRF 守卫的显式拒绝分支、
 * callback 的签名鉴权门与发送端点先于出网的所有参校分支。判据来自 handler 与服务层源码实核：
 * - POST /webhooks（webhook_handler.rs:58）：CreateWebhookRequest{name,url,events[],secret?}；
 *   出参 WebhookResponse{id,name,url,events(逗号拼接串),is_active}，不回显 secret。
 *   URL 先过 ssrf_guard::validate_url（utils/ssrf_guard.rs:33）：非 http/https → 400 点名
 *   「URL 协议不允许：ftp（仅允许 http/https）」（validation_displayable 可外显）；
 *   loopback/内网/元数据 → 400 VALIDATION_ERROR（validation 脱敏族，只钉状态与机器码）。
 * - GET /webhooks（webhook_service.rs:103）：仅 is_active=true 且（系统级或本人）可见，
 *   软删后行不出现在列表。
 * - GET /webhooks/{id}/logs（webhook_handler.rs:232）：新建行 last_triggered_at/last_status
 *   均为 null、retry_count 0、max_retry_count 5（MAX_RETRY_COUNT）；触发过测试后
 *   last_status 落入 FAILED/ERROR/FAILED_PERMANENT/SUCCESS 终态族且 last_triggered_at 非空
 *   （mark_webhook_sending 先置 SENDING 并持久化 last_payload，persist_final_status 收终态）。
 * - POST /webhooks/{id}/test（:117）：10 次/分钟限流；出参 response_body 恒被改写为
 *   「出于安全原因，已隐藏响应内容」（SSRF 缓解，与网络结果无关，可断字面量）；
 *   禁用的 webhook 触发 → 400 BUSINESS_ERROR（ensure_webhook_active）。
 * - POST /webhooks/{id}/retry（:153）：无 last_payload 时 → 400 BUSINESS_ERROR；
 *   test 之后 last_payload 已持久化，retry 走成功信封（重投原始 payload）。
 * - DELETE /webhooks/{id}（:89）：软删（service 置 is_active=false），删除响应文案「删除成功」；
 *   删后 logs 仍可读且 is_active=false（库侧可回读的软删事实）。
 * - 集成面（webhook_integration_handler.rs）：create（:120）直写 webhook 表且 platform 回显入参；
 *   list（:85）只见 is_active=true 且 platform 恒为 GENERIC；update（:180）部分字段覆盖、
 *   非所有者 403（本用例只动自建行）；delete（:163）复用软删。
 *   test-integration（:426）与核心面 test 同出参改写规则。
 * - callback（:375）：先取 X-Webhook-Signature 头，缺失 → 401；非法 hex 与不匹配签名均 401
 *   （unauthorized 脱敏族，只钉 401+UNAUTHORIZED）。签名密钥 state.webhook_secret 仅服务端持有，
 *   e2e 无法也不应伪造出合法签名——合法签名 200 腿豁免（无密钥），签名门本身即该端点的鉴权契约。
 * - wechat/dingtalk 发送（:241/:306）：出网前的确定性分支全部钉死——content 空串 400 BAD_REQUEST、
 *   message_type 非 text/markdown 400 BAD_REQUEST、integration 不存在 404 NOT_FOUND。
 *   200-SENT 成功腿豁免：需真实出公网（CI 无外网），禁止以「200 通过」式假断言凑绿。
 *
 * 出站 URL 一律用 TEST-NET-3 保留段（203.0.113.0/24，RFC 5737）：IP 字面量让 validate_url
 * 跳过 DNS 解析（ssrf_guard.rs:62-72），任何环境都不可路由，投递必失败——
 * 因此「test/retry 之后 last_status 属失败族」是跨环境确定断言，不断具体失败子族。
 */

type WebhookRow = {
  id: number;
  name: string;
  url: string;
  events: string;
  is_active: boolean;
};

type WebhookLogEntry = {
  id: number;
  name: string;
  url: string;
  events: string;
  is_active: boolean;
  last_triggered_at: string | null;
  last_status: string | null;
  retry_count: number;
  max_retry_count: number;
};

type DeliveryResult = {
  success: boolean;
  status_code?: number | null;
  response_body?: string | null;
  error?: string | null;
};

type IntegrationRow = {
  id: number;
  name: string;
  platform: string;
  webhook_url: string;
  is_active: boolean;
  last_triggered_at: string | null;
  last_status: string | null;
  created_at: string;
};

// 出站目标 URL：IP 字面量免 DNS，任何网络形态都不可路由，只用于验证投递链的状态持久化。
const TESTNET_BASE = 'https://203.0.113.7/e2e-zero-cov';

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

// 双提交 CSRF 的裸请求（callback 需自定义签名头，apiCall 族不带自定义头）。
async function postJsonRaw(
  page: Page,
  path: string,
  body: unknown,
  extraHeaders: Record<string, string>
): Promise<number> {
  const cookies = await page.context().cookies();
  const csrf = cookies.find(c => c.name === 'csrf_token');
  if (!csrf) {
    throw new Error('csrf_token cookie not found — loginViaUI 之后应存在');
  }
  const res = await page.request.fetch(`${API_BASE}${API_PREFIX}${path}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      'X-CSRF-Token': csrf.value,
      ...extraHeaders,
    },
    data: JSON.stringify(body),
    timeout: 60_000,
  });
  return res.status();
}

test.describe.serial('124 webhooks 核心面与集成面真实契约', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('SSRF 守卫显式拒绝：非法协议点名、loopback 400', async ({ page }) => {
    const ftp = await apiCallExpectFail(page, 'POST', '/webhooks', {
      name: `e2c-ftp-${uniqTag()}`,
      url: 'ftp://example.invalid/x',
      events: ['test'],
    });
    expect(ftp.status).toBe(400);
    expect(ftp.code).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(ftp.message).toBe('URL 协议不允许：ftp（仅允许 http/https）');

    const loopback = await apiCallExpectFail(page, 'POST', '/webhooks', {
      name: `e2c-lb-${uniqTag()}`,
      url: 'http://127.0.0.1:9/hook',
      events: ['test'],
    });
    expect(loopback.status).toBe(400);
    expect(loopback.code).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    const blockedHost = await apiCallExpectFail(page, 'POST', '/webhooks', {
      name: `e2c-host-${uniqTag()}`,
      url: 'https://localhost/hook',
      events: ['test'],
    });
    expect(blockedHost.status).toBe(400);
    expect(blockedHost.code).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('创建-列表-日志-重试前置拒绝-测试后状态持久化-软删回读全链', async ({ page }) => {
    const name = `e2c-core-${uniqTag()}`;
    const created = await apiCallRaw<WebhookRow>(page, 'POST', '/webhooks', {
      name,
      url: `${TESTNET_BASE}/${name}`,
      events: ['test', 'order.created'],
      secret: 'e2c-secret-not-echoed',
    });
    expect(created.id).toBeGreaterThan(0);
    expect(created.name).toBe(name);
    expect(created.events).toBe('test,order.created');
    expect(created.is_active).toBe(true);
    expect(created).not.toHaveProperty('secret');

    const listed = await apiCallRaw<WebhookRow[]>(page, 'GET', '/webhooks');
    expect(listed.some(w => w.id === created.id)).toBe(true);

    const freshLog = await apiCallRaw<WebhookLogEntry>(page, 'GET', `/webhooks/${created.id}/logs`);
    expect(freshLog.last_triggered_at).toBeNull();
    expect(freshLog.last_status).toBeNull();
    expect(freshLog.retry_count).toBe(0);
    expect(freshLog.max_retry_count).toBe(5);

    // 从未触发过时 last_payload，重试必须被拒（批次 251 的显式业务门）
    const retryBeforeTest = await apiCallExpectFail(page, 'POST', `/webhooks/${created.id}/retry`);
    expect(retryBeforeTest.status).toBe(400);
    expect(retryBeforeTest.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const testRes = await apiCall<DeliveryResult>(page, 'POST', `/webhooks/${created.id}/test`);
    expect(testRes.code).toBe(200);
    expect(typeof testRes.data.success).toBe('boolean');
    // SSRF 缓解改写与网络结果无关——恒为固定脱敏文案
    expect(testRes.data.response_body).toBe('出于安全原因，已隐藏响应内容');

    const afterTest = await apiCallRaw<WebhookLogEntry>(
      page,
      'GET',
      `/webhooks/${created.id}/logs`
    );
    expect(afterTest.last_triggered_at).not.toBeNull();
    expect(['FAILED', 'ERROR', 'FAILED_PERMANENT', 'SUCCESS']).toContain(afterTest.last_status);

    // test 已持久化 last_payload，retry 走成功信封重投
    const retryRes = await apiCall<DeliveryResult>(page, 'POST', `/webhooks/${created.id}/retry`);
    expect(retryRes.code).toBe(200);
    expect(typeof retryRes.data.success).toBe('boolean');
    expect(retryRes.data.response_body).toBe('出于安全原因，已隐藏响应内容');

    const del = await apiCall(page, 'DELETE', `/webhooks/${created.id}`);
    expect(del.code).toBe(200);
    expect(del.message).toBe('删除成功');

    const afterDelete = await apiCallRaw<WebhookRow[]>(page, 'GET', '/webhooks');
    expect(afterDelete.some(w => w.id === created.id)).toBe(false);

    // 软删的库侧事实：行还在，logs 可读且 is_active=false
    const deletedLog = await apiCallRaw<WebhookLogEntry>(
      page,
      'GET',
      `/webhooks/${created.id}/logs`
    );
    expect(deletedLog.is_active).toBe(false);

    // 已禁用行再触发测试 → 400 BUSINESS_ERROR
    const testDeleted = await apiCallExpectFail(page, 'POST', `/webhooks/${created.id}/test`);
    expect(testDeleted.status).toBe(400);
    expect(testDeleted.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('事件订阅不匹配时测试触发被拒', async ({ page }) => {
    const name = `e2c-evt-${uniqTag()}`;
    const created = await apiCallRaw<WebhookRow>(page, 'POST', '/webhooks', {
      name,
      url: `${TESTNET_BASE}/${name}`,
      events: ['order.created'],
    });
    // 订阅里没有 "test" 事件，test_webhook 触发事件固定为 test → 事件门 400 BUSINESS_ERROR
    const testRes = await apiCallExpectFail(page, 'POST', `/webhooks/${created.id}/test`);
    expect(testRes.status).toBe(400);
    expect(testRes.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    await apiCall(page, 'DELETE', `/webhooks/${created.id}`);
  });

  test('集成面创建-列表-停用-启用-改名读回与软删', async ({ page }) => {
    const name = `e2c-int-${uniqTag()}`;
    const created = await apiCallRaw<IntegrationRow>(page, 'POST', '/webhooks/integrations', {
      name,
      platform: 'E2E-TEST',
      webhook_url: `${TESTNET_BASE}/${name}`,
      secret: null,
      is_active: true,
    });
    expect(created.id).toBeGreaterThan(0);
    expect(created.name).toBe(name);
    expect(created.platform).toBe('E2E-TEST');
    expect(created.is_active).toBe(true);
    expect(created.last_status).toBeNull();

    const listed = await apiCallRaw<IntegrationRow[]>(page, 'GET', '/webhooks/integrations');
    const mine = listed.find(r => r.id === created.id);
    expect(mine).toBeDefined();
    const found = mine as unknown as IntegrationRow;
    // 列表侧 platform 恒为 GENERIC（集成面虚拟字段，与 webhook_handler 无该列同源）
    expect(found.platform).toBe('GENERIC');

    const updated = await apiCallRaw<IntegrationRow>(
      page,
      'PUT',
      `/webhooks/integrations/${created.id}`,
      { name: `${name}-renamed`, is_active: false }
    );
    expect(updated.name).toBe(`${name}-renamed`);
    expect(updated.is_active).toBe(false);
    expect(updated.platform).toBe('GENERIC');

    const afterDisable = await apiCallRaw<IntegrationRow[]>(page, 'GET', '/webhooks/integrations');
    expect(afterDisable.some(r => r.id === created.id)).toBe(false);

    const reactivated = await apiCallRaw<IntegrationRow>(
      page,
      'PUT',
      `/webhooks/integrations/${created.id}`,
      { is_active: true }
    );
    expect(reactivated.is_active).toBe(true);

    const afterEnable = await apiCallRaw<IntegrationRow[]>(page, 'GET', '/webhooks/integrations');
    expect(afterEnable.some(r => r.id === created.id)).toBe(true);

    const del = await apiCall(page, 'DELETE', `/webhooks/integrations/integration/${created.id}`);
    expect(del.code).toBe(200);
    expect(del.message).toBe('Webhook 集成已删除');
    const afterDelete = await apiCallRaw<IntegrationRow[]>(page, 'GET', '/webhooks/integrations');
    expect(afterDelete.some(r => r.id === created.id)).toBe(false);

    // 未知 id 的更新 → 404 NOT_FOUND（存在性先于所有权门）
    const missingUpdate = await apiCallExpectFail(page, 'PUT', '/webhooks/integrations/987654', {
      name: 'ghost',
    });
    expect(missingUpdate.status).toBe(404);
    expect(missingUpdate.code).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('test-integration 与核心 test 同脱敏改写规则', async ({ page }) => {
    const name = `e2c-testint-${uniqTag()}`;
    const created = await apiCallRaw<IntegrationRow>(page, 'POST', '/webhooks/integrations', {
      name,
      platform: 'E2E-TEST',
      webhook_url: `${TESTNET_BASE}/${name}`,
      is_active: true,
    });
    const res = await apiCall<DeliveryResult>(
      page,
      'POST',
      `/webhooks/integrations/test-integration/${created.id}`
    );
    expect(res.code).toBe(200);
    expect(res.message).toBe('测试消息已发送');
    expect(typeof res.data.success).toBe('boolean');
    expect(res.data.response_body).toBe('出于安全原因，已隐藏响应内容');
    await apiCall(page, 'DELETE', `/webhooks/integrations/integration/${created.id}`);
  });

  test('callback 签名门：缺头、非法 hex、伪造签名全部 401', async ({ page }) => {
    const payload = { event_type: 'e2e.zero.cov', payload: { a: 1 } };

    // 缺 X-Webhook-Signature 头（apiCallExpectFail 不带自定义头，正是该腿的形态）
    // 机器码取 error.rs CODE_UNAUTHORIZED 字面值：APP_ERROR_CODES 登记表尚无 UNAUTHORIZED 键，
    // helpers.ts 正被夹具车道单写，本支不并发改共享登记表，故此处就地字面断言。
    const noHeader = await apiCallExpectFail(
      page,
      'POST',
      '/webhooks/integrations/callback',
      payload
    );
    expect(noHeader.status).toBe(401);
    expect(noHeader.code).toBe('UNAUTHORIZED');

    // 非法 hex 签名 → 401（无效的签名格式）
    const badHex = await postJsonRaw(page, '/webhooks/integrations/callback', payload, {
      'X-Webhook-Signature': 'zz-not-hex',
    });
    expect(badHex).toBe(401);

    // 合法 hex 但不匹配（密钥仅服务端持有，e2e 无法伪造合法签名——合法签名 200 腿豁免）
    const mismatch = await postJsonRaw(page, '/webhooks/integrations/callback', payload, {
      'X-Webhook-Signature': 'deadbeef'.repeat(8),
    });
    expect(mismatch).toBe(401);
  });

  test('企微与钉钉发送面出网前的确定性拒绝：空内容、非法类型、未知集成', async ({ page }) => {
    // 两支端点逐条写死字面路径：路径常量经循环变量传参时，覆盖判定器按字面量取不到位点
    const wechatEmpty = await apiCallExpectFail(
      page,
      'POST',
      '/webhooks/integrations/wechat/send',
      {
        integration_id: 1,
        message_type: 'text',
        content: '',
      }
    );
    expect(wechatEmpty.status).toBe(400);
    expect(wechatEmpty.code).toBe(APP_ERROR_CODES.BAD_REQUEST);

    const wechatBadType = await apiCallExpectFail(
      page,
      'POST',
      '/webhooks/integrations/wechat/send',
      {
        integration_id: 1,
        message_type: 'xml',
        content: 'e2c',
      }
    );
    expect(wechatBadType.status).toBe(400);
    expect(wechatBadType.code).toBe(APP_ERROR_CODES.BAD_REQUEST);

    const wechatMissingIntegration = await apiCallExpectFail(
      page,
      'POST',
      '/webhooks/integrations/wechat/send',
      {
        integration_id: 987654,
        message_type: 'text',
        content: 'e2c',
      }
    );
    expect(wechatMissingIntegration.status).toBe(404);
    expect(wechatMissingIntegration.code).toBe(APP_ERROR_CODES.NOT_FOUND);

    const dingtalkEmpty = await apiCallExpectFail(
      page,
      'POST',
      '/webhooks/integrations/dingtalk/send',
      {
        integration_id: 1,
        message_type: 'text',
        content: '',
      }
    );
    expect(dingtalkEmpty.status).toBe(400);
    expect(dingtalkEmpty.code).toBe(APP_ERROR_CODES.BAD_REQUEST);

    const dingtalkBadType = await apiCallExpectFail(
      page,
      'POST',
      '/webhooks/integrations/dingtalk/send',
      {
        integration_id: 1,
        message_type: 'markdown_bad',
        content: 'e2c',
      }
    );
    expect(dingtalkBadType.status).toBe(400);
    expect(dingtalkBadType.code).toBe(APP_ERROR_CODES.BAD_REQUEST);

    const dingtalkMissingIntegration = await apiCallExpectFail(
      page,
      'POST',
      '/webhooks/integrations/dingtalk/send',
      {
        integration_id: 987654,
        message_type: 'text',
        content: 'e2c',
      }
    );
    expect(dingtalkMissingIntegration.status).toBe(404);
    expect(dingtalkMissingIntegration.code).toBe(APP_ERROR_CODES.NOT_FOUND);
    // 200-SENT 成功腿豁免：需真实出公网且对端可路由，CI 无外网，不写「200 通过」式假断言。
  });
});
