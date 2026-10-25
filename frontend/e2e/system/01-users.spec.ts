// 系统管理 E2E 套件 — 01 用户与角色
// 创建时间: 2026-08-19
// 覆盖用户管理页的进入、列表与新建，以及角色管理页的列表。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelectIn } from '../flow/ui-helpers';

test.describe('01 用户与角色', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入用户管理页面', async ({ page }) => {
    // 入口是功能页 /users，断言页头标题"用户管理"与页头"新建用户"按钮同页渲染，
    // 两者共同证明进到的是本页而不是别的页面。
    await page.goto('/users');
    await expect(page.getByRole('heading', { name: '用户管理' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '新建用户' })).toBeVisible();
  });

  test('01-02 用户列表可正常加载', async ({ page }) => {
    await page.goto('/users');
    await expect(page.getByRole('heading', { name: '用户管理' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('01-03 新建用户', async ({ page }) => {
    await page.goto('/users');
    await page.getByRole('button', { name: '新建用户' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 新建用户对话框真实字段为「用户名/密码/角色」（均必填）；提交按钮文案「确定」
    // （system.user.button.confirm）；成功提示为 settings.user.createSuccess=「创建成功」。
    // 全部限定到 .el-dialog 作用域避免与筛选栏同名 label 冲突。
    //
    // 密码须满足后端权威强度契约（backend/src/utils/password_validator.rs::PasswordPolicy::default +
    // handlers/user_handler.rs::validate_password_strength），而非前端本页较弱规则：
    //   长度 ≥8、必含大写+小写+数字+特殊字符各一、强度 ≥ 中等、不命中常见密码黑名单、无键盘序列。
    // 旧值 E2ePassw0rd 无特殊字符 → POST /users 400「密码必须包含特殊字符」，成功提示永不出现（真红）。
    // 不削弱后端校验迁就旧测试，改用例数据合契约：Erp!Bx#2026 含四类字符、避开黑名单/键盘序列、
    // 强度 VeryStrong。用户名用 Date.now() 保证唯一、不含于密码故不触发 contains_username_fragment。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('用户名').fill(`e2e_user_${Date.now()}`);
    await dlg.getByLabel('密码').fill('Erp!Bx#2026');
    // 角色是新建用户对话框内的 el-select。用唯一事实源 helper pickSelectIn 以
    // root=dlg + 精确 label「角色」作用域，点外层 wrapper（非只读内层 input）打开下拉选首项，
    // 消除旧 pickSelect+elSelectByLabel（子串匹配 / 点 readonly combobox）的假红。
    await pickSelectIn(dlg, page, '角色');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-04 角色列表可正常加载', async ({ page }) => {
    // 入口是功能页 /roles，断言页头标题"角色管理"与该页表格。
    await page.goto('/roles');
    await expect(page.getByRole('heading', { name: '角色管理' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });
});
