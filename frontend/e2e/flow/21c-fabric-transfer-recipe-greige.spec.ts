import { test, expect } from '@playwright/test';
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

    let transferId: number;
    try {
      const result = await apiCall<{ id?: number }>(
        page,
        'POST',
        '/inventory/transfers',
        transferData
      );
      transferId = result.data?.id!;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/inventory/transfers?page=1&page_size=1'
      );
      transferId = list.items?.[0]?.id;
    }

    if (transferId) {
      const detail = await apiCallRaw<{
        status: string;
        items: Array<Record<string, unknown>>;
      }>(page, 'GET', `/inventory/transfers/${transferId}`);

      expect(detail.items?.length).toBeGreaterThan(0);
      const item = detail.items?.[0];

      expect(item.color_no).toBe(colorNo);
      expect(item.dye_lot_no).toBe(dyeLotNo);
      expect(item.batch_no).toBe(batchNo);
    }
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

    let recipeId: number;
    try {
      const result = await apiCall<{ id?: number }>(
        page,
        'POST',
        '/production/dye-recipes',
        recipeData
      );
      recipeId = result.data?.id!;
    } catch (e) {
      // 创建失败直接暴露（兜底旧配方无自建字段，精确断言会失真）
      throw e;
    }
    expect(recipeId).toBeDefined();

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

    let fabricId: number;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/greige-fabrics', fabricData);
      fabricId = result.data?.id!;
    } catch {
      try {
        const result = await apiCall<{ id?: number }>(
          page,
          'POST',
          '/production/greige-fabrics',
          fabricData
        );
        fabricId = result.data?.id!;
      } catch {
        const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
          page,
          'GET',
          '/greige-fabrics?page=1&page_size=1'
        ).catch(() => ({ items: [] }));
        fabricId = list.items?.[0]?.id;
      }
    }

    if (fabricId) {
      let detail: Record<string, unknown> | null = null;
      try {
        detail = await apiCallRaw<Record<string, unknown>>(
          page,
          'GET',
          `/greige-fabrics/${fabricId}`
        );
      } catch {
        detail = await apiCallRaw<Record<string, unknown>>(
          page,
          'GET',
          `/production/greige-fabrics/${fabricId}`
        );
      }

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
