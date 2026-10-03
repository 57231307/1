import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import { loginAsRole, apiCallExpectFail, expectDenied, failureCode } from './helpers';

/**
 * P5.3 垂直越权端点矩阵
 *
 * admin 专属端点族（require_admin_role 保护，grep backend handlers 确认）：
 * - users（用户管理：195/312 行 admin 校验）
 * - roles（角色管理：192/267/373 行）
 * - data-permissions（137/173/191 行）
 * - field-permissions（81/114/142 行）
 * - system-update（126/183/303 行）
 * - audit-logs（146/237/364 行）
 * - login-security 管理（锁定列表）
 *
 * 验证：非 admin 角色（report_viewer / cashier）逐端点断言 403，
 *       admin 自身 200 对照（区分"端点不存在"与"权限拒绝"）。
 * 修复 09-permissions 恒真断言后的真实越权矩阵（本 spec 承接详尽断言）。
 *
 * 生产工单跨域成对矩阵（本 spec 末段）：后端把双层前缀路径
 * `/erp/production/production-orders/orders*` 的资源段消歧到 production-orders
 * （path_utils.rs ("production","orders") 映射），Rust 侧 middleware_permission_test
 * 只钉了推导层，运行时 RBAC 的负向面（销售读生产工单必须 403）与正向面
 * （production-orders:* 死授权复活后生产岗必须 200）在 e2e 零覆盖——该口径无法被证明。
 * 本节按"成对"补全：salesperson 持 orders:read 读列表/详情均 403 且失败体不泄露
 * items；production_manager 读列表 200 且分页信封形状正确；并钉销售读本域
 * /sales/orders 仍 200（消歧对其他域同名资源零回归的行为面）。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

/** admin 专属只读端点（GET，越权测试主要面） */
const ADMIN_READONLY_ENDPOINTS = [
  '/users?page=1&page_size=5',
  '/roles?page=1&page_size=5',
  '/data-permissions?page=1&page_size=5',
  '/field-permissions?page=1&page_size=5',
  '/system-update/version',
  '/system-update/status',
  '/audit-logs?page=1&page_size=5',
];

/** admin 专属写端点（POST/PUT，非 admin 必须 403） */
const ADMIN_WRITE_ENDPOINTS: Array<{
  method: string;
  path: string;
  body?: Record<string, unknown>;
}> = [
  {
    method: 'POST',
    path: '/users',
    body: { username: 'e2e_vpriv_user', password: 'Xk9#mQ2$vL8pW4nR', role_id: 1 },
  },
  { method: 'POST', path: '/roles', body: { code: 'e2e_vpriv_role', name: 'VPriv 测试角色' } },
  { method: 'POST', path: '/data-permissions', body: { scope_type: 'self' } },
];

/** 非 admin 角色（走 role-credentials.json 或 env） */
const NON_ADMIN_ROLES = ['report_viewer', 'cashier'];

test.describe('P5.3 垂直越权矩阵', () => {
  for (const role of NON_ADMIN_ROLES) {
    test.describe(`角色 ${role}`, () => {
      for (const ep of ADMIN_READONLY_ENDPOINTS) {
        test(`GET ${ep} → 403`, async ({ page }) => {
          await loginAsRole(page, role);

          const result = await apiCallExpectFail(page, 'GET', ep);
          // 非 admin 对 admin 端点：403=权限拒绝（正确）；401=会话问题（真缺陷）；200=越权（真缺陷）
          expectDenied(result, `${role} 访问 ${ep} 应 403`);
        });
      }

      for (const ep of ADMIN_WRITE_ENDPOINTS) {
        test(`${ep.method} ${ep.path} → 403`, async ({ page }) => {
          await loginAsRole(page, role);

          // apiCallExpectFail 内建 CSRF 恢复，403 只能来自权限拒绝
          const result = await apiCallExpectFail(
            page,
            ep.method as 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
            ep.path,
            ep.body
          );
          expectDenied(result, `${role} 写操作 ${ep.path} 应 403`);
        });
      }
    });
  }

  test('admin 对照：同端点可达（区分端点缺失与权限拒绝）', async ({ page }) => {
    await loginAsRole(page, 'admin');

    const reachable: string[] = [];
    const unexpected: string[] = [];
    for (const ep of ADMIN_READONLY_ENDPOINTS) {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}${ep}`);
      const status = resp.status();
      if (status < 400) reachable.push(ep);
      else if (status !== 404 && status !== 400) unexpected.push(`${ep}=${status}`);
    }

    // admin 至少可达一半端点（种子库 users/roles/audit-logs 必在）
    expect(
      reachable.length,
      `admin 可达端点数 ${reachable.length} 过少（${reachable.join(', ')}），端点族可能整体失效`
    ).toBeGreaterThanOrEqual(4);
  });
});

// ==================== 生产工单跨域成对矩阵（production-orders 消歧口径） ====================

/** 生产工单列表（双层前缀路径，消歧后 RBAC 键 = production-orders:read） */
const PROD_ORDERS_LIST = '/production/production-orders/orders?page=1&page_size=5';
/** 生产工单详情（id 是否存在无关——中间件 RBAC 先于 handler，越权必被 403 拦截） */
const PROD_ORDERS_DETAIL = '/production/production-orders/orders/1';
/** 销售订单列表（sales 域 orders 保留原名，种子即 orders:*，消歧不应波及） */
const SALES_ORDERS_LIST = '/sales/orders?page=1&page_size=5';

/** GET 原语：返回状态码 + 原文响应体（泄露断言必须打在 raw text 上，不能只看解析后字段） */
async function getRaw(page: Page, path: string) {
  const resp = await page.request.get(`${API_BASE}${API_PREFIX}${path}`, {
    headers: { 'X-Requested-With': 'XMLHttpRequest' },
  });
  return { status: resp.status(), text: await resp.text() };
}

test.describe('跨域越权成对矩阵: 生产工单 production-orders', () => {
  test('salesperson GET 生产工单列表 → 403 且响应体不泄露 items', async ({ page }) => {
    // salesperson 持 orders:read（销售域种子码）——修复前双层前缀路径把它派生成
    // 生产工单端点的放行码，即跨域越权面；消歧后该码不再覆盖 production-orders
    await loginAsRole(page, 'salesperson');
    const res = await getRaw(page, PROD_ORDERS_LIST);
    expect(
      res.status,
      'salesperson 读生产工单列表应 403（orders:read 不得覆盖 production-orders）'
    ).toBe(403);
    const json = JSON.parse(res.text) as Record<string, unknown>;
    expect(
      failureCode(json),
      `403 失败体应为 FORBIDDEN 机器码，实际体：${res.text.slice(0, 200)}`
    ).toBe('FORBIDDEN');
    // 泄露钉：拒绝路径不得把生产工单列表数据夹带进响应（信封 data/items 与业务列名双禁）
    expect(res.text).not.toContain('"items"');
    expect(res.text).not.toContain('order_no');
  });

  test('salesperson GET 生产工单详情 /{id} → 403', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const res = await getRaw(page, PROD_ORDERS_DETAIL);
    expect(res.status, 'salesperson 读生产工单详情应 403（RBAC 先于资源存在性判定）').toBe(403);
    const json = JSON.parse(res.text) as Record<string, unknown>;
    expect(failureCode(json), `403 失败体应为 FORBIDDEN，实际体：${res.text.slice(0, 200)}`).toBe(
      'FORBIDDEN'
    );
    expect(res.text).not.toContain('order_no');
  });

  test('production_manager GET 生产工单列表 → 200 且分页信封形状正确', async ({ page }) => {
    // production-orders:*（init 种子生产核心岗）修复前是死码（资源段被派生成 orders），
    // 消歧后复活——本条即"复活死授权"正向面证明；空结果不豁免形状断言
    await loginAsRole(page, 'production_manager');
    const res = await getRaw(page, PROD_ORDERS_LIST);
    expect(
      res.status,
      `production_manager 读生产工单列表应 200，实际体：${res.text.slice(0, 300)}`
    ).toBe(200);
    const json = JSON.parse(res.text) as {
      code?: number;
      data?: { items?: unknown; total?: unknown; page?: unknown; page_size?: unknown };
    };
    expect(json.code, '成功信封 code 应为 200').toBe(200);
    expect(
      Array.isArray(json.data?.items),
      `data.items 应为数组，实际：${res.text.slice(0, 300)}`
    ).toBe(true);
    expect(typeof json.data?.total, 'data.total 应为数字').toBe('number');
    expect(json.data?.page, '分页回显 page').toBe(1);
    expect(json.data?.page_size, '分页回显 page_size').toBe(5);
  });

  test('零回归: salesperson GET 本域 /sales/orders → 200（sales 域 orders 口径不变）', async ({
    page,
  }) => {
    // 消歧映射只动生产域；sales/orders* 保留原名 orders（种子即 orders:*）。
    // 本条若红成 403，说明消歧误伤了销售域资源段推导
    await loginAsRole(page, 'salesperson');
    const res = await getRaw(page, SALES_ORDERS_LIST);
    expect(res.status, `salesperson 读本域销售订单应 200，实际体：${res.text.slice(0, 300)}`).toBe(
      200
    );
    const json = JSON.parse(res.text) as { code?: number; data?: { items?: unknown } };
    expect(json.code, '成功信封 code 应为 200').toBe(200);
    expect(Array.isArray(json.data?.items), '销售订单列表 data.items 应为数组').toBe(true);
  });
});

// ==================== pieces 权限键的运行期活体证明（依据 R-7） ====================

/**
 * 匹号领域的运行时权限键是 `pieces:read` / `pieces:print`（URL 段推导，
 * path_utils.rs:102-117 默认分支取 segment4；matches_permission 按资源段精确匹配，
 * `inventory:*` 不覆盖）。三通道授予（init 矩阵 / m0069 迁移 / e2e 补授）此前只有
 * 文本级双向钉，**没有任何"真实 token 访问端点不被 RBAC 拒"的端到端证据**——
 * 而本 spec 的其余用例都用 admin（`*:*` 掩盖一切缺授），CI 永远测不到该缺口。
 * 依据 R-7：迁移在 CI 分片库对后建角色必然 0 命中，运行期证据只能由真实 token 直连给出。
 *
 * 判据只打状态码层（权限拒绝文案永久脱敏，不断 message 原文）：
 * - 正向面：`not.toBe(403)`。403 只可能来自 RBAC 拒绝；200/404/400 都说明请求已过
 *   权限门、失败发生在业务层（id 是否存在、该匹是否 dyed/实测值是否齐备与本证无关）。
 * - 负向面：salesperson 打 print 必 403（依据 R-13：print 只给库存/仓管/质检/验布岗，不给销售）。
 */
const PIECES_LIST = '/inventory/pieces?page=1&page_size=5';
/** 详情段 id 是否存在无关——RBAC 中间件先于 handler，越权必在权限层被拦 */
const PIECES_PRINT = '/inventory/pieces/1/print';

test.describe('pieces:read / pieces:print 运行期授权（依据 R-7、R-13）', () => {
  for (const role of ['warehouse_keeper', 'inventory_manager']) {
    test(`${role} GET /inventory/pieces → 非 403（pieces:read 真实进库）`, async ({ page }) => {
      await loginAsRole(page, role);
      const res = await getRaw(page, PIECES_LIST);
      expect(
        res.status,
        `${role} 读匹号列表被 RBAC 拒（403）= pieces:read 未真实授予该岗；响应体前 300 字符：${res.text.slice(0, 300)}`
      ).not.toBe(403);
    });

    test(`${role} GET /inventory/pieces/{id}/print → 非 403（pieces:print 真实进库）`, async ({
      page,
    }) => {
      await loginAsRole(page, role);
      const res = await getRaw(page, PIECES_PRINT);
      expect(
        res.status,
        `${role} 打成品布标签被 RBAC 拒（403）= pieces:print 未真实授予该岗；响应体前 300 字符：${res.text.slice(0, 300)}`
      ).not.toBe(403);
    });
  }

  test('salesperson GET /inventory/pieces → 非 403（R-13 只授 read）', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const res = await getRaw(page, PIECES_LIST);
    expect(
      res.status,
      `销售岗应能按四维筛在库匹（发货选匹依赖），实际 ${res.status}；响应体前 300 字符：${res.text.slice(0, 300)}`
    ).not.toBe(403);
  });

  test('salesperson GET /inventory/pieces/{id}/print → 403（print 不给销售）', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const res = await getRaw(page, PIECES_PRINT);
    expect(
      res.status,
      `salesperson 越权打标签必须被权限门拦成 403（R-13 最小授权），实际 ${res.status}`
    ).toBe(403);
  });
});
