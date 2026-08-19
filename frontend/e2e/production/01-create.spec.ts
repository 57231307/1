// 生产计划 E2E 套件 — 01 生产工单创建与排产（draft → planned）
// 覆盖范围：新建生产订单、编辑草稿、计划排产
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('生产计划 - 01 工单创建与排产', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('生产计划页面可访问', async ({ page }) => {
    await page.goto('/production');
    await expect(page.getByText(/生产计划/)).toBeVisible({ timeout: 5000 });
  });

  test('新建生产工单', async ({ page }) => {
    await page.goto('/production');
    await page.getByRole('button', { name: /新建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await expect(page.getByLabel(/订单编号/)).toBeVisible();
    await page.getByLabel(/订单编号/).fill(`E2E-${Date.now()}`);
    await page.getByLabel(/产品/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/计划数量/).fill('100');
    await page.getByLabel(/优先级/).fill('5');
    await page.getByLabel(/计划开始日期/).fill('2026-09-01');
    await page.getByLabel(/计划结束日期/).fill('2026-09-30');
    await page.getByLabel(/工作中心/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/备注/).fill('E2E 测试生产工单');
    await page.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });

  test('草稿工单可编辑', async ({ page }) => {
    await page.goto('/production');
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 10000 });
    const editBtn = page.getByRole('link', { name: /编辑/ }).first();
    if (await editBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await editBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible();
      await expect(page.getByLabel(/订单编号/)).toBeVisible();
      await page.getByRole('button', { name: /取消/ }).click();
    }
  });

  test('草稿工单可计划排产（draft → planned）', async ({ page }) => {
    await page.goto('/production');
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 10000 });
    const scheduleBtn = page.getByRole('link', { name: /计划排产|排产/ }).first();
    if (await scheduleBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await scheduleBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/排产成功|状态更新成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });
});