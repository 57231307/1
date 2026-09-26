// AI 分析 E2E 套件 — 01 工艺优化
// 创建时间: 2026-08-19
// 覆盖范围：AI 工艺优化推荐创建 → 查看结果
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

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
    await dlg.getByRole('button', { name: '生成推荐' }).click();
    // 成功提示是 ElMessage（teleport 到 body 的 .el-message 容器），文案取
    // aiExtend.process.recommendSuccess「推荐成功（来源…）」（process-optimization.vue:141-146 调用；
    // zh-CN.ts:1736）。原 getByText(/推荐成功/) 会命中任意含该词的页面文本且 toast 短暂，
    // 改锚定真实 toast 容器 + 真实文案（仍要求成功提示出现，不放宽）。
    await expect(page.locator('.el-message').filter({ hasText: '推荐成功' })).toBeVisible({
      timeout: 30000,
    });
  });
});
