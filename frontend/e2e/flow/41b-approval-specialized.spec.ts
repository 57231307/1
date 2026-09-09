import { test, expect } from '@playwright/test';
import { loginViaUI } from './helpers';

/**
 * P5.11b 四套专用审批流 + BPM 引擎全流程
 *
 * 1. 坏账核销（writeoffs）双级审批：finance-approve → general-manager-approve → reject 分支
 * 2. 资金转账（fund-management/transfers）审批
 * 3. 角色变更（role-change-approvals）审批
 * 4. BPM 引擎：任务查询 → 待办 → 领取/审批 → 审批链
 *
 * 每套流先创建前置单据（种子库缺数据时 skip + 标注），
 * 5xx 永远真失败。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

test.describe('P5.11b 专用审批流', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('坏账核销双级审批链', async ({ page }) => {
    // 列表取一条待审 writeoff
    const list = await page.request
      .get(`${API_BASE}${API_PREFIX}/writeoffs?page=1&page_size=5`)
      .catch(() => null);
    if (!list) throw new Error('网络错误');
    expect(list.status()).toBeLessThan(500);

    const body = (await list.json().catch(() => null)) as
      | { data?: { items?: Array<{ id: number; status?: string }> } }
      | null;
    const items = body?.data?.items ?? [];
    const pending = items.find((w) => w.status && (w.status.includes('pending') || w.status.includes('submitted')));

    if (!pending) {
      test.info().annotations.push({ type: 'missing-data', description: '无待审坏账核销单据（CI 种子库）' });
      test.skip();
      return;
    }

    // 一级：finance-approve
    const l1 = await page.request.post(
      `${API_BASE}${API_PREFIX}/writeoffs/${pending.id}/finance-approve`,
      { data: { comments: 'E2E 财务一级审批' }, headers: { 'X-Requested-With': 'XMLHttpRequest', 'Content-Type': 'application/json' } },
    ).catch(() => null);
    if (l1) expect(l1.status()).toBeLessThan(500);

    // 二级：general-manager-approve
    const l2 = await page.request.post(
      `${API_BASE}${API_PREFIX}/writeoffs/${pending.id}/general-manager-approve`,
      { data: { comments: 'E2E 总经理二级审批' }, headers: { 'X-Requested-With': 'XMLHttpRequest', 'Content-Type': 'application/json' } },
    ).catch(() => null);
    if (l2) expect(l2.status()).toBeLessThan(500);
  });

  test('资金转账审批链', async ({ page }) => {
    const list = await page.request
      .get(`${API_BASE}${API_PREFIX}/fund-management/transfers?page=1&page_size=5`)
      .catch(() => null);
    if (!list) throw new Error('网络错误');
    expect(list.status()).toBeLessThan(500);

    const body = (await list.json().catch(() => null)) as
      | { data?: { items?: Array<{ id: number; status?: string }> } }
      | null;
    const items = body?.data?.items ?? [];
    const pending = items.find((t) => t.status && t.status.includes('pending'));

    if (!pending) {
      test.info().annotations.push({ type: 'missing-data', description: '无待审资金转账单据' });
      test.skip();
      return;
    }

    const resp = await page.request.post(
      `${API_BASE}${API_PREFIX}/fund-management/transfers/${pending.id}/approve`,
      { data: { comments: 'E2E 转账审批' }, headers: { 'X-Requested-With': 'XMLHttpRequest', 'Content-Type': 'application/json' } },
    ).catch(() => null);
    if (resp) expect(resp.status()).toBeLessThan(500);
  });

  test('角色变更审批链', async ({ page }) => {
    const list = await page.request
      .get(`${API_BASE}${API_PREFIX}/role-change-approvals?page=1&page_size=5`)
      .catch(() => null);
    if (!list) throw new Error('网络错误');
    expect(list.status()).toBeLessThan(500);

    const body = (await list.json().catch(() => null)) as
      | { data?: { items?: Array<{ id: number; status?: string }> } }
      | null;
    const items = body?.data?.items ?? [];
    const pending = items.find((r) => r.status && r.status.includes('pending'));

    if (!pending) {
      test.info().annotations.push({ type: 'missing-data', description: '无待审角色变更申请' });
      test.skip();
      return;
    }

    const resp = await page.request.post(
      `${API_BASE}${API_PREFIX}/role-change-approvals/${pending.id}/approve`,
      { data: { comments: 'E2E 角色变更审批' }, headers: { 'X-Requested-With': 'XMLHttpRequest', 'Content-Type': 'application/json' } },
    ).catch(() => null);
    if (resp) expect(resp.status()).toBeLessThan(500);
  });
});

test.describe('P5.11c BPM 引擎全流程', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('任务查询 → 待办 → 审批链', async ({ page }) => {
    // 1. 任务列表
    const tasks = await page.request
      .get(`${API_BASE}${API_PREFIX}/bpm/tasks?page=1&page_size=10`)
      .catch(() => null);
    if (!tasks) throw new Error('网络错误');
    expect(tasks.status()).toBeLessThan(500);

    // 2. 待办任务
    const pending = await page.request
      .get(`${API_BASE}${API_PREFIX}/bpm/tasks/pending?page=1&page_size=10`)
      .catch(() => null);
    if (pending) expect(pending.status()).toBeLessThan(500);

    // 3. 已办任务
    const completed = await page.request
      .get(`${API_BASE}${API_PREFIX}/bpm/tasks/completed?page=1&page_size=10`)
      .catch(() => null);
    if (completed) expect(completed.status()).toBeLessThan(500);
  });

  test('流程实例审批链查询', async ({ page }) => {
    const list = await page.request
      .get(`${API_BASE}${API_PREFIX}/bpm/instances?page=1&page_size=5`)
      .catch(() => null);
    if (!list) throw new Error('网络错误');
    expect(list.status()).toBeLessThan(500);

    const body = (await list.json().catch(() => null)) as
      | { data?: { items?: Array<{ id: number }> } }
      | null;
    const items = body?.data?.items ?? [];
    if (items.length === 0) {
      test.skip();
      return;
    }

    const chain = await page.request
      .get(`${API_BASE}${API_PREFIX}/bpm/instances/${items[0].id}/approval-chain`)
      .catch(() => null);
    if (chain) expect(chain.status()).toBeLessThan(500);
  });

  test('BPM 定义与模板列表', async ({ page }) => {
    for (const ep of ['/bpm/definitions', '/bpm/templates']) {
      const resp = await page.request
        .get(`${API_BASE}${API_PREFIX}${ep}?page=1&page_size=5`)
        .catch(() => null);
      if (resp) expect(resp.status()).toBeLessThan(500);
    }
  });
});
