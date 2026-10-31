import { test, expect } from '../diagnose-fixture';
import type { Locator, Page, Response } from '@playwright/test';
import { loginViaUI, apiCallRaw, genCode, tryCleanup } from './helpers';
import {
  safeGoto,
  findTableRow,
  waitForDialog,
  submitDialog,
  formItemByExactLabel,
} from './ui-helpers';
// 产品类型单源常量（与后端 product_type 词表同源）：建品夹具取值必须经成员引用。
import { PRODUCT_TYPE } from '../../src/constants/product-type';

/**
 * 106 产品米数下线与规格派生提交形态回读锁。
 *
 * 裁定口径（产品档案规格与每匹米数收口工单）：每卷米数是照抄假字段，整列下线由
 * m0109 探测式迁移收口；每匹米数保留为换算默认值但退出产品表单采集面；
 * 规格提交与显示统一为「幅宽 cm 星号 克重 g」字面拼接。
 *
 * 本 spec 判红的缺陷方向。
 * 其一，表单若回潮出现米数输入（每匹或每卷），106-01 的米数控件零存在断言即红。
 * 其二，提交端若不再拼规格或拼成数值乘法，106-01 的请求体与回读规格断言即红
 * （显示端拼了而提交端存空也在红面内，断的是请求体与落库回读两侧）。
 * 其三，表单若把米数键重新塞回载荷，106-01 的键缺席断言即红；
 * 键缺席同时是历史每匹米数保持语义的前半段，回读仍为预置值即后半段成立。
 * 其四，幅宽清空若塌回“保持原值”，106-02 的请求体显式 null 或回读 NULL 断言即红。
 * 每种退化各有检测点，不互相重叠。
 *
 * 约束。不拦截前端请求路由、不做 mock，全程真实 UI 提交加真实 API 回读。
 * 数字输入为 el-input 文本形态（全项目禁带加减步进控件），断言按字符串与 Number 归一。
 *
 * 页面路由 product。端点（相对 baseURL）products 与 products 加 id 路径段。
 */

type Row = Record<string, unknown>;

interface SeededProduct {
  id: number;
  name: string;
}

/** 直连 API 建非面料类产品避开必录门，可带每匹米数与幅宽克重，返回主键与名称 */
async function seedProduct(
  page: Page,
  tag: string,
  extra: Record<string, unknown>
): Promise<SeededProduct> {
  const cats = await apiCallRaw<{ items?: Row[] }>(page, 'GET', '/product-categories');
  const categoryId = Number(cats?.items?.[0]?.id);
  expect(categoryId, '[106] 须有已种入的分类作建档必填来源').toBeGreaterThan(0);

  const name = `E2E106米数下线产品_${tag}`;
  const created = await apiCallRaw<Row>(page, 'POST', '/products', {
    name,
    category_id: categoryId,
    unit: '米',
    product_type: PRODUCT_TYPE.auxiliary,
    ...extra,
  });
  const id = Number(created?.id);
  expect(id, `[106] 产品建档应返回主键：${JSON.stringify(created).slice(0, 200)}`).toBeGreaterThan(
    0
  );
  return { id, name };
}

/** 触发一次 UI 提交并抓取前端真实请求体与响应（与 68 号同款形态，跳过 CSRF 瞬态 403） */
async function captureUiSubmit(
  page: Page,
  urlFragment: string,
  action: () => Promise<void>,
  what: string
): Promise<{ status: number; body: Row; bodyRaw: string }> {
  const respPromise = page.waitForResponse(
    res =>
      res.url().includes(urlFragment) &&
      res.request().method() === 'PUT' &&
      // admin 会话下结构上拿不到权限 403，该路由唯一 403 源是 CSRF 首发，
      // axios 拦截器刷新令牌后会静默重放，跳过瞬态、等真实提交响应
      res.status() !== 403,
    { timeout: 30_000 }
  );
  await action();
  const resp: Response = await respPromise;
  const raw = resp.request().postData();
  expect(raw, `${what}：未捕获到前端提交请求体`).not.toBeNull();
  return { status: resp.status(), body: JSON.parse(String(raw)) as Row, bodyRaw: String(raw) };
}

function inputOf(dialog: Locator, labelText: string): Locator {
  return formItemByExactLabel(dialog, labelText).locator('input').first();
}

/** 进产品列表、按名称过滤、定位自建行并打开编辑弹窗 */
async function openEditDialog(page: Page, productName: string): Promise<Locator> {
  await safeGoto(page, '/product');
  const keywordInput = page.locator('.filter-card input:visible').first();
  await keywordInput.fill(productName);
  await page.getByRole('button', { name: '查询' }).first().click();
  const row = await findTableRow(page, productName, 1);
  expect(row, `[106] 列表应定位到自建产品 ${productName}`).toBeTruthy();
  await row!.getByRole('button', { name: '编辑' }).first().click();
  return waitForDialog(page);
}

test.describe('106 产品米数下线与规格派生提交形态回读', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('106-01 表单无米数输入，规格按幅宽星号克重拼接提交并落库回读', async ({ page }) => {
    const { id, name } = await seedProduct(page, genCode('E106A'), {
      meters_per_piece: '60',
    });
    try {
      const seeded = await apiCallRaw<Row>(page, 'GET', `/products/${id}`);
      expect(Number(seeded.meters_per_piece), '[106-01] 预置回读每匹米数应为 60').toBe(60);

      const dlg = await openEditDialog(page, name);

      // 米数采集面零存在：任何标签含每卷米数或每匹米数的表单项出现即红
      for (const label of ['每卷米数', '每匹米数']) {
        await expect
          .poll(async () => formItemByExactLabel(dlg, label).count(), { timeout: 10_000 })
          .toBe(0);
      }

      const widthInput = inputOf(dlg, '有效幅宽(cm)');
      const gramInput = inputOf(dlg, '克重(g/m²)');
      await widthInput.fill('160');
      await gramInput.fill('180');

      // 显示端派生串：两项齐备后规格框应显示字面拼接值（星号是连接符，不是乘积 28800）
      const specInput = inputOf(dlg, '规格');
      await expect.poll(() => specInput.inputValue(), { timeout: 10_000 }).toBe('160cm*180g');

      const cap = await captureUiSubmit(
        page,
        `/api/v1/erp/products/${id}`,
        () => submitDialog(dlg, /保存/),
        '[106-01] 编辑提交'
      );
      expect(
        cap.status,
        `[106-01] 提交必须被后端接受（HTTP 恰 200），实际 status=${cap.status} body=${cap.bodyRaw}`
      ).toBe(200);
      expect(
        cap.body.specification,
        `[106-01] 请求体规格应为字面拼接串 160cm*180g，实际 ${cap.bodyRaw}`
      ).toBe('160cm*180g');
      for (const retired of ['meters_per_piece', 'meters_per_roll']) {
        expect(
          Object.prototype.hasOwnProperty.call(cap.body, retired),
          `[106-01] 产品表单不应再把 ${retired} 塞回提交载荷，实际=${cap.bodyRaw}`
        ).toBe(false);
      }

      // 落库回读：规格为拼接串；每匹米数键未随请求发送，历史值必须保持（三态键缺席语义）
      const fresh = await apiCallRaw<Row>(page, 'GET', `/products/${id}`);
      expect(fresh.specification, '回读规格应为提交端拼出的字面串').toBe('160cm*180g');
      expect(
        Number(fresh.meters_per_piece),
        '请求不带每匹米数时历史值必须保持为 60（表单摘除不改写存量）'
      ).toBe(60);
      expect(
        fresh.meters_per_roll,
        '出参不应再携带每卷米数键（读 undefined 即键已退场）'
      ).toBeUndefined();
    } finally {
      await tryCleanup(page, 'DELETE', `/products/${id}`, '[106-01] 产品');
    }
  });

  test('106-02 清空幅宽后提交，请求体显式 null 且回读为空，规格回落手工形态', async ({ page }) => {
    const { id, name } = await seedProduct(page, genCode('E106B'), {
      meters_per_piece: '80',
      width: '160',
      gram_weight: '180',
    });
    try {
      const dlg = await openEditDialog(page, name);
      const widthInput = inputOf(dlg, '有效幅宽(cm)');
      await expect
        .poll(async () => Number(await widthInput.inputValue()), { timeout: 10_000 })
        .toBe(160);

      await widthInput.fill('');
      await widthInput.press('Tab');

      // 派生不可算时规格框回落显示回显的历史规格，不再挂拼接值
      const specInput = inputOf(dlg, '规格');
      await expect.poll(() => specInput.inputValue(), { timeout: 10_000 }).not.toBe('160cm*180g');

      const cap = await captureUiSubmit(
        page,
        `/api/v1/erp/products/${id}`,
        () => submitDialog(dlg, /保存/),
        '[106-02] 清空提交'
      );
      expect(
        Object.prototype.hasOwnProperty.call(cap.body, 'width'),
        `[106-02] 清空后的幅宽必须以显式 null 键在场（省略键在后端是保持原值），实际=${cap.bodyRaw}`
      ).toBe(true);
      expect(cap.body.width, '清空后的幅宽请求值应为 null').toBeNull();
      expect(
        cap.status,
        `[106-02] 清空的提交必须被后端接受（HTTP 恰 200），实际 status=${cap.status} body=${cap.bodyRaw}`
      ).toBe(200);

      const fresh = await apiCallRaw<Row>(page, 'GET', `/products/${id}`);
      expect(
        fresh.width,
        `回读幅宽必须为 NULL，仍为旧值即清空塌成了保持原值（实际 ${JSON.stringify(fresh.width)}）`
      ).toBeNull();
      expect(Number(fresh.meters_per_piece), '每匹米数未被本次请求触碰，历史值 80 必须保持').toBe(
        80
      );
    } finally {
      await tryCleanup(page, 'DELETE', `/products/${id}`, '[106-02] 产品');
    }
  });
});
