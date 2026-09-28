// AI 分析 E2E 套件 — 02 质量预测
// 创建时间: 2026-08-19
// 覆盖范围：AI 质量预测创建 → 确认处理
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, tryCleanup } from '../flow/helpers';
import { pickSelectIn } from '../flow/ui-helpers';

// AI 推理端点有真实的按用户限流（middleware/rate_limit.rs::rate_limit_ai_endpoint，
// 10 req/min/user，整段 ai_extend 路由含 GET 列表/详情/acknowledge 都计入同一 user 桶，
// 该限流不受 is_production() 保护，CI 里始终生效）。extras 分片 --workers=2 文件间并行，
// ai/01 与 ai/02 两条链共享同一 storageState 账号（e2e_admin_s{shard}，admin 角色，故 POST
// 非 403），两条链的 GET + POST 合计可短时超过 10 req/min → POST 返回 429。
// 限流是安全设计，严禁为过测关闭/放宽。合规做法：仅对 429 按其 Retry-After 头动态等待到固定
// 窗口重置后重试。上一版用逐字相同的 61s 定长退避，两条并行链锁步（同时睡醒→同时再发→再次
// 一起击穿窗口），耗尽尝试后成功 toast 永不出；改为 Retry-After + 5-15s 随机抖动错峰唤醒。
// 非 429 的真实错误一律不重试，让断言如实失败。累计退避封顶 300s，落在用例 420s 超时内。
const AI_BACKOFF_BUDGET_MS = 300_000;

/** 取 429 响应的 Retry-After 头（秒），缺省按固定窗口 60s；加 5-15s 抖动错峰唤醒。 */
async function waitUntilWindowReset(
  page: import('@playwright/test').Page,
  resp: import('@playwright/test').Response
): Promise<void> {
  const retryAfterSec = Number(resp.headers()['retry-after'] ?? '60') || 60;
  const jitterMs = 5_000 + Math.floor(Math.random() * 10_000);
  await page.waitForTimeout(retryAfterSec * 1000 + jitterMs);
}

/** apiCall 无法直接拿到响应头，按固定窗口 60s + 5-15s 抖动错峰等待。 */
async function waitWindowResetBlind(page: import('@playwright/test').Page): Promise<void> {
  const jitterMs = 5_000 + Math.floor(Math.random() * 10_000);
  await page.waitForTimeout(60_000 + jitterMs);
}

/** 判定 apiCall 抛出的错误是否为限流 429（统一失败信封 code=TOO_MANY_REQUESTS，HTTP 429）。 */
function isRateLimited(e: unknown): boolean {
  const err = e as { status?: number; message?: string };
  return err?.status === 429 || /TOO_MANY_REQUESTS|过于频繁/.test(err?.message ?? '');
}

/**
 * 02-04 造数据前置：创建一条"未确认"质量预测（is_acknowledged=false），
 * 令 quality-prediction.vue 的确认按钮（v-if="!row.is_acknowledged"）渲染。
 * 后端 POST /ai/quality-predictions body { request }（CreateQualityPredDto），
 * 落库恒置 is_acknowledged=false，返回 { id, response }。product_id 可选（全局预测）。
 *
 * 限流合规：不再在 beforeAll/seed 里循环猛 POST——每次运行只 POST 一条；命中 429 按其
 * Retry-After（固定窗口 60s）退避到窗口重置后重试，绝不关闭/放宽后端限流。新建记录是
 * created_at 最新的一条，UI 列表按 created_at desc 展示 → 恒在第 1 页首行，行定位确定。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function seedUnacknowledgedPrediction(
  page: import('@playwright/test').Page
): Promise<number> {
  const deadline = Date.now() + AI_BACKOFF_BUDGET_MS;
  for (;;) {
    try {
      const created = await apiCall<{ id?: number }>(page, 'POST', '/ai/quality-predictions', {
        request: { inspection_type: 'all', window_days: 90 },
      });
      const id = created.data?.id;
      if (!id) throw new Error(`创建质量预测失败：${JSON.stringify(created)}`);
      CLEANUP.push({ path: `/ai/quality-predictions/${id}`, label: 'ai_quality_prediction' });
      return id;
    } catch (e) {
      if (isRateLimited(e)) {
        // 尊重限流：等窗口重置后重试（错峰抖动，避免与 ai/01 并行链锁步）；不关闭/放宽限流。
        // 非 429 的真实错误直接暴露。退避总时长封顶于用例超时。
        if (Date.now() >= deadline) {
          throw new Error('seedUnacknowledgedPrediction 累计退避超预算仍被限流，放弃');
        }
        await waitWindowResetBlind(page);
        continue;
      }
      throw e;
    }
  }
}

/**
 * 对 UI 触发的 AI POST 做 429 退避重试：点击按钮 → 等待对应 POST 响应；命中 429 则按其
 * Retry-After 头动态等待到窗口重置（加抖动错峰）后重新点击（前端失败分支不关弹窗，可直接重试）。
 * 非 429 停止重试，交由成功断言如实失败。累计退避封顶于用例 420s 超时内。
 */
async function clickSubmitWithBackoff(
  page: import('@playwright/test').Page,
  root: import('@playwright/test').Locator | import('@playwright/test').Page,
  buttonName: string | RegExp,
  postUrlMatch: string
): Promise<void> {
  const deadline = Date.now() + AI_BACKOFF_BUDGET_MS;
  for (;;) {
    const respPromise = page.waitForResponse(
      r => r.request().method() === 'POST' && r.url().includes(postUrlMatch),
      { timeout: 60_000 }
    );
    await root.getByRole('button', { name: buttonName }).click();
    const resp = await respPromise;
    if (resp.status() === 429) {
      if (Date.now() >= deadline) {
        break;
      }
      await waitUntilWindowReset(page, resp);
      continue;
    }
    break;
  }
}

test.describe('02 AI 质量预测', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 进入质量预测页面', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    await expect(page.getByRole('heading', { name: /质量预测/ })).toBeVisible({ timeout: 30000 });
  });

  test('02-02 新建质量预测', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    // 页面顶部有「新建预测」「批量预测」两个含「预测」的按钮，getByRole('button',{name:/新建|预测/})
    // 会 strict-mode 命中 2 个 → 改精确「新建预测」（aiExtend.qualityPrediction.newPredict）。
    await page.getByRole('button', { name: '新建预测' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 「产品 ID」「检验类型」label 与筛选栏同名（colProductId/colInspectionType）→ 限定到 .el-dialog；
    // 提交按钮真实文案「开始预测」（aiExtend.qualityPrediction.generate），原用例误写「确认/提交」。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('产品 ID').fill('1');
    // 检验类型是 el-select（quality-prediction.vue:500-507）：用共享 helper pickSelectIn 以
    // root(dlg)+精确 label 作用域打开下拉并选首项，消除直接点 readonly combobox input 的
    // 「element is not stable / placeholder 拦 pointer」假红。
    await pickSelectIn(dlg, page, '检验类型');
    // 成功 toast 为 aiExtend.qualityPrediction.predictSuccess「预测完成（风险…）」（zh-CN.ts:1678），
    // 原 getByText(/预测完成/) 会命中任意含该词的页面文本且 toast 短暂 → 锚定真实 .el-message 容器。
    // POST /ai/quality-predictions 可能因按用户限流返回 429 → 退避重试（不放宽限流）。
    await clickSubmitWithBackoff(page, dlg, '开始预测', '/ai/quality-predictions');
    await expect(page.locator('.el-message').filter({ hasText: '预测完成' })).toBeVisible({
      timeout: 30_000,
    });
  });

  test('02-03 质量预测列表可正常加载', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('02-04 未确认预测可确认处理', async ({ page }) => {
    // 假绿清零：原 `const ackBtn = page.getByRole('link', { name: /确认/ }); if (await ackBtn.isVisible())`
    // 两处恒空转——① 无未确认数据时按钮不渲染；② 确认控件是 el-button（role=button），
    // getByRole('link') 永远匹配不到。且 handleAck（quality-prediction.vue:121）无二次确认弹窗，
    // 原 `getByRole('button', { name: /确定/ })` 也点错目标。改为真实取未确认预测 →
    // 定位自身行 → 硬断言确认按钮渲染 → 点击 → 断言真实 toast「确认成功」。
    // 取数走 seedUnacknowledgedPrediction：每次仅新建一条未确认预测，命中 429 退避重试（不放宽限流）。
    const id = await seedUnacknowledgedPrediction(page);
    await page.goto('/ai-extend/quality-prediction');
    // 「AI 质量预测列表」aria-label 同时挂在 el-table 根与分页组件上 → getByLabel strict 命中 2 个。
    // 这里只需断言列表容器可见，用 .first() 收敛（不放宽存在性）。
    await expect(page.getByLabel('AI 质量预测列表').first()).toBeVisible({ timeout: 30000 });
    const row = page
      .getByRole('row')
      .filter({ hasText: String(id) })
      .first();
    const ackBtn = row.getByRole('button', { name: '确认', exact: true });
    await expect(ackBtn, `未确认预测 ${id} 的「确认」按钮应渲染`).toBeVisible({ timeout: 30000 });
    // acknowledge POST 同受按用户限流约束：429 退避重试；handleAck 成功才关按钮，失败保留可重试。
    await clickSubmitWithBackoff(page, row, '确认', '/acknowledge');
    await expect(page.getByText(/确认成功/)).toBeVisible({ timeout: 30000 });
  });
});
