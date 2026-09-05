import { test, expect } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  getCtx,
  genCode,
  genName,
  genDyeLotNo,
  ensureTestEntities,
} from './helpers';
import { uiCreateDialog } from './ui-helpers';

// 匹号/缸号领域真实链路测试（docs/piece-number-domain-design.md）
// 编号语义（用户 2026-09-05 二次确认）：
//   生产匹 = 生产单号下的产品生产出来的第 * 匹（batch_no 记生产单号）
//   染色匹 = 缸号/染色批次号染色后的第 * 匹（piece_no={缸号}-{seq:03}，batch_no=缸号，piece_seq 同缸递增）

interface PieceItem {
  id: number;
  piece_no: string;
  piece_type: string;
  batch_no: string;
  dye_lot_no: string | null;
  piece_seq: number | null;
  machine_no: string | null;
  machine_operator: string | null;
  product_id: number;
  warehouse_id: number;
  warehouse_name: string | null;
  warehouse_type: string | null;
}

async function fetchPieces(
  page: import('@playwright/test').Page,
  params: Record<string, string | number>
): Promise<PieceItem[]> {
  const qs = Object.entries(params)
    .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`)
    .join('&');
  const res = await apiCallRaw<{ items: PieceItem[] }>(page, 'GET', `/inventory/pieces?${qs}`);
  return res.items || [];
}

test.describe
  .serial('Shard 7: 匹号领域真实链路（报工逐匹→染色外发回仓→净布例外→仓库约束→四维追溯）', () => {
  const dyeLotNo = genDyeLotNo();
  let greigeWarehouseId = 0;
  let finishedWarehouseId = 0;
  let productionOrderId = 0;
  let productionOrderNo = '';
  let flowCardId = 0;
  let stepId = 0;
  const greigePieceNo1 = `GR-${genCode('P')}-001`;
  const greigePieceNo2 = `GR-${genCode('P')}-002`;
  const machineNo = 'M-E2E-001';

  test('7-1 前置：创建胚布仓 + 成品仓（仓库类型约束基础）', async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 复用已有仓库，缺失则补建；优先用 API 查询
    const existing = await apiCallRaw<{
      items: Array<{ id: number; warehouse_type: string | null }>;
    }>(page, 'GET', '/warehouses?page=1&page_size=200');
    const greige = existing.items.find(w => w.warehouse_type === 'greige');
    const finished = existing.items.find(w => w.warehouse_type === 'finished');
    greigeWarehouseId = greige?.id || 0;
    finishedWarehouseId = finished?.id || 0;
    // 缺失则 UI 创建（i18n 中文标签）
    if (!greigeWarehouseId) {
      console.log('[7-1] 胚布仓缺失，UI 创建');
      greigeWarehouseId =
        (await uiCreateDialog(page, '/warehouse', '/warehouses', /新建仓库/, /保存|确定/, [
          { kind: 'input', label: '仓库编码', value: genCode('E2E-GW') },
          { kind: 'input', label: '仓库名称', value: genName('E2E胚布仓') },
          { kind: 'select', label: '类型', value: '胚布仓' },
        ])) || ctx.warehouseIds[0];
    }
    if (!finishedWarehouseId) {
      console.log('[7-1] 成品仓缺失，UI 创建');
      finishedWarehouseId =
        (await uiCreateDialog(page, '/warehouse', '/warehouses', /新建仓库/, /保存|确定/, [
          { kind: 'input', label: '仓库编码', value: genCode('E2E-FW') },
          { kind: 'input', label: '仓库名称', value: genName('E2E成品仓') },
          { kind: 'select', label: '类型', value: '成品仓' },
        ])) || ctx.warehouseIds[1];
    }
    console.log('[7-1] 胚布仓 ID=', greigeWarehouseId, '成品仓 ID=', finishedWarehouseId);
    expect(greigeWarehouseId).toBeGreaterThan(0);
    expect(finishedWarehouseId).toBeGreaterThan(0);
  });

  test('7-2 创建生产订单（生产匹的承载单号）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    productionOrderNo = genCode('PO');
    console.log('[7-2] 创建生产订单', productionOrderNo, 'product_id=', ctx.productIds[0]);
    const result = await apiCall<{ id: number; order_no: string }>(
      page,
      'POST',
      '/production/production-orders/orders',
      {
        order_no: productionOrderNo,
        product_id: ctx.productIds[0],
        planned_quantity: 100,
      }
    );
    productionOrderId = result.data.id;
    productionOrderNo = result.data.order_no;
    console.log('[7-2] 生产订单创建成功 id=', productionOrderId, 'no=', productionOrderNo);
    expect(productionOrderId).toBeGreaterThan(0);
  });

  test('7-3 创建流转卡并推进至备布完成', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const card = await apiCall<{ id: number; card_no: string }>(
      page,
      'POST',
      '/production/flow-cards',
      {
        production_order_id: productionOrderId,
        product_id: ctx.productIds[0],
        product_name: genName('E2E胚布'),
        planned_fabric_weight: 100,
      }
    );
    flowCardId = card.data.id;
    console.log('[7-3] 流转卡创建 id=', flowCardId, 'card_no=', card.data.card_no);
    expect(flowCardId).toBeGreaterThan(0);
    // PENDING → SCHEDULED → PREPARING
    await apiCall(page, 'POST', `/production/flow-cards/${flowCardId}/schedule`, {});
    await apiCall(page, 'POST', `/production/flow-cards/${flowCardId}/start-preparing`);
    // 完成备布：实际配布数量
    await apiCall(page, 'POST', `/production/flow-cards/${flowCardId}/complete-preparing`, {
      actual_fabric_weight: 100,
    });
    console.log('[7-3] 流转卡状态机推至 preparing 完成');
  });

  test('7-4 生产报工逐匹登记（2 匹生产匹，胚布仓）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    // 启动工序
    const step = await apiCall<{ id: number }>(page, 'POST', '/production/flow-cards/steps/start', {
      flow_card_id: flowCardId,
    });
    stepId = step.data.id;
    console.log('[7-4] 工序记录启动 step_id=', stepId);
    expect(stepId).toBeGreaterThan(0);
    // 报工逐匹：pieces 传 2 匹，入胚布仓
    const pieces = [
      {
        piece_no: greigePieceNo1,
        machine_no: machineNo,
        machine_operator: 'E2E开机人1',
        length: 50,
        weight: 25,
        warehouse_id: greigeWarehouseId,
      },
      {
        piece_no: greigePieceNo2,
        machine_no: machineNo,
        machine_operator: 'E2E开机人2',
        length: 50,
        weight: 25,
        warehouse_id: greigeWarehouseId,
      },
    ];
    await apiCall(page, 'POST', `/production/flow-cards/steps/${stepId}/complete`, {
      actual_quantity: 100,
      qualified_quantity: 100,
      pieces,
    });
    console.log('[7-4] 报工逐匹登记完成（2 匹生产匹）');

    // 断言生产匹生成（batch_no = 生产单号）
    const greigePieces = await fetchPieces(page, {
      piece_type: 'greige',
      batch_no: productionOrderNo,
    });
    console.log('[7-4] 查询生产匹数量=', greigePieces.length, '批次号=', productionOrderNo);
    expect(greigePieces.length).toBeGreaterThanOrEqual(2);
    const p1 = greigePieces.find(p => p.piece_no === greigePieceNo1);
    const p2 = greigePieces.find(p => p.piece_no === greigePieceNo2);
    expect(p1).toBeDefined();
    expect(p2).toBeDefined();
    // batch_no = 生产单号；warehouse_type = greige；无缸号
    expect(p1!.batch_no).toBe(productionOrderNo);
    expect(p1!.warehouse_type).toBe('greige');
    expect(p1!.dye_lot_no).toBeNull();
    expect(p2!.batch_no).toBe(productionOrderNo);
    expect(p2!.machine_no).toBe(machineNo);
  });

  test('7-5 仓库类型约束：生产匹入成品仓被拒', async ({ page }) => {
    await loginViaUI(page);
    const pieceNo = `GR-${genCode('P')}-FAIL`;
    // 用 apiCall（带 CSRF 恢复）+ try/catch 捕获业务错误
    // apiCallExpectFail 不走 CSRF 恢复，会因 token 过期返回 403 误判
    let status = 0;
    let message = '';
    try {
      await apiCall(page, 'POST', `/production/flow-cards/steps/${stepId}/complete`, {
        actual_quantity: 10,
        qualified_quantity: 10,
        pieces: [
          {
            piece_no: pieceNo,
            machine_no: machineNo,
            machine_operator: 'E2E约束测试',
            length: 10,
            warehouse_id: finishedWarehouseId,
          },
        ],
      });
    } catch (e) {
      const err = e as { status?: number; message?: string };
      status = err.status || 0;
      message = err.message || '';
    }
    console.log('[7-5] 生产匹入成品仓响应 status=', status, 'message=', message);
    // 成品仓只存染色后/工艺后成品，生产匹（胚布 greige）必须入胚布仓
    // apiCall 把业务错误包装成 Error 抛出（code=BUSINESS_ERROR），status=0 也算被拒
    const rejected = status === 0 || [400, 409, 422].includes(status);
    expect(rejected).toBeTruthy();
    expect(message).toContain('BUSINESS_ERROR');
  });

  test('7-6 染色外发：订单 + 发料 + 回仓确认（染色匹生成）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const orderNo = genCode('OS');
    const issueDate = new Date().toISOString().slice(0, 10);
    console.log('[7-6] 染色外发订单', orderNo, '缸号=', dyeLotNo);

    // 1. 创建委外订单（dyeing + dye_lot_no）
    const order = await apiCall<{ id: number }>(page, 'POST', '/production/outsourcing-orders', {
      order_no: orderNo,
      order_type: 'dyeing',
      supplier_id: ctx.supplierId,
      production_order_id: productionOrderId,
      dye_lot_no: dyeLotNo,
      issue_date: issueDate,
      issue_quantity: 50,
      issue_unit: '米',
      material_cost: 1000,
    });
    const orderId = order.data.id;
    console.log('[7-6] 委外订单创建 id=', orderId);

    // 2. 发料（draft → issued）
    await apiCall(page, 'POST', `/production/outsourcing-orders/${orderId}/issue`);
    console.log('[7-6] 委外订单已发料');

    // 3. 回仓创建（draft）
    const receiptNo = genCode('RC');
    const receiptDate = new Date().toISOString().slice(0, 10);
    const receipt = await apiCall<{ id: number }>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: receiptNo,
        outsourcing_order_id: orderId,
        receipt_date: receiptDate,
        product_id: ctx.productIds[0],
        dye_lot_no: dyeLotNo,
        warehouse_id: finishedWarehouseId,
        return_quantity: 45,
        quality_status: 'passed',
        grade: 'A',
      }
    );
    const receiptId = receipt.data.id;
    console.log('[7-6] 回仓单创建 id=', receiptId, 'no=', receiptNo);

    // 4. 确认回仓（confirm 触发缸号自动建档 + 染色匹生成）
    await apiCall(page, 'POST', `/production/outsourcing-receipts/${receiptId}/confirm`);
    console.log('[7-6] 回仓确认完成（染色匹生成）');

    // 断言染色匹生成：piece_no={缸号}-001，batch_no=缸号，piece_seq=1
    const dyedPieces = await fetchPieces(page, {
      piece_type: 'dyed',
      dye_lot_no: dyeLotNo,
    });
    console.log('[7-6] 染色匹查询结果数量=', dyedPieces.length, '缸号=', dyeLotNo);
    expect(dyedPieces.length).toBeGreaterThanOrEqual(1);
    const dyed = dyedPieces[0];
    const expectedPieceNo = `${dyeLotNo}-001`;
    console.log(
      '[7-6] 染色匹 piece_no=',
      dyed.piece_no,
      '期望=',
      expectedPieceNo,
      'batch_no=',
      dyed.batch_no,
      'piece_seq=',
      dyed.piece_seq
    );
    expect(dyed.piece_no).toBe(expectedPieceNo);
    expect(dyed.batch_no).toBe(dyeLotNo);
    expect(dyed.piece_seq).toBe(1);
    expect(dyed.dye_lot_no).toBe(dyeLotNo);
    expect(dyed.warehouse_type).toBe('finished');
  });

  test('7-7 净布外发：无缸号回仓（胚布匹例外，允许入成品仓）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const orderNo = genCode('OS-NET');
    const issueDate = new Date().toISOString().slice(0, 10);
    console.log('[7-7] 净布外发订单', orderNo, '（无缸号）');

    // 净布外发：order_type=finishing，无 dye_lot_no
    const order = await apiCall<{ id: number }>(page, 'POST', '/production/outsourcing-orders', {
      order_no: orderNo,
      order_type: 'finishing',
      supplier_id: ctx.supplierId,
      production_order_id: productionOrderId,
      issue_date: issueDate,
      issue_quantity: 30,
      issue_unit: '米',
      material_cost: 500,
    });
    const orderId = order.data.id;
    await apiCall(page, 'POST', `/production/outsourcing-orders/${orderId}/issue`);
    console.log('[7-7] 净布订单已发料');

    const receiptNo = genCode('RC-NET');
    const receiptDate = new Date().toISOString().slice(0, 10);
    const receipt = await apiCall<{ id: number }>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: receiptNo,
        outsourcing_order_id: orderId,
        receipt_date: receiptDate,
        product_id: ctx.productIds[0],
        warehouse_id: finishedWarehouseId,
        return_quantity: 28,
        quality_status: 'passed',
        grade: 'A',
      }
    );
    const receiptId = receipt.data.id;
    await apiCall(page, 'POST', `/production/outsourcing-receipts/${receiptId}/confirm`);
    console.log('[7-7] 净布回仓确认完成');

    // 断言：净布匹为 greige 类型，无缸号，piece_no={receipt_no}-P01
    const netPieces = await fetchPieces(page, {
      piece_type: 'greige',
      batch_no: receiptNo,
    });
    console.log('[7-7] 净布匹查询数量=', netPieces.length, '批次号=', receiptNo);
    expect(netPieces.length).toBeGreaterThanOrEqual(1);
    const netPiece = netPieces[0];
    const expectedPieceNo = `${receiptNo}-P01`;
    console.log(
      '[7-7] 净布匹 piece_no=',
      netPiece.piece_no,
      '期望=',
      expectedPieceNo,
      'dye_lot_no=',
      netPiece.dye_lot_no
    );
    expect(netPiece.piece_no).toBe(expectedPieceNo);
    expect(netPiece.batch_no).toBe(receiptNo);
    expect(netPiece.dye_lot_no).toBeNull();
  });

  test('7-8 四维追溯：按产品 + 缸号查询染色匹', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    // 按产品过滤
    const byProduct = await fetchPieces(page, { product_id: productId, page_size: 50 });
    console.log('[7-8] 按产品查询匹数=', byProduct.length, 'product_id=', productId);
    expect(byProduct.length).toBeGreaterThan(0);
    // 按产品 + 缸号过滤（染色匹）
    const byDyeLot = await fetchPieces(page, {
      product_id: productId,
      dye_lot_no: dyeLotNo,
    });
    console.log('[7-8] 按产品+缸号查询染色匹数=', byDyeLot.length, '缸号=', dyeLotNo);
    expect(byDyeLot.length).toBeGreaterThanOrEqual(1);
    expect(byDyeLot[0].dye_lot_no).toBe(dyeLotNo);
    // 按匹类型过滤（生产匹）
    const greigeOnly = await fetchPieces(page, {
      piece_type: 'greige',
      product_id: productId,
    });
    console.log('[7-8] 按产品+生产匹查询数量=', greigeOnly.length);
    expect(greigeOnly.length).toBeGreaterThanOrEqual(2);
  });
});
