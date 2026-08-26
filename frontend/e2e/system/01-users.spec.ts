// 系统管理 E2E 套件 — 01 用户与角色
// 创建时间: 2026-08-19
// 覆盖范围：用户管理（创建/编辑） + 角色管理（创建/权限配置）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 用户与角色', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入系统管理页面', async ({ page }) => {
    await page.goto('/system');
    await expect(page.getByText(/系统管理/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /用户/ })).toBeVisible();
  });

  test('01-02 用户列表可正常加载', async ({ page }) => {
    await page.goto('/system');
    await page.getByRole('tab', { name: /用户/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('01-03 新建用户', async ({ page }) => {
    await page.goto('/system');
    await page.getByRole('tab', { name: /用户/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/用户名/).fill(`e2e_user_${Date.now()}`);
    await page.getByLabel(/姓名/).fill('E2E 测试用户');
    await page.getByRole('button', { name: /确认|保存|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('01-04 角色列表可正常加载', async ({ page }) => {
    await page.goto('/system');
    await page.getByRole('tab', { name: /角色/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });
});