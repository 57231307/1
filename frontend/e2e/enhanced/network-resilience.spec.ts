/**
 * 网络韧性测试：真实网络中断 / 真实弱网 / 中断后自愈
 *
 * 异常源全部为真实链路，不使用任何响应伪造：
 * - 网络中断：浏览器上下文级 `context.setOffline(true)`，请求真实失败于
 *   net::ERR_INTERNET_DISCONNECTED，走 axios 真实错误拦截与真实重试路径。
 * - 弱网：Chromium DevTools Protocol `Network.emulateNetworkConditions` 施加真实链路延迟，
 *   请求真实发出、响应真实返回，仅链路被真实劣化（不限吞吐，避免拖垮 SPA bundle 加载）。
 *
 * 断言依据为 src/api/request.ts 的真实实现：
 * 无 response 的网络错误使 shouldRetry() 返回 true，幂等 GET 最多重试 3 次
 * （退避 min(1000*n + rand*1000, 5000)），穷尽后 showErrorOnce(getSafeErrorMessage())
 * 弹出无状态码兜底文案 '请求失败，请稍后重试'。
 *
 * HTTP 4xx/5xx 的前端处理不在本文件重复，其异常源同为真实链路且已覆盖：
 * - 403：33 垂直越权矩阵 / 33b 角色黑名单（真实低权账号登录）
 * - 全站不崩溃：各 flow spec 的 assertPageHealthy 含"零 5xx"门禁
 * 原实现以 mockApiError / mockNetworkFailure / simulateSlowNetwork 伪造上述响应，
 * 违反 E2E 真实数据强制 IR（2026-09-07），故整体重写而非保留 skip 与豁免标记。
 */
import { test, expect, type Page } from '../diagnose-fixture';
import {
  loginAsRole,
  trackPageHealth,
  assertPageHealthy,
  expectSingleToast,
} from '../flow/helpers';

const API_PREFIX = '/api/v1/erp';
/** 网络错误需经 3 次重试（最坏累计退避约 9s）才弹提示，留足以余量 */
const RETRY_SETTLE_TIMEOUT = 45_000;

/** 收集真实失败的网络请求，用于证明重试链路确实生效而非常驻真断言 */
function trackFailedRequests(page: Page): string[] {
  const failed: string[] = [];
  page.on('requestfailed', request => {
    failed.push(`${request.method()} ${request.url()} → ${request.failure()?.errorText ?? '?'}`);
  });
  return failed;
}

/** 收集真实成功的应用数据响应（须在触发动作之前注册，否则漏采） */
function trackOkApiResponses(page: Page): number[] {
  const ok: number[] = [];
  page.on('response', response => {
    if (response.url().includes(API_PREFIX) && response.status() === 200) ok.push(200);
  });
  return ok;
}

/** 点击侧边栏首个菜单项触发 SPA 路由切换，使应用自身发起真实数据请求。
 *  按 role 而非文案定位，避免中英 i18n 切换导致定位失败。 */
async function navigateViaFirstMenuItem(page: Page): Promise<void> {
  const firstMenuItem = page.locator('[role="menuitem"]').first();
  await expect(firstMenuItem, '侧边栏应渲染可点击菜单项').toBeVisible({ timeout: 30_000 });
  const label = (await firstMenuItem.innerText()).trim();
  console.log(`[network-resilience] 点击菜单项「${label}」触发 SPA 内导航与应用自身请求`);
  await firstMenuItem.click();
}

test.describe('网络韧性：真实网络中断', () => {
  test('中断后不崩溃、按真实重试策略提示，恢复后重新拉到数据', async ({ page, context }) => {
    const collector = trackPageHealth(page);
    const failedRequests = trackFailedRequests(page);

    await loginAsRole(page, 'admin');
    await page.goto('/sales');
    await expect(page.locator('[role="menuitem"]').first()).toBeVisible({ timeout: 30_000 });
    console.log('[network-resilience] admin 真实登录完成，/sales 列表页已在线加载');

    failedRequests.length = 0;
    await context.setOffline(true);
    console.log('[network-resilience] 上下文已置离线，触发应用自身数据请求');

    await navigateViaFirstMenuItem(page);

    // 断言 1：错误提示文案精确匹配 getSafeErrorMessage 的无状态码分支
    const toast = page.locator('.el-message--error');
    await expect(toast, '离线时应弹出错误提示').toBeVisible({ timeout: RETRY_SETTLE_TIMEOUT });
    const toastText = (await toast.first().innerText()).trim();
    console.log(`[network-resilience] 离线错误提示文案：「${toastText}」`);
    expect(toastText).toBe('请求失败，请稍后重试');
    await expectSingleToast(page, '请求失败，请稍后重试');

    // 断言 2：幂等 GET 的真实重试确实发生，且失败原因是真实中断而非伪造响应
    const apiFailures = failedRequests.filter(line => line.includes(API_PREFIX));
    console.log(
      `[network-resilience] 离线期间真实失败请求 ${apiFailures.length} 条，样例：` +
        apiFailures.slice(0, 2).join(' | ')
    );
    expect(
      apiFailures.length,
      '离线期间应至少出现 2 次真实失败，以证明重试链路生效'
    ).toBeGreaterThanOrEqual(2);
    expect(
      apiFailures.every(line => line.includes('ERR_INTERNET_DISCONNECTED')),
      '失败原因应为真实网络中断'
    ).toBe(true);

    // 断言 3：中断不产生未捕获异常与 5xx；离线场景浏览器自身的 console error
    // 属预期噪声，故允许 console 噪声但禁止 pageerror 与白屏
    await assertPageHealthy(page, collector, { allowConsoleWarn: true });

    // 断言 4：恢复网络后应用自愈，真实重新拉到数据
    await context.setOffline(false);
    const okResponses = trackOkApiResponses(page);
    console.log('[network-resilience] 已恢复在线，重新加载页面验证自愈');
    await page.goto('/sales', { timeout: 60_000 });
    console.log(`[network-resilience] 恢复后捕获 200 数据响应 ${okResponses.length} 条`);
    expect(okResponses.length, '恢复在线后应用应真实拉到数据').toBeGreaterThan(0);
    await expect(page.locator('[role="menuitem"]').first()).toBeVisible({ timeout: 30_000 });
    await assertPageHealthy(page, collector, { allowConsoleWarn: true });
  });
});

test.describe('网络韧性：真实弱网链路', () => {
  test('CDP 真实链路延迟下页面仍可加载且无未捕获异常', async ({ page, context }) => {
    const collector = trackPageHealth(page);
    await loginAsRole(page, 'admin');
    console.log('[network-resilience] admin 真实登录完成，准备经 CDP 施加真实链路延迟');

    const cdp = await context.newCDPSession(page);
    await cdp.send('Network.enable');
    await cdp.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: 800,
      downloadThroughput: -1,
      uploadThroughput: -1,
    });
    console.log('[network-resilience] CDP 已施加真实弱网：每请求额外延迟 800ms');

    const startedAt = Date.now();
    await page.goto('/sales', { timeout: 90_000 });
    const elapsed = Date.now() - startedAt;
    console.log(`[network-resilience] 弱网下 /sales 加载耗时 ${elapsed}ms`);

    // 弱网必须真实生效，否则该用例退化为普通加载测试
    expect(elapsed, '真实弱网未生效（耗时未超过注入的单请求延迟）').toBeGreaterThan(800);

    await expect(page.locator('[role="menuitem"]').first()).toBeVisible({ timeout: 60_000 });
    await assertPageHealthy(page, collector);

    await cdp.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: 0,
      downloadThroughput: -1,
      uploadThroughput: -1,
    });
    await cdp.detach();
    console.log('[network-resilience] 弱网条件已解除并断开 CDP 会话');
  });
});
