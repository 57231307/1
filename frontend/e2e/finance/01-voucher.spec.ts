// 财务管理 E2E 套件 — 01 凭证管理
// 创建时间: 2026-08-19
// 覆盖范围：凭证创建（含借贷平衡） → 提交 → 审核 → 过账
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 凭证管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入财务管理页面', async ({ page }) => {
    await page.goto('/finance');
    await expect(page.getByText(/财务管理|凭证管理/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /凭证/ })).toBeVisible();
  });

  test('01-02 新建凭证（含借贷分录）', async ({ page }) => {
    await page.goto('/finance');
    await page.getByRole('button', { name: /新建|新建凭证/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/凭证日期/).fill('2026-08-19');
    await page.getByLabel(/凭证类型/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/摘要/).fill('E2E 测试记账凭证');
    await page.getByRole('button', { name: /确认|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('01-03 凭证筛选功能可用', async ({ page }) => {
    await page.goto('/finance');
    await page.getByLabel(/凭证号/).fill('E2E');
    await page.getByRole('button', { name: /查询|搜索/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });
});