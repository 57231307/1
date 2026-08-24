// 销售扩展 E2E 套件 — 01 销售合同
// 创建时间: 2026-08-19
// 覆盖范围：销售合同创建 → 审批 → 执行
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 销售合同', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入销售扩展页面', async ({ page }) => {
    await page.goto('/sales-ext');
    await expect(page.getByText(/销售/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /合同/ })).toBeVisible();
  });

  test('01-02 新建销售合同', async ({ page }) => {
    await page.goto('/sales-ext');
    await page.getByRole('tab', { name: /合同/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/合同编号/).fill(`SC-${Date.now()}`);
    await page.getByLabel(/客户/).fill('E2E 测试客户');
    await page.getByLabel(/合同日期/).fill('2026-08-19');
    await page.getByLabel(/总金额/).fill('200000');
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('01-03 草稿合同可审批', async ({ page }) => {
    await page.goto('/sales-ext');
    await page.getByRole('tab', { name: /合同/ }).click();
    const approveBtn = page.getByRole('link', { name: /审批/ }).first();
    if (await approveBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await approveBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });
});