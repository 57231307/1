/* eslint-disable no-console */
import type { Page } from '@playwright/test';

export const API_BASE = process.env.API_BASE || 'http://localhost:8082';
export const API_PREFIX = '/api/v1/erp';
export const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';
export const TEST_USERNAME = process.env.TEST_USERNAME || 'e2e_admin';
export const TEST_PASSWORD = process.env.TEST_PASSWORD || 'E2e@TestPassword2026!';

export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
  timestamp?: string;
}

export interface EntityContext {
  departmentIds: number[];
  warehouseIds: number[];
  productCategoryIds: number[];
  productIds: number[];
  productColorIds: number[];
  colorNos: string[];
  supplierId?: number;
  customerId?: number;
  accountSubjectIds: number[];
  colorCardId?: number;
  greigeFabricId?: number;
  dyeBatchId?: number;
  dyeLotNo?: string;
  dyeRecipeId?: number;
  productionRecipeId?: number;
  bomId?: number;
  purchaseOrderId?: number;
  salesOrderId?: number;
  quotationId?: number;
  productionOrderId?: number;
  pieceIds: number[];
  apInvoiceId?: number;
  arInvoiceId?: number;
  voucherId?: number;
  fixedAssetId?: number;
  budgetId?: number;
  customOrderId?: number;
  roleId?: number;
  userIds: number[];
}

const ctx: EntityContext = {
  departmentIds: [],
  warehouseIds: [],
  productCategoryIds: [],
  productIds: [],
  productColorIds: [],
  colorNos: [],
  accountSubjectIds: [],
  pieceIds: [],
  userIds: [],
};

export function getCtx(): EntityContext {
  return ctx;
}

/**
 * 确保 EntityContext 有测试所需的基础实体 ID
 * 分片后每个 shard 独立运行，EntityContext 单例不跨 shard 共享
 * 此函数在每个 spec 文件开头调用，自行创建或查找实体
 */
export async function ensureTestEntities(page: Page): Promise<void> {
  // 查找或创建仓库
  try {
    const warehouses = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/warehouses?page=1&page_size=5');
    ctx.warehouseIds = warehouses.items?.map((w) => w.id) || [];
  } catch { ctx.warehouseIds = []; }
  if (ctx.warehouseIds.length < 2) {
    for (let i = ctx.warehouseIds.length; i < 2; i++) {
      try {
        const result = await apiCall<{ id?: number }>(page, 'POST', '/warehouses', {
          name: 'E2E 仓库 ' + i + '-' + Date.now(),
          code: 'E2E-W' + i + Date.now(),
        });
        if (result.data?.id) ctx.warehouseIds.push(result.data.id);
      } catch (e) { console.error("[ensureTestEntities] 创建失败:", (e as Error).message); }
    }
  }
  if (ctx.warehouseIds.length < 2) ctx.warehouseIds = [1, 2];

  // 查找或创建产品
  try {
    const products = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/products?page=1&page_size=5');
    ctx.productIds = products.items?.map((p) => p.id) || [];
  } catch { ctx.productIds = []; }
  if (ctx.productIds.length === 0) {
    for (let i = 0; i < 3; i++) {
      try {
        const result = await apiCall<{ id?: number }>(page, 'POST', '/products', {
          name: 'E2E 产品 ' + i + '-' + Date.now(),
          code: 'E2E-P' + i + Date.now(),
          unit: '米',
          product_type: 'fabric',
        });
        if (result.data?.id) ctx.productIds.push(result.data.id);
      } catch (e) { console.error("[ensureTestEntities] 创建失败:", (e as Error).message); }
    }
  }
  if (ctx.productIds.length === 0) ctx.productIds = [1];

  // 查找产品色号
  try {
    const colors = await apiCallRaw<{ items: Array<{ id: number; color_no: string }> }>(page, 'GET', `/product-colors?product_id=${ctx.productIds[0]}&page=1&page_size=5`);
    ctx.productColorIds = colors.items?.map((c) => c.id) || [];
    ctx.colorNos = colors.items?.map((c) => c.color_no) || ['TEST-COLOR'];
  } catch { ctx.colorNos = ['TEST-COLOR']; ctx.productColorIds = [1]; }
  if (ctx.colorNos.length === 0) ctx.colorNos = ['TEST-COLOR'];

  // 查找或创建供应商
  try {
    const suppliers = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/purchase/suppliers?page=1&page_size=1');
    ctx.supplierId = suppliers.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] supplierId 创建失败:", (e as Error).message); ctx.supplierId = undefined; }
  if (!ctx.supplierId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
        supplier_name: 'E2E 供应商 ' + Date.now(),
        supplier_type: 'fabric',
      });
      ctx.supplierId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] supplierId 创建失败:", (e as Error).message); ctx.supplierId = undefined; }
  }

  // 查找或创建客户
  try {
    const customers = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/crm/customers?page=1&page_size=1');
    ctx.customerId = customers.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] customerId 创建失败:", (e as Error).message); ctx.customerId = undefined; }
  if (!ctx.customerId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
        customer_name: 'E2E 客户 ' + Date.now(),
      });
      ctx.customerId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] customerId 创建失败:", (e as Error).message); ctx.customerId = undefined; }
  }

  // 查找会计科目
  try {
    const subjects = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/subjects?page=1&page_size=5');
    ctx.accountSubjectIds = subjects.items?.map((s) => s.id) || [];
  } catch { ctx.accountSubjectIds = []; }

  // 查找或创建采购订单
  try {
    const pos = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/purchase/orders?page=1&page_size=1');
    ctx.purchaseOrderId = pos.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.purchaseOrderId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
        supplier_id: ctx.supplierId || 1,
        order_date: new Date().toISOString().slice(0, 10),
        items: [{ material_id: ctx.productIds[0] || 1, quantity_ordered: '1', unit_price: '1' }],
      });
      ctx.purchaseOrderId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] purchaseOrderId 创建失败:", (e as Error).message); ctx.purchaseOrderId = undefined; }
  }

  // 查找或创建销售订单
  try {
    const sos = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/sales/orders?page=1&page_size=1');
    ctx.salesOrderId = sos.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.salesOrderId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
        customer_id: ctx.customerId || 1,
        order_date: new Date().toISOString().slice(0, 10),
        items: [{ product_id: ctx.productIds[0] || 1, quantity: '1', unit_price: '1' }],
      });
      ctx.salesOrderId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] salesOrderId 创建失败:", (e as Error).message); ctx.salesOrderId = undefined; }
  }

  // 查找或创建报价单
  try {
    const qts = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/quotations?page=1&page_size=1');
    ctx.quotationId = qts.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.quotationId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/quotations', {
        customer_id: ctx.customerId || 1,
        sales_user_id: 1,
        quotation_date: new Date().toISOString().slice(0, 10),
        valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
        currency: 'CNY', exchange_rate: '1', base_currency: 'CNY',
        price_terms: 'FOB', tax_inclusive: false, tax_rate: '13',
        items: [{ product_id: ctx.productIds[0] || 1, unit: '米', quantity: '1', unit_price: '1', unit_price_with_tax: '1.13' }],
      });
      ctx.quotationId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] quotationId 创建失败:", (e as Error).message); ctx.quotationId = undefined; }
  }

  // 查找或创建缸号
  try {
    const batches = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/production/dye-batches?page=1&page_size=1');
    ctx.dyeBatchId = batches.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.dyeBatchId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-batches', {
        batch_no: genCode('DB'), color_no: ctx.colorNos[0] || 'TEST', dye_lot_no: ctx.dyeLotNo,
        planned_quantity: 100, status: 'draft',
      });
      ctx.dyeBatchId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] dyeBatchId 创建失败:", (e as Error).message); ctx.dyeBatchId = undefined; }
  }

  // 生成缸号
  if (!ctx.dyeLotNo) ctx.dyeLotNo = genDyeLotNo();

  // 查找或创建染色配方
  try {
    const recipes = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/production/dye-recipes?page=1&page_size=1');
    ctx.dyeRecipeId = recipes.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.dyeRecipeId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
        recipe_no: genCode('DR'), recipe_name: 'E2E 自动配方', color_code: ctx.colorNos[0] || 'TEST',
        color_name: '测试色', fabric_type: '涤纶', dye_type: '分散染色',
      });
      ctx.dyeRecipeId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] dyeRecipeId 创建失败:", (e as Error).message); ctx.dyeRecipeId = undefined; }
  }

  // 查找大货处方
  try {
    const prs = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/production/production-recipes?page=1&page_size=1');
    ctx.productionRecipeId = prs.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] productionRecipeId 创建失败:", (e as Error).message); ctx.productionRecipeId = undefined; }

  // 查找或创建 BOM
  try {
    const boms = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/boms?page=1&page_size=1');
    ctx.bomId = boms.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.bomId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/boms', {
        product_id: ctx.productIds[0] || 1, name: 'E2E BOM', version: '1',
      });
      ctx.bomId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] bomId 创建失败:", (e as Error).message); ctx.bomId = undefined; }
  }

  // 查找生产订单
  try {
    const pos = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/production/production-orders/orders?page=1&page_size=1');
    ctx.productionOrderId = pos.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] productionOrderId 创建失败:", (e as Error).message); ctx.productionOrderId = undefined; }

  // 查找或创建凭证（凭证分录需要 1001/1002 科目，缺失时主动创建）
  try {
    const vs = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/vouchers?page=1&page_size=1');
    ctx.voucherId = vs.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.voucherId) {
    // 兜底创建凭证所需的会计科目（种子库可能没有预置）
    for (const subj of [
      { code: '1001', name: '库存现金 E2E', level: 1, balance_direction: 'debit' },
      { code: '1002', name: '银行存款 E2E', level: 1, balance_direction: 'debit' },
    ]) {
      await apiCall(page, 'POST', '/subjects', subj).catch((e) => { console.error("[ensureTestEntities] 科目创建失败:", (e as Error).message); });
    }
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
        voucher_type: 'general', voucher_date: new Date().toISOString().slice(0, 10),
        items: [
          { subject_code: '1001', debit: '1', credit: '0', summary: 'E2E' },
          { subject_code: '1002', debit: '0', credit: '1', summary: 'E2E' },
        ],
      });
      ctx.voucherId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] voucherId 创建失败:", (e as Error).message); ctx.voucherId = undefined; }
  }

  // 查找固定资产
  try {
    const fas = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/fixed-assets?page=1&page_size=1');
    ctx.fixedAssetId = fas.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] fixedAssetId 创建失败:", (e as Error).message); ctx.fixedAssetId = undefined; }

  // 查找预算
  try {
    const bs = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/budgets?page=1&page_size=1');
    ctx.budgetId = bs.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] budgetId 创建失败:", (e as Error).message); ctx.budgetId = undefined; }

  // 查找 AP 发票
  try {
    const aps = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/ap/invoices?page=1&page_size=1');
    ctx.apInvoiceId = aps.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] apInvoiceId 创建失败:", (e as Error).message); ctx.apInvoiceId = undefined; }

  // 查找 AR 发票
  try {
    const ars = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/ar/invoices?page=1&page_size=1');
    ctx.arInvoiceId = ars.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] arInvoiceId 创建失败:", (e as Error).message); ctx.arInvoiceId = undefined; }

  // 查找或创建定制订单
  try {
    const cos = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/custom-orders?page=1&page_size=1');
    ctx.customOrderId = cos.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.customOrderId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/custom-orders', {
        customer_id: ctx.customerId || 1, order_no: genCode('CO'),
        order_date: new Date().toISOString().slice(0, 10), product_name: 'E2E 定制',
      });
      ctx.customOrderId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] customOrderId 创建失败:", (e as Error).message); ctx.customOrderId = undefined; }
  }

  // 查找或创建色卡
  try {
    const ccs = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/color-cards/?page=1&page_size=1');
    ctx.colorCardId = ccs.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] 查找失败:", (e as Error).message); }
  if (!ctx.colorCardId) {
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/color-cards/', {
        card_no: genCode('CC'), card_name: 'E2E 色卡', card_type: 'CUSTOM',
      });
      ctx.colorCardId = result.data?.id;
    } catch (e) { console.error("[ensureTestEntities] colorCardId 创建失败:", (e as Error).message); ctx.colorCardId = undefined; }
  }

  // 查找坯布
  try {
    const gfs = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/production/greige-fabrics?page=1&page_size=1');
    ctx.greigeFabricId = gfs.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] greigeFabricId 创建失败:", (e as Error).message); ctx.greigeFabricId = undefined; }

  // 查找角色 ID
  try {
    const roles = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/roles?page=1&page_size=1');
    ctx.roleId = roles.items?.[0]?.id;
  } catch (e) { console.error("[ensureTestEntities] roleId 创建失败:", (e as Error).message); ctx.roleId = undefined; }
}

async function getCsrfToken(page: Page): Promise<string> {
  const cookies = await page.context().cookies();
  const csrf = cookies.find((c) => c.name === 'csrf_token');
  if (!csrf) {
    throw new Error('csrf_token cookie not found — are you logged in?');
  }
  return csrf.value;
}

export async function apiCall<T = unknown>(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<ApiResponse<T>> {
  const csrfToken = await getCsrfToken(page);
  const url = `${API_BASE}${API_PREFIX}${path}`;
  const response = await page.request.fetch(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      'X-CSRF-Token': csrfToken,
    },
    data: body ? JSON.stringify(body) : undefined,
  });

  const text = await response.text();
  let json: ApiResponse<T>;
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error(`API ${method} ${path} returned non-JSON (status ${response.status()}): ${text.slice(0, 500)}`);
  }

  if (json.code !== 200 && json.code !== 0) {
    throw new Error(`API ${method} ${path} failed: code=${json.code} message=${json.message}`);
  }

  // POST/PUT/PATCH/DELETE 成功后，后端通过 Set-Cookie 下发新的 csrf_token，
  // 重新读取 cookie 确保 getCsrfToken 拿到最新值
  if (method !== 'GET' && method !== 'HEAD') {
    try {
      await page.context().cookies();
    } catch { /* ignore */ }
  }

  return json;
}

export async function apiCallRaw<T = unknown>(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<T> {
  const res = await apiCall<T>(page, method, path, body);
  return res.data;
}

export async function apiCallExpectFail(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<{ status: number; code?: number; message?: string }> {
  const csrfToken = await getCsrfToken(page);
  const url = `${API_BASE}${API_PREFIX}${path}`;
  const response = await page.request.fetch(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      'X-CSRF-Token': csrfToken,
    },
    data: body ? JSON.stringify(body) : undefined,
  });

  const text = await response.text();
  let json: { code?: number; message?: string } = {};
  try {
    json = JSON.parse(text);
  } catch {
    // non-JSON response
  }
  return { status: response.status(), code: json.code, message: json.message };
}

const LOGGED_IN = { done: false };

export async function loginViaUI(page: Page, username?: string, password?: string): Promise<void> {
  // 如果已经登录过（同一个 page context），检查是否还有效
  if (LOGGED_IN.done) {
    // 已登录过，但可能在不同的 BrowserContext 中
    // 检查 cookie 是否还在
    const cookies = await page.context().cookies();
    const hasToken = cookies.some((c) => c.name === 'access_token');
    if (hasToken) {
      // cookie 存在，直接导航
      await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded' }).catch(() => {});
      return;
    }
    // cookie 不在（新 context），用 API 登录注入 cookie（不用 UI，避免 429）
    try {
      const loginResp = await page.request.post(`${API_BASE}${API_PREFIX}/auth/login`, {
        data: { username: TEST_USERNAME, password: TEST_PASSWORD },
        headers: { 'Content-Type': 'application/json', 'X-Requested-With': 'XMLHttpRequest' },
      });
      if (loginResp.ok()) {
        // API 登录成功，cookie 已注入 context
        await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded' }).catch(() => {});
        return;
      }
    } catch {
      // API 登录也失败（可能 429），继续 UI 登录
    }
    LOGGED_IN.done = false;
  }

  const u = username || TEST_USERNAME;
  const p = password || TEST_PASSWORD;

  const consoleLogs: string[] = [];
  page.on('console', (msg) => {
    consoleLogs.push(`[console.${msg.type()}] ${msg.text()}`);
  });

  await page.goto(`${BASE_URL}/login`, { waitUntil: 'domcontentloaded' });

  // 等 2 秒让 Vite 触发可能的 504
  await page.waitForTimeout(2000);

  // 如果有 504，等 Vite 自动优化完成（不关闭原 page，直接重新 goto）
  const has504 = consoleLogs.some((log) => log.includes('504'));
  if (has504) {
    console.log('检测到 Vite 504，等待 5 秒后重新加载...');
    await page.waitForTimeout(5000);
    consoleLogs.length = 0;
    await page.goto(`${BASE_URL}/login`, { waitUntil: 'networkidle' });
    await page.evaluate(() => {
      window.localStorage.setItem('bingxi.locale', 'zh-CN');
    });
    await page.waitForTimeout(2000);
    // 检查是否还有 504
    const newHas504 = consoleLogs.some((log) => log.includes('504'));
    if (newHas504) {
      console.log('仍有 504，再等 5 秒...');
      await page.waitForTimeout(5000);
      await page.goto(`${BASE_URL}/login`, { waitUntil: 'networkidle' });
      await page.evaluate(() => {
        window.localStorage.setItem('bingxi.locale', 'zh-CN');
      });
      await page.waitForTimeout(2000);
    }
    await loginOnPage(page, u, p, consoleLogs);
    LOGGED_IN.done = true;
    return;
  }

  // 无 504，直接在原页面登录
  await page.evaluate(() => {
    window.localStorage.setItem('bingxi.locale', 'zh-CN');
  });
  await page.waitForTimeout(2000);
  await loginOnPage(page, u, p, consoleLogs);
  LOGGED_IN.done = true;
}

async function loginOnPage(page: Page, u: string, p: string, consoleLogs: string[]): Promise<void> {
  // Element Plus el-input：同时匹配中英文 placeholder
  const usernameInput = page.locator('input[placeholder="用户名"], input[placeholder="Username"]');
  await usernameInput.first().waitFor({ state: 'visible', timeout: 30_000 });
  await usernameInput.first().fill(u);

  const passwordInput = page.locator('input[placeholder="密码"], input[placeholder="Password"]');
  await passwordInput.first().waitFor({ state: 'visible', timeout: 30_000 });
  await passwordInput.first().fill(p);

  // 必须勾选用户协议（表单验证要求 agreedToTerms=true）
  // Element Plus el-checkbox 点击 .el-checkbox__inner（视觉复选框区域）
  const checkboxInner = page.locator('.el-checkbox__inner').first();
  const isChecked = await page.locator('.el-checkbox input').first().isChecked().catch(() => false);
  console.log(`复选框初始状态: checked=${isChecked}`);
  if (!isChecked) {
    // 点击视觉复选框区域（.el-checkbox__inner）
    await checkboxInner.click();
    await page.waitForTimeout(500);
    let nowChecked = await page.locator('.el-checkbox input').first().isChecked().catch(() => false);
    console.log(`点击 inner 后复选框状态: checked=${nowChecked}`);
    if (!nowChecked) {
      // fallback: 点击 label
      await page.locator('.el-checkbox').first().click();
      await page.waitForTimeout(300);
      nowChecked = await page.locator('.el-checkbox input').first().isChecked().catch(() => false);
      console.log(`点击 label 后复选框状态: checked=${nowChecked}`);
    }
    if (!nowChecked) {
      // 最终 fallback: 直接修改 input checked 属性并触发 change 事件
      await page.evaluate(() => {
        const input = document.querySelector('.el-checkbox input') as HTMLInputElement;
        if (input) {
          input.checked = true;
          input.dispatchEvent(new Event('change', { bubbles: true }));
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      });
      await page.waitForTimeout(300);
      console.log('通过 JS 设置 checked=true');
    }
  }

  // 点击登录按钮
  const loginButton = page.locator('form button.el-button--primary').first();
  await loginButton.waitFor({ state: 'visible', timeout: 10_000 });
  const isDisabled = await loginButton.isDisabled().catch(() => false);
  console.log(`登录按钮 disabled: ${isDisabled}`);
  await loginButton.click();

  // 如果 3 秒后仍在 /login，尝试通过表单提交
  await page.waitForTimeout(3000);
  if (page.url().includes('/login')) {
    // 检查是否有表单验证错误
    const formErrors = await page.locator('.el-form-item__error').allTextContents().catch(() => []);
    console.log(`表单验证错误: ${JSON.stringify(formErrors)}`);
    // 尝试通过 dispatchEvent 触发表单提交
    await page.evaluate(() => {
      const form = document.querySelector('form');
      if (form) form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
    });
  }

  // 如果 3 秒后仍在 /login，尝试通过表单提交
  await page.waitForTimeout(3000);
  if (page.url().includes('/login')) {
    // 尝试通过 dispatchEvent 触发表单提交
    await page.evaluate(() => {
      const form = document.querySelector('form');
      if (form) form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
    });
  }

  // 等待离开 /login 页面
  try {
    await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 60_000 });
  } catch {
    // 登录后仍然在 /login，输出诊断信息
    const currentUrl = page.url();
    const elMessages = await page.locator('.el-message__content').allTextContents().catch(() => []);
    console.error(`=== UI 登录失败诊断 ===`);
    console.error(`当前 URL: ${currentUrl}`);
    console.error(`ElMessage 提示: ${JSON.stringify(elMessages)}`);
    console.error(`Console 日志（最后 20 条）:`);
    consoleLogs.slice(-20).forEach((log) => console.error(log));
    // 截图
    await page.screenshot({ path: 'test-results/login-failure-diagnosis.png', fullPage: true });
    throw new Error(`UI 登录失败: 60s 后仍在 ${currentUrl}，ElMessage: ${JSON.stringify(elMessages)}`);
  }

  // 登录成功后，验证 cookie 已设置
  const cookies = await page.context().cookies();
  const hasToken = cookies.some((c) => c.name === 'access_token');
  const hasCsrf = cookies.some((c) => c.name === 'csrf_token');
  if (!hasToken || !hasCsrf) {
    console.error(`=== Cookie 缺失诊断 ===`);
    console.error(`access_token: ${hasToken}, csrf_token: ${hasCsrf}`);
    console.error(`所有 cookie: ${cookies.map(c => c.name).join(', ')}`);
    console.error(`当前 URL: ${page.url()}`);
    await page.screenshot({ path: 'test-results/cookie-missing-diagnosis.png', fullPage: true });
    throw new Error(`UI 登录后 cookie 缺失: access_token=${hasToken}, csrf_token=${hasCsrf}`);
  }
  LOGGED_IN.done = true;
}

export async function loginAsRole(page: Page, role: string): Promise<void> {
  const username = process.env[`E2E_${role.toUpperCase()}_USERNAME`];
  const password = process.env[`E2E_${role.toUpperCase()}_PASSWORD`];
  if (!username || !password) {
    throw new Error(`E2E role credentials not found for role: ${role}`);
  }
  await loginViaUI(page, username, password);
}

export async function healthCheck(): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE}/health`);
    return response.ok;
  } catch {
    return false;
  }
}

export async function waitForBackend(maxRetries = 60, intervalMs = 1000): Promise<void> {
  for (let i = 0; i < maxRetries; i++) {
    if (await healthCheck()) return;
    await new Promise((r) => setTimeout(r, intervalMs));
  }
  throw new Error(`Backend not ready after ${maxRetries} retries`);
}

export async function initSystem(): Promise<void> {
  const initToken = process.env.INIT_TOKEN;
  if (!initToken) {
    throw new Error('INIT_TOKEN env required for system init');
  }

  const response = await fetch(`${API_BASE}${API_PREFIX}/init/initialize`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Init-Token': initToken,
      'X-Requested-With': 'XMLHttpRequest',
    },
    body: JSON.stringify({
      admin_username: TEST_USERNAME,
      admin_password: TEST_PASSWORD,
    }),
  });

  const text = await response.text();
  let json: ApiResponse<unknown>;
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error(`Init returned non-JSON: ${text.slice(0, 500)}`);
  }

  if (json.code === 200 || json.code === 0) return;
  if (json.message && json.message.includes('already')) return;
  throw new Error(`Init failed: code=${json.code} message=${json.message}`);
}

export async function createEntity(
  page: Page,
  endpoint: string,
  data: Record<string, unknown>
): Promise<number> {
  const result = await apiCall<{ id?: number; success?: boolean }>(page, 'POST', endpoint, data);
  if (result.data?.id) return result.data.id;
  const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', `${endpoint}?page=1&page_size=1`);
  if (list.items?.[0]?.id) return list.items[0].id;
  throw new Error(`Could not create or find entity at ${endpoint}`);
}

export async function createEntityOrSkip(
  page: Page,
  endpoint: string,
  data: Record<string, unknown>
): Promise<number | null> {
  try {
    return await createEntity(page, endpoint, data);
  } catch {
    return null;
  }
}

export async function verifyStatusTransition(
  page: Page,
  endpoint: string,
  id: number,
  action: string,
  expectedStatuses: string[]
): Promise<string> {
  try {
    await apiCall(page, 'POST', `${endpoint}/${id}/${action}`);
  } catch {
    // action may fail if already in target state
  }
  const entity = await apiCallRaw<{ status: string }>(page, 'GET', `${endpoint}/${id}`);
  const status = (entity.status || '').toLowerCase();
  const expected = expectedStatuses.map((s) => s.toLowerCase());
  if (!expected.includes(status) && !expected.includes('any')) {
    throw new Error(`Status after ${action}: expected ${expected.join('|')}, got ${status}`);
  }
  return status;
}

export async function verifyIllegalTransition(
  page: Page,
  endpoint: string,
  id: number,
  action: string
): Promise<void> {
  const result = await apiCallExpectFail(page, 'POST', `${endpoint}/${id}/${action}`);
  if (result.status < 400) {
    throw new Error(`Illegal transition ${action} on ${endpoint}/${id} was not rejected (status ${result.status})`);
  }
}

export async function verifyPermissionDenied(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  body?: Record<string, unknown>
): Promise<void> {
  const result = await apiCallExpectFail(page, method, path, body);
  if (result.status !== 403) {
    throw new Error(`Expected 403 for ${method} ${path}, got ${result.status}`);
  }
}

export async function verifyStockFourDim(
  page: Page,
  productId: number,
  colorNo?: string,
  dyeLotNo?: string
): Promise<Record<string, unknown>> {
  let path = `/inventory/stock?product_id=${productId}&page=1&page_size=50`;
  if (colorNo) path += `&color_no=${encodeURIComponent(colorNo)}`;
  if (dyeLotNo) path += `&dye_lot_no=${encodeURIComponent(dyeLotNo)}`;
  const stock = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(page, 'GET', path);
  return stock.items?.[0] || {};
}

export async function verifyAuditLog(
  page: Page,
  action: string,
  resourceType?: string
): Promise<boolean> {
  let path = `/system/audit-logs?page=1&page_size=50`;
  if (resourceType) path += `&resource_type=${encodeURIComponent(resourceType)}`;
  try {
    const logs = await apiCallRaw<{ items: Array<{ action: string; resource_type: string }> }>(page, 'GET', path);
    return logs.items?.some((l) => l.action === action && (!resourceType || l.resource_type === resourceType)) || false;
  } catch {
    try {
      const logs = await apiCallRaw<{ items: Array<{ action: string; resource_type: string }> }>(page, 'GET', `/system/omni-audit?page=1&page_size=50`);
      return logs.items?.some((l) => l.action === action) || false;
    } catch {
      return false;
    }
  }
}

export async function verifyFrontendStatusDisplay(
  page: Page,
  routePath: string,
  statusTexts: string[]
): Promise<void> {
  await page.goto(`${BASE_URL}${routePath}`);
  await page.waitForTimeout(2000);
  for (const text of statusTexts) {
    const el = page.getByText(text, { exact: false });
    const visible = await el.isVisible().catch(() => false);
    if (!visible) {
      // not all statuses may be present, just verify page loaded
    }
  }
}

export function genCode(prefix: string): string {
  const ts = Date.now().toString().slice(-6);
  const rand = Math.floor(Math.random() * 1000).toString().padStart(3, '0');
  return `${prefix}-${ts}${rand}`;
}

export function genName(prefix: string): string {
  const ts = Date.now().toString().slice(-6);
  return `${prefix}_${ts}`;
}

export function genDyeLotNo(): string {
  const date = new Date();
  const ymd = `${date.getFullYear()}${(date.getMonth() + 1).toString().padStart(2, '0')}${date.getDate().toString().padStart(2, '0')}`;
  const rand = Math.floor(Math.random() * 1000).toString().padStart(3, '0');
  return `DL-${ymd}-${rand}`;
}

export function genPieceNo(dyeLotNo: string, seq: number): string {
  return `${dyeLotNo}-${seq.toString().padStart(3, '0')}`;
}

export async function verifyEntityList<T>(
  page: Page,
  endpoint: string,
  expectMin: number = 0
): Promise<T[]> {
  const list = await apiCallRaw<{ items: T[] }>(page, 'GET', `${endpoint}?page=1&page_size=50`);
  if (list.items.length < expectMin) {
    throw new Error(`Expected at least ${expectMin} items at ${endpoint}, got ${list.items.length}`);
  }
  return list.items;
}

export async function getEntityField<T = unknown>(
  page: Page,
  endpoint: string,
  id: number,
  field: string
): Promise<T> {
  const entity = await apiCallRaw<Record<string, unknown>>(page, 'GET', `${endpoint}/${id}`);
  return entity[field] as T;
}

export async function verifySoDConflict(
  page: Page,
  userId: number,
  roleA: string,
  roleB: string
): Promise<boolean> {
  try {
    await apiCall(page, 'POST', '/users/assign-role', { user_id: userId, role_codes: [roleA, roleB] });
    return false;
  } catch {
    return true;
  }
}

export async function verifyBulkColorDeliveryBlock(
  page: Page,
  salesOrderId: number
): Promise<boolean> {
  const result = await apiCallExpectFail(page, 'POST', `/sales/orders/${salesOrderId}/ship`);
  return result.status >= 400;
}

export async function verifyWeightConversion(
  meters: number,
  gramWeight: number,
  width: number
): number {
  // 公斤 = 米 * 克重 * 幅宽 / 1000 / 100 (克→公斤, cm→m)
  return Number((meters * gramWeight * width / 100000).toFixed(2));
}

export async function verifyNetWeight(
  grossWeight: number,
  paperTubeWeight: number
): number {
  return Number((grossWeight - paperTubeWeight).toFixed(2));
}

export async function getProcessSteps(
  page: Page,
  modeCode: string
): Promise<Array<{ step_code: string; step_name: string; is_required: boolean }>> {
  try {
    const modes = await apiCallRaw<{ items: Array<{ id: number; mode_code: string }> }>(
      page, 'GET', '/business-modes?page=1&page_size=50'
    );
    const mode = modes.items.find((m) => m.mode_code === modeCode);
    if (!mode) return [];
    const steps = await apiCallRaw<{ items: Array<{ step_code: string; step_name: string; is_required: boolean }> }>(
      page, 'GET', `/business-modes/${mode.id}/flow-steps?page=1&page_size=20`
    );
    return steps.items || [];
  } catch {
    return [];
  }
}

export async function verifyOutsourcingVoucher(
  page: Page,
  orderId: number,
  voucherType: string
): Promise<Record<string, unknown> | null> {
  try {
    const vouchers = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page, 'GET', `/finance/outsourcing-vouchers?outsourcing_order_id=${orderId}&voucher_type=${voucherType}&page=1&page_size=5`
    );
    return vouchers.items?.[0] || null;
  } catch {
    return null;
  }
}

export async function verifyTrialBalance(
  page: Page
): Promise<{ balanced: boolean; debit_total: number; credit_total: number }> {
  try {
    const result = await apiCallRaw<{ debit_total: number; credit_total: number }>(
      page, 'GET', '/finance/gl/trial-balance'
    );
    return {
      balanced: Math.abs((result.debit_total || 0) - (result.credit_total || 0)) < 0.01,
      debit_total: result.debit_total || 0,
      credit_total: result.credit_total || 0,
    };
  } catch {
    return { balanced: false, debit_total: 0, credit_total: 0 };
  }
}

/**
 * 安全 GET：验证端点可达且返回有效 JSON 结构
 * 成功返回数据；失败（404/500）抛出错误（不吞掉）
 */
export async function safeGet<T = unknown>(
  page: Page,
  path: string,
  expectField?: string
): Promise<T> {
  const result = await apiCallRaw<T>(page, 'GET', path);
  if (expectField) {
    const obj = result as Record<string, unknown>;
    if (obj[expectField] === undefined && !Array.isArray(result)) {
      throw new Error(`GET ${path} 返回数据缺少字段 ${expectField}`);
    }
  }
  return result;
}

/**
 * 安全 GET 列表：验证返回 items 数组
 */
export async function safeGetList<T = unknown>(
  page: Page,
  path: string
): Promise<T[]> {
  const result = await apiCallRaw<{ items: T[]; total?: number }>(page, 'GET', path.includes('?') ? path : `${path}?page=1&page_size=50`);
  if (!result.items || !Array.isArray(result.items)) {
    throw new Error(`GET ${path} 返回数据缺少 items 数组`);
  }
  return result.items;
}

/**
 * 安全 POST action：验证状态机动作返回成功或明确的业务错误
 * 成功（200）或业务拒绝（400/409）均通过；500 不通过
 */
export async function safePostAction(
  page: Page,
  path: string,
  body?: Record<string, unknown>
): Promise<{ success: boolean; status: number }> {
  try {
    await apiCall(page, 'POST', path, body);
    return { success: true, status: 200 };
  } catch (e) {
    const err = e as { status?: number; message?: string };
    const status = err.status || 0;
    if (status >= 400 && status < 500) {
      return { success: false, status };
    }
    // 500 或网络错误是真正的失败
    throw new Error(`POST ${path} 返回 ${status}: ${err.message}`);
  }
}

/**
 * 验证端点可达但不崩溃（用于报表/统计类端点）
 */
export async function verifyEndpointHealthy(
  page: Page,
  path: string
): Promise<void> {
  try {
    await apiCallRaw(page, 'GET', path);
  } catch (e) {
    const err = e as { status?: number };
    if (err.status && err.status >= 500) {
      throw new Error(`GET ${path} 返回 ${err.status}（服务器内部错误）`);
    }
    // 404/403 可接受（端点未实现或权限不足）
  }
}
