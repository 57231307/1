// 财务管理 E2E 套件 — 07 会计科目「停用持久化回读」+「停用科目被凭证引用防护」
//
// 缺口 4：现有 finance/03 仅在 UI 上切开关、不保存、更不回读，属假绿。
// 本套件走真实后端：
//   07-01 停用持久化：PUT /subjects/{id} status='inactive' → GET 回读 status 确为 'inactive'，
//         再置回 'active' → 回读确为 'active'（双向真实落库，非内存/非 UI）。
//   07-02 停用防护：把一枚科目置为 'inactive'，用它在平衡凭证分录里 → 后端
//         voucher_ops/crud.rs::precheck_subjects_exist_txn 读出该科目状态非 active，
//         属引用主数据的**前置状态门**未满足 → HTTP 400 + code=BUSINESS_ERROR。
//         出参 message 是脱敏常量「业务处理失败」（真实 code 只进后端 WARN 日志），
//         故本用例锁 status + code + 脱敏常量本身，不锁会被脱敏的原文。
//         （对照：同科目在 active 时同一凭证可创建成功，证明拒绝确由"停用"触发。）
//   07-04 引用存在性：分录科目 id 在库里根本不存在 → 同一预检的另一分支，
//         HTTP 404 + code=NOT_FOUND + 脱敏常量「资源未找到」（与 07-02 必须可分辨）。
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

/**
 * 后端脱敏出参常量（utils/messages.rs:43 BUSINESS_PUBLIC / :42 NOT_FOUND_PUBLIC）。
 * `AppError::business` / `AppError::not_found` 走的是脱敏变体，真实文案（含科目 code / 记录 ID）
 * 只进 tracing 日志，HTTP 出参 message 恒等于下面两个字面量——所以断言锁常量本身，
 * 而不是锁「停用」这类源码不会外显的字样。
 */
const SANITIZED_BUSINESS = '业务处理失败';
const SANITIZED_NOT_FOUND = '资源未找到';
/** error.rs `AppError::error_code()` 中 NotFound 的机器码（helpers.APP_ERROR_CODES 未收录该族） */
const CODE_NOT_FOUND = 'NOT_FOUND';

// balance_direction 写入方权威词表＝backend models/status/finance.rs 的 account_subject 常量（debit/credit）， 起禁灌中文。
async function seedSubject(
  page: import('@playwright/test').Page,
  direction: 'debit' | 'credit'
): Promise<number> {
  const code = genCode('E2E-SUBJ');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
    code,
    name: `E2E 停用测试科目${direction}${code}`,
    level: 1,
    balance_direction: direction,
  });
  const id = created.data?.id;
  if (!id) throw new Error(`建科目失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/subjects/${id}`, label: 'account_subject' });
  return id;
}

/**
 * 更新科目状态。UpdateSubjectRequestDto 里 assist_xxx 与 enable_dual_unit 为非 Option bool
 * 且无 serde default，缺失会导致反序列化 422，故须显式带上全部必填布尔字段。
 */
async function setSubjectStatus(
  page: import('@playwright/test').Page,
  id: number,
  status: 'active' | 'inactive'
): Promise<void> {
  await apiCall(page, 'PUT', `/subjects/${id}`, {
    name: undefined,
    balance_direction: undefined,
    assist_customer: false,
    assist_supplier: false,
    assist_batch: false,
    assist_color_no: false,
    enable_dual_unit: false,
    status,
  });
}

async function getSubjectStatus(
  page: import('@playwright/test').Page,
  id: number
): Promise<string> {
  const s = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/subjects/${id}`);
  if (!('status' in s))
    throw new Error(`科目详情缺少 status 键，实际键=${Object.keys(s).join(',')}`);
  return String(s.status);
}

/** 凭证详情回读的精确形状（voucher 字段 flatten 至顶层，分录在 entries） */
interface VoucherDetailLite {
  id: number;
  status: string | null;
  entries: Array<{
    line_no: number;
    subject_id: number | null;
    debit: unknown;
    credit: unknown;
    summary: string | null;
  }>;
}

/** 金额按两位小数同一标度比较（分录列为 DECIMAL(15,2)，序列化常以字符串回喂），非数值形态立即抛错不放宽 */
function amountAtScale(value: unknown, label: string): string {
  const n = typeof value === 'string' || typeof value === 'number' ? Number(value) : NaN;
  if (!Number.isFinite(n)) {
    throw new Error(`${label} 金额形态非数值（契约漂移？）：${JSON.stringify(value)}`);
  }
  return n.toFixed(2);
}

/**
 * 统计当日凭证中引用指定科目 id 的凭证号列表（零变化回查用）。
 * 列表端点 GET /vouchers 的 data 为裸数组；列表行不含分录科目，
 * 故逐候选取 GET /vouchers/{id} 详情后检查 entries 的 subject_id。
 * 本例科目 id 由 genCode 唯一前缀生成，其他用例的当日凭证引用各自科目，不污染计数。
 */
async function voucherNosReferencingSubjects(
  page: import('@playwright/test').Page,
  subjectIds: number[],
  date: string
): Promise<string[]> {
  const list = await apiCallRaw<unknown>(
    page,
    'GET',
    `/vouchers?start_date=${date}&end_date=${date}&page=1&page_size=100`
  );
  if (!Array.isArray(list)) {
    throw new Error(
      `GET /vouchers 未返回数组（列表契约漂移）：${JSON.stringify(list).slice(0, 200)}`
    );
  }
  const hits: string[] = [];
  for (const row of list) {
    const id = (row as { id?: unknown }).id;
    if (typeof id !== 'number') {
      throw new Error(`凭证列表行缺少数值 id：${JSON.stringify(row).slice(0, 200)}`);
    }
    const detail = await apiCallRaw<VoucherDetailLite>(page, 'GET', `/vouchers/${id}`);
    if (!Array.isArray(detail.entries)) {
      throw new Error(
        `GET /vouchers/${id} 未返回 entries 数组（详情契约漂移）：${JSON.stringify(detail).slice(0, 200)}`
      );
    }
    if (detail.entries.some(e => e.subject_id !== null && subjectIds.includes(e.subject_id))) {
      hits.push(String(id));
    }
  }
  return hits;
}

test.describe('07 科目停用持久化与引用防护', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    // 前置条件失败判红：init_first_period 幂等（已有 OPEN 期直接返回），
    // 非 2xx 即真实环境缺陷，不允许 console.warn 后带病继续。
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {});
  });

  test('07-01 停用/启用状态持久化：API 写入后 GET 回读真实落库状态', async ({ page }) => {
    const id = await seedSubject(page, 'debit');
    expect(await getSubjectStatus(page, id), '新建科目默认应为 active').toBe('active');

    await setSubjectStatus(page, id, 'inactive');
    expect(await getSubjectStatus(page, id), '置停用后回读应为 inactive').toBe('inactive');

    await setSubjectStatus(page, id, 'active');
    expect(await getSubjectStatus(page, id), '重新启用后回读应为 active').toBe('active');
  });

  test('07-02 停用科目被凭证分录引用被拒（400 BUSINESS_ERROR + 脱敏常量文案）', async ({
    page,
  }) => {
    const disabledId = await seedSubject(page, 'debit');
    const activeId = await seedSubject(page, 'credit');
    await setSubjectStatus(page, disabledId, 'inactive');
    expect(await getSubjectStatus(page, disabledId), '前置：该科目已停用').toBe('inactive');

    const today = new Date().toISOString().slice(0, 10);
    // 借贷相等（100/100）以越过平衡校验，命中"停用科目"前置状态门预检。
    const fail = await apiCallExpectFail(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: today,
      items: [
        { subject_id: disabledId, debit: 100, credit: 0, summary: 'E2E 停用科目借方' },
        { subject_id: activeId, debit: 0, credit: 100, summary: 'E2E 贷方' },
      ],
    });

    // 状态门 → 业务族：HTTP 400 + BUSINESS_ERROR（不得再是 BAD_REQUEST 兜底族）。
    expect(fail.status, `停用科目应被拒 400，实际 status=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    // 文案含科目编码 → 脱敏出参，锁后端常量（不是"停用"字样）。
    expect(fail.message, '脱敏站点 message 应为固定业务失败常量').toBe(SANITIZED_BUSINESS);

    // 零变化回查：被拒的建单绝不能留下引用本例两枚科目的凭证（含半截分录）。
    const leaked = await voucherNosReferencingSubjects(page, [disabledId, activeId], today);
    expect(
      leaked,
      `停用科目建单被拒后不应存在引用本例科目的凭证，实际命中凭证 id：${leaked.join(',')}`
    ).toHaveLength(0);
  });

  test('07-03 对照：同一科目启用后同一凭证可建（证明 07-02 拒绝确由停用触发）', async ({
    page,
  }) => {
    const subjectId = await seedSubject(page, 'debit');
    const creditId = await seedSubject(page, 'credit');
    const today = new Date().toISOString().slice(0, 10);

    const ok = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: today,
      items: [
        { subject_id: subjectId, debit: 100, credit: 0, summary: 'E2E 借方' },
        { subject_id: creditId, debit: 0, credit: 100, summary: 'E2E 贷方' },
      ],
    });
    const voucherId = ok.data?.id;
    expect(typeof voucherId, '启用状态下平衡凭证应创建成功').toBe('number');
    if (!voucherId) throw new Error(`对照凭证未返回 id：${JSON.stringify(ok).slice(0, 200)}`);
    CLEANUP.push({ path: `/vouchers/${voucherId}`, label: 'voucher' });

    // 丙档回读：状态落库 draft、分录两行、借贷金额按同一标度断言真值，
    // 证明 07-02 的拒绝确由停用触发而非建单链路本身不通。
    const detail = await apiCallRaw<VoucherDetailLite>(page, 'GET', `/vouchers/${voucherId}`);
    expect(detail.id, '回读凭证 id 应与建单返回一致').toBe(voucherId);
    expect(detail.status, `启用科目建单应落库为 draft，实际 ${detail.status}`).toBe('draft');
    expect(detail.entries.length, '分录行数应为提交的 2 行').toBe(2);
    const debitEntry = detail.entries.find(e => e.subject_id === subjectId);
    const creditEntry = detail.entries.find(e => e.subject_id === creditId);
    if (!debitEntry || !creditEntry) {
      throw new Error(
        `回读分录缺少借/贷科目行（debit=${JSON.stringify(debitEntry)} credit=${JSON.stringify(creditEntry)}）`
      );
    }
    expect(amountAtScale(debitEntry.debit, '借方行借额'), '借方行借额应为 100').toBe('100.00');
    expect(amountAtScale(creditEntry.credit, '贷方行贷额'), '贷方行贷额应为 100').toBe('100.00');
  });

  test('07-04 引用不存在的科目建凭证被拒（404 NOT_FOUND，与 07-02 业务族可分辨）', async ({
    page,
  }) => {
    const activeId = await seedSubject(page, 'credit');
    // 不存在的科目 ID：预检读不到任何记录 → 引用存在性缺失族（NOT_FOUND）。
    // 与「记录存在但已停用」的业务族必须落到不同 status + 不同 code 上，
    // 否则两者仍被压成一族，前端无法区分「编码写错」与「科目被停用」。
    const missingId = 999_999_999;
    const today = new Date().toISOString().slice(0, 10);

    const fail = await apiCallExpectFail(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: today,
      items: [
        { subject_id: missingId, debit: 100, credit: 0, summary: 'E2E 不存在科目借方' },
        { subject_id: activeId, debit: 0, credit: 100, summary: 'E2E 贷方' },
      ],
    });

    expect(fail.status, `查无此科目应 404，实际 status=${fail.status}`).toBe(404);
    expect(failureCode(fail), `机器码应为 NOT_FOUND，实际=${fail.code}`).toBe(CODE_NOT_FOUND);
    expect(fail.message, 'NOT_FOUND 出参同样脱敏，锁固定常量').toBe(SANITIZED_NOT_FOUND);
  });
});
