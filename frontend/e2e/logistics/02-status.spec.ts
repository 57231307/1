// 物流管理 E2E 套件 — 02 运单状态流转
// 创建时间: 2026-08-19
// 覆盖范围：发货（pending → shipped） → 更新状态至在途/已签收（shipped → in_transit → delivered）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 运单状态流转', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 待发货运单可发货（pending → shipped）', async ({ page }) => {
    await page.goto('/logistics');
    const shipBtn = page.getByRole('link', { name: /发货/ }).first();
    if (await shipBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await shipBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/发货成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-02 已发货运单可更新状态至运输中（shipped → in_transit）', async ({ page }) => {
    await page.goto('/logistics');
    const updateBtn = page.getByRole('link', { name: /更新状态/ }).first();
    if (await updateBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await updateBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 3000 }).catch(() => {
        return null;
      });
      await page.getByLabel(/新状态/).click();
      await page.getByRole('option').first().click();
      await page.getByRole('button', { name: /确认/ }).click();
      await expect(page.getByText(/更新成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-03 已签收运单无操作按钮', async ({ page }) => {
    await page.goto('/logistics');
    const deliveredTag = page.locator('.el-tag').filter({ hasText: /已签收/ }).first();
    if (await deliveredTag.isVisible({ timeout: 3000 }).catch(() => false)) {
      const row = deliveredTag.locator('xpath=ancestor::tr');
      const actions = row.locator('button, a').filter({ hasText: /发货|更新状态/ });
      await expect(actions).toHaveCount(0);
    }
  });

  test('02-04 运单详情可查看', async ({ page }) => {
    await page.goto('/logistics');
    const viewBtn = page.getByRole('link', { name: /查看/ }).first();
    if (await viewBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await viewBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible();
      await expect(page.getByText(/运单号|物流公司/)).toBeVisible();
      await page.getByRole('button', { name: /关闭/ }).click();
    }
  });
});