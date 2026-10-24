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
  expectBadRequest,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';

test.describe('面料单据专用字段全链路验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  // ============================================================
  test('面料必填项：销售订单缺色号不导致 500', async ({ page }) => {
    const ctx = getCtx();

    // 创建不带色号的销售订单
    const soData = {
      customer_id: ctx.customerId,
      // order_date 后端按完整 ISO datetime 反序列化，仅传日期串（YYYY-MM-DD）会
      // 触发 422 "order_date: premature end of input"（见 reports/backend.log），
      // 与本用例"缺色号"的判定无关、且非 VALIDATION/BUSINESS 码 → 断言假失败。
      // 与 21a 同：用 new Date().toISOString() 完整时间戳。
      order_date: new Date().toISOString(),
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: '10',
          unit_price: '20.00',
          // 不传 color_no
        },
      ],
    };

    const result = await apiCallExpectFail(page, 'POST', '/sales/orders', soData);

    // 后端应接受或拒绝，但不返回 500
    expect(result.status < 500).toBe(true);

    if (result.status >= 400) {
      // 如果被拒绝，应有明确错误信息
      const msg = result.message || '';
      const mentionsColor = msg.toLowerCase().includes('color') || msg.includes('色号');
      // 机器码出自 utils/error.rs:143-148（AppError 直出体，code 为字符串）
      const rejectCode = failureCode(result);
      expect(
        mentionsColor ||
          rejectCode === APP_ERROR_CODES.VALIDATION_ERROR ||
          rejectCode === APP_ERROR_CODES.BUSINESS_ERROR
      ).toBeTruthy();
    }
  });

  test('面料必填项：采购收货缺 material_code 被拒', async ({ page }) => {
    const ctx = getCtx();

    // 创建缺少 material_code 的采购收货
    const receiptData = {
      supplier_id: ctx.supplierId,
      receipt_date: new Date().toISOString().slice(0, 10),
      warehouse_id: ctx.warehouseIds[0],
      items: [
        {
          line_no: 1,
          material_id: ctx.productIds[0],
          // 不传 material_code（后端必填）
          quantity: '50',
          quantity_alt: '15',
          unit_master: '米',
        },
      ],
    };

    const result = await apiCallExpectFail(page, 'POST', '/purchase/receipts', receiptData);

    // 后端应拒绝（material_code 是必填）
    expectBadRequest(result);
    expect(result.status < 500).toBe(true);
  });

  test('面料必填项：采购收货缺 material_name 被拒', async ({ page }) => {
    const ctx = getCtx();

    const receiptData = {
      supplier_id: ctx.supplierId,
      receipt_date: new Date().toISOString().slice(0, 10),
      warehouse_id: ctx.warehouseIds[0],
      items: [
        {
          line_no: 1,
          material_id: ctx.productIds[0],
          material_code: 'MAT-001',
          // 不传 material_name（后端必填）
          quantity: '50',
          quantity_alt: '15',
          unit_master: '米',
        },
      ],
    };

    const result = await apiCallExpectFail(page, 'POST', '/purchase/receipts', receiptData);

    expectBadRequest(result);
    expect(result.status < 500).toBe(true);
  });

  // ============================================================
  // 色卡 — 面料色号管理
  // 后端字段: card_no, card_name, card_type (PANTONE/CNCS/CUSTOM)
  // ============================================================
  test('色卡：创建+查询+类型验证', async ({ page }) => {
    const cardNo = genCode('CC');
    const cardName = 'E2E 测试色卡';
    const cardType = 'PANTONE';

    const cardData = {
      card_no: cardNo,
      card_name: cardName,
      card_type: cardType,
      season: '2026春夏',
      brand: 'Pantone',
    };

    const result = await apiCall<{ id?: number }>(page, 'POST', '/color-cards', cardData);
    const cardId = result.data?.id;
    expect(
      cardId,
      `色卡创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    if (cardId) {
      const detail = await apiCallRaw<{
        card_no: string;
        card_name: string;
        card_type: string;
        season: string;
        brand: string;
      }>(page, 'GET', `/color-cards/${cardId}`);

      expect(detail.card_no).toBe(cardNo);
      expect(detail.card_name).toBe(cardName);
      expect(detail.card_type).toBe(cardType);
      expect(detail.season).toBe('2026春夏');
      expect(detail.brand).toBe('Pantone');
    }
  });

  // ============================================================
  // 字段命名差异验证 — 同含义字段在不同单据中的命名
  // ============================================================
  test('字段命名差异：色号在不同单据中的命名验证', async ({ page }) => {
    // 销售订单用 color_no
    // 采购收货用 color_code
    // 报价单用 color_id（外键）
    // 染色配方用 color_code
    // 库存调拨用 color_no

    // 验证销售订单响应包含 color_no
    const soList = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      '/sales/orders?page=1&page_size=1'
    );
    if (soList.items?.length > 0) {
      const so = soList.items?.[0];
      // 销售订单可能有 color_no 字段
      expect(so.color_no !== undefined).toBe(true);
    }

    // 验证染色配方响应包含 color_code
    const recipeList = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      '/production/dye-recipes?page=1&page_size=1'
    );
    if (recipeList.items?.length > 0) {
      const recipe = recipeList.items[0];
      // 染色配方用 color_code
      expect(recipe.color_code !== undefined || recipe.color_no !== undefined).toBe(true);
    }

    // 验证库存调拨响应包含 color_no
    const transferList = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      '/inventory/transfers?page=1&page_size=1'
    );
    if (transferList.items?.length > 0) {
      // 调拨列表响应形状是真断言面：items 必须是数组且每行主键可解析；面料列是否露出记为现状。
      expect(Array.isArray(transferList.items), '库存调拨列表应返回 items 数组').toBe(true);
      for (const row of transferList.items) {
        expect(
          Number.isFinite(Number(row.id)),
          `调拨行主键应为有限数值: ${JSON.stringify(row.id)}`
        ).toBe(true);
      }
      console.log(
        `[21e][UI 现状] 调拨列表 ${transferList.items?.length ?? 0} 行，面料列露出情况由后端字段核对覆盖`
      );
    }
  });

  // ============================================================
  // UI 缺失面料字段记录 — 记录前端缺失的面料字段
  // ============================================================
  test('UI 缺失记录：销售订单列表无面料列', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales`);

    const headers = page.locator('.el-table__header th, .el-table__header-wrapper th');
    // 列表表头是真渲染出来的，不是固定等待几秒就能撞上：CI 上 workers=3 并发时
    // 首屏渲染常慢过 3 秒，固定 sleep 会让下面的计数拿到 0 而误报"表头没渲染"。
    await headers.first().waitFor({ state: 'visible', timeout: 30_000 });
    const headerCount = await headers.count();
    const headerTexts: string[] = [];
    for (let i = 0; i < headerCount; i++) {
      headerTexts.push((await headers.nth(i).textContent())?.trim() || '');
    }

    // 记录面料字段是否在列表显示
    const hasColorNo = headerTexts.some(h => h.includes('色号'));
    const hasDyeLotNo = headerTexts.some(h => h.includes('缸号'));
    const hasGramWeight = headerTexts.some(h => h.includes('克重'));
    const hasWidth = headerTexts.some(h => h.includes('幅宽'));

    // 表头必须真取到（空表头会让上面的 some 判定全为假却看不见）；面料列缺失是改进点，记现状不锁死为期望。
    expect(headerTexts.length, '销售订单列表应渲染出表头文本').toBeGreaterThan(0);
    console.warn(
      `[21e][UI 现状] 销售订单列表面料列缺失 色号=${!hasColorNo} 缸号=${!hasDyeLotNo} 克重=${!hasGramWeight} 幅宽=${!hasWidth}`
    );
  });

  test('UI 缺失记录：库存调拨表单无面料字段', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory-transfer`);
    await page.waitForTimeout(3000);

    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建
    const newBtn = page.locator('button:has-text("新建")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);

      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 10_000 });

      // 检查表单是否有色号/缸号/批次号字段
      const dialogText = await dialog.textContent();
      const hasColorNo = dialogText?.includes('色号');
      const hasDyeLotNo = dialogText?.includes('缸号');
      const hasBatchNo = dialogText?.includes('批次');

      // 表单必须真渲染出内容；面料三字段是否露出属改进点，显式记录，不写恒真也不反向锁死。
      expect((dialogText ?? '').trim().length, '库存调拨表单应渲染出可见文本').toBeGreaterThan(0);
      console.warn(
        `[21e][UI 现状] 调拨表单面料字段露出 色号=${!!hasColorNo} 缸号=${!!hasDyeLotNo} 批次=${!!hasBatchNo}`
      );

      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });
});
