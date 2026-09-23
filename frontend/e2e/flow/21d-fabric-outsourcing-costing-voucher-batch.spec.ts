import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  BASE_URL,
  ensureTestEntities,
} from './helpers';

test.describe('面料单据专用字段全链路验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  // ============================================================
  test('委外加工订单：面料追溯字段验证', async ({ page }) => {
    const ctx = getCtx();
    const colorNo = ctx.colorNos[0] || 'CN-001';
    const dyeLotNo = ctx.dyeLotNo || genCode('DL');
    const batchNo = genCode('BN');

    const orderData = {
      order_no: genCode('OS'),
      order_type: 'dyeing',
      supplier_id: ctx.supplierId,
      dye_batch_id: ctx.dyeBatchId,
      color_no: colorNo,
      dye_lot_no: dyeLotNo,
      issue_date: new Date().toISOString().slice(0, 10),
      issue_quantity: '100',
      issue_unit: '米',
      material_cost: '500',
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/outsourcing-orders',
      orderData
    );
    const orderId = result.data?.id;
    expect(
      orderId,
      `委外加工订单创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    if (orderId) {
      // 添加发料明细（含面料追溯字段）
      await apiCall(page, 'POST', '/production/outsourcing-orders/items', {
        outsourcing_order_id: orderId,
        product_id: ctx.productIds[0],
        color_no: colorNo,
        dye_lot_no: dyeLotNo,
        batch_no: batchNo,
        quantity: '100',
        unit: '米',
        unit_cost: '5.00',
      });

      // 查询订单详情
      const detail = await apiCallRaw<{
        status: string;
        color_no: string;
        dye_lot_no: string;
      }>(page, 'GET', `/production/outsourcing-orders/${orderId}`);

      expect(detail.color_no).toBe(colorNo);
      expect(detail.dye_lot_no).toBe(dyeLotNo);
    }
  });

  // ============================================================
  // 成本归集 — 缸号追溯+双单位产量
  // 后端字段: batch_no, color_no, dye_lot_no,
  //   output_quantity_meters, output_quantity_kg
  // ============================================================
  test('成本归集：缸号追溯+双单位产量验证', async ({ page }) => {
    const ctx = getCtx();
    const batchNo = genCode('BN');
    const colorNo = ctx.colorNos[0] || 'CN-001';
    const dyeLotNo = ctx.dyeLotNo || genCode('DL');
    const outputMeters = '1000';
    const outputKg = '200';

    const costData = {
      collection_date: new Date().toISOString().slice(0, 10),
      cost_object_type: 'dye_batch',
      cost_object_id: ctx.dyeBatchId,
      batch_no: batchNo,
      color_no: colorNo,
      dye_lot_no: dyeLotNo,
      workshop: '染色一车间',
      direct_material: '1500.50',
      direct_labor: '800.00',
      manufacturing_overhead: '300.00',
      processing_fee: '100.00',
      dyeing_fee: '200.00',
      output_quantity_meters: outputMeters,
      output_quantity_kg: outputKg,
    };

    // 创建失败直接暴露：apiCall 非 2xx 抛错即向上冒泡（原 try/catch 仅 throw e 属空转，已删除）
    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/cost-collections',
      costData
    );
    const costId = result.data?.id;
    expect(
      costId,
      `成本归集创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    if (costId) {
      const detail = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/cost-collections/${costId}`
      );

      expect(detail.batch_no).toBe(batchNo);
      expect(detail.color_no).toBe(colorNo);
      expect(detail.dye_lot_no).toBe(dyeLotNo);
      expect(Number(detail.output_quantity_meters)).toBe(Number(outputMeters));
      expect(Number(detail.output_quantity_kg)).toBe(Number(outputKg));

      // 验证单位成本计算
      const totalCost =
        parseFloat(String(detail.direct_material || '0')) +
        parseFloat(String(detail.direct_labor || '0')) +
        parseFloat(String(detail.manufacturing_overhead || '0')) +
        parseFloat(String(detail.processing_fee || '0')) +
        parseFloat(String(detail.dyeing_fee || '0'));
      // 单位成本 = 总成本 / 产量
      const unitCostPerMeter = totalCost / parseFloat(outputMeters);
      const unitCostPerKg = totalCost / parseFloat(outputKg);
      expect(unitCostPerMeter).toBeGreaterThan(0);
      expect(unitCostPerKg).toBeGreaterThan(0);
    }
  });

  // ============================================================
  // 凭证 — 面料辅助核算项
  // 后端字段: batch_no, color_no (主表)
  // 明细: assist_batch_id, assist_color_no_id, assist_dye_lot_id,
  //   assist_grade, quantity_meters, quantity_kg, unit_price
  // ============================================================
  test('凭证：面料辅助核算项验证', async ({ page }) => {
    const batchNo = genCode('BN');
    const colorNo = 'CN-001';

    const voucherData = {
      voucher_type: 'general',
      voucher_date: new Date().toISOString().slice(0, 10),
      batch_no: batchNo,
      color_no: colorNo,
      items: [
        {
          // 原材料入库借方：预置科目里"原材料"是 1403（migration finance/mod.rs:728），
          // 不存在 1401；用不存在的编码会在凭证 create 的 assist 明细 lookup_subject_id 处
          // 抛"科目不存在"（voucher_ops/assist.rs:177-179）。
          subject_code: '1403',
          debit: '100',
          credit: '0',
          summary: '入库-涤棉坯布',
          assist_batch_id: 1,
          assist_color_no_id: 1,
          assist_grade: 'A',
          quantity_meters: '100',
          quantity_kg: '30',
          unit_price: '25.50',
        },
        {
          subject_code: '1001',
          debit: '0',
          credit: '100',
          summary: '银行存款',
        },
      ],
    };

    // 凭证创建端点是 POST /vouchers（routes/finance.rs:224 create_voucher），
    // 与下方 GET /vouchers/{id} 同前缀；不存在 /finance/vouchers 路由（原写法 404）。
    const result = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', voucherData);
    const voucherId = result.data?.id;
    expect(
      voucherId,
      `凭证创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    if (voucherId) {
      const detail = await apiCallRaw<{
        entries: Array<Record<string, unknown>>;
        batch_no: string;
        color_no: string;
      }>(page, 'GET', `/vouchers/${voucherId}`);

      // 主表面料字段验证
      expect(detail.batch_no).toBe(batchNo);
      expect(detail.color_no).toBe(colorNo);

      // 明细面料辅助核算验证
      const firstEntry = detail.entries?.[0];
      if (firstEntry) {
        expect(firstEntry.assist_grade).toBe('A');
        expect(String(firstEntry.quantity_meters)).toBe('100');
        expect(String(firstEntry.quantity_kg)).toBe('30');
        expect(String(firstEntry.unit_price)).toBe('25.50');
      }

      // 验证借贷平衡
      const totalDebit =
        detail.entries?.reduce(
          (sum: number, e: Record<string, unknown>) => sum + parseFloat(String(e.debit || '0')),
          0
        ) || 0;
      const totalCredit =
        detail.entries?.reduce(
          (sum: number, e: Record<string, unknown>) => sum + parseFloat(String(e.credit || '0')),
          0
        ) || 0;
      expect(Math.abs(totalDebit - totalCredit)).toBeLessThan(0.01);
    }
  });

  // ============================================================
  // 染色批次（缸号）— 面料追溯核心
  // 后端字段: batch_no, greige_fabric_id, color_no, dye_lot_no
  // ============================================================
  test('染色批次：缸号追溯字段验证', async ({ page }) => {
    const ctx = getCtx();
    const batchNo = genCode('DB');
    const colorNo = ctx.colorNos[0] || 'CN-001';
    const dyeLotNo = ctx.dyeLotNo || genCode('DL');

    const batchData = {
      batch_no: batchNo,
      greige_fabric_id: ctx.greigeFabricId,
      color_no: colorNo,
      dye_lot_no: dyeLotNo,
      planned_quantity: 100,
      status: 'pending_schedule',
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/dye-batches',
      batchData
    );
    const batchId = result.data?.id;
    expect(
      batchId,
      `染色批次创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    if (batchId) {
      const detail = await apiCallRaw<{
        batch_no: string;
        color_no: string;
        dye_lot_no: string;
        status: string;
      }>(page, 'GET', `/production/dye-batches/${batchId}`);

      expect(detail.batch_no).toBe(batchNo);
      expect(detail.color_no).toBe(colorNo);
      expect(detail.dye_lot_no).toBe(dyeLotNo);
    }
  });

  // ============================================================
  // 面料必填项验证 — 色号为面料行业必填
  // ============================================================
});
