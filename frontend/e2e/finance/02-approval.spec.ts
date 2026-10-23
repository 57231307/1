// 财务管理 E2E 套件 — 02 凭证审批工作流
// 创建时间: 2026-08-19
// 覆盖范围：凭证提交（draft → submitted） → 审核（submitted → reviewed） → 过账（reviewed → posted），
// 每步 UI 动作后以 GET /vouchers/{id} 回读落库真值（状态词表与审批人字段），不只断 toast。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, tryCleanup } from '../flow/helpers';

/**
 * 前置数据构造（方法一）：
 * 原用例既未切到「凭证管理」Tab（/finance 默认停在「科目管理」Tab，凭证行内按钮根本不在 DOM 里），
 * 又用 `if (await btn.isVisible())` 包裹，导致双重假绿。
 * 现改为：先建一张 draft 凭证（必要时再经 API submit/review/post 推进到目标态），切到凭证 Tab，
 * 按凭证号定位自己那一行再点行内操作，使断言必然执行。
 *
 * 渲染条件依据（frontend/src/views/finance/tabs/components/VoucherTable.vue）：
 *   提交按钮 status==='draft'（:75）、审核按钮 status==='submitted'（:83）、过账按钮 status==='reviewed'（:91）。
 * 后端词表依据（backend/src/models/status/finance.rs voucher）：
 *   draft/submitted/reviewed/posted，提交/审核/过账在 voucher_ops/workflow.rs 逐级推进；
 *   建单要求借贷平衡（voucher_ops/crud.rs:124）且日期所属会计期间未锁定（check_date_locked），故先 init 当期。
 * 注意：凭证建单后端字段是 items + subject_id（voucher_handler.rs CreateVoucherRequestDto），
 * 而前端 finance.ts 写的是 entries（命名不一致，属既有缺陷，与本用例无关仅记录），本用例直接按后端契约建单。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 取两个叶子科目 id（凭证分录要求外键真实存在），不足则建 */
async function leafSubjectIds(page: import('@playwright/test').Page): Promise<[number, number]> {
  const tree = await apiCallRaw<
    Array<{ id: number; is_leaf?: boolean; children?: unknown[] }> | { items?: unknown[] }
  >(page, 'GET', '/subjects/tree');
  const flat: Array<{ id: number; is_leaf?: boolean; children?: unknown[] }> = [];
  const walk = (nodes: Array<{ id: number; is_leaf?: boolean; children?: unknown[] }>) => {
    for (const n of nodes) {
      if (n.is_leaf) flat.push(n);
      if (Array.isArray(n.children)) walk(n.children as typeof nodes);
    }
  };
  if (Array.isArray(tree)) walk(tree);
  else if ((tree as { items?: unknown[] }).items) walk((tree as { items: typeof flat }).items);

  const ids = flat.map(s => s.id).slice(0, 2);
  while (ids.length < 2) {
    const prefix = 'E2E' + Math.floor(Math.random() * 100000);
    const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
      code: `${prefix}${ids.length}`,
      name: `E2E 凭证测试科目${prefix}${ids.length}`,
      level: 1,
      balance_direction: 'debit',
    });
    if (!created.data?.id) break;
    ids.push(created.data.id);
    CLEANUP.push({ path: `/subjects/${created.data.id}`, label: 'account_subject' });
  }
  if (ids.length < 2) throw new Error('无法准备两个叶子科目用于建凭证');
  return [ids[0], ids[1]];
}

type VoucherStatus = 'draft' | 'submitted' | 'reviewed' | 'posted';

/** 凭证详情回读的精确形状（voucher 字段 flatten 至顶层；状态词表见 backend models/status/finance.rs 的 voucher 模块，小写） */
interface VoucherLite {
  id: number;
  voucher_no: string;
  status: string | null;
  reviewed_by: number | null;
  posted_by: number | null;
}

/** 当前已认证用户 id（reviewed_by/posted_by 由后端写入 auth 上下文的 user_id） */
async function fetchMyUserId(page: import('@playwright/test').Page): Promise<number> {
  const me = await apiCallRaw<{ id?: number }>(page, 'GET', '/auth/me');
  const myId = Number(me.id);
  if (!Number.isInteger(myId) || myId <= 0) {
    throw new Error(`前置失败：/auth/me 未返回有效用户 id：${JSON.stringify(me).slice(0, 200)}`);
  }
  return myId;
}

/** 建一张推进到 targetStatus 的凭证，返回 { id, voucherNo } */
async function seedVoucher(
  page: import('@playwright/test').Page,
  targetStatus: VoucherStatus
): Promise<{ id: number; voucherNo: string }> {
  // 前置条件失败判红：init_first_period 幂等（已有 OPEN 期直接返回），
  // 非 2xx 即真实环境缺陷，不允许 console.warn 后带病继续。
  await apiCall(page, 'POST', '/finance/accounting-periods/init', {});
  const [debitSubject, creditSubject] = await leafSubjectIds(page);
  const created = await apiCall<{ id?: number; voucher_no?: string }>(page, 'POST', '/vouchers', {
    voucher_type: 'JZ',
    voucher_date: new Date().toISOString().slice(0, 10),
    items: [
      { subject_id: debitSubject, debit: 100, credit: 0, summary: 'E2E 借方' },
      { subject_id: creditSubject, debit: 0, credit: 100, summary: 'E2E 贷方' },
    ],
  });
  const id = created.data?.id;
  if (!id) throw new Error(`建凭证失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/vouchers/${id}`, label: 'voucher' });
  // 凭证号是行定位锚点，缺键即建单契约漂移，判红而非回退空串（空串 hasText 会命中任意行）
  const voucherNo = created.data?.voucher_no;
  if (!voucherNo)
    throw new Error(`建凭证未返回 voucher_no：${JSON.stringify(created).slice(0, 200)}`);

  // 后端真实端点为动词路径（backend/src/routes/finance.rs:226/230/233）：
  //   POST /vouchers/{id}/submit → draft→submitted
  //   POST /vouchers/{id}/review → submitted→reviewed
  //   POST /vouchers/{id}/post   → reviewed→posted
  // 原实现用状态名当端点段（/vouchers/{id}/submitted|reviewed|posted）→ 后端 404。
  type TransitionStatus = 'submitted' | 'reviewed' | 'posted';
  const transitionEndpoint: Record<TransitionStatus, string> = {
    submitted: 'submit',
    reviewed: 'review',
    posted: 'post',
  };
  const order: VoucherStatus[] = ['draft', 'submitted', 'reviewed', 'posted'];
  const transitions: TransitionStatus[] = ['submitted', 'reviewed', 'posted'];
  const targetIdx = order.indexOf(targetStatus);
  // 从 draft 起逐级推进：submitted → reviewed → posted，到目标态即停
  for (let i = 1; i <= targetIdx; i++) {
    await apiCall(page, 'POST', `/vouchers/${id}/${transitionEndpoint[transitions[i - 1]]}`);
  }
  return { id, voucherNo };
}

async function gotoVoucherTab(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/finance');
  await page.getByRole('tab', { name: '凭证管理' }).click();
  // 原 locator('table, .el-table') 会 strict-mode 命中 8 个（el-table 内部渲染多个 <table>），
  // 改精确到凭证列表容器（el-table 根节点 aria-label="凭证列表"）。
  await expect(page.locator('.el-table[aria-label="凭证列表"]').first()).toBeVisible({
    timeout: 30000,
  });
}

test.describe('02 凭证审批工作流', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 草稿凭证可提交审核（draft → submitted）', async ({ page }) => {
    // 方法一：建 draft 凭证 → 提交按钮渲染 → 定位自身行点击提交 → 断言提交成功 toast
    // → GET /vouchers/{id} 回读落库真值：状态词表写入方为 backend models/status/finance.rs
    // 的 voucher 模块（小写 draft/submitted/reviewed/posted），比较点逐字符相同。
    const { id, voucherNo } = await seedVoucher(page, 'draft');
    await gotoVoucherTab(page);
    const row = page.getByRole('row').filter({ hasText: voucherNo });
    const submitBtn = row.getByText('提交', { exact: false }).first();
    await expect(submitBtn, `定位 draft 凭证 ${voucherNo} 的提交按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await submitBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/提交成功/)).toBeVisible({ timeout: 30000 });

    const detail = await apiCallRaw<VoucherLite>(page, 'GET', `/vouchers/${id}`);
    expect(detail.voucher_no, '回读凭证号应与建单返回一致').toBe(voucherNo);
    expect(detail.status, `提交后应落库为 submitted，实际 ${detail.status}`).toBe('submitted');
    // submit 只推进状态（voucher_ops/workflow.rs），不得触碰审核/过账字段
    expect(detail.reviewed_by, '提交不应写入 reviewed_by').toBeNull();
    expect(detail.posted_by, '提交不应写入 posted_by').toBeNull();
  });

  test('02-02 已提交凭证可审核（submitted → reviewed）', async ({ page }) => {
    // 方法一：建凭证并 API 推进到 submitted → 审核按钮渲染 → 点击审核 → 回读状态与审核人。
    // review 在 workflow.rs 写入 reviewed_by=已认证 user_id 与 reviewed_at。
    const { id, voucherNo } = await seedVoucher(page, 'submitted');
    const myId = await fetchMyUserId(page);
    await gotoVoucherTab(page);
    const row = page.getByRole('row').filter({ hasText: voucherNo });
    const reviewBtn = row.getByText('审核', { exact: false }).first();
    await expect(reviewBtn, `定位 submitted 凭证 ${voucherNo} 的审核按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await reviewBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/审核成功/)).toBeVisible({ timeout: 30000 });

    const detail = await apiCallRaw<VoucherLite>(page, 'GET', `/vouchers/${id}`);
    expect(detail.status, `审核后应落库为 reviewed，实际 ${detail.status}`).toBe('reviewed');
    expect(
      detail.reviewed_by,
      `审核后 reviewed_by 应为执行审核的当前用户 ${myId}，实际 ${detail.reviewed_by}`
    ).toBe(myId);
    expect(detail.posted_by, '审核不应写入 posted_by').toBeNull();
  });

  test('02-03 已审核凭证可过账（reviewed → posted）', async ({ page }) => {
    // 方法一：建凭证推进到 reviewed → 过账按钮渲染 → 点击过账 → 回读状态与过账人。
    // post 在 workflow.rs 写入 posted_by=已认证 user_id，并要求该期未锁定（前置已 init 当期）。
    const { id, voucherNo } = await seedVoucher(page, 'reviewed');
    const myId = await fetchMyUserId(page);
    await gotoVoucherTab(page);
    const row = page.getByRole('row').filter({ hasText: voucherNo });
    const postBtn = row.getByText('过账', { exact: false }).first();
    await expect(postBtn, `定位 reviewed 凭证 ${voucherNo} 的过账按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await postBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/过账成功/)).toBeVisible({ timeout: 30000 });

    const detail = await apiCallRaw<VoucherLite>(page, 'GET', `/vouchers/${id}`);
    expect(detail.status, `过账后应落库为 posted，实际 ${detail.status}`).toBe('posted');
    expect(
      detail.posted_by,
      `过账后 posted_by 应为执行过账的当前用户 ${myId}，实际 ${detail.posted_by}`
    ).toBe(myId);
    // 前置 reviewed 由 seedVoucher 以同一认证用户经 API 推进，审核人字段应已就位
    expect(
      detail.reviewed_by,
      `过账前审核阶段写入的 reviewed_by 应保持不变（${myId}），实际 ${detail.reviewed_by}`
    ).toBe(myId);
  });

  test('02-04 已过账凭证无提交/审核/过账按钮', async ({ page }) => {
    // 方法一：建凭证过账到 posted，定位自身行，断言状态标签 已过账 存在且无任何行内流程按钮，
    // 并 GET 回读落库状态为 posted（按钮门控的判据就是该状态，回读证明门控前提真实成立）。
    const { id, voucherNo } = await seedVoucher(page, 'posted');
    await gotoVoucherTab(page);
    const row = page.getByRole('row').filter({ hasText: voucherNo });
    await expect(row, `未找到已过账凭证行 ${voucherNo}`).toHaveCount(1);
    await expect(row.getByText('已过账')).toBeVisible({ timeout: 10000 });
    // 动作按钮在 VoucherTable.vue 为 el-button（role=button），文案精确为 提交/审核/过账。
    // 旧 getByText(/提交|审核|过账/) 子串匹配会把状态 el-tag 已过账（含子串 过账）误计为动作按钮。
    // 改按 role=button 且可访问名逐字精确匹配，排除状态标签文本；若前端真漏渲染门控按钮仍会被计到。
    const actionBtns = row.getByRole('button', { name: /^(提交|审核|过账)$/ });
    await expect(actionBtns).toHaveCount(0);

    const detail = await apiCallRaw<VoucherLite>(page, 'GET', `/vouchers/${id}`);
    expect(detail.status, `过账链路完成后应落库为 posted，实际 ${detail.status}`).toBe('posted');
    expect(detail.posted_by, 'posted 凭证的 posted_by 应已写入（非 null）').not.toBeNull();
  });
});
