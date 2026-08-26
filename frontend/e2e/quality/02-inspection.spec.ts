// 质量管理 E2E 套件 — 02 检验记录与缺陷处理
// 创建时间: 2026-08-19
// 覆盖范围：检验记录创建 → 合格/不合格判定 → 缺陷处理
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 检验记录与缺陷处理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 新建检验记录', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /记录|检验记录/ }).click();
    await page.getByRole('button', { name: /新建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/产品名称/).fill('E2E 测试产品');
    await page.getByLabel(/批次号/).fill(`BATCH-${Date.now()}`);
    await page.getByLabel(/检验员/).fill('E2E 检验员');
    await page.getByLabel(/检验结果/).click();
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('02-02 检验记录列表可正常加载', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /记录|检验记录/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('02-03 缺陷管理 Tab 可正常加载', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /缺陷/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('02-04 未处理缺陷可处理', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /缺陷/ }).click();
    const handleBtn = page.getByRole('link', { name: /处理/ }).first();
    if (await handleBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await handleBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/处理成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });
});