import { test, expect, type Page } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

/**
 * 移动端竖屏布局与操作锁（补 CI 桌面视口测不到移动端的盲区）
 *
 * 视口取国内 2025 中高端手机竖屏代表：360x780（华为/小米/红米等主流 Android 的常见 CSS 逻辑宽，
 * 作最严档）与 393x852（iPhone 15/16 系）。断言列表页族在此两档竖屏下：
 * - 整页文档不横向溢出撑破（documentElement.scrollWidth 不超过 clientWidth，容差 2px）；
 *   el-table 列多时靠其内部横向滚动（el-main 自带 overflow:auto），不撑大文档宽，故本判定只针对
 *   整页被固定宽元素/头部撑破这一真缺陷，不误伤"表格内部可滑"的正常移动端形态；
 * - 主内容区已渲染（非白屏）；
 * - 移动端导航可操作（汉堡按钮可见且能开合侧边抽屉）。
 *
 * 说明：表格列挤压后的"字是否可读"（症状四）属内容排版，不体现在文档溢出上，仍由逐表 min-width
 * 批单独处理；本测锁的是"布局是否撑破整屏 + 移动端导航是否可用"。
 */

const MOBILE_VIEWPORTS = [
  { name: '360x780', width: 360, height: 780 },
  { name: '393x852', width: 393, height: 852 },
];

const LIST_PAGES = [
  { route: '/sales', label: '销售订单' },
  { route: '/purchase', label: '采购单' },
  { route: '/inventory', label: '库存' },
  { route: '/customer', label: '客户' },
  { route: '/supplier', label: '供应商' },
  { route: '/product', label: '产品' },
  { route: '/finance', label: '财务' },
];

async function measureDocumentOverflowX(page: Page): Promise<number> {
  return page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth
  );
}

for (const vp of MOBILE_VIEWPORTS) {
  test.describe(`移动端竖屏 ${vp.name}`, () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await loginViaUI(page);
    });

    test('汉堡可开合侧边抽屉（移动端导航可操作）', async ({ page }) => {
      await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded' });
      await page.waitForLoadState('networkidle', { timeout: 8000 }).catch(() => {});

      const hamburger = page.locator('.hamburger-btn');
      await expect(hamburger, `${vp.name} 竖屏应以汉堡按钮替代固定侧栏`).toBeVisible();
      await hamburger.click();

      const drawer = page.locator('.el-drawer').first();
      await expect(drawer, `${vp.name} 点击汉堡应打开侧边导航抽屉`).toBeVisible();
    });

    for (const p of LIST_PAGES) {
      test(`${p.label} ${p.route} 无横向溢出且主区渲染`, async ({ page }) => {
        await page.goto(`${BASE_URL}${p.route}`, { waitUntil: 'domcontentloaded' });
        await page.waitForLoadState('networkidle', { timeout: 8000 }).catch(() => {});
        await page.waitForTimeout(200);

        const main = page.locator('.el-main');
        await expect(main.first(), `${p.route} 主内容区应已渲染（非白屏）`).toBeVisible();

        const overflow = await measureDocumentOverflowX(page);
        expect(
          overflow,
          `${p.label}(${p.route}) 在 ${vp.name} 竖屏不应整页横向溢出撑破，实测 ${overflow}px`
        ).toBeLessThanOrEqual(2);
      });
    }
  });
}
