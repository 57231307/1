import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';

import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectBusinessRejection,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';

/**
 * 色卡库存登记与发放真链路。
 *
 * 断什么：PUT /color-cards/:id 的 stock_quantity 是色卡实物库存的唯一登记入口；
 * 登记之后 POST /color-cards/issues 才可能过发放守门（可用 = 库存 - 已发放），
 * 发放只累加已发放量不动库存列，库存低于已发放量或为负都点名拒绝且库里零漂移。
 * 造数全部本用例自建自持（色卡 色号 客户各一条），不复用库里的共享行。
 * 会话形态：test 取自 diagnose-fixture 的 worker 级会话夹具，beforeEach 经 loginViaUI
 * 以本 worker 专属账号建立登录态后所有请求才带认证凭据与 CSRF 头。
 */
const CARD_PATH = '/color-cards';
const ISSUES_PATH = '/color-cards/issues';
const CARD_LIST_PATH = '/color-cards/list';

const uniq = () => `${Date.now().toString().slice(-6)}${Math.floor(Math.random() * 1000)}`;

interface CardReadback {
  id?: number;
  status?: string;
  stock_quantity?: number;
  issued_quantity?: number;
  card_no?: string;
  card_name?: string;
}

async function createCardWithColor(page: Page): Promise<number> {
  const tag = uniq();
  const card = await apiCall<{ id?: number }>(page, 'POST', CARD_PATH, {
    card_no: `E2E-STK-CC${tag}`,
    card_name: `E2E 库存登记色卡 ${tag}`,
    card_type: 'CUSTOM',
  });
  const cardId = card.data?.id;
  if (!cardId) {
    throw new Error(`[110] 色卡创建未返回 id：${JSON.stringify(card).slice(0, 300)}`);
  }
  const item = await apiCall<{ id?: number }>(page, 'POST', `${CARD_PATH}/${cardId}/items`, {
    color_code: `E2E-STK${tag}`,
    color_name: 'E2E 库存登记色号',
    rgb_r: 10,
    rgb_g: 20,
    rgb_b: 30,
    hex_value: '#0A141E',
  });
  if (!item.data?.id) {
    throw new Error(`[110] 色号明细创建未返回 id：${JSON.stringify(item).slice(0, 300)}`);
  }
  return cardId;
}

async function createCustomer(page: Page): Promise<number> {
  // 发放闸门 3（color_card_issue_service.rs check_customer_credit_and_status）要求
  // customers.credit_limit > 0；本端点入参 credit_limit 是数值字符串
  // （customer_handler.rs CreateCustomerRequest，缺省与空串都落 0），造数必须显式给正值。
  const res = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_name: `E2E 库存登记客户${uniq()}`,
    customer_type: 'wholesale',
    status: 'active',
    credit_limit: '1000',
  });
  const id = res.data?.id;
  if (!id) {
    throw new Error(`[110] 客户创建未返回 id：${JSON.stringify(res).slice(0, 300)}`);
  }
  return id;
}

async function readCard(page: Page, cardId: number): Promise<CardReadback> {
  return apiCallRaw<CardReadback>(page, 'GET', `${CARD_PATH}/${cardId}`);
}

/**
 * 列表页按卡名定位到该色卡那一行。
 * 入参是已登录的 page 与卡名，返回行定位器，交给调用方用例点行内入口，不持久化。
 * 先用关键词筛选把表压到一行，不依赖新建行落在第几页第几行，避开翻页与排序漂移带来的偶发失败。
 * 注意：搜索框占位写「卡号 / 卡名」，但后端 list 的 keyword 只 LIKE card_name
 * （color_card_crud_service.rs 的 list 过滤，主干既有实现），填卡号恒查空，故这里必须填卡名。
 * 卡名由本用例自建且含唯一 tag，可精确命中单行。
 */
async function locateCardRow(page: Page, cardName: string) {
  await page.goto(CARD_LIST_PATH);
  await expect(
    page.getByText('色卡列表').first(),
    `色卡列表页应渲染，准备定位卡名 ${cardName}`
  ).toBeVisible({ timeout: 30000 });
  await page.getByPlaceholder('卡号 / 卡名').fill(cardName);
  await page.getByRole('button', { name: '查询' }).click();
  const row = page.getByRole('row').filter({ hasText: cardName }).first();
  await expect(row, `列表按关键词筛出的色卡行应可见，卡名 ${cardName}`).toBeVisible({
    timeout: 30000,
  });
  return row;
}

/**
 * 详情页基本信息页签里的可写卡面信息表单。
 * 入参是 page，返回表单定位器，交给编辑入口相关用例判编辑态是否真打开，不持久化。
 * 只读查看态不渲染该表单，因此它的有无正好区分"编辑"与"查看"两个入口，
 * 历史缺陷的形态正是两个入口产出完全一样的页面。
 */
const cardEditForm = (page: Page) => page.locator('form[aria-label="基本信息"]');

test.describe('色卡库存登记与发放', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('建卡库存为 0，登记后可发放且发放只动已发放量，两个下限各拒一次且零漂移', async ({
    page,
  }) => {
    const cardId = await createCardWithColor(page);
    const customerId = await createCustomer(page);

    const created = await readCard(page, cardId);
    expect(created.status, '新建色卡应为草稿态').toBe('draft');
    expect(created.stock_quantity, '建卡时库存恒为 0（还没印，不是被代填出来的合法值）').toBe(0);
    expect(created.issued_quantity, '建卡时已发放量应为 0').toBe(0);

    // 未登记库存即发放：守门按可用量拒绝，库里两个数量都不许被写脏
    const blocked = await apiCallExpectFail(page, 'POST', ISSUES_PATH, {
      color_card_id: cardId,
      customer_id: customerId,
      issue_qty: 1,
    });
    expect(blocked.status, `未登记库存的发放应被拒，实际 code=${blocked.code ?? '(none)'}`).toBe(
      400
    );
    expect(
      failureCode(blocked),
      `未登记库存的发放应回字符串机器码，实际 message=${blocked.message ?? '(none)'}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stillZero = await readCard(page, cardId);
    expect(stillZero.stock_quantity, '发放被拒后库存应仍是 0').toBe(0);
    expect(stillZero.issued_quantity, '发放被拒后已发放量应仍是 0').toBe(0);

    // 登记 5 张：不信任 PUT 响应体，成败以详情回读为准
    await apiCall(page, 'PUT', `${CARD_PATH}/${cardId}`, { stock_quantity: 5 });
    const afterRegister = await readCard(page, cardId);
    expect(afterRegister.stock_quantity, '详情回读库存应等于登记的 5').toBe(5);
    expect(afterRegister.issued_quantity, '登记库存不改已发放量').toBe(0);

    // 发放 2 张：库存列不动，已发放量增到 2
    const issued = await apiCall<{ id?: number }>(page, 'POST', ISSUES_PATH, {
      color_card_id: cardId,
      customer_id: customerId,
      issue_qty: 2,
    });
    expect(
      issued.data?.id,
      `登记库存后发放应成功，实际 code=${issued.code} message=${issued.message}`
    ).toBeTruthy();
    const afterIssue = await readCard(page, cardId);
    expect(afterIssue.stock_quantity, '发放不动库存列').toBe(5);
    expect(afterIssue.issued_quantity, '发放应把已发放量累加到 2').toBe(2);

    // 库存改到低于已发放量：拒绝且库里不改写
    const tooLow = await apiCallExpectFail(page, 'PUT', `${CARD_PATH}/${cardId}`, {
      stock_quantity: 1,
    });
    expectBusinessRejection(tooLow, '库存低于已发放量应被点名拒绝');
    const afterTooLow = await readCard(page, cardId);
    expect(afterTooLow.stock_quantity, '被拒的库存改写不得落库').toBe(5);
    expect(afterTooLow.issued_quantity, '被拒的库存改写不动已发放量').toBe(2);

    // 负数库存同样拒绝
    const negative = await apiCallExpectFail(page, 'PUT', `${CARD_PATH}/${cardId}`, {
      stock_quantity: -1,
    });
    expectBusinessRejection(negative, '负数库存应被点名拒绝');
    const afterNegative = await readCard(page, cardId);
    expect(afterNegative.stock_quantity, '负数拒绝后库存仍应是 5').toBe(5);

    // 收尾：色卡无物理删除端点，归档该行；失败只记日志，不影响已完成的断言
    const archived = await apiCall(page, 'DELETE', `${CARD_PATH}/${cardId}`, {});
    if (!archived.data) {
      console.warn(
        `[110] 色卡归档未回数据，实际 code=${archived.code} message=${archived.message}`
      );
    }
  });
});

test.describe('色卡卡面信息编辑入口', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('编辑入口进编辑态并真写库，详情入口不进编辑态', async ({ page }) => {
    const cardId = await createCardWithColor(page);
    const created = await readCard(page, cardId);
    const originalName = created.card_name;
    if (!originalName) {
      throw new Error(`[110] 色卡回读缺卡名，实际 ${JSON.stringify(created).slice(0, 300)}`);
    }

    // 反向判据，详情入口只给只读查看，页面上不该出现可写的卡面信息表单
    const viewRow = await locateCardRow(page, originalName);
    await viewRow.getByRole('button', { name: '详情' }).click();
    await expect(page, '详情入口应进色卡详情页').toHaveURL(/\/color-cards\/detail\/\d+$/);
    await expect(cardEditForm(page), '详情入口不应打开编辑态').toHaveCount(0);

    // 正向判据，编辑入口打开同一页签的可写表单，且回填的是库里现值
    const editRow = await locateCardRow(page, originalName);
    await editRow.getByRole('button', { name: '编辑' }).click();
    await expect(page, '编辑入口应把编辑意图带进地址栏').toHaveURL(
      /\/color-cards\/detail\/\d+\?edit=1$/
    );
    const form = cardEditForm(page);
    await expect(form, '编辑入口应打开可写的卡面信息表单').toBeVisible({ timeout: 30000 });
    const nameInput = form.getByPlaceholder('请输入卡名');
    await expect(nameInput, '编辑表单应回填库里的卡名').toHaveValue(originalName);

    const newName = `${originalName} 已改${uniq()}`;
    await nameInput.fill(newName);
    // waitForResponse 注册在点击之前，保存请求随点击立即发出，点后注册会漏接
    const putResp = page.waitForResponse(
      resp => resp.request().method() === 'PUT' && /\/color-cards\/\d+$/.test(resp.url()),
      { timeout: 30000 }
    );
    await form.getByRole('button', { name: '保存' }).click();
    const putRes = await putResp;
    expect(
      putRes.status(),
      `编辑保存应 200（4xx=权限或契约回归），实际 ${putRes.status()} ${(await putRes.text()).slice(0, 200)}`
    ).toBe(200);

    // 成败以库里真值为准，不看 PUT 响应体形状
    const afterSave = await readCard(page, cardId);
    expect(afterSave.card_name, '保存后回读的卡名应是提交的新值').toBe(newName);
    await expect(cardEditForm(page), '保存后应退出编辑态回到只读展示').toHaveCount(0);
    await expect(page, '保存后地址栏不该再留编辑意图').not.toHaveURL(/edit=1/);
    await expect(page.getByText(newName).first(), '只读展示应回显新卡名').toBeVisible({
      timeout: 30000,
    });

    await apiCall(page, 'DELETE', `${CARD_PATH}/${cardId}`, {});
  });

  test('非草稿态色卡在列表里没有编辑入口', async ({ page }) => {
    const cardId = await createCardWithColor(page);
    const created = await readCard(page, cardId);
    if (!created.card_name) {
      throw new Error(`[110] 色卡回读缺卡名，实际 ${JSON.stringify(created).slice(0, 300)}`);
    }

    // 卡面信息更新端点对非草稿态一律拒，入口就不该留着给用户点成一次必然的失败
    await apiCall(page, 'DELETE', `${CARD_PATH}/${cardId}`, {});
    const archivedCard = await readCard(page, cardId);
    expect(
      archivedCard.status,
      `归档回读状态应为 archived，实际 ${archivedCard.status ?? '(none)'}`
    ).toBe('archived');

    const row = await locateCardRow(page, created.card_name);
    await expect(row.getByRole('button', { name: '编辑' }), '非草稿态不应渲染编辑入口').toHaveCount(
      0
    );
    await expect(row.getByRole('button', { name: '详情' }), '查看入口应仍在').toBeVisible();
  });
});
