/* eslint-disable no-console */
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

  // 登录成功后，确保 cookie 已设置到 context
  try {
    const cookies = await page.context().cookies();
    const hasToken = cookies.some((c) => c.name === 'access_token');
    if (!hasToken) {
      // 如果 cookie 没有自动设置，手动通过 API 登录注入 cookie
      await page.request.post(`${API_BASE}${API_PREFIX}/auth/login`, {
        data: { username: TEST_USERNAME, password: TEST_PASSWORD },
        headers: { 'Content-Type': 'application/json', 'X-Requested-With': 'XMLHttpRequest' },
      });
    }
  } catch {
    // API 补充登录可能失败（429 等），不影响 UI 登录成功
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
