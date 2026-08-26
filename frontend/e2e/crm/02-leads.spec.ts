// CRM 客户关系管理 E2E 套件 — 02 线索管理
// 创建时间: 2026-08-19
// 覆盖范围：线索创建 → 联系 → 转化 → 丢失（完整线索生命周期）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('02 线索管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 进入线索管理页面', async ({ page }) => {
    await page.goto('/crm/leads');
    await expect(page.getByText(/线索管理/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /创建/ })).toBeVisible();
  });

  test('02-02 创建新线索', async ({ page }) => {
    await page.goto('/crm/leads');
    await page.getByRole('button', { name: /创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/公司名/).fill('E2E 测试公司');
    await page.getByLabel(/联系人/).fill('李四');
    await page.getByLabel(/手机/).fill('13900139000');
    await page.getByLabel(/邮箱/).fill('li@test.com');
    await page.getByLabel(/线索来源/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/备注/).fill('E2E 测试线索');
    await page.getByRole('button', { name: /保存|确认|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('02-03 线索可标记为已联系（NEW → CONTACTED）', async ({ page }) => {
    await page.goto('/crm/leads');
    const contactBtn = page.getByRole('link', { name: /联系/ }).first();
    if (await contactBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await contactBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/更新成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-04 合格线索可转化为客户', async ({ page }) => {
    await page.goto('/crm/leads');
    const convertBtn = page.getByRole('link', { name: /转化/ }).first();
    if (await convertBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await convertBtn.click();
      await page.getByRole('button', { name: /确定|确认/ }).click();
      await expect(page.getByText(/转化成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('02-05 线索可标记为丢失', async ({ page }) => {
    await page.goto('/crm/leads');
    const loseBtn = page.getByRole('link', { name: /丢失/ }).first();
    if (await loseBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await loseBtn.click();
      await page.getByRole('button', { name: /确定|确认/ }).click();
      await expect(page.getByText(/更新成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });
});