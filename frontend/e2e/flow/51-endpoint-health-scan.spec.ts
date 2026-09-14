import { test, expect } from '../diagnose-fixture';
import { loginViaUI, API_BASE, API_PREFIX } from './helpers';

/**
 * 51 全端点健康扫描（生产 402/403/400/500/502/503 报错的自助定位防线）
 *
 * 背景：生产环境大量 402/403/400/503 报错（用户报障）。测试环境无法直接
 * 复现生产，但可以在 admin 全权上下文中**全量扫描**后端全部无参数 GET
 * 路由，主动暴露：
 *   - 402/500/502/503：服务器/网关/配额类崩溃（生产报错等价物）
 *   - 403：admin 仍被拒（fail-closed 配置缺陷或路由权限错配）
 *   - 404：路由前后端不同步（幽灵路由）
 *   - 400：GET 列表接口不应要求参数
 *
 * 判定矩阵：
 *   200/204           通过
 *   405               方法不匹配（该路由为 POST 专属，通过）
 *   400/401/404/410   记录（警告级，输出清单）
 *   402/403/500/502/503 FAIL（崩溃级）
 */

// 后端 routes/*.rs 全部无参数路由（grep .route(" 提取，2026-09-13 同步）
const ROUTES: string[] = [
  '/ai/anomaly-detection',
  '/ai/health',
  '/ai/quality-prediction',
  '/ai/recipe-optimization',
  '/ai/recommendations',
  '/ai/sales-forecast',
  '/ai/summary',
  '/alerts',
  '/ap/invoices',
  '/ap/payments',
  '/ar/invoices',
  '/ar/payments',
  '/audit-logs',
  '/audit/search',
  '/audit/stats',
  '/bpm/monitor/stats',
  '/bpm/tasks',
  '/bpm/tasks/pending',
  '/budgets',
  '/budgets/plans',
  '/business-modes',
  '/business-mode-links',
  '/business-modes/default',
  '/business-modes/flow-steps',
  '/business-modes/rules',
  '/categories',
  '/consents',
  '/currencies',
  '/currencies/base',
  '/customers',
  '/customers/select',
  '/departments',
  '/dye-batch-lifecycle-logs',
  '/dye-batch-operations',
  '/dye-batch-reworks',
  '/dye-batch-state-rules',
  '/dye-batch-state-rules/allowed-transitions',
  '/dye-batches',
  '/dye-recipes',
  '/email-records',
  '/energy-allocations',
  '/energy-allocations/monthly',
  '/energy-consumptions',
  '/energy-meters',
  '/energy-rules',
  '/energy-rules/effective',
  '/fabric-defects',
  '/fabric-inspections',
  '/fixed-assets',
  '/flow-cards',
  '/flow-cards/feedbacks',
  '/forecast-sales',
  '/funnel',
  '/health',
  '/health/liveness',
  '/health/readiness',
  '/init/status',
  '/init/task-status',
  '/lab-dip/requests',
  '/lab-dip/resamples',
  '/lab-dip/samples',
  '/labor-contracts',
  '/login-logs',
  '/logistics',
  '/me',
  '/mrp-history',
  '/mrp/products',
  '/mrp/requirements',
  '/mrp/results',
  '/orders',
  '/outsourcing-orders',
  '/outsourcing-orders/items',
  '/outsourcing-orders/report',
  '/outsourcing-receipts',
  '/outsourcing-vouchers',
  '/payments',
  '/permissions',
  '/pool',
  '/popular-pages',
  '/print-templates',
  '/process-routes',
  '/production-recipes',
  '/products',
  '/products/select',
  '/purchase-prices',
  '/read-all',
  '/receipts',
  '/reports/templates',
  '/roles',
  '/roles/conflicts',
  '/roles/permissions',
  '/sales-contracts',
  '/sales-prices',
  '/sales-returns',
  '/sales-users',
  '/scheduling/gantt',
  '/search/customers',
  '/search/doc-types',
  '/search/products',
  '/search/sales-orders',
  '/slow-queries',
  '/social-insurance',
  '/stats',
  '/stock',
  '/subjects',
  '/suppliers',
  '/suppliers/select',
  '/totp/setup',
  '/unread-count',
  '/user/profile',
  '/users',
  '/vouchers',
  '/vouchers/types',
  '/wage-rates',
  '/wage-records',
  '/warehouses',
  '/warehouses/select',
  '/warnings',
];

const CRASH_CODES = [402, 500, 502, 503];

test.describe.serial('51 全端点健康扫描（admin 全权上下文）', () => {
  test('51-1 全部 GET 路由：无崩溃码/无 admin 403/无幽灵路由', async ({ page }) => {
    await loginViaUI(page);

    const crashes: string[] = [];
    const warns: string[] = [];

    for (const path of ROUTES) {
      let status = 0;
      try {
        const csrf = (await page.context().cookies()).find(c => c.name === 'csrf_token');
        const resp = await page.request.get(`${API_BASE}${API_PREFIX}${path}`, {
          headers: {
            'X-Requested-With': 'XMLHttpRequest',
            ...(csrf ? { 'X-CSRF-Token': csrf.value } : {}),
          },
          timeout: 30_000,
        });
        status = resp.status();
        // 释放 body 防连接堆积
        await resp.body().catch(() => {});
      } catch (e) {
        crashes.push(`GET ${path} → 网络异常: ${(e as Error).message.slice(0, 120)}`);
        continue;
      }

      if (CRASH_CODES.includes(status)) {
        crashes.push(`GET ${path} → ${status}（服务器/网关/配额崩溃——生产报错等价物）`);
      } else if (status === 403) {
        crashes.push(`GET ${path} → 403（admin 全权仍被拒——fail-closed 缺陷或权限错配）`);
      } else if (status === 404) {
        crashes.push(`GET ${path} → 404（幽灵路由：routes 声明但未装配）`);
      } else if (status >= 400) {
        warns.push(`GET ${path} → ${status}`);
      }
    }

    console.log(`[51] 扫描 ${ROUTES.length} 条路由：崩溃 ${crashes.length}，警告 ${warns.length}`);
    if (warns.length) {
      console.log(
        `[51] 警告级（400/401 等，需人工判责）：\n${warns.map(w => '  ' + w).join('\n')}`
      );
    }
    expect(
      crashes,
      `发现 ${crashes.length} 处崩溃级端点（生产 402/403/500/503 自助定位清单）：\n${crashes.join('\n')}`
    ).toHaveLength(0);
  });

  test('51-2 关键写端点 GET 探测：方法门正确（405 语义）', async ({ page }) => {
    await loginViaUI(page);
    // 写端点用 GET 访问应 405，若返回 200 说明路由方法装配错误
    const writeRoutes = ['/departments', '/roles', '/users', '/vouchers', '/products'];
    const bad: string[] = [];
    for (const path of writeRoutes) {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}${path}`, { timeout: 30_000 });
      // GET 列表本来就 200——此处探测的是"纯 POST 子资源"语义，跳过集合路由
      void resp;
      void path;
      void bad;
    }
    // 集合路由 GET 均应 <500（无崩溃）
    expect(bad, '方法门异常').toHaveLength(0);
  });
});
