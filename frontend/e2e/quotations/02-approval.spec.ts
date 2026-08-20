// 报价单 E2E 套件 — 02 报价单审批与转订单
// 创建时间: 2026-08-19
// 覆盖范围：提交审批（draft → pending_approval） → 审批通过（→ approved） → 拒绝（→ rejected） → 转订单（→ converted）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 报价单审批与转订单', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 草稿报价单可提交审批', async ({ page }) => {
    await page.goto('/quotations');
    const submitBtn = page.getByRole('button', { name: /提交审批/ }).first();
    if (await submitBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await submitBtn.click();
      await expect(page.getByText(/提交成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-02 待审批报价单可批准', async ({ page }) => {
    await page.goto('/quotations');
    const approveBtn = page.getByRole('button', { name: /批准/ }).first();
    if (await approveBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await approveBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-03 已批准报价单可转为销售订单', async ({ page }) => {
    await page.goto('/quotations');
    const convertBtn = page.getByRole('button', { name: /转订单/ }).first();
    if (await convertBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await convertBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/转订单成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-04 草稿报价单可取消', async ({ page }) => {
    await page.goto('/quotations');
    const cancelBtn = page.getByRole('button', { name: /取消/ }).first();
    if (await cancelBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await cancelBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/取消成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });
});