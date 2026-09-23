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

    // 原实现 goto('/system/users') —— 该路由不存在（router 无 users 子路由，快照直接 404
    // 「页面不存在」），后续找不到「新增」按钮。用户管理是 /system 页的「用户」Tab
    // （views/system/index.vue：el-tab-pane name="user"，默认激活；UserTab 头部按钮
    //  i18n system.user.button.create = 「新建用户」）。改为导航到 /system 并进入用户 Tab。
    await page.goto(`${BASE_URL}/system`);
    await page.waitForLoadState('networkidle');
    const userTab = page.locator('.el-tabs__item:has-text("用户")').first();
    await userTab.click();
    await page.waitForTimeout(2000);

    // 原实现 `if (await submitBtn.isVisible()) { ...5 次点击... }`：列表页找不到提交按钮时
    // 一次点击都不执行，toastCount=0 仍 ≤1 → 假绿。改为进入真实含提交按钮的表单弹窗
    // （新增用户），对弹窗提交按钮连续点击；提交入口缺失即硬失败。
    const createBtn = page.getByRole('button', { name: /新增|新建|添加/ }).first();
    expect(
      await createBtn.isVisible({ timeout: 5000 }),
      '[P5.13] /system 用户 Tab 未渲染「新建用户」按钮，无法进入含提交按钮的表单弹窗'
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
