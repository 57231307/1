/**
 * 同域多入口合并后的入口唯一性 E2E
 *
 * 功能：锁定四对重复入口合并后只保留唯一页面，且旧地址一律重定向而非 404。
 *   客户域：/customer 承接标签与负责人列，/crm 重定向到 /customer；
 *   销售分析：/sales-analysis 双页签（经营分析/多维分析），/bi/sales-analysis 重定向到 ?tab=bi；
 *   应收对账：/ar-reconciliation 双页签（标准/增强），/ar-reconciliation/enhanced 重定向到 ?tab=enhanced；
 *   面料域：/greige-fabrics 为权威页承接出入库，/fabric 重定向到 /greige-fabrics。
 * 调用方：E2E 全量套件（Playwright），由 ci-cd 的 e2e 分片执行。
 * 入参：BASE_URL 与登录账号来自 helpers 环境变量；只读页面，不造业务数据。
 * 传给谁：断言结果只进 Playwright 报告。
 * 存什么/存哪里：不落库。
 */
import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('同域多入口合并后的唯一入口', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('旧入口地址重定向到唯一入口，不再保留第二份页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm`);
    await expect(page, '/crm 应重定向到客户列表').toHaveURL(/\/customer$/);
    await expect(page.getByRole('heading', { name: '客户管理' })).toBeVisible({
      timeout: 30000,
    });

    await page.goto(`${BASE_URL}/fabric`);
    await expect(page, '/fabric 应重定向到坯布页').toHaveURL(/\/greige-fabrics$/);
    await expect(page.getByRole('heading', { name: '坯布管理' })).toBeVisible({
      timeout: 30000,
    });

    await page.goto(`${BASE_URL}/bi/sales-analysis`);
    await expect(page, '/bi/sales-analysis 应重定向到销售分析多维页签').toHaveURL(
      /\/sales-analysis\?tab=bi$/
    );

    await page.goto(`${BASE_URL}/ar-reconciliation/enhanced`);
    await expect(page, '/ar-reconciliation/enhanced 应重定向到应收对账增强页签').toHaveURL(
      /\/ar-reconciliation\?tab=enhanced$/
    );
  });

  test('客户列表承接标签与负责人列，公海池与分配入口仍在客户域', async ({ page }) => {
    await page.goto(`${BASE_URL}/customer`);
    const table = page.locator('.el-table').first();
    await expect(table).toBeVisible({ timeout: 30000 });
    await expect(table.locator('th', { hasText: '标签' })).toHaveCount(1);
    await expect(table.locator('th', { hasText: '负责人' })).toHaveCount(1);
    await expect(page.getByRole('button', { name: '公海池' })).toBeVisible();
    await expect(page.getByRole('button', { name: '客户分配' })).toBeVisible();
  });

  test('销售分析双页签共用一个入口且以 query 为状态源', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales-analysis`);
    const tabs = page.locator('.el-tabs').first();
    await expect(tabs).toBeVisible({ timeout: 30000 });
    await expect(tabs.locator('.el-tabs__item', { hasText: '经营分析' })).toHaveCount(1);
    await expect(tabs.locator('.el-tabs__item', { hasText: '多维分析' })).toHaveCount(1);
    await tabs.locator('.el-tabs__item', { hasText: '多维分析' }).click();
    await expect(page, '切到多维分析后地址应带 tab=bi').toHaveURL(/\/sales-analysis\?tab=bi$/);
  });

  test('应收对账双页签共用一个入口且以 query 为状态源', async ({ page }) => {
    await page.goto(`${BASE_URL}/ar-reconciliation`);
    const tabs = page.locator('.el-tabs').first();
    await expect(tabs).toBeVisible({ timeout: 30000 });
    await expect(tabs.locator('.el-tabs__item', { hasText: '标准对账' })).toHaveCount(1);
    await expect(tabs.locator('.el-tabs__item', { hasText: '增强对账' })).toHaveCount(1);
    await tabs.locator('.el-tabs__item', { hasText: '增强对账' }).click();
    await expect(page, '切到增强对账后地址应带 tab=enhanced').toHaveURL(
      /\/ar-reconciliation\?tab=enhanced$/
    );
  });

  test('坯布页作为面料域权威入口，表头与出入库数据列齐全', async ({ page }) => {
    await page.goto(`${BASE_URL}/greige-fabrics`);
    const table = page.locator('.el-table').first();
    await expect(table).toBeVisible({ timeout: 30000 });
    await expect(table.locator('th', { hasText: '坯布编码' })).toHaveCount(1);
    await expect(table.locator('th', { hasText: '重量(kg)' })).toHaveCount(1);
    await expect(table.locator('th', { hasText: '长度(m)' })).toHaveCount(1);
  });

  test('侧边栏不残留已撤除的重复入口，合并后的入口各只出现一次', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    const menu = page.locator('.menu');
    await expect(menu).toBeVisible({ timeout: 30000 });
    const menuItem = (label: string) => menu.locator('.el-menu-item', { hasText: label });

    await expect(menuItem('面料列表')).toHaveCount(0);
    await expect(menuItem('CRM 管理')).toHaveCount(0);
    await expect(menuItem('应收对账增强')).toHaveCount(0);
    await expect(menuItem('BI 销售分析')).toHaveCount(0);

    await expect(menuItem('坯布管理')).toHaveCount(1);
    await expect(menuItem('客户管理')).toHaveCount(1);
    await expect(menuItem('销售分析')).toHaveCount(1);
    await expect(menuItem('应收对账')).toHaveCount(1);
  });
});
