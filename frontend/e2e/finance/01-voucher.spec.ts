// 财务管理 E2E 套件 — 01 凭证管理
// 创建时间: 2026-08-19
// 覆盖范围：凭证创建（含借贷平衡） → 提交 → 审核 → 过账
import { test, expect, type Page, type Locator } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelectIn } from '../flow/ui-helpers';
import { apiCall, tryCleanup } from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 保证存在两枚可识别的叶子会计科目，返回其唯一名称。
 * 凭证分录科目为 el-tree-select，须真实存在的科目外键（voucher_ops/crud.rs precheck_subjects_exist_txn）。
 * 用例不依赖环境既有科目：直接经真实 API 建两枚并登记清理；VoucherTab onMounted 会重拉
 * /subjects/tree，故随后打开的凭证对话框科目树即含这两枚（按名称精确定位节点）。
 */
async function ensureLeafSubjects(page: Page): Promise<[string, string]> {
  const ts = Date.now();
  const names: string[] = [];
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
    names.push(name);
    i++;
  }
  return [names[0], names[1]];
}

/**
 * 在凭证分录行的科目 el-tree-select 中按名称选中科目。
 * el-tree-select 复用 el-select 触发器（.el-select__wrapper），点击后下拉面板 teleport 到 body，
 * 选项为树节点 .el-tree-node__content（非 .el-select-dropdown__item，故不能用 pickSelectIn）。
 *
 * 稳健性：CI #4662 该用例报 "locator resolved → not visible ×58"——节点一度 resolve 但点击前
 * 面板收起/节点随重渲染短暂不可见。修复策略（均为真实交互、不放宽任何断言）：
 *  1) 把节点作用域限定到「当前可见的下拉面板」容器内，避免命中另一行残留的隐藏同名节点；
 *  2) 面板与节点均等到 visible + stable 再点；
 *  3) 若节点始终不可见（面板已收起），重新点开触发器再取节点——重试轮询本身仍要求节点出现，
 *     若前端根本不渲染叶子节点（leafSubjects 为空），此函数最终仍超时判红，不掩盖源码缺陷。
 */
async function pickSubjectInTreeSelect(
  row: Locator,
  page: Page,
  subjectName: string
): Promise<void> {
  const wrapper = row.locator('.el-select__wrapper').first();
  // el-tree-select 复用 el-select 的下拉容器（EP 把树渲染在 .el-select-dropdown 内），
  // 限定到「当前可见面板」内的树节点，避免命中另一行收起后仍留在 DOM 的同名隐藏节点。
  const nodeInVisiblePanel = () =>
    page
      .locator('.el-select-dropdown:visible')
      .last()
      .locator('.el-tree-node__content')
      .filter({ hasText: subjectName })
      .first();
  const tryPick = async (openTimeout: number, nodeTimeout: number) => {
    await wrapper.click({ timeout: openTimeout });
    const node = nodeInVisiblePanel();
    await node.waitFor({ state: 'visible', timeout: nodeTimeout });
    await node.click({ timeout: nodeTimeout });
  };
  try {
    await tryPick(15_000, 8_000);
  } catch {
    // 面板在等待/点击期间因失焦动画收起（#4662 报 "resolved → not visible ×58"）
    // → 重新点开触发器再取节点；若前端根本不渲染叶子节点（leafSubjects 为空），
    //   重试仍会超时判红，不掩盖源码缺陷。
    await tryPick(15_000, 15_000);
  }
}

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
    const [debitSubjectName, creditSubjectName] = await ensureLeafSubjects(page);

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

    await pickSubjectInTreeSelect(rows.nth(0), page, debitSubjectName);
    await pickSubjectInTreeSelect(rows.nth(1), page, creditSubjectName);

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
