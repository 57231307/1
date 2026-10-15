import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw } from './helpers';

/**
 * BUG-008 产品新建覆盖
 *
 * 锁两件事：
 * - 纯新装库启动种子补分类后，分类下拉经 GET /product-categories 恒非空（分类空是被必填拦截、
 *   表现为保存失败的真因，本断言防其回归）。
 * - 用已种入分类作必填来源经 API 建产品并读回 category_id/unit 持久（产品建档金路径落库回归）。
 */
test.describe('产品新建持久化与分类种子', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('分类下拉数据源非空（启动种子回归锁）', async ({ page }) => {
    const res = await apiCallRaw<{ items?: Array<Record<string, unknown>> }>(
      page,
      'GET',
      '/product-categories'
    );
    const cats = res?.items ?? [];
    expect(Array.isArray(cats), `分类列表 items 应为数组：${JSON.stringify(res).slice(0, 120)}`).toBe(
      true
    );
    expect(
      cats.length,
      '纯新装库须由启动种子补入坯布/成品布/辅料分类；为空则产品建档必填 category_id 无法选中'
    ).toBeGreaterThan(0);
  });

  test('API 新建产品落库并读回持久（分类必填链）', async ({ page }) => {
    const cats = await apiCallRaw<{ items?: Array<Record<string, unknown>> }>(
      page,
      'GET',
      '/product-categories'
    );
    const categoryId = Number(cats?.items?.[0]?.id);
    expect(categoryId, '须有已种入的分类作建档必填来源').toBeGreaterThan(0);

    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/products', {
      name: `e2e-prod-${Date.now().toString().slice(-6)}`,
      category_id: categoryId,
      unit: '米',
    });
    const id = Number(created?.id);
    expect(id, `产品建档应返回主键：${JSON.stringify(created).slice(0, 160)}`).toBeGreaterThan(0);
    try {
      const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${id}`);
      expect(Number(detail?.category_id), `读回 category_id 应为 ${categoryId}`).toBe(categoryId);
      expect(detail?.unit, `读回 unit 应为 米：${JSON.stringify(detail).slice(0, 160)}`).toBe('米');
    } finally {
      await apiCallRaw(page, 'DELETE', `/products/${id}`).catch(() => undefined);
    }
  });
});
