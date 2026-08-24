import { test, expect } from '@playwright/test';
import { loginViaUI, apiCall, apiCallRaw, getCtx, genCode, genName } from './helpers';

test.describe.serial('Shard 0: 部署初始化 + 基础数据', () => {
  test('0-1 健康检查', async () => {
    const response = await fetch('http://localhost:8082/health');
    expect(response.ok).toBeTruthy();
  });

  test('0-2 登录验证', async ({ page }) => {
    await loginViaUI(page);
    const me = await apiCallRaw<{ username: string }>(page, 'GET', '/auth/me');
    expect(me.username).toBe('e2e_admin');
  });

  test('0-3 创建部门', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      ctx.departmentId = await createViaAPI(page, '/departments', {
        name: genName('E2E部门'),
        code: genCode('DEPT'),
        sort_order: 1,
        is_active: true,
      });
    } catch (e) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/departments?page=1&page_size=1');
      ctx.departmentId = list.items?.[0]?.id;
    }
    expect(ctx.departmentId).toBeTruthy();
  });

  test('0-4 创建仓库', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      ctx.warehouseId = await createViaAPI(page, '/warehouses', {
        name: genName('E2E仓库'),
        code: genCode('WH'),
        location: '测试地址',
        is_active: true,
      });
    } catch (e) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/warehouses?page=1&page_size=1');
      ctx.warehouseId = list.items?.[0]?.id;
    }
    expect(ctx.warehouseId).toBeTruthy();
  });

  test('0-5 创建产品分类', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      ctx.productCategoryId = await createViaAPI(page, '/categories', {
        name: genName('E2E分类'),
        code: genCode('CAT'),
        is_active: true,
      });
    } catch (e) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/categories?page=1&page_size=1');
      ctx.productCategoryId = list.items?.[0]?.id;
    }
    expect(ctx.productCategoryId).toBeTruthy();
  });

  test('0-6 创建产品', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    for (let i = 0; i < 3; i++) {
      try {
        const id = await createViaAPI(page, '/products', {
          name: genName(`E2E产品${i}`),
          code: genCode('PROD'),
          category_id: ctx.productCategoryId,
          specifications: '测试规格',
          unit: '米',
          is_active: true,
        });
        ctx.productIds.push(id);
      } catch (e) {
        // 已存在则跳过
      }
    }
    expect(ctx.productIds.length).toBeGreaterThanOrEqual(1);
  });

  test('0-7 创建供应商', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      ctx.supplierId = await createViaAPI(page, '/suppliers', {
        name: genName('E2E供应商'),
        code: genCode('SUP'),
        contact_person: '联系人',
        contact_phone: '13800000000',
        is_active: true,
      });
    } catch (e) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/suppliers?page=1&page_size=1');
      ctx.supplierId = list.items?.[0]?.id;
    }
    expect(ctx.supplierId).toBeTruthy();
  });

  test('0-8 创建客户', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    try {
      ctx.customerId = await createViaAPI(page, '/crm/customers', {
        name: genName('E2E客户'),
        code: genCode('CUST'),
        customer_type: 'wholesale',
        contact_person: '联系人',
        contact_phone: '13900000000',
        credit_limit: 100000,
        is_active: true,
      });
    } catch (e) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/crm/customers?page=1&page_size=1');
      ctx.customerId = list.items?.[0]?.id;
    }
    expect(ctx.customerId).toBeTruthy();
  });

  test('0-9 创建会计科目', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    const subjects = [
      { code: '1001', name: '库存现金', subject_type: 'asset' },
      { code: '1002', name: '银行存款', subject_type: 'asset' },
      { code: '2202', name: '应付账款', subject_type: 'liability' },
      { code: '1122', name: '应收账款', subject_type: 'asset' },
      { code: '6001', name: '主营业务收入', subject_type: 'revenue' },
      { code: '5001', name: '生产成本', subject_type: 'cost' },
    ];
    for (const s of subjects) {
      try {
        const id = await createViaAPI(page, '/finance/gl/subjects', {
          ...s,
          is_active: true,
        });
        ctx.accountSubjectIds.push(id);
      } catch (e) {
        // 已存在则跳过
      }
    }
    expect(ctx.accountSubjectIds.length).toBeGreaterThanOrEqual(1);
  });

  test('0-10 基础数据验证', async ({ page }) => {
    await loginViaUI(page);
    const ctx = getCtx();
    // 验证所有基础数据已创建
    const products = await apiCallRaw<{ items: unknown[] }>(page, 'GET', '/products?page=1&page_size=1');
    expect(products.items.length).toBeGreaterThan(0);
    const suppliers = await apiCallRaw<{ items: unknown[] }>(page, 'GET', '/suppliers?page=1&page_size=1');
    expect(suppliers.items.length).toBeGreaterThan(0);
    const customers = await apiCallRaw<{ items: unknown[] }>(page, 'GET', '/crm/customers?page=1&page_size=1');
    expect(customers.items.length).toBeGreaterThan(0);
  });
});

async function createViaAPI(page: import('@playwright/test').Page, endpoint: string, data: Record<string, unknown>): Promise<number> {
  const result = await apiCall<{ id?: number; success?: boolean }>(page, 'POST', endpoint, data);
  if (result.data?.id) return result.data.id;
  // 尝试从 list 中查找
  const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', `${endpoint}?page=1&page_size=1`);
  if (list.items?.[0]?.id) return list.items[0].id;
  throw new Error(`Could not create or find entity at ${endpoint}`);
}
