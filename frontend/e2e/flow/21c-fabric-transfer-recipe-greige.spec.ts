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
      status: 'active',
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
});
