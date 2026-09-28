// AI 分析 E2E 套件 — 01 工艺优化
// 创建时间: 2026-08-19
// 覆盖范围：AI 工艺优化推荐创建 → 查看结果
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

// AI 推理端点有真实的按用户限流（backend middleware/rate_limit.rs::rate_limit_ai_endpoint，
// 10 req/min/user，整段 ai_extend 路由含 GET 列表都计数）。同分片同一账号在 AI 套件里连续
// 建单/翻页会短时超限，UI 建单 POST 返回 429 → submitCreate 落入 catch 弹「创建失败」而非
// 成功 toast（用例红）。限流是安全设计，严禁为过测放宽/关闭；合规做法是：仅对 429 按其
// Retry-After（固定窗口 60s）退避到窗口重置后重试，非 429 的真实错误一律不重试、让断言如实失败。
const AI_WINDOW_RESET_MS = 61_000;

async function submitCreateWithBackoff(
  page: import('@playwright/test').Page,
  dlg: import('@playwright/test').Locator,
  submitName: string,
  postUrlMatch: string,
  successText: string
): Promise<void> {
  for (let attempt = 0; attempt < 5; attempt++) {
    const respPromise = page.waitForResponse(
      r => r.request().method() === 'POST' && r.url().includes(postUrlMatch),
      { timeout: 60_000 }
    );
    await dlg.getByRole('button', { name: submitName }).click();
    const resp = await respPromise;
    if (resp.status() === 429) {
      // 尊重限流：等窗口重置后重试（submitCreate 失败时不关弹窗，可直接再次提交）
      await page.waitForTimeout(AI_WINDOW_RESET_MS);
      continue;
    }
    // 2xx 或真实业务错误（4xx/5xx 非 429）都停止重试；后者会让下方成功断言如实失败
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
