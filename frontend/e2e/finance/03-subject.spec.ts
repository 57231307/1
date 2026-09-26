// 财务管理 E2E 套件 — 03 会计科目管理
// 创建时间: 2026-08-19
// 覆盖范围：科目创建 → 编辑 → 删除
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';

/**
 * 03-03 启用/停用切换（方法一）：
 * 原用例 `page.locator('.el-switch').first()` 在科目列表里恒不可见——SubjectTab.vue
 * 的表格状态列是 el-tag（:86），唯一的 el-switch 在「编辑」对话框内（SubjectTab.vue:181，
 * active-value=1/inactive-value=0）。故原用例永远走不进 if，零断言假绿。
 * 现改为：建一条科目 → 定位该行点编辑 → 对话框内切换状态开关 → 断言开关状态真实翻转。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function seedSubject(page: import('@playwright/test').Page): Promise<string> {
  const code = genCode('E2E-SUBJ');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
    code,
    name: `E2E 启用停用测试科目 ${code}`,
    level: 1,
    balance_direction: 'debit',
  });
  if (!created.data?.id) throw new Error(`建科目失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/subjects/${created.data.id}`, label: 'account_subject' });
  return code;
}

test.describe('03 会计科目管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 会计科目 Tab 可正常加载', async ({ page }) => {
    await page.goto('/finance');
    await page.getByRole('tab', { name: '科目管理' }).click();
    // 原 locator('table, .el-table') strict-mode 命中多个 → 精确到会计科目列表容器。
    await expect(page.locator('.el-table[aria-label="会计科目列表"]').first()).toBeVisible({
      timeout: 30000,
    });
  });

  test('03-02 新建会计科目', async ({ page }) => {
    await page.goto('/finance');
    await page.getByRole('tab', { name: '科目管理' }).click();
    await page.getByRole('button', { name: '新建科目' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 科目新建对话框真实字段为「科目编码」「科目名称」（余额方向默认「借方」且必填已预置），
    // 并无「类别」字段（原 getByLabel(/类别/) 臆测）；提交按钮真实文案「确定」；成功提示「创建成功」。
    // 限定到 .el-dialog 作用域。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('科目编码').fill(`E2E-${Date.now()}`);
    await dlg.getByLabel('科目名称').fill('E2E 测试科目');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30000 });
  });

  test('03-03 科目支持启用/停用切换', async ({ page }) => {
    // 方法一：先建一条科目，定位其行打开编辑对话框，操作对话框内的状态开关并断言翻转
    const code = await seedSubject(page);
    await page.goto('/finance');
    await page.getByRole('tab', { name: '科目管理' }).click();
    const row = page.getByRole('row').filter({ hasText: code });
    await expect(row, `未定位到新建科目 ${code}`).toHaveCount(1);
    await row.getByText('编辑', { exact: false }).first().click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 10000 });
    const switchEl = dialog.locator('.el-switch');
    await expect(switchEl, '编辑对话框内未渲染状态开关').toBeVisible();
    // Element Plus 把 role=switch/aria-checked 挂在内层 input.el-switch__input 上，
    // 外层 .el-switch 读 aria-checked 恒 null；对齐 31c-deactivation-matrix 既有范式，
    // 改为比较外层 .el-switch 的 class 中 is-checked 是否翻转（断言强度不变：仍要求翻转）。
    const before = (await switchEl.getAttribute('class')) ?? '';
    await switchEl.click();
    const after = (await switchEl.getAttribute('class')) ?? '';
    expect(after.includes('is-checked'), '点击状态开关后未发生启用/停用翻转').not.toBe(
      before.includes('is-checked')
    );
  });
});
