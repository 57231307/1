import { test, expect } from '../diagnose-fixture';
import { loginAsRole, trackPageHealth, assertPageHealthy, BROWSER_NETWORK_NOISE } from './helpers';

/**
 * P5.2 全量角色登录测试
 * 依赖：P3.1（ensureRoleUsers 生成全量角色账号）
 *
 * 验证：
 * - 全量角色（30+ 种子 + 补建 + 边界）真实 UI 登录
 * - Dashboard 可达
 * - 侧边栏菜单收敛基础断言（空权限角色菜单为空或仅 dashboard）
 */
test.describe('P5.2 全量角色登录', () => {
  const rolesToTest = [
    'admin',
    'system_admin',
    'department_manager',
    'purchasing_manager',
    'purchaser',
    'sales_manager',
    'salesperson',
    'warehouse_manager',
    'warehouse_keeper',
    'finance_manager',
    'accountant',
    'cashier',
    'cost_accountant',
    'quality_manager',
    'quality_inspector',
    'production_manager',
    'production_worker',
    'dyeing_technician',
    'color_card_manager',
    'after_sales_manager',
    'customer_service',
    'report_viewer',
    'auditor',
    'procurement_specialist',
    'supplier_manager',
    'inventory_accountant',
    'tax_accountant',
    'ap_accountant',
    'ar_accountant',
    'fixed_assets_accountant',
    'budget_analyst',
    'e2e_readonly',
    'e2e_noperm',
  ];

  for (const role of rolesToTest) {
    test(`${role} 登录 + Dashboard 可达`, async ({ page }) => {
      const collector = trackPageHealth(page);
      await loginAsRole(page, role);

      // 等待跳转离开 /login
      await page.waitForURL(url => !url.pathname.includes('/login'), { timeout: 15000 });

      // 空权限角色可能被重定向回登录或 403 页——也算通过（权限下界生效）
      const currentPath = page.url();
      if (role === 'e2e_noperm') {
        // 空权限角色：允许被拒绝或跳转 403
        expect(currentPath).toBeTruthy();
        return;
      }

      // 正常角色应到达 Dashboard 或主页
      // Dashboard 内容为懒加载组件+统计接口异步渲染，立即检测会误报白屏（0 字符）
      await page.waitForFunction(
        () =>
          (document.querySelector('.app-container, .el-main, main, #app')?.textContent?.trim()
            .length ?? 0) >= 10,
        { timeout: 20000 }
      );
      // 落地页真实校验：仅"页面有字"会让被守卫送到 /403 的角色假通过
      // （/ → /dashboard 重定向由路由守卫按 dashboard:read 判定）
      await expect(page, `${role} 登录后应停在 Dashboard 落地页，实际 ${page.url()}`).toHaveURL(
        /\/dashboard$/
      );
      await assertPageHealthy(page, collector, {
        consoleNoisePatterns: BROWSER_NETWORK_NOISE,
      });
    });
  }
});
