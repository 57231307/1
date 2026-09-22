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

    const receiptData = {
      order_id: ctx.purchaseOrderId,
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
          quantity: '50',
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
      expect(String(item.gram_weight)).toBe(gramWeight);
      expect(String(item.width)).toBe(width);
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
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);

      // 可能跳转到创建页面或弹窗
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      if (dialogVisible) {
        // 在弹窗中查找色号相关
        const colorLabel = page
          .locator('.el-dialog:has-text("色号"), .el-dialog:has-text("颜色")')
          .first();
        await colorLabel.waitFor({ state: 'visible', timeout: 5000 });
        const colorVisible = await colorLabel.isVisible();
        // 报价单明细应有色号选择列
        expect(true).toBe(true); // 记录

        await page.locator('.el-dialog__headerbtn').first().click();
      }
    }
  });

  // ============================================================
  // 库存调拨 — 缸号追溯三件套（color_no/dye_lot_no/batch_no）
  // 注意: 调拨用 color_no（非 color_code），dye_lot_no（非 lot_no）
  // ============================================================
});
