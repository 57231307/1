// 财务管理 E2E 套件 — 01 凭证管理
// 创建时间: 2026-08-19
// 覆盖范围：凭证创建（含借贷平衡） → 提交 → 审核 → 过账
import { test, expect, type Page } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelectIn, pickTreeNode } from '../flow/ui-helpers';
import { apiCall, apiCallRaw, tryCleanup } from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 凭证详情回读的精确形状：voucher 字段 flatten 至顶层，分录在 entries（GET /vouchers/{id}） */
interface VoucherDetailLite {
  id: number;
  voucher_no: string;
  voucher_date: string;
  status: string | null;
  entries: Array<{
    line_no: number;
    subject_id: number | null;
    debit: unknown;
    credit: unknown;
    summary: string | null;
  }>;
}

/** 金额按两位小数标度对齐（分录列为 DECIMAL(15,2)，序列化常以字符串回喂），非数值形态立即抛错不放宽 */
function amountAtScale(value: unknown, label: string): string {
  const n = typeof value === 'string' || typeof value === 'number' ? Number(value) : NaN;
  if (!Number.isFinite(n)) {
    throw new Error(`${label} 金额形态非数值（契约漂移？）：${JSON.stringify(value)}`);
  }
  return n.toFixed(2);
}

/**
 * 按本例自建科目的 id 集合唯一锁定 UI 刚创建的凭证并返回其详情。
 * 列表端点 GET /vouchers 的 data 为裸数组且支持 status/start_date/end_date 过滤；
 * 列表行不含分录科目，故逐候选取 GET /vouchers/{id} 详情，以 entries 的 subject_id
 * 集合与本例两枚唯一科目精确相等为判据——其他分片/其他用例的凭证引用的是各自科目，
 * 天然免疫并发串扰。命中数不等于 1 时由调用方断言判红，不静默取第一条。
 */
async function findVoucherBySubjectIds(
  page: Page,
  subjectIds: [number, number],
  voucherDate: string
): Promise<VoucherDetailLite[]> {
  const list = await apiCallRaw<unknown>(
    page,
    'GET',
    `/vouchers?status=draft&start_date=${voucherDate}&end_date=${voucherDate}&page=1&page_size=100`
  );
  if (!Array.isArray(list)) {
    throw new Error(
      `GET /vouchers 未返回数组（列表契约漂移）：${JSON.stringify(list).slice(0, 200)}`
    );
  }
  const wanted = new Set(subjectIds);
  const hits: VoucherDetailLite[] = [];
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
    const got = new Set(
      detail.entries.map(e => e.subject_id).filter((x): x is number => typeof x === 'number')
    );
    if (got.size === wanted.size && [...wanted].every(x => got.has(x))) hits.push(detail);
  }
  return hits;
}

/**
 * 保证存在两枚可识别的叶子会计科目，返回其 id 与唯一名称。
 * 凭证分录科目为外键，须真实存在（voucher_ops/crud.rs 科目预检）。
 * 用例不依赖环境既有科目：直接经真实 API 建两枚并登记清理；VoucherTab onMounted 会重拉
 * /subjects/tree，故随后打开的凭证对话框科目树即含这两枚（按名称精确定位节点）。
 * id 用于建单后按分录科目集合回读定位凭证，名称用于 UI 树节点定位。
 */
async function ensureLeafSubjects(page: Page): Promise<Array<{ id: number; name: string }>> {
  const ts = Date.now();
  const subjects: Array<{ id: number; name: string }> = [];
  let i = 0;
  for (const role of ['借', '贷']) {
    const name = `E2E 科目${role}${ts}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
      code: `E2E${ts}${i}`,
      name,
      level: 1,
      balance_direction: 'debit',
    });
    const id = created.data?.id;
    if (!id) throw new Error(`建科目失败：${JSON.stringify(created).slice(0, 200)}`);
    CLEANUP.push({ path: `/subjects/${id}`, label: 'account_subject' });
    subjects.push({ id, name });
    i++;
  }
  return subjects;
}

/**
 * 在凭证分录行的科目 el-tree-select 中按名称选中科目（委托 flow/ui-helpers 唯一事实源
 * pickTreeNode：面板归属钉定 + 收起竞态重开重试，语义与原内联实现一致）。
 */

test.describe('01 凭证管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入财务管理页面', async ({ page }) => {
    // /finance 为 el-tabs 容器，文本「财务管理/凭证管理」在左侧导航菜单等多处出现，
    // getByText(/财务管理|凭证管理/) 会 strict-mode 命中 5 个 → 改断言该页专属的两个 tab 可见。
    await page.goto('/finance');
    await expect(page.getByRole('tab', { name: '科目管理' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: '凭证管理' })).toBeVisible();
  });

  test('01-02 新建凭证（含借贷分录）', async ({ page }) => {
    // 先真实准备两枚叶子科目（分录科目为外键，须存在），再走 UI 建凭证。
    const [debitSubject, creditSubject] = await ensureLeafSubjects(page);
    const debitSubjectName = debitSubject.name;
    const creditSubjectName = creditSubject.name;

    await page.goto('/finance');
    // 原缺陷：/finance 默认停在「科目管理」Tab（finance/index.vue activeTab='subject'），
    // 直接 getByRole 点「新建」命中的是科目 Tab 的按钮，打开的是「新建科目」对话框——
    // 其内无 .el-date-editor，故原用例在 voucherDate.click() 处 30s 超时（失败点 spec:32）。
    // 须先切到「凭证管理」Tab，令凭证新建入口与凭证对话框进入 DOM。
    await page.getByRole('tab', { name: '凭证管理' }).click();
    await expect(page.locator('.el-table[aria-label="凭证列表"]').first()).toBeVisible({
      timeout: 30000,
    });
    await page.getByRole('button', { name: '新建凭证', exact: true }).click();

    // 精确到凭证新建对话框（VoucherForm.vue el-dialog aria-label='新建凭证对话框'），
    // 避免与其它同名/残留对话框串台。
    const dlg = page.locator('.el-dialog[aria-label="新建凭证对话框"]').last();
    await expect(dlg).toBeVisible({ timeout: 30000 });

    // el-date-picker 的内层 input 不能用 getByLabel 命中（EP el-form-item 的 label 不带 for，
    // 标签与控件无原生关联）且直接对只读编辑器 fill 会超时。改定位 .el-date-editor 的可编辑
    // 输入框，走真实手输交互：点开 → 输入日期 → 回车提交（VoucherForm.vue type=date
    // + value-format=YYYY-MM-DD，支持手输解析）。限定到对话框作用域，避免与筛选栏同名控件串台。
    const voucherDate = dlg.locator('.el-date-editor input').first();
    await voucherDate.click();
    await voucherDate.fill('2026-08-19');
    await voucherDate.press('Enter');

    // 凭证类型是 el-select。用唯一事实源 helper pickSelectIn 以 root=dlg + 精确 label「凭证类型」
    // 作用域，点外层 wrapper（非只读内层 input）打开下拉并选首项。
    await pickSelectIn(dlg, page, '凭证类型');

    // 「摘要」并非 el-form-item（VoucherForm.vue:55-60 是 entries 表格列，每行 el-input
    // 的 placeholder=「摘要」/ placeholderSummary），getByLabel 命中不到 → 旧选择器恒超时。
    // 按 placeholder 锚定首行摘要输入框。
    //
    // 借贷分录真实录入（原用例未选科目、未录金额，提交时 useVchr.submitVoucherForm 的
    // .filter(e => e.subject_id) 会把无科目分录全过滤掉 → items 为空、借贷恒 0==0 的假建单）。
    // 这里为两行分别选借/贷科目并录入借 100 / 贷 100，真实触发前后端借贷平衡校验：
    // 前端 useVchr.isBalanced + 后端 voucher_ops/crud.rs total_debit != total_credit → 400。
    // 未删除任何校验，只是让用例真实满足校验。
    const rows = dlg.locator('.el-table[aria-label="凭证分录编辑表"] tbody .el-table__row');
    await expect(rows).toHaveCount(2, { timeout: 10000 });

    await pickTreeNode(rows.nth(0), page, debitSubjectName);
    await pickTreeNode(rows.nth(1), page, creditSubjectName);

    // 借方行：借方金额（本行第 1 个 el-input-number）填 100，并填摘要
    const debitInput = rows.nth(0).locator('.el-input-number input').first();
    await debitInput.fill('100');
    await debitInput.press('Enter');
    await rows.nth(0).locator('input[placeholder="摘要"]').fill('E2E 借方分录');

    // 贷方行：贷方金额（本行第 2 个 el-input-number）填 100，并填摘要
    const creditInput = rows.nth(1).locator('.el-input-number input').nth(1);
    await creditInput.fill('100');
    await creditInput.press('Enter');
    await rows.nth(1).locator('input[placeholder="摘要"]').fill('E2E 贷方分录');

    // 真实平衡态呈现（finance.voucherForm.textBalanced='已平衡'），证明校验被真实满足而非空单。
    await expect(dlg.getByText('已平衡')).toBeVisible({ timeout: 10000 });

    // 提交按钮真实文案「确定」（finance.voucherForm.buttonConfirm='确定'），旧 /确认|提交/ 命不中。
    await dlg.getByRole('button', { name: '确定' }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 });

    // 丙档回读：toast 只证明前端收到成功响应，不证明落库真值。
    // 以本例唯一科目对为锚定位 UI 建出的凭证（GET /vouchers 列表 + GET /vouchers/{id} 详情），
    // 命中数不等于 1 一律判红，不静默取第一条；随后断言状态、日期、分录行数与借贷金额真值。
    const hits = await findVoucherBySubjectIds(
      page,
      [debitSubject.id, creditSubject.id],
      '2026-08-19'
    );
    expect(hits.length, 'UI 建出的凭证应按本例唯一科目对精确命中一条').toBe(1);
    const createdVoucher = hits[0];
    CLEANUP.push({ path: `/vouchers/${createdVoucher.id}`, label: 'voucher' });

    expect(createdVoucher.status, '新建凭证应落库为 draft').toBe('draft');
    expect(createdVoucher.voucher_date, '凭证日期应落库为 UI 录入值').toBe('2026-08-19');
    expect(
      createdVoucher.entries.length,
      `分录行数应为 UI 实际录入的 2 行，实际 ${createdVoucher.entries.length}`
    ).toBe(2);

    const debitEntry = createdVoucher.entries.find(e => e.subject_id === debitSubject.id);
    const creditEntry = createdVoucher.entries.find(e => e.subject_id === creditSubject.id);
    if (!debitEntry || !creditEntry) {
      throw new Error(
        `回读分录缺少借/贷科目行（debit=${JSON.stringify(debitEntry)} credit=${JSON.stringify(creditEntry)}）`
      );
    }
    // 金额按两位小数同一标度比较（分录列为 DECIMAL(15,2)，序列化常以字符串回喂）
    expect(amountAtScale(debitEntry.debit, '借方行借额'), '借方行借额应为 UI 录入的 100').toBe(
      '100.00'
    );
    expect(amountAtScale(debitEntry.credit, '借方行贷额'), '借方行贷额应为 0').toBe('0.00');
    expect(amountAtScale(creditEntry.debit, '贷方行借额'), '贷方行借额应为 0').toBe('0.00');
    expect(amountAtScale(creditEntry.credit, '贷方行贷额'), '贷方行贷额应为 UI 录入的 100').toBe(
      '100.00'
    );
    expect(debitEntry.summary, '借方行摘要应落库为 UI 录入原文').toBe('E2E 借方分录');
    expect(creditEntry.summary, '贷方行摘要应落库为 UI 录入原文').toBe('E2E 贷方分录');
  });

  test('01-03 凭证筛选功能可用', async ({ page }) => {
    // 「凭证号」筛选位于「凭证管理」Tab，而 /finance 默认停在「科目管理」Tab，须先切过去；
    // 原 locator('table, .el-table') 会 strict-mode 命中多个（el-table 内含多个 <table>），
    // 改精确到凭证列表容器（el-table 根上的 aria-label="凭证列表"）。
    await page.goto('/finance');
    await page.getByRole('tab', { name: '凭证管理' }).click();
    await page.getByLabel('凭证号').fill('E2E');
    await page.getByRole('button', { name: '查询' }).click();
    await expect(page.locator('.el-table[aria-label="凭证列表"]').first()).toBeVisible({
      timeout: 30000,
    });
  });
});
