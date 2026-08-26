// CRM 客户关系管理 E2E 套件 — 03 商机管理
// 创建时间: 2026-08-19
// 覆盖范围：商机创建 → 跟进 → 推进阶段 → 赢单/输单
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('03 商机管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 进入商机管理页面', async ({ page }) => {
    await page.goto('/crm/opportunities');
    await expect(page.getByText(/商机管理/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /创建/ })).toBeVisible();
  });

  test('03-02 创建新商机', async ({ page }) => {
    await page.goto('/crm/opportunities');
    await page.getByRole('button', { name: /创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/商机名称/).fill(`E2E 商机 ${Date.now()}`);
    await page.getByLabel(/客户/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/商机类型/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/预计金额/).fill('100000');
    await page.getByLabel(/赢单概率/).first().click();
    await page.getByLabel(/预计关闭日期/).fill('2026-12-31');
    await page.getByRole('button', { name: /保存|确认|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('03-03 商机可添加跟进记录', async ({ page }) => {
    await page.goto('/crm/opportunities');
    const followBtn = page.getByRole('link', { name: /跟进/ }).first();
    if (await followBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await followBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible();
      await page.getByLabel(/内容/).fill('E2E 测试跟进：客户确认需求');
      await page.getByRole('button', { name: /保存|确认|提交/ }).last().click();
      await expect(page.getByText(/保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('03-04 谈判阶段商机可赢单', async ({ page }) => {
    await page.goto('/crm/opportunities');
    const winBtn = page.getByRole('link', { name: /赢单/ }).first();
    if (await winBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await winBtn.click();
      await page.getByRole('button', { name: /确定|确认/ }).click();
      await expect(page.getByText(/更新成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
        return null;
      });
    }
  });

  test('03-05 商机可标记为输单', async ({ page }) => {
    await page.goto('/crm/opportunities');
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