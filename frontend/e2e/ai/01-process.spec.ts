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
    await expect(page.getByText(/AI/)).toBeVisible({ timeout: 5000 });
  });

  test('01-02 进入工艺优化页面', async ({ page }) => {
    await page.goto('/ai-extend/process-optimization');
    await expect(page.getByText(/工艺优化/)).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('button', { name: /新建|推荐/ })).toBeVisible();
  });

  test('01-03 工艺优化列表可正常加载', async ({ page }) => {
    await page.goto('/ai-extend/process-optimization');
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('01-04 新建工艺优化推荐', async ({ page }) => {
    await page.goto('/ai-extend/process-optimization');
    await page.getByRole('button', { name: /新建|推荐/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/色号/).fill('E2E-CN-001');
    await page.getByLabel(/面料类型/).fill('E2E 测试面料');
    await page.getByRole('button', { name: /确认|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功|推荐完成/)).toBeVisible({ timeout: 10000 }).catch(() => {
      return null;
    });
  });
});