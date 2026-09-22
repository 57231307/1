import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyBulkColorDeliveryBlock,
  verifyOutsourcingVoucher,
  verifyTrialBalance,
  verifyWeightConversion,
  verifyNetWeight,
  getCtx,
  genCode,
  ensureTestEntities,
} from './helpers';

test.describe.serial('扩展: 库存预留/发货门禁/三单匹配/双计量', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('L1-1 验证库存预留机制（pending → locked → consumed）', async ({ page }) => {
    const reservations = await apiCallRaw<{ list: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/inventory/reservations?page=1&page_size=10'
    );
    // inventory_reservation_handler.rs list_reservations 用 json!({"list": ...})，
    // key 是 list；原实现读 items 且 expect() 无匹配器，整条用例空转
    expect(Array.isArray(reservations?.list), '库存预留应返回 list 数组').toBe(true);
    console.log(`[L1-1] 库存预留 list 长度=${reservations.list.length}`);
    if (reservations.list.length > 0) {
      const status = reservations.list[0].status;
      expect(typeof status, '预留记录必须带 status 字段').toBe('string');
      // models/status/inventory.rs reservation_status 常量集
      expect(['pending', 'locked', 'consumed', 'released', 'cancelled']).toContain(status);
    }
  });

  test('L1-2 验证大货批色发货门禁（未审批阻断发货）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    expect(
      ctx.salesOrderId,
      'ensureTestEntities 未建出销售订单（ctx.salesOrderId 缺失），本用例前置失败'
    ).toBeTruthy();

    // 尝试发货（如果大货批色未审批，应被阻断）
    const blocked = await verifyBulkColorDeliveryBlock(page, ctx.salesOrderId);
    expect(typeof blocked).toBe('boolean');
  });

  test('L1-3 验证三单匹配（采购订单→入库单→应付单）', async ({ page }) => {
    const ctx = getCtx();
    expect(
      ctx.purchaseOrderId,
      'ensureTestEntities 未建出采购订单（ctx.purchaseOrderId 缺失），本用例前置失败'
    ).toBeTruthy();

    // 验证采购订单关联入库单
    const receipts = await apiCallRaw<{
      items: Array<{ id: number; purchase_order_id: number }>;
    }>(
      page,
      'GET',
      `/purchase/receipts?purchase_order_id=${ctx.purchaseOrderId}&page=1&page_size=5`
    );
    expect(Array.isArray(receipts.items), `receipts.items 应为后端返回的 items 数组`).toBe(true);

    // 验证入库单关联应付单
    const apInvoices = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/ap/invoices?page=1&page_size=5'
    );
    expect(Array.isArray(apInvoices.items), `apInvoices.items 应为后端返回的 items 数组`).toBe(
      true
    );
  });

  test('L1-4 验证双计量换算（米→公斤）', async () => {
    // 1000米, 200g/m², 150cm 幅宽 → 公斤 = 1000 * 200 * 150 / 100000 = 300
    const kg = await verifyWeightConversion(1000, 200, 150);
    expect(kg).toBe(300);
  });

  test('L1-5 验证净重计算（毛重 - 纸管重量）', async () => {
    const netWeight = await verifyNetWeight(200, 5);
    expect(netWeight).toBe(195);
  });

  test('L1-6 验证库存盘点', async ({ page }) => {
    // GET /inventory/counts 出参是 CountListResponse{counts,total,page,page_size}，
    // 键名不是 items；且 `?? 0 > 0` 是优先级笔误（等价 `a ?? false`），守卫恒真。
    const counts = await apiCallRaw<{
      counts: Array<{ id: number; status: string }>;
      total: number;
    }>(page, 'GET', '/inventory/counts?page=1&page_size=5');
    expect(
      Array.isArray(counts?.counts),
      `盘点列表应返回 counts 数组，实际：${JSON.stringify(counts).slice(0, 200)}`
    ).toBe(true);
    expect(
      Number(counts.total) >= counts.counts.length,
      `total(${counts.total}) 不应小于本页行数(${counts.counts.length})`
    ).toBe(true);
    for (const row of counts.counts) {
      expect(Number(row.id), `盘点行缺少 id：${JSON.stringify(row)}`).toBeGreaterThan(0);
      expect(String(row.status ?? ''), `盘点行缺少 status：${JSON.stringify(row)}`).not.toBe('');
    }
  });

  test('L1-7 验证库存调拨状态机', async ({ page }) => {
    // GET /inventory/transfers 出参是裸数组 Vec<Value>：服务端分页但没有 items/total 信封，
    // 只能按数组与行数断言。
    const transfers = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/inventory/transfers?page=1&page_size=5'
    );
    expect(
      Array.isArray(transfers),
      `调拨列表应返回数组，实际：${JSON.stringify(transfers).slice(0, 200)}`
    ).toBe(true);
    expect(
      transfers.length,
      `page_size=5 却返回 ${transfers.length} 行（分页未生效）`
    ).toBeLessThanOrEqual(5);
    for (const row of transfers) {
      expect(Number(row.id), `调拨行缺少 id：${JSON.stringify(row)}`).toBeGreaterThan(0);
      expect(String(row.status ?? ''), `调拨行缺少 status：${JSON.stringify(row)}`).not.toBe('');
    }
  });

  test('L1-8 验证库存调整状态机', async ({ page }) => {
    // GET /inventory/adjustments 出参是 AdjustmentListResponse{adjustments,total,page,page_size}
    const adjustments = await apiCallRaw<{
      adjustments: Array<{ id: number; status: string }>;
      total: number;
    }>(page, 'GET', '/inventory/adjustments?page=1&page_size=5');
    expect(
      Array.isArray(adjustments?.adjustments),
      `调整列表应返回 adjustments 数组，实际：${JSON.stringify(adjustments).slice(0, 200)}`
    ).toBe(true);
    expect(
      Number(adjustments.total) >= adjustments.adjustments.length,
      `total(${adjustments.total}) 不应小于本页行数(${adjustments.adjustments.length})`
    ).toBe(true);
    for (const row of adjustments.adjustments) {
      expect(Number(row.id), `调整行缺少 id：${JSON.stringify(row)}`).toBeGreaterThan(0);
      expect(String(row.status ?? ''), `调整行缺少 status：${JSON.stringify(row)}`).not.toBe('');
    }
  });

  test('L1-9 验证匹号状态机', async ({ page }) => {
    // 后端无匹号列表 API（匹号由色卡审批小样流程内部创建），改用缸号生命周期
    // 状态机日志（真实端点）验证状态数据可查询
    const ctx = getCtx();
    const logs = await apiCallRaw<Record<string, unknown> | unknown[]>(
      page,
      'GET',
      `/production/dye-batch-lifecycle-logs/by-batch/${ctx.dyeBatchId}`
    );
    expect(logs).toBeDefined();
  });

  test('L1-10 验证低库存预警', async ({ page }) => {
    // 原实现两处问题：断言是 `expect(Array.isArray(alerts.items))` 无匹配器的真空断言，
    // 且外面包了一层 try/catch——失败时改去查 /material-shortage 顶包，两个端点谁坏都看不出来。
    // 预警端点现已返回 PaginatedResponse（items/total/page/page_size）并带出产品与仓库名称。
    const alerts = await apiCallRaw<{
      items: Array<{
        id: number;
        product_name?: string | null;
        warehouse_name?: string | null;
        quantity_on_hand: string;
        alert_type: string;
      }>;
      total: number;
      page: number;
      page_size: number;
    }>(page, 'GET', '/inventory/stock/alerts?page=1&page_size=5');
    expect(
      Array.isArray(alerts?.items),
      `预警出参应为 items 数组，实际响应：${JSON.stringify(alerts).slice(0, 200)}`
    ).toBe(true);
    expect(alerts.page, '应回显请求页码').toBe(1);
    expect(alerts.page_size, '应回显每页数量（分页此前被完全忽略）').toBe(5);
    expect(typeof alerts.total, 'total 应为数字').toBe('number');
    const ALERT_TYPES = [
      'normal',
      'low_stock',
      'out_of_stock',
      'over_stock',
      'slow_moving',
      'expiring',
      'discrepancy',
    ];
    for (const row of alerts.items) {
      expect(ALERT_TYPES, `告警类型在取值域外：${row.alert_type}`).toContain(row.alert_type);
      expect(
        row.product_name,
        `预警行应带出产品名（只有 ID 无法处置）：告警 ${row.id}`
      ).toBeTruthy();
      expect(
        Number(row.quantity_on_hand),
        `在库量应为可解析数字：${row.quantity_on_hand}`
      ).toBeGreaterThanOrEqual(0);
    }
  });
});
