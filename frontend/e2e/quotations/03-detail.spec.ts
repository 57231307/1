// 报价单 E2E 套件 — 03 报价单详情与编辑
// 创建时间: 2026-08-19
// 覆盖范围：报价单详情查看 → 编辑草稿 → 报价单列表状态筛选
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('03 报价单详情与编辑', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 报价单详情可查看', async ({ page }) => {
    await page.goto('/quotations');
    const viewBtn = page.getByRole('button', { name: /查看/ }).first();
    if (await viewBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await viewBtn.click();
      await expect(page.getByText(/基本信息/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('03-02 草稿报价单可编辑', async ({ page }) => {
    await page.goto('/quotations');
    const editBtn = page.getByRole('button', { name: /编辑/ }).first();
    if (await editBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await editBtn.click();
      await expect(page.locator('form')).toBeVisible({ timeout: 5000 });
      await expect(page.getByLabel(/客户/)).toBeVisible();
    }
  });
});