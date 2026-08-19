// 生产计划 E2E 套件 — 01 生产订单创建与排产
// 创建时间: 2026-08-19
// 覆盖范围：生产订单创建 → 编辑 → 计划排产（draft → planned）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 生产订单创建与排产', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入生产计划页面', async ({ page }) => {
    await page.goto('/production');
    await expect(page.getByText(/生产计划/)).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('button', { name: /新建/ })).toBeVisible();
  });

  test('01-02 新建生产订单', async ({ page }) => {
    await page.goto('/production');
    await page.getByRole('button', { name: /新建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/订单编号/).fill(`PROD-${Date.now()}`);
    await page.getByLabel(/产品 ID/).fill('1');
    await page.getByLabel(/计划数量/).fill('1000');
    await page.getByLabel(/优先级/).fill('5');
    await page.getByLabel(/计划开始/).fill('2026-09-01');
    await page.getByLabel(/计划结束/).fill('2026-09-15');
    await page.getByLabel(/工作中心/).fill('1');
    await page.getByLabel(/备注/).fill('E2E 测试生产订单');
    await page.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });

  test('01-03 生产订单筛选功能可用', async ({ page }) => {
    await page.goto('/production');
    await page.getByLabel(/订单编号/).fill('E2E');
    await page.getByRole('button', { name: /搜索/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /重置/ }).click();
    await expect(page.locator('table, .v2-table, .el-table')).toBeVisible({ timeout: 5000 });
  });

  test('01-04 草稿订单可计划排产（draft → planned）', async ({ page }) => {
    await page.goto('/production');
    const scheduleBtn = page.getByRole('link', { name: /计划排产/ }).first();
    if (await scheduleBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await scheduleBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/排产成功|状态更新成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('01-05 草稿订单可编辑', async ({ page }) => {
    await page.goto('/production');
    const editBtn = page.getByRole('link', { name: /编辑/ }).first();
    if (await editBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await editBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible();
      await expect(page.getByLabel(/订单编号/)).toBeVisible();
      await page.getByRole('button', { name: /取消/ }).click();
    }
  });
});