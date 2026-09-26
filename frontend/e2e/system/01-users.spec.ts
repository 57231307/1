// 系统管理 E2E 套件 — 01 用户与角色
// 创建时间: 2026-08-19
// 覆盖范围：用户管理（创建/编辑） + 角色管理（创建/权限配置）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelect, elSelectByLabel } from '../flow/ui-helpers';

test.describe('01 用户与角色', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入系统管理页面', async ({ page }) => {
    // /system 为纯 el-tabs 容器，页面级无任何「系统管理」heading；
    // 默认激活「用户管理」tab，其内容 UserTab 顶部 h2 标题为「用户管理」（system.user.title）。
    await page.goto('/system');
    await expect(page.getByRole('tab', { name: '用户管理' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('heading', { name: '用户管理' })).toBeVisible();
  });

  test('01-02 用户列表可正常加载', async ({ page }) => {
    await page.goto('/system');
    await page.getByRole('tab', { name: '用户管理' }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('01-03 新建用户', async ({ page }) => {
    await page.goto('/system');
    await page.getByRole('tab', { name: '用户管理' }).click();
    await page.getByRole('button', { name: '新建用户' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 新建用户对话框真实字段为「用户名/密码/角色」（均必填，密码需 ≥8 且含大小写与数字，见
    // UserTab validatePassword），无「姓名」输入项；提交按钮文案「确定」（system.user.button.confirm）；
    // 成功提示为 settings.user.createSuccess=「创建成功」。全部限定到 .el-dialog 作用域避免与筛选栏同名 label 冲突。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('用户名').fill(`e2e_user_${Date.now()}`);
    await dlg.getByLabel('密码').fill('E2ePassw0rd');
    await pickSelect(page, elSelectByLabel(dlg, '角色'));
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-04 角色列表可正常加载', async ({ page }) => {
    await page.goto('/system');
    await page.getByRole('tab', { name: '角色管理' }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });
});
