// 财务管理 E2E 套件 — 06 借贷不平衡凭证被拒
//
// 判族（族口径）：借贷是否平衡完全由提交进来的分录金额决定，属「提交数据一致性校验」
// → **校验族**（VALIDATION_ERROR），绝不是 BAD_REQUEST 兜底族，也不是状态门。
// 后端 voucher_ops/crud.rs::validate_voucher_create_req 在 dev/prod 两分支都做
// total_debit != total_credit → `Self::balance_error(...)` = 脱敏 `AppError::validation(
// "凭证借贷不平衡：借方 X != 贷方 Y")`；文案含借/贷合计金额数字，按 utils/error.rs 安全边界
// **不外显**，出参 message 恒为常量「请求参数验证失败」（utils/messages.rs:41）。
// 该检查先于科目预检与期间锁，故对任意合法科目同样生效。
// 断真实 status(400) + 真实 code(VALIDATION_ERROR) + 脱敏常量（不是 /不平衡/ 原文），
// 并在被拒后回查当日凭证中不存在引用本例唯一科目的记录（零变化，含半截分录）。
// 对照例创建成功后回读详情：状态落库 draft、分录两行、借贷金额同一标度真值。
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

/** 校验族脱敏站点的固定出参文案（utils/messages.rs:41 VALIDATION_PUBLIC） */
const SANITIZED_VALIDATION = '请求参数验证失败';

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
 * 本例科目 id 由 genCode 唯一前缀生成、不与其他用例共享，其他分片新建的
 * 当日凭证引用的是各自科目，不会污染计数，天然免疫并发串扰。
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

/** 建一枚真实叶子科目（凭证分录科目为外键，须存在），返回 { id, code }。
 * balance_direction 写入方权威词表＝backend models/status/finance.rs 的 account_subject 常量（debit/credit）， 起禁灌中文。 */
async function seedLeafSubject(
  page: import('@playwright/test').Page,
  direction: 'debit' | 'credit'
): Promise<{ id: number; code: string }> {
  const code = genCode('E2E-VBS');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
    code,
    name: `E2E 平衡测试科目${direction}${code}`,
    level: 1,
    balance_direction: direction,
  });
  const id = created.data?.id;
  if (!id) throw new Error(`建科目失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/subjects/${id}`, label: 'account_subject' });
  return { id, code };
}

test.describe('06 借贷不平衡凭证被拒', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    // 前置条件失败判红：init_first_period 幂等（已有 OPEN 期直接返回），
    // 非 2xx 即真实环境缺陷，不允许 console.warn 后带病继续。
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {});
  });

  test('06-01 借 != 贷 的凭证被拒 400 + VALIDATION_ERROR + 脱敏常量文案（非 500）', async ({
    page,
  }) => {
    const { id: debitSubject } = await seedLeafSubject(page, 'debit');
    const { id: creditSubject } = await seedLeafSubject(page, 'credit');
    const today = new Date().toISOString().slice(0, 10);

    // 借 100 / 贷 99：故意不平 1 元。
    const fail = await apiCallExpectFail(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: today,
      items: [
        { subject_id: debitSubject, debit: 100, credit: 0, summary: 'E2E 借方' },
        { subject_id: creditSubject, debit: 0, credit: 99, summary: 'E2E 贷方' },
      ],
    });

    expect(fail.status, `不平衡应 400，实际 status=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 VALIDATION_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    // 后端该站点是脱敏 validation：真实金额只进日志，出参必须是固定常量（锁常量而非 /不平衡/ 原文）。
    // 逐字符等于非空常量已同时保证业务文案非空。
    expect(fail.message, '脱敏站点 message 应为固定验证失败常量').toBe(SANITIZED_VALIDATION);

    // 零变化回查：被拒的建单绝不能留下引用本例唯一科目的凭证（含半截分录）。
    // 本例科目 id 唯一（genCode 前缀），其他用例的当日凭证引用各自科目，不污染计数。
    const leaked = await voucherNosReferencingSubjects(page, [debitSubject, creditSubject], today);
    expect(
      leaked,
      `不平衡建单被拒后不应存在引用本例科目的凭证，实际命中凭证 id：${leaked.join(',')}`
    ).toHaveLength(0);
  });

  test('06-02 借贷相等的凭证应能创建（对照，证明 06-01 的拒绝确由不平衡触发）', async ({
    page,
  }) => {
    const { id: debitSubject } = await seedLeafSubject(page, 'debit');
    const { id: creditSubject } = await seedLeafSubject(page, 'credit');
    const today = new Date().toISOString().slice(0, 10);

    const ok = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: today,
      items: [
        { subject_id: debitSubject, debit: 100, credit: 0, summary: 'E2E 借方' },
        { subject_id: creditSubject, debit: 0, credit: 100, summary: 'E2E 贷方' },
      ],
    });
    const voucherId = ok.data?.id;
    expect(typeof voucherId, '平衡凭证应创建成功并返回 id').toBe('number');
    if (!voucherId) throw new Error(`对照凭证未返回 id：${JSON.stringify(ok).slice(0, 200)}`);
    CLEANUP.push({ path: `/vouchers/${voucherId}`, label: 'voucher' });

    // 丙档回读：状态词表比较点与写入方（backend models/status/finance.rs 的 voucher 模块）
    // 逐字符相同（小写 draft）；金额按两位小数同一标度断言借贷真值与分录行数。
    const detail = await apiCallRaw<VoucherDetailLite>(page, 'GET', `/vouchers/${voucherId}`);
    expect(detail.id, '回读凭证 id 应与建单返回一致').toBe(voucherId);
    expect(detail.status, `平衡凭证应落库为 draft，实际 ${detail.status}`).toBe('draft');
    expect(detail.entries.length, `分录行数应为提交的 2 行，实际 ${detail.entries.length}`).toBe(2);

    const debitEntry = detail.entries.find(e => e.subject_id === debitSubject);
    const creditEntry = detail.entries.find(e => e.subject_id === creditSubject);
    if (!debitEntry || !creditEntry) {
      throw new Error(
        `回读分录缺少借/贷科目行（debit=${JSON.stringify(debitEntry)} credit=${JSON.stringify(creditEntry)}）`
      );
    }
    expect(amountAtScale(debitEntry.debit, '借方行借额'), '借方行借额应为 100').toBe('100.00');
    expect(amountAtScale(debitEntry.credit, '借方行贷额'), '借方行贷额应为 0').toBe('0.00');
    expect(amountAtScale(creditEntry.debit, '贷方行借额'), '贷方行借额应为 0').toBe('0.00');
    expect(amountAtScale(creditEntry.credit, '贷方行贷额'), '贷方行贷额应为 100').toBe('100.00');
    expect(debitEntry.summary, '借方行摘要应落库为提交原文').toBe('E2E 借方');
    expect(creditEntry.summary, '贷方行摘要应落库为提交原文').toBe('E2E 贷方');
  });
});
