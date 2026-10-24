import { test, expect } from '@playwright/test';
import type { Page } from '@playwright/test';

import {
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
 */
const CARD_PATH = '/color-cards';
const ISSUES_PATH = '/color-cards/issues';

const uniq = () => `${Date.now().toString().slice(-6)}${Math.floor(Math.random() * 1000)}`;

interface CardReadback {
  id?: number;
  status?: string;
  stock_quantity?: number;
  issued_quantity?: number;
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
  const res = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_name: `E2E 库存登记客户${uniq()}`,
    customer_type: '企业',
    status: 'active',
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

test.describe('色卡库存登记与发放', () => {
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
      console.warn(`[110] 色卡归档未回数据，实际 code=${archived.code} message=${archived.message}`);
    }
  });
});
