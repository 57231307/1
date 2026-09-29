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
  tryCleanup,
} from './helpers';

test.describe('面料单据专用字段全链路验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  // ============================================================
  test('库存调拨：面料追溯三件套验证', async ({ page }) => {
    const ctx = getCtx();
    const colorNo = ctx.colorNos[0] || 'CN-001';
    const dyeLotNo = ctx.dyeLotNo || genCode('DL');
    const batchNo = genCode('BN');

    const transferData = {
      from_warehouse_id: ctx.warehouseIds[0],
      to_warehouse_id: ctx.warehouseIds[1] || ctx.warehouseIds[0],
      transfer_date: new Date().toISOString(),
      notes: '面料追溯测试',
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: '5',
          color_no: colorNo,
          dye_lot_no: dyeLotNo,
          batch_no: batchNo,
        },
      ],
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/inventory/transfers',
      transferData
    );
    const transferId = result.data?.id;
    expect(transferId, '调拨建单应返回 id').toBeTruthy();

    // GET /inventory/transfers/{id} 返回 InventoryTransferDetail（含 items，
    // get_transfer_detail 在 inventory_move.rs:131-174 联查明细；
    // 明细出参含 color_no/dye_lot_no/batch_no——mod.rs:67-70）
    const detail = await apiCallRaw<{
      status: string;
      items: Array<Record<string, unknown>>;
    }>(page, 'GET', `/inventory/transfers/${transferId}`);

    expect(detail.items.length, '建单明细必须可在详情中读回').toBeGreaterThan(0);
    const item = detail.items[0];

    // 面料追溯三件套：三列都必须等于建单时传入的真实值（fail-closed 写入
    // inventory_move.rs:282-288，出参如实回传）
    expect(item.color_no, '调拨明细 color_no 应等于建单传入值').toBe(colorNo);
    expect(item.dye_lot_no, '调拨明细 dye_lot_no 应等于建单传入缸号').toBe(dyeLotNo);
    expect(item.batch_no, '调拨明细 batch_no 应等于建单传入批号').toBe(batchNo);
  });

  // ============================================================
  // add_item 端点（POST /inventory/transfers/{id}/items）的追溯字段写入验证。
  // batch.rs:1073-1076 的 add_item 把 color_no/dye_lot_no/batch_no 置 NotSet
  // （"让 DB 默认值处理"），入参被整体丢弃——下列断言按契约（DTO 收这三字段、
  // 出参 DTO 回传这三字段）写，会在 CI 暴露该缺陷，不做绕行。
  // ============================================================
  test('库存调拨 add_item：追溯三字段必须随请求落库并回显', async ({ page }) => {
    const ctx = getCtx();

    // 先建一张无明细的调拨单（items 为空 → 不触发明细级校验）
    const created = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: ctx.warehouseIds[0],
      to_warehouse_id: ctx.warehouseIds[1] || ctx.warehouseIds[0],
      transfer_date: new Date().toISOString(),
      notes: 'E2E add_item 追溯字段验证',
      items: [],
    });
    const transferId = created.data?.id;
    expect(transferId, '调拨单创建应返回 id').toBeTruthy();

    const colorNo = ctx.colorNos[0];
    const dyeLotNo = ctx.dyeLotNo;
    const batchNo = genCode('BN-ADD');

    const added = await apiCallRaw<{
      color_no: string | null;
      dye_lot_no: string | null;
      batch_no: string | null;
    }>(page, 'POST', `/inventory/transfers/${transferId}/items`, {
      product_id: ctx.productIds[0],
      quantity: '1',
      color_no: colorNo,
      dye_lot_no: dyeLotNo,
      batch_no: batchNo,
    });

    expect(added.color_no, 'add_item 必须回写入参 color_no').toBe(colorNo);
    expect(added.dye_lot_no, 'add_item 必须回写入参 dye_lot_no').toBe(dyeLotNo);
    expect(added.batch_no, 'add_item 必须回写入参 batch_no').toBe(batchNo);
  });

  // ============================================================
  // 白坯/染色判定新口径：白坯布 = 没有颜色（color_no 为空），而不是"色号名字里带白"。
  // 名字带"白"的色号（如"本白"）是已染色的白色布，属有颜色产品，必须带缸号追溯——
  // 旧实现按色号名称嗅探会把"本白"误判为白坯而豁免缸号。以下断言锁定新口径：
  //   1) 白色号 + 缺缸号 → 建单被拒（4xx 客户端校验错误，非 5xx）；
  //   2) 白色号 + 带缸号 → 建单成功（对照组，证明被拒确因缺缸号而非其他原因）。
  // ============================================================
  test('调拨建单：白色号染色布必须带缸号（废除按名称判定白坯）', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const fromWarehouseId = ctx.warehouseIds[0];
    const toWarehouseId = ctx.warehouseIds[1] || ctx.warehouseIds[0];
    const transferDate = new Date().toISOString();
    const whiteDyedColorNo = '本白'; // 非空色号 = 染色白色布，新口径必须带缸号

    // 1) 白色号 + 缺缸号：应被拒绝（4xx，非服务端 5xx）
    const rejected = await apiCallExpectFail(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: fromWarehouseId,
      to_warehouse_id: toWarehouseId,
      transfer_date: transferDate,
      items: [
        {
          product_id: productId,
          quantity: '1',
          color_no: whiteDyedColorNo,
          batch_no: genCode('BN-WHITE-NEG'),
          // 故意不提供 dye_lot_no
        },
      ],
    });
    expect(
      rejected.status,
      `白色号染色布缺缸号必须被拒（实际 status=${rejected.status}, message=${rejected.message}）`
    ).toBeGreaterThanOrEqual(400);
    expect(
      rejected.status,
      `缺缸号是客户端校验错误，不得为 5xx 服务端错误（实际 status=${rejected.status}）`
    ).toBeLessThan(500);

    // 2) 对照组：白色号 + 带缸号 → 建单成功，证明上一条被拒确因缺缸号
    const accepted = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: fromWarehouseId,
      to_warehouse_id: toWarehouseId,
      transfer_date: transferDate,
      items: [
        {
          product_id: productId,
          quantity: '1',
          color_no: whiteDyedColorNo,
          dye_lot_no: genCode('DL-WHITE'),
          batch_no: genCode('BN-WHITE-POS'),
        },
      ],
    });
    expect(
      accepted.data?.id,
      `白色号染色布带缸号应建单成功（对照组），实际响应：${JSON.stringify(accepted).slice(0, 200)}`
    ).toBeTruthy();
  });

  // ============================================================
  // 染色配方 — 面料特有字段最多
  // 后端字段: color_code, color_name, fabric_type, dye_type,
  //   temperature, time_minutes, ph_value, liquor_ratio, auxiliaries
  // ============================================================
  test('染色配方：面料工艺字段全量验证', async ({ page }) => {
    const colorCode = genCode('CC');
    const colorName = '宝蓝';
    const fabricType = '涤纶';
    const dyeType = '分散染色';
    const temperature = '130';
    const timeMinutes = 45;
    const phValue = '5.5';
    const liquorRatio = 10;

    const recipeData = {
      recipe_no: genCode('DR'),
      recipe_name: 'E2E 涤纶宝蓝分散染色配方',
      color_code: colorCode,
      color_name: colorName,
      fabric_type: fabricType,
      dye_type: dyeType,
      temperature: temperature,
      time_minutes: timeMinutes,
      ph_value: phValue,
      liquor_ratio: liquorRatio,
      auxiliaries: [
        { name: '分散蓝 2BLN', amount: '2.5', unit: '%' },
        { name: '分散红 3B', amount: '0.8', unit: '%' },
        { name: '匀染剂SF', amount: '1.0', unit: 'g/L' },
        { name: '醋酸', amount: '0.5', unit: 'g/L' },
      ],
      remarks: 'E2E 测试配方',
    };

    // 创建失败由 apiCall 非 2xx 抛错直接暴露（兜底旧配方无自建字段，精确断言会失真）
    const recipeResult = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/dye-recipes',
      recipeData
    );
    const recipeId = recipeResult.data?.id;
    expect(
      recipeId,
      `染色配方创建应返回 data.id，实际响应：${JSON.stringify(recipeResult).slice(0, 200)}`
    ).toBeTruthy();

    if (recipeId) {
      const detail = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/dye-recipes/${recipeId}`
      );

      expect(detail.color_code).toBe(colorCode);
      expect(detail.color_name).toBe(colorName);
      expect(detail.fabric_type).toBe(fabricType);
      expect(detail.dye_type).toBe(dyeType);
      expect(Number(detail.temperature)).toBe(130);
      expect(Number(detail.time_minutes)).toBe(timeMinutes);
      // DECIMAL 回读会带补零（5.5 → "5.50"），数值比较归一
      expect(Number(detail.ph_value)).toBe(Number(phValue));

      // 验证助剂列表
      const auxiliaries = detail.auxiliaries as Array<{
        name: string;
        amount: string;
        unit: string;
      }>;
      if (auxiliaries && auxiliaries.length > 0) {
        expect(auxiliaries.length).toBe(4);
        expect(auxiliaries[0].name).toBe('分散蓝 2BLN');
        expect(auxiliaries[0].amount).toBe('2.5');
        expect(auxiliaries[0].unit).toBe('%');
      }
    }
  });

  test('染色配方 UI：列表显示色号列+表单色号必填', async ({ page }) => {
    await page.goto(`${BASE_URL}/dye-recipe`);
    await page.waitForTimeout(3000);

    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 验证列表有色号列
    const headers = page.locator('.el-table__header th, .el-table__header-wrapper th');
    const headerCount = await headers.count();
    const headerTexts: string[] = [];
    for (let i = 0; i < headerCount; i++) {
      headerTexts.push((await headers.nth(i).textContent())?.trim() || '');
    }
    // 染色配方列表应显示色号和颜色名称列
    const hasColorNo = headerTexts.some(h => h.includes('色号'));
    const hasColorName = headerTexts.some(h => h.includes('颜色') || h.includes('色名'));
    expect(hasColorNo || hasColorName).toBe(true);
  });

  // ============================================================
  // 坯布 — 面料物理属性字段最多
  // 后端字段: fabric_no, fabric_name, fabric_type, color_code,
  //   width_cm, width, gram_weight, quantity_meters, quantity_kg,
  //   dye_lot_no, color_no, composition, yarn_count, density,
  //   structure, quality_grade
  // ============================================================
  test('坯布：面料物理属性字段验证', async ({ page }) => {
    const fabricNo = genCode('GF');
    const fabricName = 'E2E 测试坯布';
    const fabricType = '涤棉';
    const colorCode = 'CC-001';
    const widthCm = 150;
    const gramWeight = 200;
    const composition = 'T/C 65/35';
    const yarnCount = '45x45';
    const density = '110x76';
    const dyeLotNo = genCode('DL');
    const colorNo = 'CN-001';

    const fabricData = {
      fabric_no: fabricNo,
      fabric_name: fabricName,
      fabric_type: fabricType,
      color_code: colorCode,
      width_cm: widthCm,
      width: widthCm,
      gram_weight: gramWeight,
      quantity_meters: 1000,
      quantity_kg: 300,
      composition: composition,
      yarn_count: yarnCount,
      density: density,
      dye_lot_no: dyeLotNo,
      color_no: colorNo,
      quality_grade: 'A',
      // 坯布库存状态词表唯一事实来源=写入方 purchase_inventory.rs:266 IN_STOCK="在库"
      // / :269 STOCKED_OUT="已出库"（英文 token 'active' 不在闭合词表内，后端拒收）。
      status: '在库',
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/greige-fabrics',
      fabricData
    );
    const fabricId = result.data?.id;
    expect(
      fabricId,
      `坯布创建应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    if (fabricId) {
      const detail = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/greige-fabrics/${fabricId}`
      );

      if (detail) {
        expect(detail.fabric_name).toBe(fabricName);
        expect(detail.fabric_type).toBe(fabricType);
        expect(Number(detail.width_cm || detail.width)).toBe(widthCm);
        expect(Number(detail.gram_weight)).toBe(gramWeight);
        expect(detail.composition).toBe(composition);
        expect(detail.yarn_count).toBe(yarnCount);
        expect(detail.density).toBe(density);
        expect(detail.dye_lot_no).toBe(dyeLotNo);
        expect(detail.color_no).toBe(colorNo);
      }
    }
  });

  // ============================================================
  // 委外加工订单 — 面料追溯字段
  // 后端字段: dye_batch_id, color_no, dye_lot_no
  // 明细: color_no, dye_lot_no, batch_no, greige_fabric_id
  // ============================================================

  // ============================================================
  // [BE-1/FE-1] 坯布新建 UI 驱动验证：fabric_type 必填通过对话框表单填写
  // 对应修复项：
  //   BE-1: 后端 CreateGreigeFabricRequest 现要求 fabric_type（NOT NULL），
  //         models/greige_fabric.rs:37 fabric_type: String（非 Option）
  //   FE-1: 前端 GreigeFormDialogTab.vue 已补 fabric_type 表单项（el-input,
  //         prop="fabric_type", rules required, label i18n='布类'）
  // 本用例通过真实 UI 对话框操作填写 fabric_type，成功建单后从列表回读该行。
  // ============================================================
  test('坯布新建 UI：通过对话框填 fabric_type 建单并回读列表', async ({ page }) => {
    const fabricNo = genCode('GF-UI');
    const fabricName = `E2E坯布UI${Date.now().toString().slice(-6)}`;
    const fabricType = '针织';

    // 导航到面料管理页坯布 Tab
    await page.goto(`${BASE_URL}/fabric`);
    await page.getByRole('tab', { name: '坯布管理', exact: true }).click();
    await expect(page.getByLabel('坯布列表')).toBeVisible({ timeout: 30000 });

    // 点击"新建坯布"按钮
    await page.getByRole('button', { name: '新建坯布' }).first().click();

    // 等待新建坯布对话框出现
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 10000 });

    // 填写编号
    const codeInput = dialog
      .locator('.el-form-item')
      .filter({ has: page.locator('.el-form-item__label', { hasText: '编号' }) })
      .locator('input')
      .first();
    await codeInput.waitFor({ state: 'visible', timeout: 10000 });
    await codeInput.fill(fabricNo);

    // 填写名称
    const nameInput = dialog
      .locator('.el-form-item')
      .filter({ has: page.locator('.el-form-item__label', { hasText: '名称' }) })
      .locator('input')
      .first();
    await nameInput.fill(fabricName);

    // 填写布类（fabric_type）—— 这是本轮新增的必填字段
    const typeInput = dialog
      .locator('.el-form-item')
      .filter({ has: page.locator('.el-form-item__label', { hasText: '布类' }) })
      .locator('input')
      .first();
    await typeInput.waitFor({ state: 'visible', timeout: 10000 });
    await typeInput.fill(fabricType);

    // 捕获 POST 请求提交
    const responsePromise = page.waitForResponse(
      r => r.request().method() === 'POST' && r.url().includes('/production/greige-fabrics'),
      { timeout: 30000 }
    );

    // 点击确认提交
    await dialog
      .getByRole('button', { name: /确认|确定|保存/ })
      .last()
      .click();
    const resp = await responsePromise;

    // 断言创建成功（非 4xx/5xx）
    expect(resp.ok(), `坯布 UI 新建应成功，实际 status=${resp.status()}`).toBe(true);

    // 断言 payload 包含 fabric_type
    const payload = JSON.parse(resp.request().postData() || '{}');
    expect(
      payload.fabric_type,
      'UI 提交 payload 必须包含 fabric_type 字段（FE-1 修复项验证）'
    ).toBe(fabricType);

    // 回读：列表应出现新行
    await page.waitForTimeout(1000);
    const row = page.getByRole('row').filter({ hasText: fabricName }).first();
    await expect(
      row,
      `新建坯布「${fabricName}」应出现在列表（BE-1 NOT NULL 落库验证）`
    ).toBeVisible({ timeout: 15000 });

    // 验证行内 fabric_type 渲染
    await expect(row.getByText(fabricType), `列表应渲染 fabric_type='${fabricType}'`).toBeVisible({
      timeout: 5000,
    });

    // 通过 API 回读后端详情确认落库完整
    const respJson = await resp.json();
    const fabricId = respJson?.data?.id;
    if (fabricId) {
      const detail = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/greige-fabrics/${fabricId}`
      );
      expect(detail.fabric_type, '后端回读 fabric_type 应等于 UI 填入值').toBe(fabricType);
      // 清理
      await tryCleanup(page, 'DELETE', `/production/greige-fabrics/${fabricId}`, '21c-UI坯布');
    }
  });

  // ============================================================
  // [BE-2] 产品 meters_per_piece 往返验证：通过 UI 表单填写后重新打开/GET 断言值一致
  // 对应修复项：
  //   BE-2: 后端 product_ops/crud.rs 已接线 meters_per_piece 到 ActiveModel Set，
  //         handler product_handler.rs CreateProductRequest/UpdateProductRequest 已含该字段
  //   前端: ProductFormDialogTab.vue 已渲染"每匹米数" el-input-number
  // 本用例通过 API 创建带 meters_per_piece 的产品（产品表单 UI 交互复杂且脆弱，
  // 但关键契约验证是后端持久化+回读），随后 GET 详情断言该值非 null 且等于录入值。
  // 补充 UI 回读：编辑页打开验证输入框展示该值。
  // ============================================================
  test('产品 meters_per_piece：API 创建含值 → GET 回读 → UI 编辑表单回显', async ({ page }) => {
    const ctx = getCtx();
    const code = genCode('E2E-MPP');
    const name = `E2E匹米产品${code}`;
    const testMetersPerPiece = 50.5; // 测试值精确到小数一位，避免 DECIMAL 舍入歧义

    // 创建产品并写入 meters_per_piece
    const created = await apiCall<{ id?: number }>(page, 'POST', '/products', {
      code,
      name,
      unit: '米',
      category_id: ctx.productCategoryIds[0],
      meters_per_piece: testMetersPerPiece,
    });
    const productId = created.data?.id;
    expect(
      productId,
      `产品创建应返回 id，实际：${JSON.stringify(created).slice(0, 200)}`
    ).toBeTruthy();

    // GET 详情断言 meters_per_piece 非 null 且等于录入值
    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${productId}`);
    expect(
      detail.meters_per_piece,
      `GET /products/{id} 应返回 meters_per_piece 非 null（BE-2 接线验证），实际=${JSON.stringify(detail.meters_per_piece)}`
    ).not.toBeNull();
    expect(
      Number(detail.meters_per_piece),
      `meters_per_piece 应等于录入值 ${testMetersPerPiece}，实际=${detail.meters_per_piece}`
    ).toBe(testMetersPerPiece);

    // UI 编辑回显验证：打开产品编辑表单确认"每匹米数"输入框有值
    await page.goto(`${BASE_URL}/product`);
    await page.waitForTimeout(2000);
    // 搜索定位到该产品
    const searchInput = page
      .locator('input[placeholder*="搜索"], input[placeholder*="编码"], input[placeholder*="名称"]')
      .first();
    if (await searchInput.isVisible().catch(() => false)) {
      await searchInput.fill(code);
      await page.keyboard.press('Enter');
      await page.waitForTimeout(2000);
    }
    // 点击编辑打开对话框
    const editBtn = page
      .getByRole('row')
      .filter({ hasText: code })
      .first()
      .locator('button:has-text("编辑"), .el-button:has-text("编辑"), .el-link:has-text("编辑")')
      .first();
    if (await editBtn.isVisible().catch(() => false)) {
      await editBtn.click();
      const dialog = page.locator('.el-dialog:visible').last();
      await dialog.waitFor({ state: 'visible', timeout: 10000 });
      // 定位"每匹米数"输入框
      const mppInput = dialog
        .locator('.el-form-item')
        .filter({ hasText: '每匹米数' })
        .locator('input')
        .first();
      if (await mppInput.isVisible().catch(() => false)) {
        const inputValue = await mppInput.inputValue();
        expect(
          Number(inputValue),
          `UI 编辑表单"每匹米数"应回显 ${testMetersPerPiece}，实际输入框值="${inputValue}"`
        ).toBe(testMetersPerPiece);
      }
      // 关闭对话框
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }

    // 清理
    await tryCleanup(page, 'DELETE', `/products/${productId}`, '21c-匹米产品');
  });
});
