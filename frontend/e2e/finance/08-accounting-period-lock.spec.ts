// 财务管理 E2E 套件 — 08 会计期间锁定（关账后录入被拒 + business_displayable 真实文案）
//
// 缺口 5。核心真相（务必理解，否则测例会假绿或跑偏）
//   - voucher_ops/crud.rs::validate_voucher_create_req 的"期间锁"仅在 is_production() 下执行；
//     CI e2e 以 APP_ENV=development 运行（见 ci-cd.yml E2E job），故【凭证创建】不检锁。
//   - 但 voucher_ops/workflow.rs::post / unpost、ar_ops/collection.rs::create_payment
//     调用的是 AccountingPeriodService::check_date_locked(_txn) —— 【无环境开关】，任何模式都生效。
//   因此本套件用"关账后向该期过账凭证 / 收款"来验证锁，而非"创建凭证"；两者共用同一 check_date_locked。
//   所有锁拒绝走 AppError::business_displayable → HTTP 400 + 机器码 BUSINESS_ERROR + 可外显真实文案。
// 隔离：使用远未来专用月份（2031-06 / 2031-10 / 2035-03），不触碰其他分片共用的当期，避免串扰。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
  genCode,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

interface PeriodRow {
  id: number;
  year: number;
  period: number;
  status: string;
}

/** 列出全部会计期间（GET /finance/accounting-periods，data 为裸数组）。 */
async function listPeriods(page: import('@playwright/test').Page): Promise<PeriodRow[]> {
  const arr = await apiCallRaw<unknown>(page, 'GET', '/finance/accounting-periods');
  if (!Array.isArray(arr)) throw new Error(`期间列表非数组：${JSON.stringify(arr).slice(0, 200)}`);
  return arr as PeriodRow[];
}

/**
 * 确保 (year,period) 期间存在且处于 CLOSED，返回其 id。
 * - 不存在 → POST /finance/accounting-periods 创建（OPEN）→ close。
 * - 存在且 OPEN → close。
 * - 存在且 CLOSED → 直接复用（重跑幂等：不重复关账，避免残留未过账凭证阻塞二次 close）。
 * 关账针对"空月"（无未过账凭证）必然成功。
 */
async function ensurePeriodClosed(
  page: import('@playwright/test').Page,
  year: number,
  period: number
): Promise<number> {
  let rows = await listPeriods(page);
  let found = rows.find(p => p.year === year && p.period === period);
  if (!found) {
    await apiCall(page, 'POST', '/finance/accounting-periods', { year, period });
    rows = await listPeriods(page);
    found = rows.find(p => p.year === year && p.period === period);
  }
  if (!found) throw new Error(`无法定位/创建期间 ${year}-${period}`);
  if (found.status === 'OPEN') {
    await apiCall(page, 'POST', `/finance/accounting-periods/${found.id}/close`);
    rows = await listPeriods(page);
    found = rows.find(p => p.id === found?.id);
  }
  expect(found?.status, `期间 ${year}-${period} 应为 CLOSED`).toBe('CLOSED');
  return found!.id;
}

// balance_direction 写入方权威词表＝backend models/status/finance.rs 的 account_subject 常量（debit/credit）， 起禁灌中文。
async function seedLeafSubject(
  page: import('@playwright/test').Page,
  direction: 'debit' | 'credit'
): Promise<number> {
  const code = genCode('E2E-PLK');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
    code,
    name: `E2E 期间锁科目${direction}${code}`,
    level: 1,
    balance_direction: direction,
  });
  const id = created.data?.id;
  if (!id) throw new Error(`建科目失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/subjects/${id}`, label: 'account_subject' });
  return id;
}

test.describe('08 会计期间锁定', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('08-01 关账后向该期过账凭证被拒（400 BUSINESS_ERROR + "已结账"真实文案）', async ({
    page,
  }) => {
    // 远未来专用月，空月关账必成。
    const periodId = await ensurePeriodClosed(page, 2031, 6);

    // 建平衡凭证（dev 下创建不检锁），逐级推进到 reviewed。
    const debit = await seedLeafSubject(page, 'debit');
    const credit = await seedLeafSubject(page, 'credit');
    const created = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: '2031-06-15',
      items: [
        { subject_id: debit, debit: 500, credit: 0, summary: 'E2E 借' },
        { subject_id: credit, debit: 0, credit: 500, summary: 'E2E 贷' },
      ],
    });
    const voucherId = created.data?.id;
    expect(typeof voucherId, '平衡凭证应先创建成功').toBe('number');
    if (voucherId) CLEANUP.push({ path: `/vouchers/${voucherId}`, label: 'voucher' });
    await apiCall(page, 'POST', `/vouchers/${voucherId}/submit`);
    await apiCall(page, 'POST', `/vouchers/${voucherId}/review`);

    // 过账触发无环境开关的 check_date_locked → 该期已 CLOSED → 400。
    const fail = await apiCallExpectFail(page, 'POST', `/vouchers/${voucherId}/post`);
    expect(fail.status, `过账应被锁拒 400，实际=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(fail.message ?? '', '文案应为"已结账"锁定语义').toContain('已结账');
    expect(periodId, '前置：期间 id 有效').toBeGreaterThan(0);
  });

  test('08-02 关账后向该期收款被拒 + 反结账持久化回读', async ({ page }) => {
    const periodId = await ensurePeriodClosed(page, 2031, 10);

    // AR 收款走同一 check_date_locked_txn（无环境开关）。期间锁先于客户校验执行，
    // 故 customer_id 用占位 1 也不影响锁拒绝断言。
    const fail = await apiCallExpectFail(page, 'POST', '/ar/payments', {
      customer_id: 1,
      amount: 100,
      payment_method: '银行转账',
      payment_date: '2031-10-20',
    });
    expect(fail.status, `收款应被锁拒 400，实际=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(fail.message ?? '', '文案应为"已结账"锁定语义').toContain('已结账');

    // 反结账：CLOSED → OPEN，并回读持久化状态。
    await apiCall(page, 'POST', `/finance/accounting-periods/${periodId}/reopen`, {
      reason: 'E2E 反结账持久化验证',
    });
    const rows = await listPeriods(page);
    const reopened = rows.find(p => p.id === periodId);
    expect(reopened?.status, '反结账后期间应为 OPEN').toBe('OPEN');
  });

  test('08-03 未设置期间的日期录入被拒（business_displayable "不在任何已设置的会计期间内"文案）', async ({
    page,
  }) => {
    // 2035-03 从不建期，命中 check_date_locked 的"无期间"分支。
    const rows = await listPeriods(page);
    expect(
      rows.some(p => p.year === 2035 && p.period === 3),
      '前置：2035-03 不应已存在期间'
    ).toBe(false);

    const fail = await apiCallExpectFail(page, 'POST', '/ar/payments', {
      customer_id: 1,
      amount: 100,
      payment_method: '银行转账',
      payment_date: '2035-03-10',
    });
    expect(fail.status, `无期间录入应被拒 400，实际=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(fail.message ?? '', '文案应为"不在任何已设置的会计期间内"').toContain('不在任何');
  });
});
