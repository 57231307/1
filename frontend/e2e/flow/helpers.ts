import type { Page, APIRequestContext } from '@playwright/test';

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

export interface LoginResult {
  user: Record<string, unknown>;
  permissions: string[];
  cookies: Record<string, string>;
}

export interface EntityContext {
  departmentId?: number;
  roleId?: number;
  warehouseId?: number;
  productCategoryId?: number;
  productIds: number[];
  supplierId?: number;
  customerId?: number;
  accountSubjectIds: number[];
  accountingPeriodId?: number;
  colorCardId?: number;
  processRouteId?: number;
  chemicalId?: number;
  bomId?: number;
  purchaseOrderId?: number;
  salesOrderId?: number;
  quotationId?: number;
  productionOrderId?: number;
  fixedAssetId?: number;
  budgetId?: number;
}

const ctx: EntityContext = {
  productIds: [],
  accountSubjectIds: [],
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
    throw new Error(`API ${method} ${path} returned non-JSON: ${text.slice(0, 500)}`);
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

export async function loginViaUI(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/login`);
  await page.waitForSelector('input[name="username"]', { state: 'visible', timeout: 30_000 });
  await page.fill('input[name="username"]', TEST_USERNAME);
  await page.fill('input[name="password"]', TEST_PASSWORD);
  const checkbox = page.locator('.el-checkbox').first();
  const isChecked = await checkbox.locator('input').isChecked().catch(() => false);
  if (!isChecked) {
    await checkbox.click().catch(() => {});
  }
  await page.click('button[type="submit"]');
  await page.waitForURL(/\/(dashboard|$)/, { timeout: 30_000 });
}

export async function loginViaAPI(request: APIRequestContext): Promise<LoginResult> {
  const response = await request.post(`${API_BASE}${API_PREFIX}/auth/login`, {
    data: {
      username: TEST_USERNAME,
      password: TEST_PASSWORD,
    },
    headers: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });

  if (!response.ok()) {
    throw new Error(`Login failed: ${response.status()} ${response.statusText()}`);
  }

  const json: ApiResponse<LoginResult> = await response.json();
  if (json.code !== 200 && json.code !== 0) {
    throw new Error(`Login failed: ${json.message}`);
  }

  const cookies = response.headers()['set-cookie'] || '';
  return json.data;
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
  if (!result.data?.id) {
    throw new Error(`Create ${endpoint} did not return id: ${JSON.stringify(result)}`);
  }
  return result.data.id;
}

export async function verifyEntity<T>(
  page: Page,
  endpoint: string
): Promise<T> {
  return apiCallRaw<T>(page, 'GET', endpoint);
}

export async function waitForApiResponse(
  page: Page,
  urlPattern: string,
  timeout = 30_000
): Promise<unknown> {
  const startTime = Date.now();
  while (Date.now() - startTime < timeout) {
    try {
      const response = await page.evaluate(async (pattern) => {
        const res = await fetch(pattern);
        return res.json();
      }, urlPattern);
      if (response) return response;
    } catch {
      // retry
    }
    await page.waitForTimeout(500);
  }
  throw new Error(`API ${urlPattern} did not respond within ${timeout}ms`);
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
