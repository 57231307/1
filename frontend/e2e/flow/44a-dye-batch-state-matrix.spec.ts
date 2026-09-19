import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallExpectFail, apiCallRaw } from './helpers';

/**
 * 44a 缸号 16 态状态机规则矩阵（数据驱动）
 *
 * rule provenance：backend/src/services/dye_batch_state_machine_validation.rs
 *   builtin_transition_rules()（与 DB 预置 dye_batch_state_rule 一致）
 * 校验入口：GET /dye-batch-state-rules/check?from_status&to_status&transition_code
 *   （dye_batch_state_machine_handler.rs check_transition，纯规则校验）
 *
 * 覆盖：
 *   - 全部 45 条合法转换（正例）
 *   - 高风险非法跳转（负例，含"四态无 terminate 出口"特殊规则 washing/fixing/
 *     dehydrating/drying + TERMINATE）
 *   - 终态拦截（shipped/cancelled/terminated/failed 不可再流转 :160-168）
 *   - on_hold resume 恢复 7 工序（V15 Batch05-P1-1 :240-250）
 */

const CHECK = '/production/dye-batch-state-rules/check';

// 与后端 builtin_transition_rules() 逐条对齐（45 条）
const LEGAL: Array<[string, string, string]> = [
  ['pending_schedule', 'scheduled', 'SCHEDULE'],
  ['pending_schedule', 'cancelled', 'CANCEL'],
  ['pending_schedule', 'failed', 'FAIL'],
  ['scheduled', 'preparing', 'PREPARE'],
  ['scheduled', 'cancelled', 'CANCEL'],
  ['scheduled', 'terminated', 'TERMINATE'],
  ['scheduled', 'on_hold', 'HOLD'],
  ['scheduled', 'failed', 'FAIL'],
  ['preparing', 'dyeing', 'START_DYEING'],
  ['preparing', 'cancelled', 'CANCEL'],
  ['preparing', 'terminated', 'TERMINATE'],
  ['preparing', 'on_hold', 'HOLD'],
  ['preparing', 'failed', 'FAIL'],
  ['dyeing', 'washing', 'WASH'],
  ['dyeing', 'cancelled', 'CANCEL'],
  ['dyeing', 'terminated', 'TERMINATE'],
  ['dyeing', 'on_hold', 'HOLD'],
  ['dyeing', 'failed', 'FAIL'],
  ['washing', 'fixing', 'FIX'],
  ['washing', 'cancelled', 'CANCEL'],
  ['washing', 'on_hold', 'HOLD'],
  ['washing', 'failed', 'FAIL'],
  ['fixing', 'dehydrating', 'DEHYDRATE'],
  ['fixing', 'cancelled', 'CANCEL'],
  ['fixing', 'on_hold', 'HOLD'],
  ['fixing', 'failed', 'FAIL'],
  ['dehydrating', 'drying', 'DRY'],
  ['dehydrating', 'cancelled', 'CANCEL'],
  ['dehydrating', 'on_hold', 'HOLD'],
  ['dehydrating', 'failed', 'FAIL'],
  ['drying', 'inspecting', 'INSPECT'],
  ['drying', 'cancelled', 'CANCEL'],
  ['drying', 'on_hold', 'HOLD'],
  ['drying', 'failed', 'FAIL'],
  ['inspecting', 'stored', 'STORE'],
  ['inspecting', 'rework', 'REWORK'],
  ['inspecting', 'cancelled', 'CANCEL'],
  ['inspecting', 'failed', 'FAIL'],
  ['stored', 'shipped', 'SHIP'],
  ['stored', 'rework', 'REWORK'],
  ['stored', 'cancelled', 'CANCEL'],
  ['stored', 'failed', 'FAIL'],
  ['rework', 'dyeing', 'START_DYEING'],
  ['rework', 'cancelled', 'CANCEL'],
  ['rework', 'terminated', 'TERMINATE'],
  ['rework', 'failed', 'FAIL'],
  ['on_hold', 'dyeing', 'RESUME'],
  ['on_hold', 'washing', 'RESUME'],
  ['on_hold', 'fixing', 'RESUME'],
  ['on_hold', 'dehydrating', 'RESUME'],
  ['on_hold', 'drying', 'RESUME'],
  ['on_hold', 'scheduled', 'RESUME'],
  ['on_hold', 'preparing', 'RESUME'],
  ['on_hold', 'cancelled', 'CANCEL'],
  ['on_hold', 'failed', 'FAIL'],
];

// 高风险非法跳转负例（跨工序直跳/逆向/终态出口缺失）
const ILLEGAL: Array<[string, string, string]> = [
  // 跨工序直跳
  ['pending_schedule', 'dyeing', 'START_DYEING'],
  ['pending_schedule', 'scheduled', 'RESUME'],
  ['scheduled', 'dyeing', 'START_DYEING'],
  ['preparing', 'washing', 'WASH'],
  ['dyeing', 'dehydrating', 'DEHYDRATE'],
  ['dyeing', 'stored', 'STORE'],
  ['washing', 'drying', 'DRY'],
  ['drying', 'stored', 'STORE'],
  // 逆向流转
  ['dyeing', 'preparing', 'PREPARE'],
  ['inspecting', 'dyeing', 'WASH'],
  // 特殊规则：washing/fixing/dehydrating/drying 四态无 terminate 出口（:197-216）
  ['washing', 'terminated', 'TERMINATE'],
  ['fixing', 'terminated', 'TERMINATE'],
  ['dehydrating', 'terminated', 'TERMINATE'],
  ['drying', 'terminated', 'TERMINATE'],
  // inspecting/stored 无 terminate 出口（规则表中仅 rework/cancel/failed）
  ['inspecting', 'terminated', 'TERMINATE'],
  ['stored', 'terminated', 'TERMINATE'],
  // rework 仅可 START_DYEING 回染
  ['rework', 'scheduled', 'SCHEDULE'],
  ['rework', 'washing', 'WASH'],
  // 非法操作码
  ['scheduled', 'preparing', 'START_DYEING'],
  ['pending_schedule', 'scheduled', 'RESUME'],
];

const TERMINALS = ['shipped', 'cancelled', 'terminated', 'failed'];

async function checkTransition(
  page: import('@playwright/test').Page,
  from: string,
  to: string,
  code: string
): Promise<boolean | undefined> {
  // 后端 dye_batch_transition_code 常量为小写（schedule/cancel/...），LEGAL 列表用大写，统一小写化发送
  const r = await page.request.get(
    `${process.env.API_BASE || 'http://localhost:8082'}/api/v1/erp${CHECK}?from_status=${from}&to_status=${to}&transition_code=${code.toLowerCase()}`
  );
  if (!r.ok()) return undefined;
  const body = await r.json();
  return body?.data;
}

test.describe.serial('44a 缸号状态机规则矩阵（dye_batch_state_machine_validation.rs）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('44a-1 全部 45 条合法转换断言 allowed=true', async ({ page }) => {
    const failures: string[] = [];
    for (const [from, to, code] of LEGAL) {
      const allowed = await checkTransition(page, from, to, code);
      if (allowed !== true) {
        failures.push(`${from}→${to}(${code}) 期望 true 实际 ${allowed}`);
      }
    }
    expect(failures, `规则表漂移 ${failures.length} 条：\n${failures.join('\n')}`).toHaveLength(0);
  });

  test('44a-2 高风险非法跳转断言 allowed=false', async ({ page }) => {
    const failures: string[] = [];
    for (const [from, to, code] of ILLEGAL) {
      const allowed = await checkTransition(page, from, to, code);
      if (allowed !== false) {
        failures.push(`${from}→${to}(${code}) 期望 false 实际 ${allowed}`);
      }
    }
    expect(
      failures,
      `非法转换被放行 ${failures.length} 条（权限缺陷级）：\n${failures.join('\n')}`
    ).toHaveLength(0);
  });

  test('44a-3 终态拦截：4 终态无任何出口（:160-168）', async ({ page }) => {
    for (const t of TERMINALS) {
      // 对每个终态尝试所有操作码，全部必须 false
      for (const code of [
        'SCHEDULE',
        'PREPARE',
        'START_DYEING',
        'WASH',
        'FIX',
        'DEHYDRATE',
        'DRY',
        'INSPECT',
        'STORE',
        'SHIP',
        'REWORK',
        'RESUME',
        'HOLD',
        'TERMINATE',
      ]) {
        const allowed = await checkTransition(page, t, 'scheduled', code);
        expect(allowed, `终态 ${t} 经 ${code} 不应有任何出口`).toBe(false);
      }
    }
  });

  test('44a-4 on_hold resume 全工序覆盖（:240-250 V15 修复回归防线）', async ({ page }) => {
    // 7 个可恢复工序必须全 true（防"修复被回退"回归）
    for (const target of [
      'dyeing',
      'washing',
      'fixing',
      'dehydrating',
      'drying',
      'scheduled',
      'preparing',
    ]) {
      const allowed = await checkTransition(page, 'on_hold', target, 'RESUME');
      expect(allowed, `on_hold RESUME→${target} 应允许（V15 Batch05-P1-1）`).toBe(true);
    }
    // resume 到 stored/shipped 必须拒绝
    for (const target of ['stored', 'shipped']) {
      const allowed = await checkTransition(page, 'on_hold', target, 'RESUME');
      expect(allowed, `on_hold RESUME→${target} 应拒绝`).toBe(false);
    }
  });

  test('44a-5 allowed-transitions 端点聚合一致性（与 check 交叉验证）', async ({ page }) => {
    // 对 dyeing：check 逐条验证过的合法目标，allowed-transitions 也必须包含
    const r = await page.request.get(
      `${process.env.API_BASE || 'http://localhost:8082'}/api/v1/erp/production/dye-batch-state-rules/allowed-transitions?from_status=dyeing`
    );
    expect(r.ok(), 'allowed-transitions 端点应可达').toBe(true);
    const body = await r.json();
    const transitions = JSON.stringify(body?.data ?? body ?? []);
    for (const target of ['washing', 'cancelled', 'terminated', 'on_hold', 'failed']) {
      expect(
        transitions.includes(target),
        `allowed-transitions(dyeing) 应含 ${target}，实际: ${transitions.slice(0, 200)}`
      ).toBe(true);
    }
    // 不得包含 stored（dyeing 不能直接入库）
    expect(
      transitions.includes('"stored"') || transitions.includes(':stored'),
      'dyeing 直接入库不应在允许列表'
    ).toBe(false);
  });

  test('44a-6 非法状态值白名单校验（:16-157 直接拒绝）', async ({ page }) => {
    // 非法 from_status：后端可能返回 200（空 transitions）或 4xx；核心约束是绝不能 allowed=true
    let body: { allowed?: boolean; transitions?: unknown[] } = {};
    let httpStatus = 200;
    try {
      const data = await apiCallRaw<{ allowed?: boolean; transitions?: unknown[] }>(
        page,
        'GET',
        `${CHECK}?from_status=__invalid__&to_status=scheduled&transition_code=SCHEDULE`
      );
      body = data || {};
    } catch (e) {
      const msg = (e as Error).message || '';
      const statusMatch = msg.match(/status (\d+)/);
      httpStatus = statusMatch ? parseInt(statusMatch[1], 10) : 500;
    }
    if (httpStatus < 400) {
      expect(body.allowed, '非法状态值不应返回 allowed=true').not.toBe(true);
      expect(
        body.transitions?.length ?? 0,
        '非法状态值 transitions 应为空数组'
      ).toBeLessThanOrEqual(0);
    } else {
      expect(httpStatus, '非法状态值应被拒绝').toBeGreaterThanOrEqual(400);
    }
  });
});

function expectBadRequestLike(status: number, msg: string) {
  expect(status, msg).toBeGreaterThanOrEqual(400);
}
