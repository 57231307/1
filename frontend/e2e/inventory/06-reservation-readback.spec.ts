// 库存预留（inventory/reservations）落库回读 E2E — inventory/06
// 真实链路（逐端点对着后端源码核实，非编造）：
//   routes/inventory.rs:186-206 reservation_routes → 全部经 verifyEndpointHealthy(strict)
//   POST   /inventory/reservations            create_reservation（status=pending，inventory_reservation_service.rs:34-49）
//   GET    /inventory/reservations            list_reservations（响应体 data 形状为 {list,total,page,page_size}，
//                                             handler inventory_reservation_handler.rs:77-82，**不是 items**）
//   POST   /inventory/reservations/{id}/lock   lock_reservation（pending→locked，service.rs:68-86）
//   POST   /inventory/reservations/{id}/release release_reservation（locked/pending→released + released_at，service.rs:105-126）
//   DELETE /inventory/reservations/{id}        delete_reservation（仅 pending 可删，否则 400 BUSINESS_ERROR，service.rs:214-220）
// 落库前提（m0010_add_inventory_extensions.rs:50-65）：
//   inventory_reservations.order_id/product_id/warehouse_id 均为 NOT NULL 且带 FK，
//   order_id → sales_orders.id、product_id → products.id、warehouse_id → warehouses.id。
//   故预留必须引用真实存在的销售订单/产品/仓库，不能凭空造 id（否则 FK 裸 500 被当成"预留创建成功"是假绿）。
// 状态词表来源（写入方唯一真源，models/status/purchase_inventory.rs:48-58，小写）：pending/locked/consumed/released/cancelled。
// 断言全部走 GET 列表回读真实落库字段（status/quantity/product/warehouse/order/released_at），不只看 toast/200。
// 列表信封真相（inventory_reservation_handler.rs:77-82）：data={list,total,page,page_size}，
// **不是 items**；requireReservationList 显式断言 list 为数组，禁止 `?? []` 吞缺键。
//
// 已知后端真实缺陷（本 spec 保持判红、不放宽、不掩盖，详见交付报告）：
// reservation handler 把 service 的 4xx 业务/校验错误统一 `.map_err(AppError::internal)`
// 压成 500 INTERNAL_ERROR：handler :107-108(create) / :141-144(delete) / :162-165(lock) /
// :191-194(release)。service 本身返回的是 AppError::business/not_found（400/404），
// 状态门与 FK 负例的 expectBusinessRejection 因此会红——那是 handler 吞码缺陷，不是测试臆造契约。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  genCode,
  tryCleanup,
  verifyEndpointHealthy,
  expectBusinessRejection,
} from '../flow/helpers';

/** rust_decimal 序列化为 number|string 均可能，统一走 String→Number（与既有 04-write-down 口径一致） */
const toDec = (v: unknown): number => Number(String(v));

interface ReservationRow {
  id: number;
  order_id: number;
  product_id: number;
  warehouse_id: number;
  quantity: unknown;
  status: string;
  released_at?: string | null;
  notes?: string | null;
}

/** 按 product_id 过滤读回该产品的预留行列表（data.list，非 items；缺键/形状漂移即判红） */
async function readReservationRows(page: import('@playwright/test').Page, productId: number) {
  const res = await apiCallRaw<{ list?: unknown; total?: number }>(
    page,
    'GET',
    `/inventory/reservations?product_id=${productId}&page=1&page_size=100`
  );
  expect(
    Array.isArray(res?.list),
    `GET /inventory/reservations 响应 data.list 必须为数组（handler :77-82 信封 {list,total,page,page_size}），实际 ${JSON.stringify(res)}`
  ).toBe(true);
  return res.list as ReservationRow[];
}

/** 用 id 精确回读单条预留（product 维度过滤后前端定位，避免全局分页漏读） */
async function readReservationById(
  page: import('@playwright/test').Page,
  productId: number,
  reservationId: number
): Promise<ReservationRow | undefined> {
  const rows = await readReservationRows(page, productId);
  return rows.find(r => r.id === reservationId);
}

test.describe('库存预留 - 06 预留链路落库回读', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('预留全链路：创建(pending)→回读→lock→release→delete，落库字段逐步回读', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置：预留需至少一个产品').toBeTruthy();
    expect(warehouseId, '前置：预留需至少一个仓库').toBeTruthy();

    // ---- 预留 order_id 有 FK→sales_orders：自建一个真实销售订单作为预留载体 ----
    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E预留客户 ${genCode('RESV-C')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, `创建客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    const order = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: customerId,
      // 与 flow/helpers ensureTestEntities(:573-577) 已验证可落库的建单契约一致：
      // order_date 显式传（后端 Option、缺省取 now，services/so/order_crud.rs:265-266）；
      // quantity/unit_price 走字符串（rust_decimal 反序列化既有口径）；
      // 数量取 1 —— 建单会锁库存（helpers :532 注释），避免依赖额外库存 seed。
      order_date: new Date().toISOString(),
      items: [{ product_id: productId, quantity: '1', unit_price: '20' }],
    });
    const orderId = order.data?.id;
    expect(orderId, `创建销售订单失败（预留 FK 依赖）：${JSON.stringify(order)}`).toBeTruthy();

    const quantity = 12.5;
    const notes = `E2E 预留回读 ${genCode('RESV')}`;
    let reservationIdRef = 0;

    try {
      // 已注册端点严格健康（禁止 optional 吞 404/403）
      await verifyEndpointHealthy(page, `/inventory/reservations?product_id=${productId}`);

      // ---- 1. 创建预留（pending）----
      const created = await apiCall<Record<string, unknown>>(
        page,
        'POST',
        '/inventory/reservations',
        {
          product_id: productId,
          warehouse_id: warehouseId,
          quantity: String(quantity),
          order_id: orderId,
          notes,
        }
      );
      const reservationId = Number(created.data?.id);
      reservationIdRef = reservationId;
      expect(reservationId, `创建预留未返回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);

      // 回读：初始落库 status=pending、quantity/引用一致
      const pendingRow = await readReservationById(page, productId, reservationId);
      expect(pendingRow, `GET 列表应能回读到新建预留 ${reservationId}`).toBeDefined();
      expect(pendingRow!.status, `回读初始状态应为 pending，实际 ${pendingRow!.status}`).toBe(
        'pending'
      );
      expect(toDec(pendingRow!.quantity), `回读数量应等于提交值 ${quantity}`).toBeCloseTo(
        quantity,
        4
      );
      expect(pendingRow!.product_id, '回读 product_id 应等于引用产品').toBe(productId);
      expect(pendingRow!.warehouse_id, '回读 warehouse_id 应等于引用仓库').toBe(warehouseId);
      expect(pendingRow!.order_id, '回读 order_id 应等于引用销售订单').toBe(orderId);
      expect(pendingRow!.released_at, 'pending 阶段 released_at 应为空').toBeFalsy();

      // ---- 2. lock：pending→locked ----
      await apiCall(page, 'POST', `/inventory/reservations/${reservationId}/lock`);
      const lockedRow = await readReservationById(page, productId, reservationId);
      expect(lockedRow, `lock 后应能回读预留 ${reservationId}`).toBeDefined();
      expect(lockedRow!.status, `lock 后落库状态应为 locked，实际 ${lockedRow!.status}`).toBe(
        'locked'
      );

      // 非法状态门：locked 再 lock → 400 业务拒绝（service.rs:68 仅 pending 可锁）
      const relock = await apiCallExpectFail(
        page,
        'POST',
        `/inventory/reservations/${reservationId}/lock`
      );
      expectBusinessRejection(relock, '已 locked 的预留重复 lock 应被业务拒绝（400，非 5xx 裸崩）');

      // ---- 3. release：locked→released + released_at 落值 ----
      await apiCall(page, 'POST', `/inventory/reservations/${reservationId}/release`);
      const releasedRow = await readReservationById(page, productId, reservationId);
      expect(
        releasedRow!.status,
        `release 后落库状态应为 released，实际 ${releasedRow!.status}`
      ).toBe('released');
      expect(releasedRow!.released_at, 'release 应落库 released_at（service.rs:117）').toBeTruthy();

      // 非法状态门：released 再 release → 400 业务拒绝（service.rs:105 仅 locked/pending 可释放）
      const rerelease = await apiCallExpectFail(
        page,
        'POST',
        `/inventory/reservations/${reservationId}/release`
      );
      expectBusinessRejection(
        rerelease,
        '已 released 的预留重复释放应被业务拒绝（400，非 5xx 裸崩）'
      );

      // ---- 4. delete 状态门：非 pending（已 released）不可删 → 400 业务拒绝 ----
      const delBlocked = await apiCallExpectFail(
        page,
        'DELETE',
        `/inventory/reservations/${reservationId}`
      );
      expectBusinessRejection(
        delBlocked,
        '非 pending 预留删除应被业务拒绝（400，service.rs:214-220），不应裸 500'
      );
      // 回读：删除被拒后行仍在（守卫未误删）
      const stillThere = await readReservationById(page, productId, reservationId);
      expect(stillThere, `删除被拒后预留 ${reservationId} 应仍存在`).toBeDefined();
    } finally {
      // 预留行随订单 FK 存在；released 状态预留不可删（会被业务拒），best-effort 清理，
      // 失败仅告警不掩盖；随后删除 FK 父行（销售订单/客户）。
      await tryCleanup(
        page,
        'DELETE',
        `/inventory/reservations/${reservationIdRef}`,
        'reservation'
      );
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
    }
  });

  test('预留删除正常路径：pending 预留可删除，删除后列表不再回读到该行', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置：预留需至少一个产品').toBeTruthy();
    expect(warehouseId, '前置：预留需至少一个仓库').toBeTruthy();

    const cust = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E预留删客户 ${genCode('RESVD')}`,
    });
    const customerId = cust.data?.id;
    expect(customerId, `创建客户失败：${JSON.stringify(cust)}`).toBeTruthy();

    const order = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: customerId,
      // 同上：order_date + 字符串数量/单价（真实落库契约），建单锁库存取量 1
      order_date: new Date().toISOString(),
      items: [{ product_id: productId, quantity: '1', unit_price: '10' }],
    });
    const orderId = order.data?.id;
    expect(orderId, `创建销售订单失败（预留 FK 依赖）：${JSON.stringify(order)}`).toBeTruthy();

    try {
      const created = await apiCall<{ id?: number }>(page, 'POST', '/inventory/reservations', {
        product_id: productId,
        warehouse_id: warehouseId,
        quantity: '3',
        order_id: orderId,
        notes: `E2E 待删预留 ${genCode('RESVDEL')}`,
      });
      const reservationId = Number(created.data?.id);
      expect(reservationId, 'pending 预留应创建成功').toBeGreaterThan(0);

      // 删除 pending（合法路径，service.rs:214 允许）
      await apiCall(page, 'DELETE', `/inventory/reservations/${reservationId}`);

      // 回读：删除后按 id 不再出现
      const gone = await readReservationById(page, productId, reservationId);
      expect(gone, `删除后预留 ${reservationId} 不应再被列表回读到`).toBeUndefined();
    } finally {
      await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, 'sales_order');
      await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, 'customer');
    }
  });

  test('预留创建引用不存在销售订单：FK 前置应被拒（4xx），不静默落脏行', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置：预留需至少一个产品').toBeTruthy();
    expect(warehouseId, '前置：预留需至少一个仓库').toBeTruthy();

    // 一个必然不存在的 order_id：FK 违反。
    // 判责：create_reservation(service.rs:24-50) 无 order 存在性前置校验，直接 insert →
    // 由 DB FK 抛错。若后端把裸 FK 错误透传为 5xx(DATABASE_ERROR)，属源码缺少前置业务校验的真实缺陷
    // （见交付报告），此处**不放宽**为 ">=400 即通过"：5xx 判红，暴露缺陷而非掩盖。
    const fail = await apiCallExpectFail(page, 'POST', '/inventory/reservations', {
      product_id: productId,
      warehouse_id: warehouseId,
      quantity: '2',
      order_id: 999_999_999,
      notes: 'E2E FK 负例',
    });
    expect(
      fail.status,
      `引用不存在销售订单的预留创建不应 2xx（实际 ${fail.status} code=${fail.code} msg=${fail.message}）`
    ).toBeLessThan(500);
    expect(fail.status, 'FK 负例应 4xx 拒绝（>=400）').toBeGreaterThanOrEqual(400);
  });
});
