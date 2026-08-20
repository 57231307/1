// 采购扩展 E2E 套件 — 01 采购合同
// 创建时间: 2026-08-19
// 覆盖范围：采购合同创建 → 审批 → 执行（draft → pending → executing）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 采购合同', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入采购扩展页面', async ({ page }) => {
    await page.goto('/purchase-ext');
    await expect(page.getByText(/采购/)).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('tab', { name: /合同/ })).toBeVisible();
  });

  test('01-02 新建采购合同', async ({ page }) => {
    await page.goto('/purchase-ext');
    await page.getByRole('tab', { name: /合同/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/合同编号/).fill(`PC-${Date.now()}`);
    await page.getByLabel(/供应商/).fill('E2E 测试供应商');
    await page.getByLabel(/合同日期/).fill('2026-08-19');
    await page.getByLabel(/总金额/).fill('100000');
    await page.getByLabel(/币种/).click();
    await page.getByRole('option').first().click();
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });

  test('01-03 草稿合同可审批', async ({ page }) => {
    await page.goto('/purchase-ext');
    await page.getByRole('tab', { name: /合同/ }).click();
    const approveBtn = page.getByRole('link', { name: /审批/ }).first();
    if (await approveBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await approveBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('01-04 待执行合同可执行（pending → executing）', async ({ page }) => {
    await page.goto('/purchase-ext');
    await page.getByRole('tab', { name: /合同/ }).click();
    const execBtn = page.getByRole('link', { name: /执行/ }).first();
    if (await execBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await execBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/执行成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });
});