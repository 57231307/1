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

/**
 * 106 产品米数单入口的提交形态回读锁。
 *
 * 本 spec 判红产品米数单入口裁定的两个后续缺陷方向。
 * 其一，改动每匹米数后提交，GET 详情回读两列都是新值，证明前端单入口与后端
 * 同值双写端到端真的落库。
 * 其二，清空每匹米数后提交，GET 详情回读两列都是 NULL，证明用户的清空不会被
 * 静默吞成“键缺席不更新”。清空必须送显式 null（RFC 7386 三态），请求体里键
 * 必须在场且值为 null，回读必须真空，请求体形态与落库结果两侧断言都不可省。
 *
 * 检测力自证。若前端退回把清空做成省略键，106-02 在请求体键在场断言即红。
 * 若后端退回单层两态（显式 null 与键缺席不可区分），106-02 请求体绿但回读仍
 * 断到旧值立即红。若同值双写被摘掉，106-01 回读每卷米数读不到新值立即红，
 * 且 106-02 预置回读每卷米数即红。每种退化各有检测点，不互相重叠。
 *
 * 约束。不拦截前端请求路由、不做 mock，全程真实 UI 提交加真实 API 回读。
 * 编辑弹窗的米数输入为 el-input-number，precision 为 4，故显示带四位小数，
 * 断言一律 Number 归一后比较。清空经 value-on-clear 归一为 null。
 *
 * 页面路由 product。端点（相对 baseURL）products 与 products 加 id 路径段。
 */

type Row = Record<string, unknown>;

interface SeededProduct {
  id: number;
  name: string;
}

async function seedProductWithYardage(
  page: Page,
  tag: string,
  metersPerPiece: string
): Promise<SeededProduct> {
  const cats = await apiCallRaw<{ items?: Row[] }>(page, 'GET', '/product-categories');
  const categoryId = Number(cats?.items?.[0]?.id);
  expect(categoryId, '[106] 须有已种入的分类作建档必填来源').toBeGreaterThan(0);

  const name = `E2E106米数产品_${tag}`;
  const created = await apiCallRaw<Row>(page, 'POST', '/products', {
    name,
    category_id: categoryId,
    unit: '米',
    meters_per_piece: metersPerPiece,
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

function metersInput(dialog: Locator, labelText: string): Locator {
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

test.describe('106 产品米数单入口提交形态回读', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('106-01 改值后提交，两列同值落库并回读为新值', async ({ page }) => {
    const { id, name } = await seedProductWithYardage(page, genCode('E106A'), '60');
    try {
      // 前置钉桩：创建只带每匹米数，后端同值双写已把两列都落成 60
      const seeded = await apiCallRaw<Row>(page, 'GET', `/products/${id}`);
      expect(Number(seeded.meters_per_piece), '[106-01] 预置回读每匹米数应为 60').toBe(60);
      expect(Number(seeded.meters_per_roll), '[106-01] 预置回读每卷米数应被同源带成 60').toBe(60);

      const dlg = await openEditDialog(page, name);
      const pieceInput = metersInput(dlg, '每匹米数');
      const rollInput = metersInput(dlg, '每卷米数');
      await expect
        .poll(async () => Number(await pieceInput.inputValue()), { timeout: 10_000 })
        .toBe(60);

      await pieceInput.fill('120');
      await pieceInput.press('Tab');
      // 只读带出列应同步显示新值（这一步红说明单入口带出链路断了）
      await expect
        .poll(async () => Number(await rollInput.inputValue()), { timeout: 10_000 })
        .toBe(120);

      const cap = await captureUiSubmit(
        page,
        `/api/v1/erp/products/${id}`,
        () => submitDialog(dlg, /保存/),
        '[106-01] 编辑提交'
      );
      expect(
        cap.status,
        `[106-01] 改值提交必须被后端接受（HTTP 恰 200），实际 status=${cap.status} body=${cap.bodyRaw}`
      ).toBe(200);
      expect(cap.body.meters_per_piece, `[106-01] 请求体每匹米数应为新值 120，实际 ${cap.bodyRaw}`).toBe(
        120
      );
      expect(cap.body.meters_per_roll, `[106-01] 请求体每卷米数应同源带出 120，实际 ${cap.bodyRaw}`).toBe(
        120
      );

      const fresh = await apiCallRaw<Row>(page, 'GET', `/products/${id}`);
      expect(Number(fresh.meters_per_piece), '回读每匹米数应为新值 120').toBe(120);
      expect(
        Number(fresh.meters_per_roll),
        '回读每卷米数必须同值落库为 120，读不到或读到旧值即同值双写被摘'
      ).toBe(120);
    } finally {
      await tryCleanup(page, 'DELETE', `/products/${id}`, '[106-01] 产品');
    }
  });

  test('106-02 清空后提交，请求体两键显式 null 且回读两列为空', async ({ page }) => {
    const { id, name } = await seedProductWithYardage(page, genCode('E106B'), '80');
    try {
      const dlg = await openEditDialog(page, name);
      const pieceInput = metersInput(dlg, '每匹米数');
      const rollInput = metersInput(dlg, '每卷米数');
      // 前置钉桩：回显与只读带出都到位，清空动作才有对照面
      await expect
        .poll(async () => Number(await rollInput.inputValue()), { timeout: 10_000 })
        .toBe(80);

      await pieceInput.fill('');
      await pieceInput.press('Tab');
      await expect.poll(() => pieceInput.inputValue(), { timeout: 10_000 }).toBe('');
      await expect.poll(() => rollInput.inputValue(), { timeout: 10_000 }).toBe('');

      const cap = await captureUiSubmit(
        page,
        `/api/v1/erp/products/${id}`,
        () => submitDialog(dlg, /保存/),
        '[106-02] 清空提交'
      );
      // 请求体形态断言：清空必须送显式 null，做成省略键就是“清空不生效”的前端半段
      expect(
        Object.prototype.hasOwnProperty.call(cap.body, 'meters_per_piece'),
        `[106-02] 清空后的每匹米数必须以显式 null 键在场（省略键在后端是保持原值），实际=${cap.bodyRaw}`
      ).toBe(true);
      expect(cap.body.meters_per_piece, '清空后的每匹米数请求值应为 null').toBeNull();
      expect(
        Object.prototype.hasOwnProperty.call(cap.body, 'meters_per_roll'),
        `[106-02] 清空后的每卷米数同样必须键在场且为 null，实际=${cap.bodyRaw}`
      ).toBe(true);
      expect(cap.body.meters_per_roll, '清空后的每卷米数请求值应为 null').toBeNull();
      expect(
        cap.status,
        `[106-02] 清空的提交必须被后端接受（HTTP 恰 200），实际 status=${cap.status} body=${cap.bodyRaw}`
      ).toBe(200);

      // 落库结果断言：后端若把显式 null 塌回“不更新”，这里仍读到旧值立即红
      const fresh = await apiCallRaw<Row>(page, 'GET', `/products/${id}`);
      expect(
        fresh.meters_per_piece,
        `回读每匹米数必须为 NULL，仍为旧值即清空塌成了保持原值（实际 ${JSON.stringify(
          fresh.meters_per_piece
        )}）`
      ).toBeNull();
      expect(
        fresh.meters_per_roll,
        `回读每卷米数必须为 NULL，两列同源不得一清一留（实际 ${JSON.stringify(
          fresh.meters_per_roll
        )}）`
      ).toBeNull();
    } finally {
      await tryCleanup(page, 'DELETE', `/products/${id}`, '[106-02] 产品');
    }
  });
});
