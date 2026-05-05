import { Page, expect } from '@playwright/test';

const API_URL = 'http://127.0.0.1:8082/api/v1/erp';

export async function loginViaApi(page: Page): Promise<string> {
  const response = await page.request.post(`${API_URL}/auth/login`, {
    data: { username: 'admin', password: 'admin123456' },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await response.json();
  expect(body.success).toBe(true);
  const token = body.data.token;
  await page.evaluate((t) => localStorage.setItem('auth_token', t), token);
  return token;
}

export async function navigateTo(page: Page, path: string): Promise<void> {
  await page.goto(`http://127.0.0.1:3000${path}`, { waitUntil: 'networkidle', timeout: 15000 });
}

export async function waitForApp(page: Page): Promise<void> {
  await page.waitForTimeout(2000);
}

export async function apiGet(endpoint: string, token: string): Promise<any> {
  const fetch = (await import('node-fetch')).default || globalThis.fetch;
  const res = await fetch(`${API_URL}${endpoint}`, {
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
  });
  return res.json();
}

export async function apiPost(endpoint: string, token: string, data: any): Promise<any> {
  const fetch = (await import('node-fetch')).default || globalThis.fetch;
  const res = await fetch(`${API_URL}${endpoint}`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  return res.json();
}

export async function apiPut(endpoint: string, token: string, data: any): Promise<any> {
  const fetch = (await import('node-fetch')).default || globalThis.fetch;
  const res = await fetch(`${API_URL}${endpoint}`, {
    method: 'PUT',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  return res.json();
}

export async function apiDelete(endpoint: string, token: string): Promise<any> {
  const fetch = (await import('node-fetch')).default || globalThis.fetch;
  const res = await fetch(`${API_URL}${endpoint}`, {
    method: 'DELETE',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
  });
  return res.json();
}

export function generateTestId(): string {
  return `test_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`;
}

export const testResults: { module: string; test: string; status: 'pass' | 'fail' | 'skip'; error?: string; duration?: number }[] = [];

export function recordResult(module: string, test: string, status: 'pass' | 'fail' | 'skip', error?: string, duration?: number) {
  testResults.push({ module, test, status, error, duration });
}

export function getResultsSummary() {
  const total = testResults.length;
  const passed = testResults.filter(r => r.status === 'pass').length;
  const failed = testResults.filter(r => r.status === 'fail').length;
  const skipped = testResults.filter(r => r.status === 'skip').length;
  return { total, passed, failed, skipped, passRate: total > 0 ? ((passed / total) * 100).toFixed(1) : '0' };
}
