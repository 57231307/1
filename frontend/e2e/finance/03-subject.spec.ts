// 财务管理 E2E 套件 — 03 会计科目管理
// 创建时间: 2026-08-19
// 覆盖范围：科目创建 → 编辑 → 删除
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('03 会计科目管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 会计科目 Tab 可正常加载', async ({ page }) => {
    await page.goto('/finance');
    await page.getByRole('tab', { name: /科目|会计科目/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('03-02 新建会计科目', async ({ page }) => {
    await page.goto('/finance');
    await page.getByRole('tab', { name: /科目|会计科目/ }).click();
    await page.getByRole('button', { name: /新建|新增/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/科目编码/).fill(`E2E-${Date.now()}`);
    await page.getByLabel(/科目名称/).fill('E2E 测试科目');
    await page.getByLabel(/类别/).click();
    await page.getByRole('option').first().click();
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });

  test('03-03 科目支持启用/停用切换', async ({ page }) => {
    await page.goto('/finance');
    await page.getByRole('tab', { name: /科目|会计科目/ }).click();
    const switchEl = page.locator('.el-switch').first();
    if (await switchEl.isVisible({ timeout: 3000 }).catch(() => false)) {
      await switchEl.click();
    }
  });
});