import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  genCode,
  failureCode,
  APP_ERROR_CODES,
  type ApiFailureResult,
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
 * 负值拒绝的精确契约断言（收紧原「status>=400 + 三族机器码来者不拒」假绿）：
 * - HTTP 必须恰为 400（backend/utils/error.rs:356-373）；
 * - 化料 B1-B4：chemical_ops/master.rs 数值范围门用 AppError::business 构造
 *   → 机器码必为 BUSINESS_ERROR；
 * - 信用额度 B6：handlers/customer_credit_handler.rs `req.validate()`（validator 框架，
 *   utils/validator.rs::validate_credit_limit_range）经 error.rs
 *   From<validator::ValidationErrors> → VALIDATION_ERROR。
 * 机器码族由每条调用点显式声明；永不读文案（脱敏分层不可判）。
 */
function expectRejected(r: ApiFailureResult, what: string, expectedCode: string): void {
  expect(r.status, `${what}：应恰为 HTTP 400，实际 ${JSON.stringify(r)}`).toBe(400);
  expect(
    failureCode(r),
    `${what}：机器码应为 ${expectedCode}，实际 ${failureCode(r)}（${JSON.stringify(r)}）`
  ).toBe(expectedCode);
}

/**
 * 负例的落库侧证据：建单被拒不能只在响应面上判红，还要证明库里没留下这一行。
 * 读回走按编码查询端点（get_by_code 过滤未删除行，取不到即 AppError::not_found），
 * 被拒的建单不应有任何写入，因此期望恰为 404 加机器码 NOT_FOUND。
 */
async function expectChemicalNotCreated(page: Page, code: string, what: string): Promise<void> {
  const back = await apiCallExpectFail(page, 'GET', `/chemicals/by-code/${code}`);
  expect(
    back.status,
    `${what}：被拒后库里不应留下化料行，按 code=${code} 读回实际 status=${
      back.status
    } message=${back.message ?? '(none)'}`
  ).toBe(404);
  expect(
    failureCode(back),
    `${what}：读回应报 NOT_FOUND 机器码，实际 raw code=${JSON.stringify(back.code)}`
  ).toBe('NOT_FOUND');
}

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

  test('52-B1 化料标准价为负拒绝', async ({ page }) => {
    const code = `52A${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      standard_price: '-1.00',
    });
    // chemical_ops/master.rs 标准价门 → AppError::business
    expectRejected(r, '标准价负数必须拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
    await expectChemicalNotCreated(page, code, '标准价负数建单');
  });

  test('52-B2 化料成本价为负拒绝', async ({ page }) => {
    const code = `52B${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      cost_price: '-0.50',
    });
    // chemical_ops/master.rs 成本价门 → AppError::business
    expectRejected(r, '成本价负数必须拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
    await expectChemicalNotCreated(page, code, '成本价负数建单');
  });

  test('52-B3 化料安全库存为负拒绝', async ({ page }) => {
    const code = `52C${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      safety_stock: '-5',
    });
    // chemical_ops/master.rs 安全库存门 → AppError::business（后端脱敏出参，只判状态+机器码；
    // dye_category 由 chemBase 提供，确保命中的是库存校验分支）
    expectRejected(r, '安全库存负数必须拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
    await expectChemicalNotCreated(page, code, '安全库存负数建单');
  });

  test('52-B4 化料再订货点为负拒绝', async ({ page }) => {
    const code = `52D${genCode('C').slice(-5)}`;
    const r = await apiCallExpectFail(page, 'POST', '/chemicals', {
      ...chemBase(code),
      reorder_point: '-2',
    });
    // chemical_ops/master.rs 再订货点门 → AppError::business
    expectRejected(r, '再订货点负数必须拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
    await expectChemicalNotCreated(page, code, '再订货点负数建单');
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
    // 写后读回真值：详情出参是未裁剪的主数据模型，四个数值列都是 Decimal，
    // JSON 回读为字符串形态，取数值比较；未提交的列必须保持 NULL，不得被填占位值
    const saved = await apiCallRaw<{
      chemical_code?: string;
      chemical_name?: string;
      chemical_type?: string;
      unit?: string | null;
      standard_price?: string | number | null;
      cost_price?: string | number | null;
      safety_stock?: string | number | null;
      reorder_point?: string | number | null;
    }>(page, 'GET', `/chemicals/${id}`);
    expect(saved.chemical_code, `回读的化料编码应等于提交值 ${code}`).toBe(code);
    expect(saved.chemical_type, '化料类别应等于提交值').toBe('dye');
    expect(saved.unit, '计量单位应等于提交值').toBe('kg');
    expect(Number(saved.standard_price), '回读的标准价应等于提交值 10.50').toBe(10.5);
    expect(Number(saved.safety_stock), '回读的安全库存应等于提交值 5').toBe(5);
    // cost_price 与 reorder_point 在实体里是非 Option 的 Decimal 字段，依据 chemical_master 建表 DDL
    // 的 cost_price 与 reorder_point 两列 NOT NULL 且无默认值，实体侧对应字段不带 Option，
    // 列没有"未设"存身空间，未提交时由 validate_chemical_create_input 按 0 落账，因此这里如实锁 0。
    // "没录成本价"与"声明成本价 0"在库里不可分是本仓静默缺省族的在册欠账，
    // 收口方向是缺键点名拒绝（与采购明细行同一口径），那要动建单校验与全部化料建单载荷，另行立项，
    // 不在本用例里替未来行为写期望。
    expect(Number(saved.cost_price), '未提交成本价当前按 0 落库（缺键不拒绝的既有口径）').toBe(0);
    expect(Number(saved.reorder_point), '未提交再订货点当前按 0 落库（同上）').toBe(0);

    // 显式提交金额列要能逐列回读，防"提交被丢"与"落库被改写"两类真回归
    const pricedCode = `52F${genCode('C').slice(-5)}`;
    const priced = await apiCall<{ id?: number }>(page, 'POST', '/chemicals', {
      ...chemBase(pricedCode),
      cost_price: '3.25',
      reorder_point: '12',
    });
    const pricedId = priced?.data?.id;
    expect(pricedId, '显式提交成本价与再订货点的化料应创建成功').toBeTruthy();
    if (pricedId) CLEANUP.push({ path: `/chemicals/${pricedId}`, label: '[52-B5] 带价化料' });
    const pricedSaved = await apiCallRaw<{
      chemical_code?: string;
      cost_price?: string | number | null;
      reorder_point?: string | number | null;
    }>(page, 'GET', `/chemicals/${pricedId}`);
    expect(pricedSaved.chemical_code, '带价样本回读的编码应等于提交值').toBe(pricedCode);
    expect(Number(pricedSaved.cost_price), '回读成本价应等于提交值 3.25').toBe(3.25);
    expect(Number(pricedSaved.reorder_point), '回读再订货点应等于提交值 12').toBe(12);
  });

  test('52-B6 信用额度为负拒绝', async ({ page }) => {
    const r = await apiCallExpectFail(page, 'POST', '/crm/customer-credits', {
      customer_id: 1,
      credit_limit: '-100.00',
    });
    // handlers/customer_credit_handler.rs req.validate()（validator 框架，
    // utils/validator.rs::validate_credit_limit_range）→ From<ValidationErrors>
    // 归 error.rs VALIDATION_ERROR 族（error.rs 模块文档：HTTP 400/code=VALIDATION_ERROR）
    expectRejected(r, '负信用额度必须拒绝', APP_ERROR_CODES.VALIDATION_ERROR);
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
