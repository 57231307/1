// AI 分析 E2E 套件 — 01 工艺优化
// 创建时间: 2026-08-19
// 覆盖范围：AI 工艺优化推荐创建 → 查看结果
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

// AI 推理端点有真实的按用户限流（backend middleware/rate_limit.rs::rate_limit_ai_endpoint，
// 10 req/min/user，整段 ai_extend 路由含 GET 列表/详情/acknowledge 都计入同一 user 桶，
// 且该限流不受 is_production() 保护——CI 里与生产一样始终生效）。extras 分片按用例 hash 混排
// 多目录并以 --workers=2 文件间并行，ai/01 与 ai/02 两条链共享同一 storageState 账号
// （e2e_admin_s{shard}，admin 角色，故 POST 非 403），两条链的 GET 列表 + POST 建单合计可短时
// 超过 10 req/min → 建单 POST 返回 429，submitCreate 落入 catch 弹「创建失败」而非成功 toast。
// 限流是安全设计，严禁为过测放宽/关闭。合规做法：仅对 429 按其 Retry-After 头动态等待到固定
// 窗口重置后重试——关键是用抖动打散两条并行链的唤醒时刻：上一版用逐字相同的 61s 定长退避，
// 两条链会锁步（同时睡醒→同时再发→再次一起击穿窗口），5 次耗尽后成功 toast 永不出。改为
// Retry-After + 宽抖动错峰唤醒，两链错开后必有一条落入真正空闲的窗口。非 429 的真实错误
// （4xx/5xx）一律不重试，让断言如实失败。累计退避封顶贴近用例 420s 超时（留出首次请求
// 与最后一次成功 toast 渲染的时间窗），使持续的桶竞争有更长清空机会——仍不改后端阈值、
// 不放宽成功断言，属既有 429 退避范式的错峰/预算强化。
const AI_BACKOFF_BUDGET_MS = 360_000;

/** 取 429 响应的 Retry-After 头（秒），缺省按固定窗口 60s；加 15-45s 宽抖动错峰唤醒。 */
async function waitUntilWindowReset(
  page: import('@playwright/test').Page,
  resp: import('@playwright/test').Response
): Promise<void> {
  const retryAfterSec = Number(resp.headers()['retry-after'] ?? '60') || 60;
  // 宽抖动（15-45s，较原 5-15s 显著拉大）进一步打散两条并行 AI 链的唤醒时刻，
  // 降低再次同刻击穿 10/min 窗口的概率；错峰是消除 flaky 的关键，不涉断言。
  const jitterMs = 15_000 + Math.floor(Math.random() * 30_000);
  await page.waitForTimeout(retryAfterSec * 1000 + jitterMs);
}

async function submitCreateWithBackoff(
  page: import('@playwright/test').Page,
  dlg: import('@playwright/test').Locator,
  submitName: string,
  postUrlMatch: string,
  successText: string
): Promise<void> {
  // 首次提交前随机错峰：ai/01 与 ai/02 两条并行链共享同一 storageState 用户的 10/min AI 桶，
  // 若同时在窗口内发起 GET 列表 + POST 建单会立即合计超限，首个 POST 直接 429。随机前置
  // 等待把两链的首发时刻打散到窗口不同相位，让其一先落入空闲窗——非退避、不放宽任何断言。
  await page.waitForTimeout(2_000 + Math.floor(Math.random() * 6_000));
  const deadline = Date.now() + AI_BACKOFF_BUDGET_MS;
  for (;;) {
    const respPromise = page.waitForResponse(
      r => r.request().method() === 'POST' && r.url().includes(postUrlMatch),
      { timeout: 60_000 }
    );
    await dlg.getByRole('button', { name: submitName }).click();
    const resp = await respPromise;
    if (resp.status() === 429) {
      // 仅退避到 Retry-After 指示的窗口重置之后；退避总时长封顶于用例超时。
      if (Date.now() >= deadline) {
        break;
      }
      // submitCreate 失败分支不关弹窗，可直接再次提交。
      await waitUntilWindowReset(page, resp);
      continue;
    }
    // 2xx 或真实业务错误（4xx/5xx 非 429）都停止重试；后者让下方成功断言如实失败。
    break;
  }
  await expect(page.locator('.el-message').filter({ hasText: successText })).toBeVisible({
    timeout: 30_000,
  });
}

test.describe('01 AI 工艺优化', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入 AI 分析页面', async ({ page }) => {
    await page.goto('/ai-extend');
    await expect(page.getByRole('heading', { name: /AI/ })).toBeVisible({ timeout: 30000 });
  });

  test('01-02 进入工艺优化页面', async ({ page }) => {
    await page.goto('/ai-extend/process-optimization');
    await expect(page.getByRole('heading', { name: /工艺优化/ })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /新建|推荐/ })).toBeVisible();
  });

  test('01-03 工艺优化列表可正常加载', async ({ page }) => {
    await page.goto('/ai-extend/process-optimization');
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('01-04 新建工艺优化推荐', async ({ page }) => {
    await page.goto('/ai-extend/process-optimization');
    await page.getByRole('button', { name: '新建推荐' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 「色号」「面料类型」label 在筛选栏与创建对话框中同名（aiExtend.process.colColorNo/colFabricType），
    // 不限定会 getByLabel strict-mode 命中 2 个 → 全部限定到 .el-dialog。
    // 提交按钮真实文案「生成推荐」（aiExtend.process.generate）；成功 toast 为
    // aiExtend.process.recommendSuccess「推荐成功（来源…）」，原用例误写「推荐完成」。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('色号').fill('E2E-CN-001');
    await dlg.getByLabel('面料类型').fill('E2E 测试面料');
    // 成功提示是 ElMessage（teleport 到 body 的 .el-message 容器），文案取
    // aiExtend.process.recommendSuccess「推荐成功（来源…）」（process-optimization.vue:141-146 调用；
    // zh-CN.ts:1753）。POST 可能因 AI 按用户限流返回 429，走退避重试（见 submitCreateWithBackoff）。
    await submitCreateWithBackoff(page, dlg, '生成推荐', '/ai/process-optimizations', '推荐成功');
  });
});
