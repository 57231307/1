// 越权（IDOR / data-scope）真实 403 断言 — enhanced/role-idor-sales-and-customers
// 覆盖的真实功能面（逐端点对后端源码核实，非编造）：
//   1) 销售订单行级数据范围守卫：GET /sales/orders/{id}（routes/sales.rs /orders/{id}
//      → handlers/sales_order_handler.rs:164 get_order → services/so/order_query.rs:333
//      get_order_detail → validate_order_data_scope（order_query.rs:388-403）→
//      utils/data_scope.rs:149 check_resource_owner（Self 仅本人 created_by）→
//      AppError::permission_denied → HTTP 403 code=FORBIDDEN。
//      出参 message 为脱敏常量「无权限」（utils/error.rs:500 + messages.rs PERMISSION_PUBLIC）。
//   2) 销售订单列表 self 行级过滤：GET /sales/orders（order_query.rs:103-143
//      apply_department_scope(CreatedBy)）——self 用户列表不含他人 created_by 行。
//   3) 客户端点 RBAC 分层事实：GET /crm/customers/{id} 对未持 customers:read 权限码的
//      非 admin 角色被权限中间件拦截（middleware/permission.rs:158 forbidden_response，
//      直出体 message=「权限不足，无法访问该资源」原文透传，utils/response.rs:142-143）。
// 反假绿设计（双向前置证明）：
//   数据范围 403 用例中，入侵者必须先以 200 读到订单列表（证明 RBAC orders:read 已过门、
//   且列表按 self 过滤不含目标行），其对他人订单的 403 才只能是 handler 行级校验所致；
//   同时以 message 钉死 403 来源层——「无权限」= handler 数据范围层，
//   「权限不足，无法访问该资源」= RBAC 中间件层，两种 403 禁止混为一谈。
// 角色键（只取仓库真实种子角色，键=role-credentials.json 中由 global-setup ensureRoleUsers
//   写入的角色 code；曾有不存在的 purchase_clerk/sales_rep 误用造成整批假绿，禁用）：
//   salesperson / sales_manager：SEED_ROLES（global-setup.ts:391-423）+ SEED_ROLE_EXTRA_PERMISSIONS
//     授 orders:read（global-setup.ts:453,456）；经 POST /roles 创建的角色 data_scope 默认
//     「self」（backend/src/services/role_permission_service.rs:184-185），与
//     data-scope-isolation.spec.ts 既有定性结论同口径。
//   写操作（seed 客户/订单与清理）由默认 e2e_admin 上下文完成（RBAC 全通、all scope），
//   订单 created_by=管理员 id，self 角色越权读必然过不了 check_resource_owner。
// 仍覆盖不到的风险点（如实标注，交回编排方）：
//   当前角色种子里没有任何**非 admin** 角色持有 customers:read 权限码，
//   因此「客户详情/360 数据范围层 403」（customer_ops/crud.rs:88-94、crm/cust.rs:159-165）
//   在现有授权面下不可达——第 3 节用例只能如实钉 RBAC 层 403，钉不了 data-scope 层。
import { test, expect } from '@playwright/test';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  tryCleanup,
  getRoleCredential,
  loginInIsolatedContext,
  expectDenied,
  type IsolatedAuthedSession,
} from '../flow/helpers';

/** 用某角色凭证在独立 context 真实登录；缺失凭证属 setup 缺陷，判红不 skip 掩盖 */
async function openRoleSession(
  browser: import('@playwright/test').Browser,
  role: string
): Promise<IsolatedAuthedSession> {
  const cred = getRoleCredential(role);
  if (!cred) {
    throw new Error(
      `角色 ${role} 凭证不存在——global-setup ensureRoleUsers 未就绪，属 setup 缺陷判红`
    );
  }
  return loginInIsolatedContext(browser, cred.username, cred.password);
}

/** 取一个真实产品 id（订单明细 FK 依赖产品存在）——/products 返回 PaginatedResponse{items} */
async function pickProductId(page: import('@playwright/test').Page): Promise<number> {
  const res = await apiCallRaw<{ items: Array<{ id: number }> }>(
    page,
    'GET',
    '/products?page=1&page_size=5'
  );
  expect(Array.isArray(res.items), 'GET /products 响应缺 data.items 数组').toBe(true);
  const id = res.items[0]?.id;
  if (!id) throw new Error('前置：需要一个已存在的产品用于订单明细 FK');
  return id;
}

test.describe('越权 IDOR/data-scope — 销售订单与客户（self 角色）', () => {
  test('销售订单：self 角色列表 200（RBAC 已过门）且不含他人行，按 id 越权读他人订单返回真实 403（数据范围层）', async ({
    page,
    browser,
  }) => {
    const adminId = (await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me')).id;
    const productId = await pickProductId(page);

    // owner=默认 e2e_admin（all scope）：seed 客户 + 订单（created_by=admin，
    // 行金额=qty*price=10*10=100，order_crud.rs:349 round 2，无折扣/税）
    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E越权订单客户 ${genCode('IDO')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, `seed 客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    const order = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: customerId,
      items: [{ product_id: productId, quantity: 10, unit_price: 10 }],
    });
    const orderId = order.data?.id;
    expect(orderId, `seed 销售订单失败：${JSON.stringify(order)}`).toBeTruthy();

    const intruders: Array<{ role: string; session: IsolatedAuthedSession }> = [];
    try {
      // 前置（存在性 + 归属真值回读）：admin 读自己刚建的订单 → 2xx，且 created_by 落库为本人
      const own = await apiCallRaw<{
        id: number;
        created_by: number;
        total_amount: unknown;
      }>(page, 'GET', `/sales/orders/${orderId}`);
      expect(own.id, 'admin 应能读到自己创建的订单').toBe(orderId);
      expect(own.created_by, '订单 created_by 应落库为 admin（self 越权前提）').toBe(adminId);
      expect(
        Number(String(own.total_amount)),
        `订单金额应落库为 seed 值 100，实际 ${own.total_amount}`
      ).toBeCloseTo(100, 2);

      for (const role of ['salesperson', 'sales_manager']) {
        const session = await openRoleSession(browser, role);
        intruders.push({ role, session });

        // 反假绿关键步①：intruder 以 orders:read 真实 200 读到订单列表
        //（apiCall 非 2xx 会抛 → 能走到下一行即证明 RBAC 门已过，其详情 403 不可能是缺路由权限）
        const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
          session.page,
          'GET',
          '/sales/orders?page=1&page_size=100'
        );
        expect(Array.isArray(list.items), `${role} 订单列表响应缺 data.items 数组`).toBe(true);

        // 反假绿关键步②：self 行级过滤生效——列表不得出现 admin 的私有订单
        expect(
          list.items.some(o => o.id === orderId),
          `${role}(self scope) 列表不应出现 created_by=admin 的私有订单 ${orderId}`
        ).toBe(false);

        // 越权按 id 读他人订单 → 真实 403（status+code+来源层三钉）
        const cross = await apiCallExpectFail(session.page, 'GET', `/sales/orders/${orderId}`);
        expectDenied(cross, `${role}(self) 按 id 读他人销售订单应返回 403`);
        expect(cross.code, `403 失败体 code 应为 FORBIDDEN，实际 ${JSON.stringify(cross)}`).toBe(
          'FORBIDDEN'
        );
        expect(
          String(cross.message),
          `${role} 的 403 应来自 handler 行级数据范围校验（脱敏常量「无权限」，utils/error.rs:500），而非 RBAC 直出「权限不足，无法访问该资源」（那意味着 orders:read 授码漂移）`
        ).toBe('无权限');
      }
    } finally {
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
      for (const { session } of intruders) await session.close();
    }
  });

  test('客户：未持 customers:read 的非 admin 角色被 RBAC 层 403（如实钉来源层）；admin 对照 2xx + 字段回读', async ({
    page,
    browser,
  }) => {
    // admin seed 客户并回读（证明资源真实存在且可达——后续 403 由身份/权限码决定）
    const customerName = `E2E越权分层客户 ${genCode('ICO')}`;
    const oc = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: customerName,
    });
    const customerId = oc.data?.id;
    expect(customerId, `seed 客户失败：${JSON.stringify(oc)}`).toBeTruthy();

    const intruders: IsolatedAuthedSession[] = [];
    try {
      const adminRead = await apiCallRaw<{ id: number; customer_name: string }>(
        page,
        'GET',
        `/crm/customers/${customerId}`
      );
      expect(adminRead.id, 'admin 应读到自己的客户').toBe(customerId);
      expect(adminRead.customer_name, '客户详情应回读到落库名称').toBe(customerName);

      for (const role of ['salesperson', 'customer_service']) {
        const session = await openRoleSession(browser, role);
        intruders.push(session);

        // 现状种子权限里不存在任何非 admin 角色的 customers:read
        //（global-setup SEED_ROLE_EXTRA_PERMISSIONS 未授予；后端 permission.rs 的
        // sales_rep/crm_rep 授码对应角色不在前端种子键内）——
        // 因此此处的 403 只能钉 RBAC 中间件层（message 直出原文），**不得**冒充数据范围层；
        // 数据范围层（crud.rs:88-94）对他人客户不可达，作为覆盖缺口如实交回编排方。
        const cross = await apiCallExpectFail(session.page, 'GET', `/crm/customers/${customerId}`);
        expectDenied(cross, `${role} 读他人客户应返回 403（RBAC 层，权限码缺失）`);
        expect(
          String(cross.message),
          `${role} 的 403 应钉在 RBAC 直出体（middleware/permission.rs:158，message 透传原文）；若变为「无权限」说明种子授码变了，本用例应随之重写为数据范围钉`
        ).toBe('权限不足，无法访问该资源');

        // 列表入口同样被 RBAC 拦（钉列表与详情同层，防止把列表 403 误当行级过滤）
        const listFail = await apiCallExpectFail(
          session.page,
          'GET',
          '/crm/customers?page=1&page_size=10'
        );
        expectDenied(listFail, `${role} 无 customers:read，客户列表也应 403（RBAC 层）`);
        expect(String(listFail.message), `${role} 列表 403 应与详情同层（RBAC 直出）`).toBe(
          '权限不足，无法访问该资源'
        );
      }
    } finally {
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
      for (const s of intruders) await s.close();
    }
  });

  test('data-scope 正向对照：self 入侵者对同一路径模式的可读面 200，证明上一用例的订单 403 由行级归属而非路径不可达', async ({
    page,
    browser,
  }) => {
    // 该用例钉住"两种 403 互斥"的对照面：salesperson 对 /sales/orders 集合（RBAC 已过门）
    // 读列表 200，但对 admin 的单条订单详情 403——同一 resource 同一 action，
    // 差别仅在记录归属 ⇒ 403 由 created_by 数据范围决定，杜绝把 RBAC/路由缺失伪装成越权防护。
    const adminId = (await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me')).id;
    const productId = await pickProductId(page);
    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E越权对照客户 ${genCode('ICAD')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, `seed 客户失败：${JSON.stringify(cust)}`).toBeTruthy();
    const order = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: customerId,
      items: [{ product_id: productId, quantity: 2, unit_price: 5 }],
    });
    const orderId = order.data?.id;
    expect(orderId, `seed 销售订单失败：${JSON.stringify(order)}`).toBeTruthy();

    const intruder = await openRoleSession(browser, 'salesperson');
    try {
      // created_by 归属真值（越权前提的最后一环）
      const asAdmin = await apiCallRaw<{ created_by: number }>(
        page,
        'GET',
        `/sales/orders/${orderId}`
      );
      expect(asAdmin.created_by, '对照订单 created_by 应为 admin').toBe(adminId);

      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
        intruder.page,
        'GET',
        '/sales/orders?page=1&page_size=100'
      );
      expect(Array.isArray(list.items), 'salesperson 列表应 2xx 且含 data.items').toBe(true);

      const cross = await apiCallExpectFail(intruder.page, 'GET', `/sales/orders/${orderId}`);
      expectDenied(cross, 'salesperson 按 id 读 admin 订单应 403（数据范围层）');
      expect(String(cross.message), '403 来源层应为 handler 数据范围校验脱敏文案').toBe('无权限');
    } finally {
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
      await intruder.close();
    }
  });
});
