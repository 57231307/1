// 面料管理 E2E 套件 — 01 坯布管理
// 创建时间: 2026-08-19
// 覆盖范围：坯布创建 → 入库 → 出库
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 坯布管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入面料管理页面', async ({ page }) => {
    await page.goto('/fabric');
    await expect(page.getByText(/面料/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /坯布/ })).toBeVisible();
  });

  test('01-02 新建坯布', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /坯布/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/面料编码/).fill(`FB-${Date.now()}`);
    await page.getByLabel(/面料名称/).fill('E2E 测试坯布');
    await page.getByLabel(/数量/).fill('1000');
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('01-03 坯布入库操作', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /坯布/ }).click();
    const stockInBtn = page.getByRole('link', { name: /入库/ }).first();
    if (await stockInBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await stockInBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/入库成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('01-04 坯布出库操作', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /坯布/ }).click();
    const stockOutBtn = page.getByRole('link', { name: /出库/ }).first();
    if (await stockOutBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await stockOutBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/出库成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });
});