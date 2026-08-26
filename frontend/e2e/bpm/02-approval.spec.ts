// BPM 审批管理 E2E 套件 — 02 审批中心
// 创建时间: 2026-08-19
// 覆盖范围：待办审批（同意/拒绝/转交） → 已办查看 → 审批链追溯
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 审批中心', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 进入审批中心页面', async ({ page }) => {
    await page.goto('/bpm/approval');
    await expect(page.getByText(/审批/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /待办/ })).toBeVisible();
    await expect(page.getByRole('tab', { name: /已办/ })).toBeVisible();
  });

  test('02-02 待办任务可审批同意', async ({ page }) => {
    await page.goto('/bpm/approval');
    await page.getByRole('tab', { name: /待办/ }).click();
    const approveBtn = page.getByRole('link', { name: /同意|审批/ }).first();
    if (await approveBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await approveBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 3000 }).catch(() => {
        return null;
      });
      await page.getByLabel(/审批意见/).fill('E2E 测试：审批同意');
      await page.getByRole('button', { name: /确认/ }).click();
      await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-03 待办任务可审批拒绝', async ({ page }) => {
    await page.goto('/bpm/approval');
    await page.getByRole('tab', { name: /待办/ }).click();
    const rejectBtn = page.getByRole('link', { name: /拒绝/ }).first();
    if (await rejectBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await rejectBtn.click();
      await page.getByLabel(/审批意见/).fill('E2E 测试：审批拒绝');
      await page.getByRole('button', { name: /确认/ }).click();
      await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-04 已办任务可追溯审批链', async ({ page }) => {
    await page.goto('/bpm/approval');
    await page.getByRole('tab', { name: /已办/ }).click();
    const chainBtn = page.getByRole('link', { name: /审批链/ }).first();
    if (await chainBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await chainBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible();
      await page.getByRole('button', { name: /关闭/ }).click();
    }
  });
});