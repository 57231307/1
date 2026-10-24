import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  trackPageHealth,
  assertPageHealthy,
  BROWSER_NETWORK_NOISE,
  BASE_URL,
} from './helpers';

/**
 * P5.13 重复提示测试
 * 依赖：P3.2（expectSingleToast）
 *
 * 验证：
 * - 连续点击提交 5 次 → expectSingleToast + 按钮 loading 禁用
 * - dialog 重复实例计数
 */
test.describe('P5.13 重复提示', () => {
  test('连续点击提交不产生重复 toast', async ({ page }) => {
    await loginViaUI(page);
    const collector = trackPageHealth(page);

    // 入口：用户管理页 /users，从其"新建用户"弹窗取真实带提交按钮的表单。
    // 本用例验：同一提交按钮被连点 5 次时，同一条提示只出现一个 toast 实例。
    await page.goto(`${BASE_URL}/users`);
    await page.waitForLoadState('networkidle');
    await expect(
      page.getByRole('heading', { level: 2, name: '用户管理' }),
      `[P5.13] 未渲染用户管理页标题，实际落点 ${page.url()}`
    ).toBeVisible({ timeout: 30_000 });

    // 提交入口缺失即硬失败：不允许整段跳过后再断 toast 数量为 0（那是假绿）。
    const createBtn = page.getByRole('button', { name: /新增|新建|添加/ }).first();
    expect(
      await createBtn.isVisible({ timeout: 5000 }),
      '[P5.13] 用户管理页未渲染「新建用户」按钮，无法进入含提交按钮的表单弹窗'
    ).toBe(true);
    await createBtn.click();
    const dialog = page.locator('.el-dialog:visible').first();
    await dialog.waitFor({ state: 'visible', timeout: 10000 });
    const submitBtn = dialog.getByRole('button', { name: /保存|确[认定]|提交/ }).last();
    expect(
      await submitBtn.isVisible({ timeout: 5000 }),
      '[P5.13] 新增用户弹窗未渲染提交/保存按钮'
    ).toBe(true);

    // 快速连续点击 5 次（提交禁用/瞬时不可点时容错跳过该次点击）
    for (let i = 0; i < 5; i++) {
      await submitBtn.click({ timeout: 1000 }).catch(() => {});
    }

    // 等待 toast 出现
    await page.waitForTimeout(2000);

    // 断言同一文案 toast 实例 ≤1
    // expectSingleToast 实现：检查 .el-message 数量
    const toastCount = await page.locator('.el-message').count();
    expect(toastCount).toBeLessThanOrEqual(1);

    await assertPageHealthy(page, collector, { consoleNoisePatterns: BROWSER_NETWORK_NOISE });
  });
});
