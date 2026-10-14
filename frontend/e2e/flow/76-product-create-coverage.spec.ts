import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw } from './helpers';
import { createProductUI } from './ui-helpers';

/**
 * BUG-008 产品新建覆盖
 *
 * 锁两件事：
 * - 纯新装库启动种子补分类后，分类下拉经 GET /product-categories 恒非空（分类空是被必填拦截、
 *   表现为保存失败的真因，本断言防其回归）。
 * - 通过既有 createProductUI 走真实 UI 金路径新建产品并成功返回主键，再 API 读回持久字段。
 *   （createProductUI 此前无任何 spec 调用 = 产品 UI 建档零覆盖的假绿缺口，本 spec 激活它。）
 */
test.describe('产品新建持久化与分类种子', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('分类下拉数据源非空（启动种子回归锁）', async ({ page }) => {
    const cats = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/product-categories'
    );
    expect(
      Array.isArray(cats),
      `分类列表 data 应为数组：${JSON.stringify(cats).slice(0, 120)}`
    ).toBe(true);
    expect(
      cats.length,
      '纯新装库须由启动种子补入坯布/成品布/辅料分类；为空则产品建档必填 category_id 无法选中'
    ).toBeGreaterThan(0);
  });

  test('UI 新建产品金路径成功并读回持久', async ({ page }) => {
    const id = await createProductUI(page);
    expect(
      id,
      'createProductUI 应经 UI 成功建产品并返回主键；返回 undefined 表示分类下拉或提交失败'
    ).toBeGreaterThan(0);

    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${id}`);
    expect(Number(detail?.id), `读回详情 id 应为 ${id}`).toBe(id);
    expect(
      detail?.category_id,
      `UI 建的产品应落库 category_id（证明分类下拉确有候选被选中）：${JSON.stringify(detail).slice(0, 160)}`
    ).toBeTruthy();

    await apiCallRaw(page, 'DELETE', `/products/${id}`);
  });
});
