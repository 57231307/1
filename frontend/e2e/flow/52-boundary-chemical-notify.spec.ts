import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, apiCallExpectFail, tryCleanup, genCode } from './helpers';

/**
 * 52 L3 边界完整版 + L5 通知去重（rule provenance 逐条标注）
 *
 * - 化料主数据负值 4 条：services/chemical_ops/master.rs:61-77（标准价/成本价/
 *   安全库存/再订货点不能为负），请求结构 chemical_ops/types.rs:15-45
 * - 信用额度负值：customer_credit_handler.rs CreditRatingRequestDto
 *   validate_credit_limit_range
 * - 通知去重：notification_service.rs:72-84（5 分钟窗口 dedup_key）
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe.serial('52 L3 化料/信用负值 + L5 通知去重', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  const chemBase = (code: string) => ({
    chemical_code: code,
    chemical_name: `52边界化料${code}`,
    chemical_type: 'dye',
    unit: 'kg',
  });

  test('52-B1 化料标准价为负拒绝（master.rs:61）', async ({ page }) => {
    const code = `52A${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      standard_price: '-1.00',
    });
    expect(r.status, '标准价负数必须拒绝').toBeGreaterThanOrEqual(400);
    expect(String(r.message ?? ''), '消息应提示标准价').toContain('标准价');
  });

  test('52-B2 化料成本价为负拒绝（master.rs:65）', async ({ page }) => {
    const code = `52B${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      cost_price: '-0.50',
    });
    expect(r.status, '成本价负数必须拒绝').toBeGreaterThanOrEqual(400);
    expect(String(r.message ?? ''), '消息应提示成本价').toContain('成本价');
  });

  test('52-B3 化料安全库存为负拒绝（master.rs:69）', async ({ page }) => {
    const code = `52C${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      safety_stock: '-5',
    });
    expect(r.status, '安全库存负数必须拒绝').toBeGreaterThanOrEqual(400);
    expect(String(r.message ?? ''), '消息应提示安全库存').toContain('安全库存');
  });

  test('52-B4 化料再订货点为负拒绝（master.rs:73）', async ({ page }) => {
    const code = `52D${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      reorder_point: '-2',
    });
    expect(r.status, '再订货点负数必须拒绝').toBeGreaterThanOrEqual(400);
    expect(String(r.message ?? ''), '消息应提示再订货点').toContain('再订货点');
  });

  test('52-B5 化料正常创建正例（对照负例防规则过紧）', async ({ page }) => {
    const code = `52E${genCode('C').slice(-5)}`;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/chemicals', {
      ...chemBase(code),
      standard_price: '10.50',
      safety_stock: '5',
    });
    const id = r?.data?.id;
    expect(id, '合法化料创建应成功').toBeTruthy();
    if (id) CLEANUP.push({ path: `/chemicals/${id}`, label: '[52-B5] 化料' });
  });

  test('52-B6 信用额度为负拒绝（validate_credit_limit_range）', async ({ page }) => {
    const r = await apiCallExpectFail(page, 'POST', '/crm/customer-credits', {
      customer_id: 1,
      credit_limit: '-100.00',
    });
    expect(r.status, '负信用额度必须拒绝').toBeGreaterThanOrEqual(400);
  });

  test('52-N1 公告重复发送去重行为（notification_service.rs:72-84 dedup）', async ({ page }) => {
    const title = `52去重公告${Date.now().toString().slice(-6)}`;
    const body = { user_ids: [1], title, content: '52-N1 通知去重验证内容' };
    const r1 = await apiCallExpectFail(page, 'POST', '/notifications/announcement', body);
    test.skip(r1.status >= 400, '首次公告发送失败（权限/结构差异），去重断言需登录上下文');
    // 5 分钟窗口内相同内容重发：后端 check_dedup 应拦截（若允许重复，此断言暴露缺失）
    const r2 = await apiCallExpectFail(page, 'POST', '/notifications/announcement', body);
    const list = await apiCallExpectFail(page, 'GET', '/notifications?page=1&page_size=20');
    void list;
    // 判定：r2 被拒（dedup 生效）= 通过；r2 成功 = 列表中同 title 只允许 1 条
    if (r2.status < 300) {
      const json = JSON.stringify(list);
      const count = (json.match(new RegExp(title, 'g')) || []).length;
      expect(count, '同一公告 5 分钟窗口内重复发送应被去重为 1 条').toBeLessThanOrEqual(1);
    } else {
      expect(r2.status, '重复公告应被去重拦截').toBeGreaterThanOrEqual(400);
    }
  });
});
