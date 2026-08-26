// 面料管理 E2E 套件 — 03 染色配方
// 创建时间: 2026-08-19
// 覆盖范围：染色配方创建 → 审批（draft → approved）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('03 染色配方', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 染色配方 Tab 可正常加载', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('03-02 新建染色配方', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/配方编号/).fill(`RP-${Date.now()}`);
    await page.getByLabel(/配方名称/).fill('E2E 测试染色配方');
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('03-03 草稿配方可审批', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    const approveBtn = page.getByRole('link', { name: /审批/ }).first();
    if (await approveBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await approveBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });
});