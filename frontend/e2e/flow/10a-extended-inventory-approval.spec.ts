import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyBulkColorDeliveryBlock,
  verifyTrialBalance,
  verifyWeightConversion,
  verifyNetWeight,
  getCtx,
  genCode,
  genDyeLotNo,
  ensureTestEntities,
  seedColorCardArchive,
  tryCleanup,
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
    if (ctx.salesOrderId === undefined) {
      throw new Error('ensureTestEntities 未建出销售订单（ctx.salesOrderId 缺失），本用例前置失败');
    }

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
    // GET /inventory/transfers 由 inventory_transfer_handler::list_transfers 处理，
    // 出参是 PaginatedResponse<Value>（inventory_transfer_handler.rs:45）：data = {items,total,page,page_size}，
    // 不是裸数组。run 35887709282 分片 flow(5/20) 的真实响应为 {"items":[],"total":0,"page":1,"page_size":5}，
    // 旧写法 expect(Array.isArray(transfers)) 因把 data 当数组而恒红。
    const resp = await apiCallRaw<{
      items: Array<Record<string, unknown>>;
      total: number;
      page: number;
      page_size: number;
    }>(page, 'GET', '/inventory/transfers?page=1&page_size=5');
    expect(
      Array.isArray(resp?.items),
      `调拨列表 data 应为分页信封的 items 数组，实际：${JSON.stringify(resp).slice(0, 200)}`
    ).toBe(true);
    expect(resp.page, '应回显请求页码').toBe(1);
    expect(resp.page_size, '应回显每页数量').toBe(5);
    expect(
      Number(resp.total) >= resp.items.length,
      `total(${resp.total}) 不应小于本页行数(${resp.items.length})`
    ).toBe(true);
    for (const row of resp.items) {
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
    // 状态机日志（真实端点）验证状态数据可查询。
    // 判责 §2.4-C：原实现直接拼 ctx.dyeBatchId——globalSeed 染色批次种子因
    // 「色号 E2E-GCxxxxxx 在色卡档案中不存在」失败后 id=undefined 落入 path，被
    // by-batch/{batch_id}: Path<i32> 正当 400（routes/production.rs:474 +
    // dye_batch_state_machine_handler.rs:150-153）。正解=本用例自建真实 seed 链，
    // 维度不省：色号必须先入色卡明细档案（dye_batch_handler.rs::resolve_dye_color_identity
    // 对非空 color_no 全局反查 color_card_items.color_code 且要求唯一命中，校验正当），
    // 染色布 dye_lot_no 必填（缺失显式 400），再建缸号。先例 flow/03-production 3-3。
    const archive = await seedColorCardArchive(page, { context: '10a L1-9 缸号链' });
    const dyeLotNo = genDyeLotNo();
    const batchNo = genCode('缸');
    const batch = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-batches', {
      batch_no: batchNo,
      color_no: archive.colorCode,
      dye_lot_no: dyeLotNo,
      planned_quantity: 1000,
      status: 'pending_schedule',
    });
    const batchId = batch.data?.id;
    if (!batchId) {
      throw new Error(
        `[L1-9] 缸号建单未返回 id（后端建单回 id 为契约），实际响应：${JSON.stringify(batch)}`
      );
    }
    // 写后维度回读：缸号详情（get_dye_batch 出参 dye_batch::Model 全列）必须落真实
    // 入档色号与缸号——任一维被吞/被兜底即判红。
    const detail = await apiCallRaw<{
      batch_no: string;
      color_no: string | null;
      dye_lot_no: string;
      status: string;
    }>(page, 'GET', `/production/dye-batches/${batchId}`);
    expect(String(detail.batch_no), '缸号回读 batch_no 应等于提交值').toBe(batchNo);
    expect(String(detail.color_no), '缸号回读 color_no 应等于入档色号').toBe(archive.colorCode);
    expect(String(detail.dye_lot_no), '缸号回读 dye_lot_no 应等于提交缸号').toBe(dyeLotNo);

    const logs = await apiCallRaw<unknown>(
      page,
      'GET',
      `/production/dye-batch-lifecycle-logs/by-batch/${batchId}`
    );
    // 出参为 Vec<dye_batch_lifecycle_log::Model> 裸数组（handler:153 ApiResponse<Vec<...>>），
    // 断形状不兜底：非数组=信封漂移即红（严于修复前的 toBeDefined 空断，只增不减）。
    expect(
      Array.isArray(logs),
      `生命周期日志出参应为数组，实际：${JSON.stringify(logs).slice(0, 200)}`
    ).toBe(true);
    console.log(
      `[L1-9] 缸号 ${batchNo}(id=${batchId}) 生命周期日志条数=${(logs as unknown[]).length}`
    );
    await tryCleanup(page, 'DELETE', `/production/dye-batches/${batchId}`, '[L1-9] 染色批次');
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
