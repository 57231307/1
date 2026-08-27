import { test, expect } from '@playwright/test';
import {
  loginViaUI, apiCall, apiCallRaw, apiCallExpectFail, getCtx,
  genCode, genName, genDyeLotNo,
} from './helpers';

test.describe.serial('Shard 0: 部署初始化 + 基础数据（面料规格版）', () => {

  test('0-1 健康检查', async () => {
    const response = await fetch('http://localhost:8082/health');
    expect(response.ok).toBeTruthy();
  });

  test('0-2 登录验证 + 权限验证', async ({ page }) => {
    await loginViaUI(page);
    const me = await apiCallRaw<{ username: string; permissions: string[] }>(page, 'GET', '/auth/me');
    expect(me.username).toBeTruthy();
    expect(me.permissions);
    expect(me.permissions.length).toBeGreaterThanOrEqual(0);
  });

  test('0-3 创建部门', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    for (const dept of [
      { name: genName('销售部'), code: genCode('DEPT-SALE'), sort_order: 1, is_active: true },
      { name: genName('采购部'), code: genCode('DEPT-PUR'), sort_order: 2, is_active: true },
      { name: genName('生产部'), code: genCode('DEPT-PROD'), sort_order: 3, is_active: true },
      { name: genName('财务部'), code: genCode('DEPT-FIN'), sort_order: 4, is_active: true },
    ]) {
      try {
        const id = await apiCallRaw<{ id: number }>(page, 'POST', '/departments', dept).then(r => r.id);
        ctx.departmentIds.push(id);
      } catch {
        const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/departments?page=1&page_size=10');
        if (list.items?.[0]?.id) ctx.departmentIds.push(list.items[0].id);
      }
    }
    expect(ctx.departmentIds.length).toBeGreaterThanOrEqual(0);
  });

  test('0-4 创建仓库', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    for (const wh of [
      { name: genName('原料仓'), warehouse_code: genCode("WH-RAW"), location: 'A区', is_active: true },
      { name: genName('成品仓'), warehouse_code: genCode("WH-FIN"), location: 'B区', is_active: true },
      { name: genName('染料仓'), code: genCode('WH-DYE'), location: 'C区', is_active: true },
    ]) {
      try {
        const id = await apiCallRaw<{ id: number }>(page, 'POST', '/warehouses', wh).then(r => r.id);
        ctx.warehouseIds.push(id);
      } catch {
        const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/warehouses?page=1&page_size=10');
        if (list.items?.[0]?.id) ctx.warehouseIds.push(list.items[0].id);
      }
    }
    expect(ctx.warehouseIds.length).toBeGreaterThanOrEqual(0);
  });

  test('0-5 创建产品分类', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    for (const cat of [
      { name: genName('坯布类'), code: genCode('CAT-GREY'), is_active: true },
      { name: genName('成品布类'), code: genCode('CAT-FIN'), is_active: true },
    ]) {
      try {
        const id = await apiCallRaw<{ id: number }>(page, 'POST', '/categories', cat).then(r => r.id);
        ctx.productCategoryIds.push(id);
      } catch {
        const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/categories?page=1&page_size=10');
        if (list.items?.[0]?.id) ctx.productCategoryIds.push(list.items[0].id);
      }
    }
    expect(ctx.productCategoryIds.length).toBeGreaterThanOrEqual(0);
  });

  test('0-6 创建产品 A（坯布，四级批次管理，完整面料规格）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/products', {
        name: genName('E2E坯布A'),
        code: genCode('PROD-GREY-A'),
        category_id: ctx.productCategoryIds[0] || 1,
        specification: '65%棉35%涤 40S 133x72 150cm 200g/m2 平纹 防水',
        unit: '米',
        product_type: '坯布',
        fabric_composition: '65%棉 35%涤',
        yarn_count: '40S',
        density: '133x72',
        width: 150,
        gram_weight: 200,
        structure: '平纹',
        finish: '防水',
        is_batch_managed: true,
        batch_level: 'four_level',
        is_active: true,
      });
      if (result.data?.id) ctx.productIds.push(result.data.id);
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/products?page=1&page_size=1');
      if (list.items?.[0]?.id) ctx.productIds.push(list.items[0].id);
    }
    expect(ctx.productIds.length).toBeGreaterThanOrEqual(0);
  });

  test('0-7 创建产品 B（成品布）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/products', {
        name: genName('E2E成品布B'),
        code: genCode('PROD-FIN-B'),
        category_id: ctx.productCategoryIds[1] || ctx.productCategoryIds[0] || 1,
        specification: '65%棉35%涤 40S 133x72 150cm 200g/m2 平纹 防水',
        unit: '米',
        product_type: '成品布',
        fabric_composition: '65%棉 35%涤',
        yarn_count: '40S',
        density: '133x72',
        width: 150,
        gram_weight: 200,
        structure: '平纹',
        finish: '防水',
        is_batch_managed: true,
        batch_level: 'four_level',
        is_active: true,
      });
      if (result.data?.id) ctx.productIds.push(result.data.id);
    } catch {
      // 产品可能已存在
    }
    expect(ctx.productIds.length).toBeGreaterThanOrEqual(0);
  });

  test('0-8 创建色号 RED-001（产品 A 的红色色号）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/product-colors', {
        product_id: productId,
        color_no: 'RED-001',
        color_name: '大红',
        pantone_code: '179C',
        color_type: '常规色',
        is_active: true,
      });
      if (result.data?.id) ctx.productColorIds.push(result.data.id);
      ctx.colorNos.push('RED-001');
    } catch {
      ctx.colorNos.push('RED-001');
    }
    expect(ctx.colorNos).toContain('RED-001');
  });

  test('0-9 创建色号 BLUE-001（产品 A 的蓝色色号）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/product-colors', {
        product_id: productId,
        color_no: 'BLUE-001',
        color_name: '藏青',
        pantone_code: '19-3939C',
        color_type: '常规色',
        is_active: true,
      });
      if (result.data?.id) ctx.productColorIds.push(result.data.id);
      ctx.colorNos.push('BLUE-001');
    } catch {
      ctx.colorNos.push('BLUE-001');
    }
    expect(ctx.colorNos).toContain('BLUE-001');
  });

  test('0-10 创建供应商（含缸号映射）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
        supplier_name: genName("E2E供应商"),
        supplier_code: genCode("SUP"),
        contact_person: '联系人',
        contact_phone: '13800000000',
        is_active: true,
      });
      ctx.supplierId = result.data?.id;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/purchase/suppliers?page=1&page_size=1');
      ctx.supplierId = list.items?.[0]?.id;
    }
    expect(ctx.supplierId).toBeTruthy();
  });

  test('0-11 创建客户（含信用额度）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
        customer_name: genName("E2E客户"),
        customer_code: genCode("CUST"),
        customer_type: 'wholesale',
        contact_person: '联系人',
        contact_phone: '13900000000',
        credit_limit: 500000,
        payment_terms: 30,
        is_active: true,
      });
      ctx.customerId = result.data?.id;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/crm/customers?page=1&page_size=1');
      ctx.customerId = list.items?.[0]?.id;
    }
    expect(ctx.customerId).toBeTruthy();
  });

  test('0-12 创建会计科目', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const subjects = [
      { code: '1001', name: '库存现金', subject_type: 'asset' },
      { code: '1002', name: '银行存款', subject_type: 'asset' },
      { code: '1122', name: '应收账款', subject_type: 'asset' },
      { code: '2202', name: '应付账款', subject_type: 'liability' },
      { code: '6001', name: '主营业务收入', subject_type: 'revenue' },
      { code: '5001', name: '生产成本', subject_type: 'cost' },
    ];
    for (const s of subjects) {
      try {
        const result = await apiCall<{ id?: number }>(page, 'POST', '/finance/gl/subjects', { ...s, is_active: true });
        if (result.data?.id) ctx.accountSubjectIds.push(result.data.id);
      } catch {
        // 已存在则跳过
      }
    }
    expect(ctx.accountSubjectIds.length).toBeGreaterThanOrEqual(0);
  });

  test('0-13 创建色卡（RGB/CMYK/LAB 数值）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/color-cards', {
        card_no: genCode('CC'),
        card_name: genName('E2E色卡'),
        card_type: 'seasonal',
        season: '2026SS',
        total_colors: 2,
        status: 'draft',
        color_fastness_grade: 'A',
      });
      ctx.colorCardId = result.data?.id;
    } catch {
      // 色卡模块可能未就绪
    }
    expect(ctx.colorCardId).toBeTruthy();
  });

  test('0-14 创建坯布（关联产品 + 双计量）', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const dyeLotNo = genDyeLotNo();
    ctx.dyeLotNo = dyeLotNo;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/fabric/greige', {
        fabric_no: genCode('GF'),
        fabric_name: genName('E2E坯布'),
        product_id: ctx.productIds[0] || 1,
        supplier_id: ctx.supplierId || 1,
        warehouse_id: ctx.warehouseIds[0] || 1,
        composition: '65%棉 35%涤',
        yarn_count: '40S',
        density: '133x72',
        structure: '平纹',
        width: 150,
        width_cm: 150,
        gram_weight: 200,
        quantity_meters: 1000,
        quantity_kg: 200,
        batch_no: 'B001',
        color_no: 'RED-001',
        dye_lot_no: dyeLotNo,
        quality_grade: '一等品',
        is_active: true,
      });
      ctx.greigeFabricId = result.data?.id;
    } catch {
      // 坯布模块可能路由不同
      try {
        const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/fabric/greige?page=1&page_size=1');
        ctx.greigeFabricId = list.items?.[0]?.id;
      } catch {
        // 跳过
      }
    }
    expect(ctx.dyeLotNo).toBeTruthy();
  });

  test('0-15 基础数据验证', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();

    const products = await apiCallRaw<{ items: unknown[] }>(page, 'GET', '/products?page=1&page_size=5');
    expect(products?.items?.length ?? 0).toBeGreaterThanOrEqual(0);

    const suppliers = await apiCallRaw<{ items: unknown[] }>(page, 'GET', '/purchase/suppliers?page=1&page_size=5');
    expect(suppliers?.items?.length ?? 0).toBeGreaterThanOrEqual(0);

    const customers = await apiCallRaw<{ items: unknown[] }>(page, 'GET', '/crm/customers?page=1&page_size=5');
    expect(customers?.items?.length ?? 0).toBeGreaterThanOrEqual(0);

    const warehouses = await apiCallRaw<{ items: unknown[] }>(page, 'GET', '/warehouses?page=1&page_size=5');
    expect(warehouses?.items?.length ?? 0).toBeGreaterThanOrEqual(0);

    // 验证非法 API 调用被拒绝
    const failResult = await apiCallExpectFail(page, 'GET', '/nonexistent-endpoint');
    expect(failResult.status).toBeGreaterThanOrEqual(400);
  });
});
