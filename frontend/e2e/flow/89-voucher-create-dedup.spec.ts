import { test, expect, type Locator, type Page } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';
import { pickSelectIn } from './ui-helpers';

/**
 * 凭证新建连点防重锁
 *
 * 功能，锁住凭证页新建提交在途期间的二次进入，重复点击只允许落一张凭证。
 * 为什么测不出，凭证页的提交校验全是同步判断，两次点击在同一个任务里都能通过校验，
 * 桌面视口与只测正向流程的用例都不会触发第二次点击，重复落库只能靠请求计数看见。
 * 调用方，frontend e2e flow 分片。入参，页面与登录态由 fixture 提供。
 * 传给谁，断言结果只进 Playwright 报告。
 * 存什么，不向真实库写凭证：建单 POST 被拦截并伪造成功响应，随后列表重读走真实 GET。
 *
 * 两条凭证新建入口并存且只有一条有锁，本用例打的是 frontend/src/views/voucher 那支
 * （/finance 的凭证 Tab 另有 voucherSubmitLoading，不在本用例范围）。
 */

const VOUCHER_ROUTE = '/voucher';
const DIALOG_ARIA = '新增凭证';

async function gotoVoucherList(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}${VOUCHER_ROUTE}`, { waitUntil: 'domcontentloaded' });
  await page.waitForLoadState('networkidle', { timeout: 8000 }).catch(() => {});
}

/** 分录行内的科目下拉不在 el-form-item 里，只能按行作用域点开面板取首项 */
async function pickFirstSubjectInRow(row: Locator, page: Page): Promise<void> {
  const wrapper = row.locator('.el-select__wrapper').first();
  await wrapper.click();
  const panel = page.locator('.el-select__popper').last();
  await expect(panel, '科目下拉面板应出现').toBeVisible({ timeout: 10000 });
  const option = panel.locator('.el-select-dropdown__item').first();
  await option.waitFor({ state: 'visible', timeout: 10000 });
  await option.click();
  await panel.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
}

async function fillAmount(control: Locator, value: string): Promise<void> {
  await control.fill(value);
  await control.press('Enter');
}

test.describe('凭证新建防重', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('连点两次确定只发出一条建单请求', async ({ page }) => {
    await gotoVoucherList(page);

    let postCount = 0;
    const payloads: Array<Record<string, unknown>> = [];
    await page.route('**/api/v1/erp/vouchers', async route => {
      if (route.request().method() !== 'POST') {
        await route.continue();
        return;
      }
      postCount += 1;
      const raw = route.request().postData();
      if (raw) payloads.push(JSON.parse(raw) as Record<string, unknown>);
      // 延迟放行，让第二次点击落在真实在途窗口内；锁没落就得并发出第二张单
      await new Promise(resolve => setTimeout(resolve, 800));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          code: 200,
          message: 'ok',
          data: { id: 987654321, voucher_no: 'E2E-FAKE', status: 'draft' },
        }),
      });
    });

    await page.getByRole('button', { name: '新增凭证', exact: true }).click();
    const dialog = page.locator(`.el-dialog[aria-label="${DIALOG_ARIA}"]`).last();
    await expect(dialog).toBeVisible({ timeout: 30000 });

    const dateInput = dialog.locator('.el-date-editor input').first();
    await dateInput.click();
    await dateInput.fill('2026-08-19');
    await dateInput.press('Enter');
    await pickSelectIn(dialog, page, '凭证类型');

    const rows = dialog.locator('.entries-row');
    await expect(rows, '新建凭证默认给出一行分录').toHaveCount(1, { timeout: 10000 });
    await pickFirstSubjectInRow(rows.nth(0), page);
    await fillAmount(rows.nth(0).locator('.el-input-number input').first(), '100');

    await dialog.getByRole('button', { name: '+ 添加分录' }).click();
    await expect(rows, '添加后应得到第二行分录').toHaveCount(2, { timeout: 10000 });
    await pickFirstSubjectInRow(rows.nth(1), page);
    await fillAmount(rows.nth(1).locator('.el-input-number input').nth(1), '100');

    // 借贷平衡是提交前置校验的真实结果，未平衡时提交会在锁内早退，计数断言会失真
    await expect(dialog.getByText('借贷平衡'), '两行分录应真实平衡').toBeVisible({ timeout: 10000 });

    const confirm = dialog.locator('.el-dialog__footer .el-button--primary').last();
    // 同一个同步任务里点两下：重渲染还没把按钮置灰，第二下会真实进入处理器
    await confirm.evaluate((el: HTMLElement) => {
      el.click();
      el.click();
    });

    await expect(confirm, '提交在途时确定按钮应进入加载态').toHaveClass(/is-loading/, {
      timeout: 3000,
    });
    await expect(dialog, '建单成功后对话框应关闭').toBeHidden({ timeout: 30000 });

    expect(postCount, '连点两次只允许发出一张建单请求').toBe(1);
    expect(payloads).toHaveLength(1);
    const items = payloads[0].items as Array<Record<string, unknown>>;
    expect(items, '提交的凭证应带齐借贷两行分录').toHaveLength(2);
    expect(payloads[0].voucher_type, '凭证类型应取自后端词表').toBeTruthy();
  });
});
