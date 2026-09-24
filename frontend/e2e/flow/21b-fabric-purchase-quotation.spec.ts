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
  test('采购收货：面料追溯字段（material_code/color_code/lot_no）验证', async ({ page }) => {
    const ctx = getCtx();
    const materialCode = `MAT-${Date.now().toString().slice(-6)}`;
    const materialName = '测试面料-涤棉混纺';
    const colorCode = ctx.colorNos[0] || 'CC-E2E-001';
    const lotNo = ctx.dyeLotNo || genCode('L');
    const batchNo = genCode('BN');
    const gramWeight = '180';
    const width = '145';
    const receiptQty = '50';

    // 收货量必须 ≤ 采购订单该产品未收数量（后端无超收容差，见 backend.log
    // "入库量 50 超过采购订单未收数量，拒绝建单"）。ctx.purchaseOrderId 常复用既有单、
    // 未收量不可控——建一张本用例专属 PO（订购量=收货量），提交+审批到可收货状态再整单收货，
    // 使收货建单稳定成功（与 12 白坯出库同一取数口径）。
    const po = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      items: [
        { material_id: ctx.productIds[0], quantity_ordered: receiptQty, unit_price: '20.00' },
      ],
    });
    const poId = po.data?.id;
    expect(poId, `前置采购单创建失败：${JSON.stringify(po).slice(0, 200)}`).toBeTruthy();
    await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`);
    await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`);

    const receiptData = {
      order_id: poId,
      supplier_id: ctx.supplierId,
      receipt_date: new Date().toISOString().slice(0, 10),
      warehouse_id: ctx.warehouseIds[0],
      items: [
        {
          line_no: 1,
          material_id: ctx.productIds[0],
          material_code: materialCode,
          material_name: materialName,
          batch_no: batchNo,
          color_code: colorCode,
          lot_no: lotNo,
          grade: 'A',
          gram_weight: gramWeight,
          width: width,
          quantity: receiptQty,
          quantity_alt: '15',
          unit_master: '米',
          unit_alt: '公斤',
          unit_price: '20.00',
        },
      ],
    };

    const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', receiptData);
    const receiptId = result.data?.id;
    expect(
      receiptId,
      `采购收货创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    if (receiptId) {
      // GET /purchase/receipts/{id} 返回裸 purchase_receipt::Model（handler 直接
      // ApiResponse::success(model)，purchase_receipt_handler.rs:84-117 → query.rs:54-61），
      // 出参没有 items——明细必须另取已注册端点
      // GET /purchase/receipts/{id}/items（routes/purchase.rs:115-119 → query.rs:64-75，
      // 返回裸数组 Vec<purchase_receipt_item::Model>）
      const detail = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/purchase/receipts/${receiptId}`
      );
      expect(detail?.id, '入库单详情应返回 id').toBeTruthy();
      expect(
        Array.isArray((detail as { items?: unknown }).items),
        '详情端点不应返回 items 数组（明细在独立端点）'
      ).toBe(false);

      const items = await apiCallRaw<Array<Record<string, unknown>>>(
        page,
        'GET',
        `/purchase/receipts/${receiptId}/items`
      );
      expect(Array.isArray(items), '明细端点 data 应为数组').toBe(true);
      expect(items.length, '入库明细至少一条').toBeGreaterThan(0);
      const item = items[0];

      // 面料追溯字段精确验证
      expect(item.material_code).toBe(materialCode);
      expect(item.material_name).toBe(materialName);
      expect(item.color_code).toBe(colorCode);
      expect(item.lot_no).toBe(lotNo);
      expect(item.batch_no).toBe(batchNo);
      expect(item.grade).toBe('A');
      // gram_weight / width 后端为 Decimal（purchase_receipt_dto.rs:86,89 /
      // models/purchase_receipt_item.rs:24,26），serde 序列化为两位小数字符串
      // （"180.00"/"145.00"）。断言未对齐真实出参格式：比对前做数值归一（非放宽）。
      expect(Number(item.gram_weight)).toBe(Number(gramWeight));
      expect(Number(item.width)).toBe(Number(width));
    }
  });

  // ============================================================
  // 报价单 — 用 color_id（外键）而非 color_no（字符串）
  // 后端字段: color_id, specification, unit, quantity,
  //   unit_price, unit_price_with_tax
  // ============================================================
  test('报价单：色号字段（color_id）+规格+含税价验证', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];

    // 查询产品色号 ID
    let colorId: number | null = null;
    // 真实端点：GET /products/{id}/colors（返回数组，非分页包装）
    const colors = await apiCallRaw<Array<{ id: number; color_no: string }>>(
      page,
      'GET',
      `/products/${productId}/colors`
    );
    if (colors?.length > 0) {
      colorId = colors[0].id;
    }

    const unitPrice = '25.50';
    const taxRate = '13';
    const expectedWithTax = (parseFloat(unitPrice) * (1 + parseFloat(taxRate) / 100)).toFixed(2);

    const quotationData = {
      customer_id: ctx.customerId,
      sales_user_id: 1,
      quotation_date: new Date().toISOString().slice(0, 10),
      valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
      currency: 'CNY',
      exchange_rate: '1',
      base_currency: 'CNY',
      price_terms: 'FOB',
      tax_inclusive: false,
      tax_rate: taxRate,
      items: [
        {
          product_id: productId,
          color_id: colorId,
          specification: 'T/C 65/35 45x45 110x76',
          unit: '米',
          quantity: '100',
          unit_price: unitPrice,
          unit_price_with_tax: expectedWithTax,
        },
      ],
    };

    // 创建失败直接暴露（兜底旧报价单无自建面料字段，精确断言会失真）
    const qResult = await apiCall<{ id?: number }>(page, 'POST', '/quotations', quotationData);
    const quotationId = qResult.data?.id;
    expect(quotationId).toBeDefined();

    if (quotationId) {
      const detail = await apiCallRaw<{
        status: string;
        items: Array<Record<string, unknown>>;
      }>(page, 'GET', `/quotations/${quotationId}`);

      expect(detail.items?.length).toBeGreaterThan(0);
      const item = detail.items?.[0];

      expect(Number(item.product_id)).toBe(productId);
      expect(item.specification).toBe('T/C 65/35 45x45 110x76');
      expect(item.unit).toBe('米');
      expect(Number(item.quantity)).toBe(100);
      if (colorId) {
        expect(Number(item.color_id)).toBe(colorId);
      }
    }
  });

  test('报价单 UI：明细编辑器有色号选择列', async ({ page }) => {
    await page.goto(`${BASE_URL}/quotations`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建报价单
    const newBtn = page
      .locator(
        'button:has-text("新建"), button:has-text("创建"), .el-button--primary:has-text("新")'
      )
      .first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    await newBtn.click();

    // list.vue:20 的"新建"是 $router.push('/quotations/new')，跳转到整页新建表单
    // （create.vue → QuotationItemEditor），并非弹窗。原实现等待 .el-dialog 前提错误（CI 超时）。
    // 改为等待新建路由生效并渲染报价明细编辑表。
    await page.waitForURL(/\/quotations\/new/, { timeout: 10_000 });
    const editorTable = page.locator('[aria-label="报价明细编辑表"]').first();
    await editorTable.waitFor({ state: 'visible', timeout: 10_000 });

    // 明细编辑器必须渲染"色号"选择列（QuotationItemEditor.vue:43 colColor='色号'）。
    // 表头在明细为空时也渲染，故断言列头存在即为"有色号选择列"的真实证据
    // （替换原 expect(true).toBe(true) 恒真占位）。
    const colorColHeader = editorTable.locator('th .cell').filter({ hasText: '色号' }).first();
    await colorColHeader.waitFor({ state: 'visible', timeout: 5000 });
    expect(await colorColHeader.isVisible(), '报价明细编辑器应含"色号"选择列').toBe(true);
  });

  // ============================================================
  // 库存调拨 — 缸号追溯三件套（color_no/dye_lot_no/batch_no）
  // 注意: 调拨用 color_no（非 color_code），dye_lot_no（非 lot_no）
  // ============================================================
});
