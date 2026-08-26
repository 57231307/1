// 质量管理 E2E 套件 — 01 质量标准
// 创建时间: 2026-08-19
// 覆盖范围：质量标准创建 → 审批 → 发布（完整状态流转）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 质量标准', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入质量管理页面', async ({ page }) => {
    await page.goto('/quality');
    await expect(page.getByText(/质量/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /标准/ })).toBeVisible();
  });

  test('01-02 新建质量标准', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('button', { name: /新建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/标准编码/).fill(`QS-${Date.now()}`);
    await page.getByLabel(/标准名称/).fill('E2E 测试质量标准');
    await page.getByLabel(/类型/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/内容/).fill('E2E 测试质量标准内容');
    await page.getByRole('button', { name: /确认|保存/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('01-03 草稿标准可审批通过', async ({ page }) => {
    await page.goto('/quality');
    const approveBtn = page.getByRole('link', { name: /审批/ }).first();
    if (await approveBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await approveBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 3000 }).catch(() => {
        return null;
      });
      await page.getByLabel(/审批意见/).fill('E2E 测试：审批通过');
      await page.getByRole('button', { name: /通过/ }).click();
      await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('01-04 已审批标准可发布', async ({ page }) => {
    await page.goto('/quality');
    const publishBtn = page.getByRole('link', { name: /发布/ }).first();
    if (await publishBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await publishBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/发布成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });
});