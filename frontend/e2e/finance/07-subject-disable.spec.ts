// 财务管理 E2E 套件 — 07 会计科目「停用持久化回读」+「停用科目被凭证引用防护」
//
// 任务 #942 缺口 4：现有 finance/03 仅在 UI 上切开关、不保存、更不回读，属假绿。
// 本套件走真实后端：
//   07-01 停用持久化：PUT /subjects/{id} status='inactive' → GET 回读 status 确为 'inactive'，
//         再置回 'active' → 回读确为 'active'（双向真实落库，非内存/非 UI）。
//   07-02 停用防护：把一枚科目置为 'inactive'，用它在平衡凭证分录里 → 后端
//         voucher_ops/crud.rs::precheck_subjects_exist_txn 仅认 ACTIVE 科目 → 400 "已停用"。
//         （对照：同科目在 active 时同一凭证可创建成功，证明拒绝确由"停用"触发。）
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

async function seedSubject(
  page: import('@playwright/test').Page,
  direction: '借' | '贷'
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

test.describe('07 科目停用持久化与引用防护', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {}).catch(e =>
      console.warn('[07] 会计期间初始化失败:', (e as Error).message)
    );
  });

  test('07-01 停用/启用状态持久化：API 写入后 GET 回读真实落库状态', async ({ page }) => {
    const id = await seedSubject(page, '借');
    expect(await getSubjectStatus(page, id), '新建科目默认应为 active').toBe('active');

    await setSubjectStatus(page, id, 'inactive');
    expect(await getSubjectStatus(page, id), '置停用后回读应为 inactive').toBe('inactive');

    await setSubjectStatus(page, id, 'active');
    expect(await getSubjectStatus(page, id), '重新启用后回读应为 active').toBe('active');
  });

  test('07-02 停用科目被凭证分录引用被拒（400 BAD_REQUEST + "已停用"文案）', async ({ page }) => {
    const disabledId = await seedSubject(page, '借');
    const activeId = await seedSubject(page, '贷');
    await setSubjectStatus(page, disabledId, 'inactive');
    expect(await getSubjectStatus(page, disabledId), '前置：该科目已停用').toBe('inactive');

    const today = new Date().toISOString().slice(0, 10);
    // 借贷相等（100/100）以越过平衡校验，命中"停用科目"预检。
    const fail = await apiCallExpectFail(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: today,
      items: [
        { subject_id: disabledId, debit: 100, credit: 0, summary: 'E2E 停用科目借方' },
        { subject_id: activeId, debit: 0, credit: 100, summary: 'E2E 贷方' },
      ],
    });

    expect(fail.status, `应被拒 400，实际 status=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 BAD_REQUEST，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BAD_REQUEST
    );
    expect(fail.message ?? '', '文案应指明科目已停用').toContain('停用');
  });

  test('07-03 对照：同一科目启用后同一凭证可建（证明 07-02 拒绝确由停用触发）', async ({
    page,
  }) => {
    const subjectId = await seedSubject(page, '借');
    const creditId = await seedSubject(page, '贷');
    const today = new Date().toISOString().slice(0, 10);

    const ok = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: today,
      items: [
        { subject_id: subjectId, debit: 100, credit: 0, summary: 'E2E 借方' },
        { subject_id: creditId, debit: 0, credit: 100, summary: 'E2E 贷方' },
      ],
    });
    expect(typeof ok.data?.id, '启用状态下平衡凭证应创建成功').toBe('number');
    if (ok.data?.id) CLEANUP.push({ path: `/vouchers/${ok.data.id}`, label: 'voucher' });
  });
});
