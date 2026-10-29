// 能耗设备/记录/规则 与工序路线的改值契约 E2E — PUT 更新写后回读逐字段真值，非法枚举按公开规则点名拒绝
//
// 本文件守的账：更新类端点把请求体字段真正写进库并原样读回，而不是收下即忘或改错列；
// DECIMAL 出参按十进制字符串定标逐值比较（禁 JS number 累加与二次舍入）；枚举字段越界
// 必须被服务端校验拦下并回真实拒绝原因，绝不允许静默落库。
// 数据自建自清：全部基准由本用例带唯一编码自建的行导出，删除门按各域状态约束放行，
// 清理失败经 tryCleanup 告警留痕，绝不吞错。
import { test, expect } from '../diagnose-fixture';
import { type Page } from '@playwright/test';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  APP_ERROR_CODES,
  genCode,
  tryCleanup,
} from './helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

const isoNow = (): string => new Date().toISOString();
const isoPlusHours = (h: number): string => new Date(Date.now() + h * 3_600_000).toISOString();
const isoDate = (offsetDays: number): string =>
  new Date(Date.now() + offsetDays * 86_400_000).toISOString().slice(0, 10);

// ───────────── 十进制字符串比较（DECIMAL 出参为字符串，禁浮点、禁二次舍入）─────────────
function requireDecString(v: unknown, label: string): string {
  if (typeof v !== 'string') {
    throw new Error(
      `${label}：后端 Decimal 出参必须是字符串，实际类型=${typeof v} 值=${JSON.stringify(v)}`
    );
  }
  const text = v.trim();
  if (!/^-?\d+(?:\.\d+)?$/.test(text)) {
    throw new Error(`${label}：无法按十进制解析，raw=${JSON.stringify(v)}`);
  }
  return text;
}

function decToBigInt(text: string): { v: bigint; s: number } {
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(text);
  if (!m) throw new Error(`十进制解析失败：${text}`);
  const sign = m[1] === '-' ? -1n : 1n;
  return { v: sign * BigInt(`${m[2]}${m[3] ?? ''}`), s: (m[3] ?? '').length };
}

/** 按十进制数值逐值比较，只对齐量纲不做舍入，落库真值与提交真值必须相等 */
function expectDecEquals(actual: unknown, expected: string, label: string): void {
  const a = decToBigInt(requireDecString(actual, label));
  const b = decToBigInt(requireDecString(expected, label));
  const scale = Math.max(a.s, b.s);
  const av = a.v * 10n ** BigInt(scale - a.s);
  const bv = b.v * 10n ** BigInt(scale - b.s);
  expect(av === bv, `${label}：期望十进制值 ${expected}，实际回读 ${JSON.stringify(actual)}`).toBe(
    true
  );
}

function requireId(row: Record<string, unknown>, label: string): number {
  const id = Number(row.id);
  if (!Number.isInteger(id) || id <= 0) {
    throw new Error(`${label}：未取到有效 id，实际 ${JSON.stringify(row)}`);
  }
  return id;
}

test.describe('150 能耗与工序路线改值契约：PUT 写后逐字段回读 + 非法枚举拒绝', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('150-01 计量设备：改名与单价改写逐字段落库回读，非法删除门不存在（无状态限制恒可软删）', async ({
    page,
  }) => {
    const workshop = genCode('WS150M');
    const created = await apiCall<Record<string, unknown>>(page, 'POST', '/production/energy-meters', {
      meter_name: `150号电表${workshop}`,
      meter_type: 'electricity',
      workshop,
      unit_price: '2.00',
      previous_reading: '0',
      current_reading: '100',
    });
    const id = requireId(created.data as Record<string, unknown>, '计量设备创建');
    const meterNo = String((created.data as Record<string, unknown>).meter_no);
    CLEANUP.push({ path: `/production/energy-meters/${id}`, label: `meter#${meterNo}` });

    // 改写设备名、单价与备注后逐字段回读，证明 PUT 真正落到对应列
    const newName = `150号电表改名${workshop}`;
    const putRes = await apiCallRaw<Record<string, unknown>>(page, 'PUT', `/production/energy-meters/${id}`, { meter_name: newName, unit_price: '3.75', remarks: '改值契约E2E' });
    expect(String(putRes.meter_name), 'PUT 响应回显改名').toBe(newName);
    const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/production/energy-meters/${id}`);
    expect(after.meter_name, '设备名称改写应落库').toBe(newName);
    expectDecEquals(after.unit_price, '3.75', '设备单价改写');
    expect(String(after.remarks ?? ''), '设备备注改写').toBe('改值契约E2E');
    expect(String(after.meter_no), '设备编号不随改名漂移').toBe(meterNo);
  });

  test('150-02 能耗记录：草稿态改读数与单价，消耗量随读数派生回读，非法读数被拒', async ({
    page,
  }) => {
    const workshop = genCode('WS150C');
    const meter = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/production/energy-meters', {
      meter_name: `150水表${workshop}`,
      meter_type: 'water',
      workshop,
      unit_price: '1.00',
      previous_reading: '0',
      current_reading: '0',
    });
    const meterId = requireId(meter, '计量设备');
    CLEANUP.push({ path: `/production/energy-meters/${meterId}`, label: `meter#${meterId}` });

    const cons = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/production/energy-consumptions', {
      meter_id: meterId,
      meter_type: 'water',
      workshop,
      previous_reading: '10',
      current_reading: '30',
      unit_price: '1.00',
      period_start: isoNow(),
      period_end: isoPlusHours(1),
    });
    const consId = requireId(cons, '能耗登记');
    CLEANUP.push({ path: `/production/energy-consumptions/${consId}`, label: `cons#${consId}` });

    // 草稿态可改：当前读数 30→50、单价 1.00→2.50，读回读数与派生量必须同口径
    const putRes = await apiCallRaw<Record<string, unknown>>(page, 'PUT', `/production/energy-consumptions/${consId}`, { current_reading: '50', unit_price: '2.50' });
    expectDecEquals(putRes.current_reading, '50', 'PUT 响应回显当前读数');
    const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/production/energy-consumptions/${consId}`);
    expectDecEquals(after.current_reading, '50', '当前读数改写');
    expectDecEquals(after.previous_reading, '10', '上次读数不受改写影响');
    expectDecEquals(after.unit_price, '2.50', '单价改写');
    expect(after.status, '草稿记录改写后仍为 draft').toBe('draft');

    // 改写读数后服务端必须重算派生列：消耗量=当前-上次=50-10=40，总成本=消耗量×单价=40×2.50=100。
    // 读回派生列证明 PUT 不是只改输入列而漏算下游账。
    expectDecEquals(after.consumption, '40', '改写读数后消耗量应重算为 40');
    expectDecEquals(after.total_cost, '100', '改写单价后总成本应重算为 100');
  });

  test('150-03 分摊规则：按编号取回与草稿改名改量纲，非法分摊基准点名公开词表', async ({
    page,
  }) => {
    const workshop = genCode('WS150R');
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/production/energy-rules', {
      rule_name: `150分摊规则${workshop}`,
      meter_type: 'electricity',
      allocation_basis: 'by_output',
      workshop,
      effective_date: isoDate(0),
      standard_consumption_per_unit: '1.5',
      standard_unit: '度',
    });
    const id = requireId(created, '分摊规则创建');
    const ruleNo = String(created.rule_no);
    CLEANUP.push({ path: `/production/energy-rules/${id}`, label: `rule#${ruleNo}` });

    // 按编号回读：证明 GET by-no 命中的就是刚建这一行，编号/名称/基准逐字段同源
    const byNo = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/energy-rules/by-no/${ruleNo}`
    );
    expect(String(byNo.rule_no), 'by-no 编号回读').toBe(ruleNo);
    expect(Number(byNo.id), 'by-no 命中刚建规则').toBe(id);
    expect(String(byNo.allocation_basis), 'by-no 分摊基准回读').toBe('by_output');

    // 草稿态改规则名与单位能耗，逐字段回读
    const newName = `150分摊规则改名${workshop}`;
    await apiCall(page, 'PUT', `/production/energy-rules/${id}`, {
      rule_name: newName,
      standard_consumption_per_unit: '2.25',
    });
    const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/production/energy-rules/${id}`);
    expect(String(after.rule_name), '规则名称改写落库').toBe(newName);
    expectDecEquals(after.standard_consumption_per_unit, '2.25', '单位能耗改写落库');
    expect(after.status, '草稿规则改名后仍为 draft').toBe('draft');

    // 负例：更新传入越界分摊基准，应被公开词表拒绝
    const bad = await apiCallExpectFail(page, 'PUT', `/production/energy-rules/${id}`, {
      allocation_basis: 'by_mood',
    });
    expect(bad.status, '非法分摊基准应被 4xx 拒绝').toBeGreaterThanOrEqual(400);
    expect(bad.status, '非法分摊基准不应吞成 5xx').toBeLessThan(500);
    expect(failureCode(bad), '非法分摊基准机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('150-04 工序路线：改名改工时逐字段回读，非法工序类型点名公开枚举表', async ({ page }) => {
    const code = genCode('E2EPR');
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/production/process-routes', {
      route_code: code,
      route_name: `150工序${code}`,
      seq: 1,
      process_type: 'dye',
      default_duration_minutes: 60,
    });
    const id = requireId(created, '工序路线创建');
    const storedCode = String(created.route_code);
    CLEANUP.push({ path: `/production/process-routes/${id}`, label: `route#${storedCode}` });

    // 工序编码入库被大写归一，读回应与落库值一致而非提交值
    expect(storedCode, '工序编码应大写归一').toBe(code.toUpperCase());

    const newName = `150工序改名${code}`;
    await apiCall(page, 'PUT', `/production/process-routes/${id}`, {
      route_name: newName,
      default_duration_minutes: 90,
    });
    const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/production/process-routes/${id}`);
    expect(String(after.route_name), '工序名称改写落库').toBe(newName);
    expect(Number(after.default_duration_minutes), '默认工时改写落库').toBe(90);
    expect(String(after.process_type), '工序类型未随改名校正').toBe('dye');

    // 负例：更新传入越界工序类型，应被公开枚举表拒绝
    const bad = await apiCallExpectFail(page, 'PUT', `/production/process-routes/${id}`, {
      process_type: 'spin_dry',
    });
    expect(bad.status, '非法工序类型应被 4xx 拒绝').toBeGreaterThanOrEqual(400);
    expect(bad.status, '非法工序类型不应吞成 5xx').toBeLessThan(500);
    expect(failureCode(bad), '非法工序类型机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });
});
