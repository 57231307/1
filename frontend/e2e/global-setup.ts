import { request } from '@playwright/test';
import { writeFileSync, mkdirSync } from 'fs';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
const TEST_USERNAME = process.env.TEST_USERNAME || 'e2e_admin';
const TEST_PASSWORD = process.env.TEST_PASSWORD || 'E2e@TestPassword2026!';
const STORAGE_STATE_PATH = 'e2e/.auth/storage-state.json';

export default async function globalSetup() {
  const ctx = await request.newContext({
    baseURL: API_BASE,
    extraHTTPHeaders: {
      'Content-Type': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    },
  });

  const resp = await ctx.post(`${API_PREFIX}/auth/login`, {
    data: { username: TEST_USERNAME, password: TEST_PASSWORD },
  });

  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`globalSetup 登录失败: HTTP ${resp.status()} ${body}`);
  }

  const cookies = await ctx.storageState();
  const accessCookie = cookies.cookies.find(c => c.name === 'access_token');
  if (!accessCookie) {
    throw new Error('globalSetup 登录后未获得 access_token cookie');
  }

  mkdirSync('e2e/.auth', { recursive: true });
  writeFileSync(STORAGE_STATE_PATH, JSON.stringify(cookies, null, 2));
  await ctx.dispose();
}
