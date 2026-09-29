// 财务管理 E2E 套件 — 06 借贷不平衡凭证被拒
//
// 任务 #942 缺口 3：提交借贷不平的分录应被拒为 400 + 业务码（BAD_REQUEST），绝非 500。
// 后端 voucher_ops/crud.rs::validate_voucher_create_req 在 dev/prod 两分支都做
// total_debit != total_credit → AppError::bad_request("凭证借贷不平衡：借方 X != 贷方 Y")，
// 该检查先于科目预检与期间锁，故对任意合法科目同样生效。断真实状态码 + 机器码 + 文案。
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

/** 建一枚真实叶子科目（凭证分录科目为外键，须存在），返回 { id, code }。 */
async function seedLeafSubject(
  page: import('@playwright/test').Page,
  direction: '借' | '贷'
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
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {}).catch(e =>
      console.warn('[06] 会计期间初始化失败:', (e as Error).message)
    );
  });

  test('06-01 借 != 贷 的凭证被拒 400 + BAD_REQUEST + 真实不平衡文案（非 500）', async ({
    page,
  }) => {
    const { id: debitSubject } = await seedLeafSubject(page, '借');
    const { id: creditSubject } = await seedLeafSubject(page, '贷');
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
    expect(failureCode(fail), `机器码应为 BAD_REQUEST，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BAD_REQUEST
    );
    expect(fail.message ?? '', '文案应指明借贷不平衡').toMatch(/借贷不平衡|不平衡/);
  });

  test('06-02 借贷相等的凭证应能创建（对照，证明 06-01 的拒绝确由不平衡触发）', async ({
    page,
  }) => {
    const { id: debitSubject } = await seedLeafSubject(page, '借');
    const { id: creditSubject } = await seedLeafSubject(page, '贷');
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
    if (voucherId) {
      CLEANUP.push({ path: `/vouchers/${voucherId}`, label: 'voucher' });
      // 回读真实凭证：状态持久化为 draft，证明平平衡分录确实入库（对照 06-01 的不平被拒）。
      const detail = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/vouchers/${voucherId}`
      );
      expect(String(detail.status), '平衡凭证应落库为 draft').toBe('draft');
    }
  });
});
