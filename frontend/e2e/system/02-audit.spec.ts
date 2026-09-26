// 系统管理 E2E 套件 — 02 审计日志
// 创建时间: 2026-08-19（本轮去假绿改造 2026-09-23）
// 覆盖范围：审计日志查看 → 关键字筛选 → 详情抽屉 → 操作类型筛选
//
// 去假绿说明：
// 1) 02-03 原用 `getByRole('link', { name: /详情/ })` 找详情按钮 —— 但 V2Table 操作列是
//    `h(ElButton,{link:true})` 渲染的 <button>（role=button，不是 link），选择器恒不命中 →
//    `if (await detailBtn.isVisible())` 恒假 → 0 断言 → 假绿。改为 getByRole('button')。
// 2) 数据缺失不再放宽断言：真实 UI 登录本身会写一条 LOGIN 审计（auth_handler.rs
//    build_login_success_audit → OperationType::Login），故列表必然有行；02-01/02-02/02-04
//    对筛选下发用 waitForRequest 断言真实 query 参数，而非"表格始终可见"的空断言。
//    关键字标签为「关键字」（auditLog.filter.keyword），原用例写 /关键词/ 不匹配。
import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL } from '../flow/helpers';
import { pickSelect, elSelectByLabel } from '../flow/ui-helpers';

test.describe('02 审计日志', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('02-01 进入审计日志页面', async ({ page }) => {
    // 原 getByText('审计日志筛选表单') 臆测：该串仅为 el-form 的 aria-label 属性
    // （auditLog.filter.ariaLabel），页面无此可见文本，getByText 恒不命中。
    // 改断言该页真实可见的专属控件：导出 CSV 按钮（auditLog.filter.exportCsv）+ 查询按钮 + 列表。
    await page.goto(`${BASE_URL}/system/audit-log`);
    await expect(page.getByRole('button', { name: '导出 CSV' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '查询' })).toBeVisible();
    await expect(page.getByRole('table').first()).toBeVisible({
      timeout: 30000,
    });
  });

  test('02-02 审计日志筛选功能可用', async ({ page }) => {
    await page.goto(`${BASE_URL}/system/audit-log`);
    await expect(page.getByLabel('关键字')).toBeVisible({ timeout: 30000 });
    // 点查询 → 断言筛选参数真实下发到后端（列表请求带 keyword=）
    const keywordReq = page.waitForRequest(
      r => /\/audit-logs\?/.test(r.url()) && new URL(r.url()).searchParams.get('keyword') === 'E2E'
    );
    await page.getByLabel('关键字').fill('E2E');
    await page.getByRole('button', { name: '查询' }).click();
    await keywordReq;
    // 重置 → 断言查询请求再次下发且不再带 keyword
    const resetReq = page.waitForRequest(
      r => /\/audit-logs\?/.test(r.url()) && !new URL(r.url()).searchParams.get('keyword')
    );
    await page.getByRole('button', { name: '重置' }).click();
    await resetReq;
    await expect(page.getByRole('table').first()).toBeVisible();
  });

  test('02-03 审计日志详情可查看', async ({ page }) => {
    await page.goto(`${BASE_URL}/system/audit-log`);
    // 真实登录已写入 LOGIN 审计 → 列表必然有行且渲染「详情」按钮（el-button link，role=button）
    const detailBtn = page.getByRole('button', { name: '详情' }).first();
    await expect(
      detailBtn,
      '审计列表应存在行且渲染「详情」按钮（UI 登录即产生 LOGIN 审计）'
    ).toBeVisible({ timeout: 30000 });
    await detailBtn.click();
    const drawer = page.locator('.el-drawer');
    await expect(drawer, '点击详情应打开抽屉').toBeVisible({ timeout: 30000 });
    // 抽屉标题 + 描述项标签（auditLog.detail.*）
    await expect(drawer.getByText('审计日志详情')).toBeVisible();
    await expect(drawer.getByText('操作时间')).toBeVisible();
    await expect(drawer.getByText('操作类型')).toBeVisible();
  });

  test('02-04 审计日志支持按操作类型筛选', async ({ page }) => {
    await page.goto(`${BASE_URL}/system/audit-log`);
    await expect(page.getByLabel('操作类型', { exact: true })).toBeVisible({ timeout: 30000 });
    // 选项标签来自 auditLog.operationType.login = 「登录」（值 LOGIN）
    await pickSelect(page, elSelectByLabel(page, '操作类型', true), '登录');
    const typeReq = page.waitForRequest(
      r =>
        /\/audit-logs\?/.test(r.url()) &&
        new URL(r.url()).searchParams.get('operation_type') === 'LOGIN'
    );
    await page.getByRole('button', { name: '查询' }).click();
    await typeReq;
    await expect(page.getByRole('table').first()).toBeVisible();
  });
});
