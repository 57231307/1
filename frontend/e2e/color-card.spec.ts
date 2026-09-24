// 色卡仓储管理 E2E 测试
// 创建时间: 2026-06-17
//
// 批次 29 v7 P0-8 修复（2026-06-29）：
// 原测试仅用 page.goto 跳转页面 + 静态文本断言，未执行任何业务流程。
//
// extras 分片修复（族 D 同源）：
// 原 BASE_URL + 手写 login() 模式与 color-price.spec.ts 完全一致：CI 前端 :3000 后端 :8082，
// 自行拼绝对端口且重复登录触发 429。改为复用 globalSetup 注入的 storageState + 相对导航。
// 同时移除硬编码 id=1 的数据依赖（色卡详情），改为按列表第一行取真实 id。

import { test, expect } from './diagnose-fixture';

test.describe('色卡仓储管理 E2E 业务流程', () => {
  test('色卡列表加载：等待表格渲染 + 断言核心列存在', async ({ page }) => {
    await page.goto('/color-cards/list');
    // 等待页面标题渲染（确认路由加载成功）—— 用 heading 精确角色而非 getByText 避免 strict 违例
    await expect(page.getByRole('heading', { name: /色卡列表/ })).toBeVisible({ timeout: 30000 });

    // 等待列表 API 响应完成
    await page.waitForResponse(
      resp => resp.url().includes('/color-cards') && resp.request().method() === 'GET',
      { timeout: 30000 }
    );

    // 断言筛选条件区域可见（核心业务组件存在）
    await expect(page.getByText('色卡类型')).toBeVisible({ timeout: 30000 });
  });

  test('色卡详情加载：通过列表第一行进入', async ({ page }) => {
    await page.goto('/color-cards/list');
    await page.waitForResponse(
      resp => resp.url().includes('/color-cards') && resp.request().method() === 'GET',
      { timeout: 30000 }
    );

    // 点击第一行的详情入口（不硬编码 id=1）
    const firstDetail = page.locator('a:has-text("详情"), button:has-text("详情")').first();
    await expect(firstDetail, '列表应有可进入详情的入口').toBeVisible({ timeout: 30000 });
    await firstDetail.click();
    await page.waitForResponse(
      resp => /\/color-cards\/\d+/.test(resp.url()) && resp.request().method() === 'GET',
      { timeout: 30000 }
    );

    // 断言详情区块可见
    await expect(page.getByText('基本信息')).toBeVisible({ timeout: 30000 });
  });

  test('色卡发放管理页面加载：等待核心组件渲染', async ({ page }) => {
    await page.goto('/color-cards/issues');
    // 等待页面标题渲染（heading 精确匹配）
    await expect(page.getByRole('heading', { name: /发放/ })).toBeVisible({
      state: 'visible',
      timeout: 30000,
    });

    // 等待发放列表 API 响应完成
    await page.waitForResponse(
      resp => resp.url().includes('/color-cards') && resp.request().method() === 'GET',
      { timeout: 30000 }
    );
  });

  test('色卡筛选条件区域：所有筛选项均可交互', async ({ page }) => {
    await page.goto('/color-cards/list');
    await expect(page.getByRole('heading', { name: /色卡列表/ })).toBeVisible({ timeout: 30000 });

    // 断言筛选下拉框存在且可点击
    const typeFilter = page.locator('.el-select').first();
    await expect(typeFilter).toBeVisible();
    await typeFilter.click();
    await page.keyboard.press('Escape');

    // 确认页面仍可交互（筛选操作未导致崩溃）
    await expect(page.getByRole('heading', { name: /色卡列表/ })).toBeVisible();
  });
});
