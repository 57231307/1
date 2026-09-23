import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  tryCleanup,
  genCode,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';

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

/**
 * 负例允许的 AppError 拒绝机器码（backend/src/utils/error.rs:461-476 error_code()）。
 * 原先各处写 `String(r.code ?? '')` 配正则：既绕过类型收窄（数字码会被误当字符串），
 * 又把机器码字面量散在 4 个用例里。改为对白名单做精确成员判断。
 */
const APP_REJECT_CODES: string[] = [
  APP_ERROR_CODES.VALIDATION_ERROR,
  APP_ERROR_CODES.BUSINESS_ERROR,
  APP_ERROR_CODES.BAD_REQUEST,
];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe.serial('52 L3 化料/信用负值 + L5 通知去重', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // master.rs:53-54：chemical_type=dye 时必须提供 dye_category，否则在价格/库存负值校验
  // 之前就返回"染料类型必须提供 dye_category"，B3/B4 的 message 断言会拿到错行。
  const chemBase = (code: string) => ({
    chemical_code: code,
    chemical_name: `52边界化料${code}`,
    chemical_type: 'dye',
    dye_category: 'reactive',
    unit: 'kg',
  });

  test('52-B1 化料标准价为负拒绝（master.rs:61）', async ({ page }) => {
    const code = `52A${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      standard_price: '-1.00',
    });
    expect(r.status, '标准价负数必须拒绝').toBeGreaterThanOrEqual(400);
    // 后端 public_message 脱敏，改断言 code
    expect(APP_REJECT_CODES, 'code 应为校验/业务/请求类机器码').toContain(failureCode(r));
  });

  test('52-B2 化料成本价为负拒绝（master.rs:65）', async ({ page }) => {
    const code = `52B${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      cost_price: '-0.50',
    });
    expect(r.status, '成本价负数必须拒绝').toBeGreaterThanOrEqual(400);
    expect(APP_REJECT_CODES, 'code 应为校验/业务/请求类机器码').toContain(failureCode(r));
  });

  test('52-B3 化料安全库存为负拒绝（master.rs:69）', async ({ page }) => {
    const code = `52C${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      safety_stock: '-5',
    });
    expect(r.status, '安全库存负数必须拒绝').toBeGreaterThanOrEqual(400);
    // 后端 HTTP 响应统一脱敏（utils/error.rs:95-96），business 文案只有"业务处理失败"，
    // 断言 code 而非 message（与 B1/B2 一致）；dye_category 由 chemBase 提供，确保命中的是库存校验分支
    expect(APP_REJECT_CODES, 'code 应为校验/业务/请求类机器码').toContain(failureCode(r));
  });

  test('52-B4 化料再订货点为负拒绝（master.rs:73）', async ({ page }) => {
    const code = `52D${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      reorder_point: '-2',
    });
    expect(r.status, '再订货点负数必须拒绝').toBeGreaterThanOrEqual(400);
    expect(APP_REJECT_CODES, 'code 应为校验/业务/请求类机器码').toContain(failureCode(r));
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

  test('52-N1 公告发送契约与去重语义（create_announcement / notification_service dedup）', async ({
    page,
  }) => {
    // 后端真相：POST /notifications/announcement → create_announcement（analytics.rs notifications
    //   nest 在 /api/v1/erp/notifications 下）。仅管理员可发（is_admin_role），入参
    //   CreateAnnouncementRequest{user_ids 非空, title, content}，返回 AnnouncementResult{delivered_count}。
    // 去重真相：create_announcement → send_system_announcement 传 dedup_key: None
    //   （event_notification_service.rs:786），而 notification_service.rs:107-114 的 5 分钟去重
    //   仅在带 dedup_key 时生效——公告从不走该路径。旧用例"重复发送应被去重为 1 条"的前提是错的，
    //   且首发送失败即 test.skip 属条件 skip 假绿。此处按后端真相判红/判绿、不再 skip：
    //   (1) 首次发送成功且 delivered_count == 目标用户数（失败即 apiCall 抛真实 code/message）；
    //   (2) 相同内容二次发送同样成功（公告不走去重是既定契约），若后端将来改为去重，此断言以真实响应暴露。
    const me = await apiCall<{ id: number }>(page, 'GET', '/auth/me');
    expect(Number(me?.data?.id), `未取得当前用户 id：${JSON.stringify(me)}`).toBeGreaterThan(0);
    const uid = me.data.id;
    const title = `52公告契约${Date.now().toString().slice(-6)}`;
    const body = { user_ids: [uid], title, content: '52-N1 公告发送契约验证内容' };

    const r1 = await apiCall<{ delivered_count: number }>(
      page,
      'POST',
      '/notifications/announcement',
      body
    );
    expect(
      Number(r1.data?.delivered_count),
      `首次公告应投递给 1 个目标用户（失败会由 apiCall 抛出真实响应），实际：${JSON.stringify(r1)}`
    ).toBe(1);

    const r2 = await apiCall<{ delivered_count: number }>(
      page,
      'POST',
      '/notifications/announcement',
      body
    );
    expect(
      Number(r2.data?.delivered_count),
      `公告按契约不走去重（send_system_announcement 传 dedup_key:None），二次发送应同样成功，实际：${JSON.stringify(r2)}`
    ).toBe(1);
  });
});
