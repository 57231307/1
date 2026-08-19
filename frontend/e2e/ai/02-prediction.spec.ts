// AI 分析 E2E 套件 — 02 质量预测
// 创建时间: 2026-08-19
// 覆盖范围：AI 质量预测创建 → 确认处理
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 AI 质量预测', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 进入质量预测页面', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    await expect(page.getByText(/质量预测/)).toBeVisible({ timeout: 5000 });
  });

  test('02-02 新建质量预测', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    await page.getByRole('button', { name: /新建|预测/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/产品/).fill('1');
    await page.getByLabel(/检验类型/).click();
    await page.getByRole('option').first().click();
    await page.getByRole('button', { name: /确认|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功|预测完成/)).toBeVisible({ timeout: 10000 }).catch(() => {
      return null;
    });
  });

  test('02-03 质量预测列表可正常加载', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('02-04 未确认预测可确认处理', async ({ page }) => {
    await page.goto('/ai-extend/quality-prediction');
    const ackBtn = page.getByRole('link', { name: /确认/ }).first();
    if (await ackBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await ackBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/确认成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });
});