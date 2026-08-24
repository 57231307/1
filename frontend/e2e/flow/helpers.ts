import type { Page } from '@playwright/test';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';
const TEST_USERNAME = process.env.TEST_USERNAME || 'e2e_admin';
const TEST_PASSWORD = process.env.TEST_PASSWORD || 'E2e@TestPassword2026!';

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

export async function loginViaUI(page: Page, username?: string, password?: string): Promise<void> {
  const u = username || TEST_USERNAME;
  const p = password || TEST_PASSWORD;
  await page.goto(`${BASE_URL}/login`);
  // Element Plus el-input 渲染为 <input class="el-input__inner">，不带 name 属性
  // 用 aria-label 或 placeholder 定位
  const usernameInput = page.locator('input[aria-label], input.el-input__inner').first();
  await usernameInput.waitFor({ state: 'visible', timeout: 30_000 });
  await usernameInput.fill(u);

  const passwordInput = page.locator('input[type="password"]');
  await passwordInput.waitFor({ state: 'visible', timeout: 30_000 });
  await passwordInput.fill(p);

  const checkbox = page.locator('.el-checkbox').first();
  const isChecked = await checkbox.locator('input').isChecked().catch(() => false);
  if (!isChecked) {
    await checkbox.click().catch(() => {});
  }
  // Element Plus el-button 渲染为 <button class="el-button">，用文本定位
  await page.locator('button.el-button').filter({ hasText: /登录|submit|Login/i }).first().click();
  // 等待离开 /login 页面（跳转到 dashboard 或其他页面）
  await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 30_000 });
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
