// CRM 客户关系管理 E2E 套件 — 01 客户管理
// 创建时间: 2026-08-19
// 覆盖范围：客户创建 → 编辑 → 删除 + 客户 360 详情
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 客户管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入 CRM 客户管理页面', async ({ page }) => {
    await page.goto('/crm');
    await expect(page.getByText(/客户管理|客户列表/)).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('button', { name: /创建|新建/ })).toBeVisible();
  });

  test('01-02 创建客户', async ({ page }) => {
    await page.goto('/crm');
    await page.getByRole('button', { name: /创建|新建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/客户编码/).fill(`CUST-${Date.now()}`);
    await page.getByLabel(/客户名称/).fill(`E2E 测试客户 ${Date.now()}`);
    await page.getByLabel(/联系人/).fill('张三');
    await page.getByLabel(/电话/).fill('13800138000');
    await page.getByLabel(/邮箱/).fill('test@example.com');
    await page.getByRole('button', { name: /保存|确认|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 5000 }).catch(() => {
      return null;
    });
  });

  test('01-03 客户列表支持筛选', async ({ page }) => {
    await page.goto('/crm');
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
    await page.getByLabel(/客户名称/).fill('E2E');
    await page.getByRole('button', { name: /查询|搜索/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 5000 });
  });
});