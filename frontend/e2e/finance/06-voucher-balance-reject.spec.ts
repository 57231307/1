// 财务管理 E2E 套件 — 06 借贷不平衡凭证被拒
//
// 判族（#942 族口径）：借贷是否平衡完全由提交进来的分录金额决定，属「提交数据一致性校验」
// → **校验族**（VALIDATION_ERROR），绝不是 BAD_REQUEST 兜底族，也不是状态门。
// 后端 voucher_ops/crud.rs::validate_voucher_create_req 在 dev/prod 两分支都做
// total_debit != total_credit → `Self::balance_error(...)` = 脱敏 `AppError::validation(
// "凭证借贷不平衡：借方 X != 贷方 Y")`；文案含借/贷合计金额数字，按 utils/error.rs 安全边界
// **不外显**，出参 message 恒为常量「请求参数验证失败」（utils/messages.rs:41）。
// 该检查先于科目预检与期间锁，故对任意合法科目同样生效。
// 断真实 status(400) + 真实 code(VALIDATION_ERROR) + 脱敏常量（不是 /不平衡/ 原文）。
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

/** 建一枚真实叶子科目（凭证分录科目为外键，须存在），返回 { id, code }。
 *  balance_direction 写入方权威词表＝backend models/status/finance.rs 的 account_subject 常量（debit/credit），#198 起禁灌中文。 */
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
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {}).catch(e =>
      console.warn('[06] 会计期间初始化失败:', (e as Error).message)
    );
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
    expect(fail.message, '脱敏站点 message 应为固定验证失败常量').toBe(SANITIZED_VALIDATION);
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
