// AI 分析 E2E 套件 — 02 质量预测
// 创建时间: 2026-08-19
// 覆盖范围：AI 质量预测创建 → 确认处理
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, tryCleanup } from '../flow/helpers';

/**
 * 02-04 造数据前置：创建一条"未确认"质量预测（is_acknowledged=false），
 * 令 quality-prediction.vue:449 的确认按钮（v-if="!row.is_acknowledged"）渲染。
 * 后端 POST /ai/quality-predictions body { request }（CreateQualityPredDto），
 * 落库恒置 is_acknowledged=false（ai_extend_service.rs:491），返回 { id, response }。
 * product_id 可选（全局预测），无需关联产品即可确认。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function seedUnacknowledgedPrediction(
  page: import('@playwright/test').Page
): Promise<number> {
  const created = await apiCall<{ id?: number }>(page, 'POST', '/ai/quality-predictions', {
    request: { inspection_type: 'all', window_days: 90 },
  });
  const id = created.data?.id;
  if (!id) throw new Error(`创建质量预测失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/ai/quality-predictions/${id}`, label: 'ai_quality_prediction' });
  return id;
}

test.describe('02 AI 质量预测', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 进入质量预测页面', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    await expect(page.getByText(/质量预测/)).toBeVisible({ timeout: 30000 });
  });

  test('02-02 新建质量预测', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    await page.getByRole('button', { name: /新建|预测/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/产品/).fill('1');
    await page.getByLabel(/检验类型/).click();
    await page.getByRole('option').first().click();
    await page
      .getByRole('button', { name: /确认|提交/ })
      .last()
      .click();
    await expect(page.getByText(/创建成功|保存成功|预测完成/)).toBeVisible({
      timeout: 30000,
    });
  });

  test('02-03 质量预测列表可正常加载', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('02-04 未确认预测可确认处理', async ({ page }) => {
    // 假绿清零：原 `const ackBtn = page.getByRole('link', { name: /确认/ }); if (await ackBtn.isVisible())`
    // 两处恒空转——① 无未确认数据时按钮不渲染；② 确认控件是 el-button（role=button），
    // getByRole('link') 永远匹配不到。且 handleAck（quality-prediction.vue:121）无二次确认弹窗，
    // 原 `getByRole('button', { name: /确定/ })` 也点错目标。改为真实造未确认预测 →
    // 定位自身行 → 硬断言确认按钮渲染 → 点击 → 断言真实 toast「确认成功」。
    const id = await seedUnacknowledgedPrediction(page);
    await page.goto('/ai-extend/quality-prediction');
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
    const row = page
      .getByRole('row')
      .filter({ hasText: String(id) })
      .first();
    const ackBtn = row.getByRole('button', { name: '确认', exact: true });
    await expect(ackBtn, `未确认预测 ${id} 的「确认」按钮应渲染`).toBeVisible({ timeout: 30000 });
    await ackBtn.click();
    await expect(page.getByText(/确认成功/)).toBeVisible({ timeout: 30000 });
  });
});
