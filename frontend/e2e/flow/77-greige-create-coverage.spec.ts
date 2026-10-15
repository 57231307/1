import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw, apiCallExpectFail } from './helpers';

/**
 * BUG-009 坯布建档覆盖 + 外键预检回归
 *
 * 锁三件事（均以 e2e_admin 会话，鉴权旁路，聚焦功能与契约真实态）：
 * - 供应商演示种子（m0015）可见：GET /purchase/suppliers 的 items 含演示供应商，证明"供应商无匹配"
 *   并非源自身无数据。
 * - 坯布建档经 API 真实落库并可读回 fabric_type（前端必填项与后端默认链一致）。
 * - 脏 supplier_id 建档返回 4xx（预检为业务错误）而非裸 5xx——锁 create_greige_fabric 的
 *   supplier_id/warehouse_id 外键预检修复。
 */
const suffix = Date.now().toString().slice(-6);

test.describe('坯布建档持久化与外键预检', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('供应商演示种子可见', async ({ page }) => {
    const res = await apiCallRaw<{ items?: Array<Record<string, unknown>> }>(
      page,
      'GET',
      '/purchase/suppliers?page=1&page_size=50'
    );
    const items = res?.items ?? [];
    expect(
      Array.isArray(items),
      `供应商列表 items 应为数组：${JSON.stringify(res).slice(0, 120)}`
    ).toBe(true);
    expect(
      items.some(
        s => s.supplier_code === 'SUP-DEMO-FAB-01' || s.supplier_code === 'SUP-DEMO-FAB-02'
      ),
      'm0015 应种入演示供应商，admin 会话下拉非空'
    ).toBe(true);
  });

  test('坯布建档落库并读回 fabric_type', async ({ page }) => {
    const sups = await apiCallRaw<{ items?: Array<Record<string, unknown>> }>(
      page,
      'GET',
      '/purchase/suppliers?page=1&page_size=1'
    );
    const supplierId = Number(sups?.items?.[0]?.id);
    expect(supplierId, '应能取到一个真实供应商 id 作来源候选').toBeGreaterThan(0);

    const created = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/greige-fabrics',
      {
        fabric_name: `e2e-gf-${suffix}`,
        fabric_type: '坯布',
        supplier_id: supplierId,
      }
    );
    const id = Number(created?.id);
    expect(id, `坯布建档应返回主键：${JSON.stringify(created).slice(0, 160)}`).toBeGreaterThan(0);
    try {
      const detail = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/greige-fabrics/${id}`
      );
      expect(
        detail?.fabric_type,
        `读回应持久化 fabric_type：${JSON.stringify(detail).slice(0, 160)}`
      ).toBe('坯布');
    } finally {
      // 在库坯布按设计不允许直接删除（须先出库），清理失败不判红，仅尽力回收。
      await apiCallRaw(page, 'DELETE', `/production/greige-fabrics/${id}`).catch(() => undefined);
    }
  });

  test('脏 supplier_id 建档返回 4xx 而非 5xx（外键预检）', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/production/greige-fabrics', {
      fabric_name: `e2e-gf-bad-${suffix}`,
      fabric_type: '坯布',
      supplier_id: 99999999,
    });
    expect(
      fail.status,
      `不存在的 supplier_id 应被预检为业务错误 4xx，实际 ${fail.status}（裸 FK 违例会是 500）`
    ).toBeLessThan(500);
  });
});
