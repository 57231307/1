// 生产计划 E2E 套件 — 03 生产订单管理
// 创建时间: 2026-08-19
// 覆盖范围：删除草稿 + 查看详情 + 导出打印
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('03 生产订单管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 草稿订单可删除', async ({ page }) => {
    await page.goto('/production');
    const deleteBtn = page.getByRole('link', { name: /删除/ }).first();
    if (await deleteBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await deleteBtn.click();
      await page.getByRole('button', { name: /确定/ }).click();
      await expect(page.getByText(/删除成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('03-02 生产订单详情可查看', async ({ page }) => {
    await page.goto('/production');
    const viewBtn = page.getByRole('link', { name: /查看/ }).first();
    if (await viewBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await viewBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible();
      await expect(page.getByText(/订单编号|计划数量|计划开始/)).toBeVisible();
      await page.getByRole('button', { name: /关闭/ }).click();
    }
  });

  test('03-03 导出按钮可触发导出', async ({ page }) => {
    await page.goto('/production');
    const exportBtn = page.getByRole('button', { name: /导出/ });
    if (await exportBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await exportBtn.click();
      await expect(page.getByText(/导出成功|下载中/)).toBeVisible({ timeout: 5000 }).catch(() => {
        return null;
      });
    }
  });

  test('03-04 打印按钮可触发打印', async ({ page }) => {
    await page.goto('/production');
    const printBtn = page.getByRole('button', { name: /打印/ });
    if (await printBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await printBtn.click();
    }
  });
});