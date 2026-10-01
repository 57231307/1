// 财务域全流程契约级 E2E — 12 凭证 + 会计科目 + 会计期间
//
// 定位：与 finance/01（凭证创建→过账 UI+API）、finance/02（审批工作流 happy path）、
// finance/06（借贷不平拒绝）、finance/07（停用持久化+引用防护）、finance/08（关账后过账/收款锁）
// 互补，本文件补未覆盖的契约面（用户缺陷①②靶心）：
//   ①凭证/科目全键建单 → GET 详情/列表逐字段回读（含分录双命名键、源单号、批色号）；
//   ②期间重复建单等创建链 4xx 的 message 必须是具体原因（不得为脱敏常量）。
// 契约真值源：
//   CreateVoucherRequestDto handlers/voucher_handler.rs:44-55；VoucherItemDto:64-95
//     （alias 双命名：account_subject_id/debit_amount/credit_amount/description）
//   详情出参 voucher Model flatten + entries 双命名 VoucherItemResponseDto（:167-196）
//   列表出参 **裸数组** Vec<voucher::Model>（list_vouchers handler:181-207，非 PaginatedResponse，
//     按源码显式断真实形状）
//   凭证状态词表小写 draft/submitted/reviewed/posted（models/status/finance.rs:62-74）；
//     门 services/voucher_ops/workflow.rs:45(submit)/90(review)/133(post)/205(unpost) 均
//     business_displayable ⇒ 可断具体文案；update/delete 仅 draft（voucher_ops/crud.rs:423-427/530-533）
//   CreateSubjectRequestDto handlers/account_subject_handler.rs:35-53（assist_* 为必填 bool）；
//     重复编码 business_displayable（services/account_subject_service.rs:83-88）；
//     删除门 :280-329（子科目/凭证引用/余额记录 均 AppError::business→BUSINESS_ERROR，
//     引用条数文案按 error.rs 安全边界脱敏）
//   会计期间 handlers/missing_handlers.rs:103-164（CreateAccountingPeriodPayload year+period，
//     period 1-12 校验；重复 business **脱敏** ⇒ 该断言预期判红并报 file:line）；
//     状态词表 OPEN/CLOSED（models/status/finance.rs:77-83）；
//     update 仅 OPEN/CLOSING（missing_handlers.rs:194-200）；close/reopen
//     handlers/accounting_period_handler.rs:36-71（reopen 需 {reason}）
// 诚实标注：
//   - 关账后录入拒绝链已由 finance/08 覆盖，本文件不重写；
//   - 凭证打印 /vouchers/{id}/print 返回 docx 二进制由 UI 点验另属 print 套件。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

const DESENSITIZED_CONSTANTS = ['请求参数验证失败', '业务处理失败'];

function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
): void {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(obj[key], `${label}：键 "${key}" 期望=${JSON.stringify(expected)}`).toEqual(expected);
}

function expectDecimal(obj: Record<string, unknown>, key: string, expected: number, label: string) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：缺金额键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：${key} 非数字，raw=${JSON.stringify(obj[key])}`);
  }
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(0.005);
}

function requireId(obj: Record<string, unknown>, label: string): number {
  const id = Number(obj.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：无有效 id，实际键=${Object.keys(obj).join(',')}`);
  }
  return id;
}

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

/** 自建两枚一级科目（借/贷），返回 [借科目, 贷科目]。DTO 全集见 account_subject_handler.rs:35-53。 */
async function seedSubjects(
  page: import('@playwright/test').Page,
  tag: string
): Promise<[Record<string, unknown>, Record<string, unknown>]> {
  const out: Record<string, unknown>[] = [];
  for (const dir of ['借', '贷']) {
    const code = genCode(`S${dir === '借' ? 'D' : 'C'}`);
    const s = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/subjects', {
      code,
      name: `E2E${tag}科目${code}`,
      level: 1,
      parent_id: null,
      balance_direction: dir,
      assist_customer: false,
      assist_supplier: false,
      assist_batch: false,
      assist_color_no: false,
      enable_dual_unit: false,
    });
    if (s.id !== undefined) {
      CLEANUP.push({ path: `/subjects/${requireId(s, '建科目')}`, label: 'subject' });
    }
    out.push(s);
  }
  return [out[0], out[1]];
}

/** 建平衡凭证（借贷各一），items 用 finance.ts 命名（subject_id/debit/credit/summary）。 */
async function createBalancedVoucher(
  page: import('@playwright/test').Page,
  debitSubject: number,
  creditSubject: number,
  amount: number,
  overrides: Record<string, unknown> = {}
): Promise<Record<string, unknown>> {
  const v = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/vouchers', {
    voucher_type: '记账凭证',
    voucher_date: todayStr(),
    source_type: 'MANUAL',
    source_module: 'E2E',
    source_bill_no: genCode('VBLSRC'),
    batch_no: 'B-E2E-V',
    color_no: 'C-E2E-V',
    items: [
      { line_no: 1, subject_id: debitSubject, debit: amount, credit: 0, summary: 'E2E借方摘要' },
      { line_no: 2, subject_id: creditSubject, debit: 0, credit: amount, summary: 'E2E贷方摘要' },
    ],
    ...overrides,
  });
  CLEANUP.push({ path: `/vouchers/${requireId(v, '建凭证')}`, label: 'voucher' });
  return v;
}

test.describe('12 凭证/科目/期间全流程契约链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('12-01 凭证全键建单 → 详情 flatten+entries 双命名逐字段回读；列表按裸数组真实形状命中', async ({
    page,
  }) => {
    const [dSub, cSub] = await seedSubjects(page, '凭证');
    const v = await createBalancedVoucher(
      page,
      requireId(dSub, '借科目'),
      requireId(cSub, '贷科目'),
      1234.56
    );
    const id = requireId(v, '建凭证');

    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    // voucher Model flatten 至顶层（VoucherDetailResponse handler:167-172）
    expectKeyValue(detail, 'voucher_type', '记账凭证', '凭证详情');
    expectKeyValue(detail, 'voucher_date', todayStr(), '凭证详情');
    expectKeyValue(detail, 'status', 'draft', '凭证初始（finance.rs:64）');
    expectKeyValue(detail, 'source_type', 'MANUAL', '凭证详情');
    expectKeyValue(detail, 'source_module', 'E2E', '凭证详情');
    expectKeyValue(detail, 'source_bill_no', v.source_bill_no, '凭证详情(源单号回显)');
    expectKeyValue(detail, 'batch_no', 'B-E2E-V', '凭证详情');
    expectKeyValue(detail, 'color_no', 'C-E2E-V', '凭证详情');
    expect(
      typeof detail.voucher_no === 'string' && (detail.voucher_no as string).length > 0,
      `voucher_no 应后端生成，实际=${JSON.stringify(detail.voucher_no)}`
    ).toBe(true);

    const entries = detail.entries as Array<Record<string, unknown>>;
    expect(Array.isArray(entries), '凭证详情 entries 应为数组').toBe(true);
    expect(entries.length, '分录应 2 条').toBe(2);
    const e1 = entries.find(e => Number(e.line_no) === 1);
    const e2 = entries.find(e => Number(e.line_no) === 2);
    if (!e1 || !e2) throw new Error('分录 line_no 1/2 缺失');
    // 双命名契约（json_helpers 式源码明确冗余键，显式断两侧同值，非形状探测）
    expectKeyValue(e1, 'subject_id', requireId(dSub, '借科目'), '分录1');
    expectKeyValue(e1, 'account_subject_id', e1.subject_id, '分录1 双命名');
    expectDecimal(e1, 'debit', 1234.56, '分录1');
    expectDecimal(e1, 'debit_amount', 1234.56, '分录1 双命名');
    expectKeyValue(e1, 'summary', 'E2E借方摘要', '分录1');
    expectKeyValue(e1, 'description', 'E2E借方摘要', '分录1 双命名');
    expectKeyValue(e1, 'subject_name', dSub.name, '分录1 科目名反查');
    expectDecimal(e2, 'credit', 1234.56, '分录2');

    // 列表出参 = 裸数组（handler:181-207 非 PaginatedResponse，按源码断）
    const listData = await apiCallRaw<unknown>(
      page,
      'GET',
      `/vouchers?voucher_type=记账凭证&page=1&page_size=100`
    );
    if (!Array.isArray(listData)) {
      throw new Error(
        `GET /vouchers data 应为裸数组（handler 源码），实际=${JSON.stringify(listData).slice(0, 200)}`
      );
    }
    const row = (listData as Array<Record<string, unknown>>).find(r => Number(r.id) === id);
    if (!row) throw new Error(`凭证列表未含自建 id=${id}`);
    expectKeyValue(row, 'voucher_no', detail.voucher_no, '列表行');
    expectKeyValue(row, 'batch_no', 'B-E2E-V', '列表行');
  });

  test('12-02 凭证 PUT：改类型+items 整体替换回读、省略 voucher_date 保持；submitted 后改被拒文案外显', async ({
    page,
  }) => {
    const [dSub, cSub] = await seedSubjects(page, '改凭证');
    const v = await createBalancedVoucher(page, requireId(dSub, '借'), requireId(cSub, '贷'), 500);
    const id = requireId(v, '建凭证');
    const origDate = (await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`))
      .voucher_date;

    await apiCall(page, 'PUT', `/vouchers/${id}`, {
      voucher_type: '收款凭证',
      items: [
        {
          line_no: 1,
          subject_id: requireId(dSub, '借'),
          debit: 700,
          credit: 0,
          summary: '替换后借',
        },
        {
          line_no: 2,
          subject_id: requireId(cSub, '贷'),
          debit: 0,
          credit: 700,
          summary: '替换后贷',
        },
      ],
      // voucher_date 省略 → UpdateVoucherRequestDto 单层 Option（handler:422-426）保持原值
    });
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d1, 'voucher_type', '收款凭证', '凭证改后');
    expectKeyValue(d1, 'voucher_date', origDate, '省略键应保持原日期');
    const entries = d1.entries as Array<Record<string, unknown>>;
    expect(entries.length, 'items 应整体替换为 2 条').toBe(2);
    expectDecimal(entries[0], 'debit', 700, '替换后分录1');
    expectKeyValue(entries[0], 'summary', '替换后借', '替换后分录1');

    // submitted 后 PUT 被拒（仅 draft 可改，voucher_ops/crud.rs:423-427 business_displayable）
    await apiCall(page, 'POST', `/vouchers/${id}/submit`);
    const failUpd = await apiCallExpectFail(page, 'PUT', `/vouchers/${id}`, {
      voucher_type: '违规',
    });
    expect(failUpd.status, 'submitted 改应 400').toBe(400);
    expect(failureCode(failUpd), '机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      typeof failUpd.message === 'string' && !DESENSITIZED_CONSTANTS.includes(failUpd.message),
      `草稿门文案应外显（business_displayable crud.rs:423-427），实际=${JSON.stringify(failUpd.message)}`
    ).toBe(true);
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d2, 'voucher_type', '收款凭证', '被拒后无痕迹');
    expectKeyValue(d2, 'status', 'submitted', '被拒后状态未漂移');
  });

  test('12-03 状态机全链与门控负例：review(draft)拒→submit→post(submitted)拒→review→post→unpost→再post；每步先响应后回查', async ({
    page,
  }) => {
    const [dSub, cSub] = await seedSubjects(page, '链');
    const v = await createBalancedVoucher(
      page,
      requireId(dSub, '借'),
      requireId(cSub, '贷'),
      888.88
    );
    const id = requireId(v, '建凭证');

    // draft 直接 review 被拒，文案可外显（workflow.rs:90-92 business_displayable）
    const f1 = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/review`);
    expect(f1.status, 'draft review 应 400').toBe(400);
    expect(failureCode(f1), 'draft review 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      !DESENSITIZED_CONSTANTS.includes(String(f1.message)),
      `review 门文案应外显，实际=${JSON.stringify(f1.message)}`
    ).toBe(true);
    let d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d, 'status', 'draft', '非法流转后无痕');

    const sub = await apiCall(page, 'POST', `/vouchers/${id}/submit`);
    expect(sub.code, 'submit 信封 200').toBe(200);
    d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d, 'status', 'submitted', 'submit 回查（finance.rs:67）');

    // submitted 直接 post 被拒（需 reviewed，workflow.rs:133-135 business_displayable）
    const f2 = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/post`);
    expect(f2.status, 'submitted post 应 400').toBe(400);
    expect(failureCode(f2), 'submitted post 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    await apiCall(page, 'POST', `/vouchers/${id}/review`);
    d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d, 'status', 'reviewed', 'review 回查（finance.rs:70）');

    await apiCall(page, 'POST', `/vouchers/${id}/post`);
    d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d, 'status', 'posted', 'post 回查（finance.rs:73）');
    expect(d.posted_by !== null, '过账应记 posted_by').toBe(true);

    // unpost：posted→reviewed（workflow.rs:205-222）
    await apiCall(page, 'POST', `/vouchers/${id}/unpost`);
    d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d, 'status', 'reviewed', 'unpost 回查');

    // 重复 unpost 被拒（非 posted，workflow.rs:205-207 business_displayable）且无痕
    const f3 = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/unpost`);
    expect(f3.status, 'reviewed unpost 应 400').toBe(400);
    expect(failureCode(f3), '重复 unpost 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    d = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d, 'status', 'reviewed', '非法 unpost 后未漂移');
  });

  test('12-04 凭证删除门：posted 删被拒无痕；draft 删后 GET 404/NOT_FOUND；科目被凭证引用删除被拒', async ({
    page,
  }) => {
    const [dSub, cSub] = await seedSubjects(page, '删');
    const dId = requireId(dSub, '借科目');
    const v = await createBalancedVoucher(page, dId, requireId(cSub, '贷'), 100);
    const id = requireId(v, '建凭证');
    await apiCall(page, 'POST', `/vouchers/${id}/submit`);
    await apiCall(page, 'POST', `/vouchers/${id}/review`);
    await apiCall(page, 'POST', `/vouchers/${id}/post`);

    const failDel = await apiCallExpectFail(page, 'DELETE', `/vouchers/${id}`);
    expect(failDel.status, 'posted 删应 400').toBe(400);
    expect(
      !DESENSITIZED_CONSTANTS.includes(String(failDel.message)),
      `删除门文案应外显（crud.rs:530-533 business_displayable），实际=${JSON.stringify(failDel.message)}`
    ).toBe(true);
    const d0 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${id}`);
    expectKeyValue(d0, 'status', 'posted', '删除被拒后凭证完好');

    // 该科目被凭证分录引用 → 删除被拒（account_subject_service.rs:295-315：
    // AppError::business（含引用条数文案保持脱敏）⇒ 机器码 BUSINESS_ERROR）
    const failSubjDel = await apiCallExpectFail(page, 'DELETE', `/subjects/${dId}`);
    expect(failSubjDel.status, '被引用科目删除应 400').toBe(400);
    expect(failureCode(failSubjDel), '被引用科目机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const subjStill = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/subjects/${dId}`);
    expectKeyValue(subjStill, 'id', dId, '引用防护后科目仍在');

    // draft 凭证可删 → 404
    const v2 = await createBalancedVoucher(page, dId, requireId(cSub, '贷'), 1);
    const id2 = requireId(v2, '建草稿凭证');
    await apiCall(page, 'DELETE', `/vouchers/${id2}`);
    const gone = await apiCallExpectFail(page, 'GET', `/vouchers/${id2}`);
    expect(gone.status, '删除后 GET 404').toBe(404);
    expect(failureCode(gone), '删除后机器码').toBe('NOT_FOUND');
  });

  test('12-05 科目全字段建单→逐字段回读→重复编码 400+外显文案；PUT 改 name/停用状态回读、未碰字段保持', async ({
    page,
  }) => {
    const code = genCode('SUB');
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/subjects', {
      code,
      name: 'E2E-科目-原名',
      level: 1,
      balance_direction: '借',
      assist_customer: true,
      assist_supplier: false,
      assist_batch: true,
      assist_color_no: false,
      enable_dual_unit: true,
    });
    const id = requireId(created, '建科目');
    CLEANUP.push({ path: `/subjects/${id}`, label: 'subject' });
    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/subjects/${id}`);
    expectKeyValue(detail, 'code', code, '科目详情');
    expectKeyValue(detail, 'name', 'E2E-科目-原名', '科目详情');
    expectKeyValue(detail, 'level', 1, '科目详情');
    expectKeyValue(detail, 'balance_direction', '借', '科目详情');
    expectKeyValue(detail, 'assist_customer', true, '科目详情');
    expectKeyValue(detail, 'assist_supplier', false, '科目详情');
    expectKeyValue(detail, 'assist_batch', true, '科目详情');
    expectKeyValue(detail, 'enable_dual_unit', true, '科目详情');

    // 重复编码 → business_displayable，文案回显用户提交的编码可外显（service:83-88）
    const fDup = await apiCallExpectFail(page, 'POST', '/subjects', {
      code,
      name: '重复',
      level: 1,
      assist_customer: false,
      assist_supplier: false,
      assist_batch: false,
      assist_color_no: false,
      enable_dual_unit: false,
    });
    expect(fDup.status, '重复编码应 400').toBe(400);
    expect(failureCode(fDup), '重复编码机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      typeof fDup.message === 'string' &&
        fDup.message.includes(code) &&
        !DESENSITIZED_CONSTANTS.includes(fDup.message),
      `重复编码应回显编码的具体文案，实际=${JSON.stringify(fDup.message)}`
    ).toBe(true);

    // PUT（UpdateSubjectRequestDto handler:59-69）：改 name + status，balance_direction 省略保持
    await apiCall(page, 'PUT', `/subjects/${id}`, {
      name: 'E2E-科目-改后',
      balance_direction: null,
      assist_customer: true,
      assist_supplier: false,
      assist_batch: true,
      assist_color_no: false,
      enable_dual_unit: true,
      status: 'inactive',
    });
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/subjects/${id}`);
    expectKeyValue(d2, 'name', 'E2E-科目-改后', '科目 PUT');
    expectKeyValue(d2, 'balance_direction', '借', '科目 PUT(单层 Option 省略/null 应保持)');
    expectKeyValue(d2, 'status', 'inactive', '科目 PUT 停用持久化');
    expectKeyValue(d2, 'assist_batch', true, '科目 PUT(未碰保持)');
  });

  test('12-06 会计期间：新建回读→重复建 4xx 必须具体原因（预期判红候选）→非法 period 拒绝', async ({
    page,
  }) => {
    const created = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/finance/accounting-periods',
      {
        year: 2035,
        period: 8,
      }
    );
    const id = Number(created.id);
    if (!Number.isFinite(id) || id <= 0)
      throw new Error(`期间创建未返回 id：${JSON.stringify(created)}`);
    CLEANUP.push({ path: `/finance/accounting-periods/${id}`, label: 'period' });
    expectKeyValue(created, 'year', 2035, '期间详情');
    expectKeyValue(created, 'period', 8, '期间详情');
    expectKeyValue(created, 'status', 'OPEN', '期间初始（finance.rs:79）');
    expectKeyValue(
      created,
      'period_name',
      '2035 年 08 月',
      '期间名格式（missing_handlers.rs:149）'
    );
    expect(typeof created.start_date === 'string', 'start_date 应为后端时间串').toBe(true);
    expect(typeof created.end_date === 'string', 'end_date 应为后端时间串').toBe(true);

    // period 越界 → VALIDATION_ERROR（#[validate(range(1,12))] missing_handlers.rs:106-108）
    const fRange = await apiCallExpectFail(page, 'POST', '/finance/accounting-periods', {
      year: 2035,
      period: 13,
    });
    expect(fRange.status, 'period 越界应 400').toBe(400);
    expect(failureCode(fRange), 'period 越界机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 重复期间 → 当前 AppError::business（missing_handlers.rs:119-125）⇒ message 脱敏为
    // 「业务处理失败」——本断言按任务红线要求 message 必须具体，**预期判红**并点名该 file:line；
    // 这是缺陷②「提交只报请求错误看不到原因」的族系实证，禁止放宽。
    const fDup = await apiCallExpectFail(page, 'POST', '/finance/accounting-periods', {
      year: 2035,
      period: 8,
    });
    expect(fDup.status, '重复期间应 400').toBe(400);
    expect(failureCode(fDup), '重复期间机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      typeof fDup.message === 'string' && !DESENSITIZED_CONSTANTS.includes(fDup.message),
      `重复期间 4xx 的 message 必须是具体原因而非脱敏常量（backend/src/handlers/missing_handlers.rs:119-125 使用 AppError::business 未迁移 displayable ⇒ 判红交编排派修），实际=${JSON.stringify(fDup.message)}`
    ).toBe(true);

    // 期间列表（GET /finance/accounting-periods 返回数组）含自建期间
    const list = await apiCallRaw<unknown>(page, 'GET', '/finance/accounting-periods');
    if (!Array.isArray(list)) throw new Error('期间列表应为裸数组（missing_handlers.rs:72-85）');
    const row = (list as Array<Record<string, unknown>>).find(r => Number(r.id) === id);
    if (!row) throw new Error('期间列表未含自建期间');
    expectKeyValue(row, 'status', 'OPEN', '期间列表行');
  });

  test('12-07 期间 PUT 门与 close→reopen 链：status 白名单、CLOSED 后改被拒、reopen 回 OPEN、无凭证可删→404', async ({
    page,
  }) => {
    const created = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/finance/accounting-periods',
      {
        year: 2036,
        period: 2,
      }
    );
    const id = Number(created.id);
    if (!Number.isFinite(id)) throw new Error(`期间未返回 id：${JSON.stringify(created)}`);

    // PUT rename 成功（OPEN 期间）
    await apiCall(page, 'PUT', `/finance/accounting-periods/${id}`, {
      period_name: '2036 年 02 月(改名)',
    });
    const d1 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/accounting-periods/${id}`
    );
    expectKeyValue(d1, 'period_name', '2036 年 02 月(改名)', '期间改名回读');

    // PUT status 非法值 → bad_request（missing_handlers.rs:194-200，BadRequest 携带原文）
    const fStatus = await apiCallExpectFail(page, 'PUT', `/finance/accounting-periods/${id}`, {
      status: 'PAUSED',
    });
    expect(fStatus.status, '非法 status 应 400').toBe(400);
    expect(failureCode(fStatus), '非法 status 机器码').toBe(APP_ERROR_CODES.BAD_REQUEST);
    const d2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/accounting-periods/${id}`
    );
    expectKeyValue(d2, 'status', 'OPEN', '非法状态被拒后未漂移');

    // close → CLOSED；CLOSED 后改名被拒（missing_handlers.rs:188-191 business ⇒ 断码 + 无痕）
    await apiCall(page, 'POST', `/finance/accounting-periods/${id}/close`);
    const d3 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/accounting-periods/${id}`
    );
    expectKeyValue(d3, 'status', 'CLOSED', 'close 回查（finance.rs:82）');
    const fClosedUpd = await apiCallExpectFail(page, 'PUT', `/finance/accounting-periods/${id}`, {
      period_name: '违规改名',
    });
    expect(fClosedUpd.status, 'CLOSED 改应 400').toBe(400);
    expect(failureCode(fClosedUpd), 'CLOSED 改机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const d4 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/accounting-periods/${id}`
    );
    expectKeyValue(d4, 'period_name', '2036 年 02 月(改名)', 'CLOSED 改被拒后名称未动');

    // reopen（需 reason，accounting_period_handler.rs:54-58）→ OPEN
    await apiCall(page, 'POST', `/finance/accounting-periods/${id}/reopen`, {
      reason: 'E2E 反结账验证',
    });
    const d5 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/accounting-periods/${id}`
    );
    expectKeyValue(d5, 'status', 'OPEN', 'reopen 回查');

    // 无凭证的期间可删 → GET 404
    await apiCall(page, 'DELETE', `/finance/accounting-periods/${id}`);
    const gone = await apiCallExpectFail(page, 'GET', `/finance/accounting-periods/${id}`);
    expect(gone.status, '期间删除后 404').toBe(404);
    expect(failureCode(gone), '期间删除后机器码').toBe('NOT_FOUND');
  });

  test('12-08 期间锁闭环：关账该期→向该期录入凭证并过账被拒（与 finance/08 互补的独立期间样本），反开后成功', async ({
    page,
  }) => {
    const created = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/finance/accounting-periods',
      {
        year: 2037,
        period: 3,
      }
    );
    const periodId = Number(created.id);
    if (!Number.isFinite(periodId)) throw new Error(`期间未返回 id：${JSON.stringify(created)}`);
    await apiCall(page, 'POST', `/finance/accounting-periods/${periodId}/close`);

    const [dSub, cSub] = await seedSubjects(page, '锁');
    // 期间锁在过账生效（workflow.rs post → check_date_locked 无环境开关，finance/08 头注）
    const v = await createBalancedVoucher(page, requireId(dSub, '借'), requireId(cSub, '贷'), 123, {
      voucher_date: '2037-03-15',
    });
    const vid = requireId(v, '建期间凭证');
    await apiCall(page, 'POST', `/vouchers/${vid}/submit`);
    await apiCall(page, 'POST', `/vouchers/${vid}/review`);

    const fPost = await apiCallExpectFail(page, 'POST', `/vouchers/${vid}/post`);
    expect(fPost.status, '已关账期间过账应 400').toBe(400);
    expect(
      !DESENSITIZED_CONSTANTS.includes(String(fPost.message)),
      `期间锁文案应外显（business_displayable），实际=${JSON.stringify(fPost.message)}`
    ).toBe(true);
    const d1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${vid}`);
    expectKeyValue(d1, 'status', 'reviewed', '过账被拒后凭证停在 reviewed，未半改');

    // reopen 后过账成功（证明拒绝确由关账触发，而非其他前置）
    await apiCall(page, 'POST', `/finance/accounting-periods/${periodId}/reopen`, {
      reason: 'E2E 解锁验证',
    });
    await apiCall(page, 'POST', `/vouchers/${vid}/post`);
    const d2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/vouchers/${vid}`);
    expectKeyValue(d2, 'status', 'posted', '反开后过账成功回查');
  });
});
