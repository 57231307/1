import { test, expect, type Locator, type Page } from '../diagnose-fixture';
import { apiCallRaw, BASE_URL, API_PREFIX, loginViaUI, tryCleanup } from './helpers';
import {
  pickSelectIn,
  openScopedDropdown,
  dropdownItems,
} from './ui-helpers';

/**
 * 凭证新建连点防重锁
 *
 * 功能，锁住凭证页新建提交在途期间的二次进入：同一张凭证连点两下确定，只允许落一张凭证。
 * 为什么这样测，凭证页提交入口 useVchrLst.handleSubmit 是同步在途锁（submitting 先于 await 置位），
 * 桌面视口/只测正向流程的用例不触发第二次点击；且提交校验全同步，重复落库在 UI 上看不见，
 * 只能靠真实建单请求计数加后端回读才判得准，故本用例既被动观察 POST 又回读落库结果。
 * 调用方，frontend e2e flow 分片。入参，页面与登录态由 worker fixture 提供；分录科目取
 * globalSetup 播种的真实科目树，凭证类型取后端 available_voucher_types 词表，其余走真实接口。
 * 传给谁，断言结果只进 Playwright 报告。
 * 存什么，经 UI 真实提交 POST /vouchers 落一张 draft 凭证（不伪造响应），
 * 用例结束 afterEach 按 id 真实 DELETE 清理，不污染分片共享库。
 *
 * 两条凭证新建入口并存且只有一条有锁，本用例打的是 frontend/src/views/voucher 那支
 * （/finance 的凭证 Tab 另有 voucherSubmitLoading，不在本用例范围）。
 * 判据落在后端真实回读：GET /vouchers 命中该凭证号的张数、GET /vouchers/{id} 的
 * status/entries/voucher_type，而非前端提示文案。
 */

const VOUCHER_ROUTE = '/voucher';
const DIALOG_ARIA = '新增凭证';
// 借贷平衡是提交前置校验（useVchrLst 与 voucher_ops/crud.rs total_debit!=total_credit→400），
// 两行各填 100 对称；未平衡时 handleSubmit 会在锁内早退，计数断言失真。
const VOUCHER_DATE = '2026-08-19';
const createPathname = `${API_PREFIX}/vouchers`;

/** 真实建单 POST：方法为 POST 且 pathname 恰为 /api/v1/erp/vouchers（排除 generate-no/types/状态机端点） */
function isCreateVoucherPost(method: string, url: string): boolean {
  return method === 'POST' && new URL(url).pathname === createPathname;
}

/** GET /vouchers 出参为裸数组（ApiResponse<Vec<voucher::Model>>），键名同后端 model（snake_case） */
interface VoucherListRow {
  id: number;
  voucher_no: string;
  status: string;
}
/** GET /vouchers/{id} 出参 flatten voucher::Model + entries 分录列表 */
interface VoucherDetail {
  id: number;
  voucher_no: string;
  voucher_type: string;
  status: string;
  entries: unknown[];
}

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function gotoVoucherList(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}${VOUCHER_ROUTE}`, { waitUntil: 'domcontentloaded' });
  await page.waitForLoadState('networkidle', { timeout: 8000 }).catch(() => {});
}

/** 分录行内的科目下拉不在 el-form-item 里，按行作用域打开该字段自己的面板取首项（helper 钉归属） */
async function pickFirstSubjectInRow(row: Locator, page: Page): Promise<void> {
  const panel = await openScopedDropdown(row, page, { timeout: 10_000 });
  const option = dropdownItems(panel).first();
  await option.waitFor({ state: 'visible', timeout: 10_000 });
  await option.click();
  await panel.waitFor({ state: 'hidden', timeout: 5_000 }).catch(() => {});
}

async function fillAmount(control: Locator, value: string): Promise<void> {
  await control.fill(value);
  // 手写录入框在 change 提交模型，Tab 触发 blur 让借/贷金额写入分录，逐键不写模型
  await control.press('Tab');
}

test.describe('凭证新建防重', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('连点两次确定只发出一条建单请求', async ({ page }) => {
    await gotoVoucherList(page);

    // 真实链路观察（不拦截、不改写响应）：POST 直达后端真落库，只被动记录请求与建单响应回传的凭证号。
    const createRequests: string[] = [];
    const persisted: Array<{ id: number; voucher_no: string }> = [];
    const createResponses: Array<Promise<void>> = [];
    page.on('request', req => {
      if (isCreateVoucherPost(req.method(), req.url())) createRequests.push(req.url());
    });
    page.on('response', res => {
      if (!isCreateVoucherPost(res.request().method(), res.url())) return;
      createResponses.push(
        res.json().then(
          body => {
            const data = (body as { data?: { id?: number; voucher_no?: string } }).data;
            const id = data?.id;
            if (id) persisted.push({ id, voucher_no: String(data?.voucher_no ?? '') });
          },
          () => {
            // 建单非 2xx/非 JSON：不塞 persisted，交由下方张数断言如实判红，不掩盖后端拒绝
          }
        )
      );
    });

    await page.getByRole('button', { name: '新增凭证', exact: true }).click();
    const dialog = page.locator(`.el-dialog[aria-label="${DIALOG_ARIA}"]`).last();
    await expect(dialog).toBeVisible({ timeout: 30000 });

    const dateInput = dialog.locator('.el-date-editor input').first();
    await dateInput.click();
    await dateInput.fill(VOUCHER_DATE);
    await dateInput.press('Enter');
    await pickSelectIn(dialog, page, '凭证类型');

    const rows = dialog.locator('.entries-row');
    await expect(rows, '新建凭证默认给出一行分录').toHaveCount(1, { timeout: 10000 });
    await pickFirstSubjectInRow(rows.nth(0), page);
    await expect(rows.nth(0).getByRole('spinbutton'), '分录金额不应是步进角色').toHaveCount(0);
    await fillAmount(rows.nth(0).locator('.col-debit input'), '100');

    await dialog.getByRole('button', { name: '+ 添加分录' }).click();
    await expect(rows, '添加后应得到第二行分录').toHaveCount(2, { timeout: 10000 });
    await pickFirstSubjectInRow(rows.nth(1), page);
    await fillAmount(rows.nth(1).locator('.col-credit input'), '100');

    // 借贷平衡是提交前置校验的真实结果，未平衡时 handleSubmit 会在锁内早退，计数断言会失真
    await expect(dialog.getByText('借贷平衡'), '两行分录应真实平衡').toBeVisible({
      timeout: 10000,
    });

    const confirm = dialog.locator('.el-dialog__footer .el-button--primary').last();
    // 同一个同步任务里点两下：重渲染还没把按钮置灰，第二下会真实进入处理器并被在途锁挡回
    await confirm.evaluate((el: HTMLElement) => {
      el.click();
      el.click();
    });

    await expect(confirm, '提交在途时确定按钮应进入加载态').toHaveClass(/is-loading/, {
      timeout: 3000,
    });
    await expect(dialog, '建单成功后对话框应关闭').toBeHidden({ timeout: 30000 });
    await Promise.allSettled(createResponses);

    // 一、请求层锁：连点两下只允许发出一条 POST /vouchers
    expect(createRequests, '连点两次只允许发出一张建单请求').toHaveLength(1);
    // 二、后端真落库：只应持久化一张凭证（若前端锁失效并发建单，此处如实 >1 判红，不放过）
    expect(persisted, '连点两次只允许真实落库一张凭证').toHaveLength(1);
    const { id: createdId, voucher_no: createdNo } = persisted[0];
    expect(createdNo, '真实建单响应应回传凭证号').toBeTruthy();
    CLEANUP.push({ path: `/vouchers/${createdId}`, label: 'voucher' });

    // 三、真实回读张数：GET /vouchers 命中该凭证号的行数恰为 1（列表出参裸数组，前端按全号精确匹配）
    const list = await apiCallRaw<VoucherListRow[]>(
      page,
      'GET',
      `/vouchers?voucher_no=${encodeURIComponent(createdNo)}`
    );
    const matched = list.filter(r => r.voucher_no === createdNo);
    expect(matched, `真实回读凭证 ${createdNo} 应只有一张`).toHaveLength(1);
    expect(matched[0].status, '新建凭证初始状态应为 draft').toBe('draft');

    // 四、真实回读内容：详情 entries 借贷两行真实落库、凭证类型取自后端词表
    const detail = await apiCallRaw<VoucherDetail>(page, 'GET', `/vouchers/${createdId}`);
    expect(detail.voucher_no, '详情凭证号应与建单响应一致').toBe(createdNo);
    expect(detail.status, '详情初始状态应为 draft').toBe('draft');
    expect(detail.entries, '提交的凭证应真实带齐借贷两行分录').toHaveLength(2);
    expect(detail.voucher_type, '凭证类型应取自后端词表并真实落库').toBeTruthy();
  });
});
