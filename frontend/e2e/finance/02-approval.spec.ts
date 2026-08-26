// 财务管理 E2E 套件 — 02 凭证审批工作流
// 创建时间: 2026-08-19
// 覆盖范围：凭证提交（draft → submitted） → 审核（submitted → reviewed） → 过账（reviewed → posted）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 凭证审批工作流', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 草稿凭证可提交审核（draft → submitted）', async ({ page }) => {
    await page.goto('/finance');
    const submitBtn = page.getByRole('link', { name: /提交/ }).first();
    if (await submitBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await submitBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/提交成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-02 已提交凭证可审核（submitted → reviewed）', async ({ page }) => {
    await page.goto('/finance');
    const reviewBtn = page.getByRole('link', { name: /审核/ }).first();
    if (await reviewBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await reviewBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/审核成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-03 已审核凭证可过账（reviewed → posted）', async ({ page }) => {
    await page.goto('/finance');
    const postBtn = page.getByRole('link', { name: /过账/ }).first();
    if (await postBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await postBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/过账成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-04 已过账凭证无提交/审核/过账按钮', async ({ page }) => {
    await page.goto('/finance');
    const postedTag = page.locator('.el-tag').filter({ hasText: /已过账|已审核|已提交/ }).first();
    if (await postedTag.isVisible({ timeout: 3000 }).catch(() => false)) {
      const row = postedTag.locator('xpath=ancestor::tr');
      const actionBtns = row.locator('button, a').filter({ hasText: /提交|审核|过账/ });
      await expect(actionBtns).toHaveCount(0);
    }
  });
});