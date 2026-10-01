// 主数据域全流程契约级 E2E — 26 往来主数据（供应商 / 客户 / 客户信用：建→流转→GET 回读字段值）
//
// **声明：本文件未在本地实跑（本机禁跑 Playwright），仅按后端源码契约编写，待 CI/联调验证。**
//
// 本链证明什么（不再是"页面能打开/能弹 toast"）：
//   1) 供应商创建后**服务端取号与中文默认值真实落库**：supplier_code 由统一生成器取号
//      （前缀 SUP，backend/src/services/supplier_service.rs:42-50），supplier_type 默认
//      "普通供应商"（supplier_service.rs:103-106）、taxpayer_type 默认"一般纳税人"
//      （supplier_service.rs:118-121）——GET 回读逐字符相等；名称唯一性冲突归业务族
//      （supplier_service.rs:83-89 business_displayable）。
//   2) 供应商引用守卫：被采购订单引用时删除必须 400 BUSINESS_ERROR（非裸 500 靠 FK 兜底）
//      （supplier_service.rs:515-521 "该供应商已被N引用"）；解除引用后删除成功、再 GET 404。
//   3) 客户创建契约：编码缺省时服务端生成（前缀 CUS，customer_ops/update.rs:57-65）、
//      缺省 status=active（词表 models/status/general.rs:53 master_data::ACTIVE，写入点
//      customer_ops/update.rs:104-107）、缺省 country="中国"（update.rs:95-97）、
//      payment_terms 缺省 30（customer_handler.rs:299-301 + constants.rs:20）；
//      customer_type 取值校验（customer_handler.rs:74-88 白名单 retail/wholesale/
//      distributor/manufacturer/other）越界归 VALIDATION_ERROR；编码重复归业务族
//      （customer_ops/crud.rs:47-49）。
//   4) 客户删除是**软删除状态机**：DELETE 后记录仍在（status=inactive，词表
//      general.rs:56，写入点 customer_ops/crud.rs:178），重复 DELETE 被状态门拒绝
//      （crud.rs:170-175）。断"404"即假绿。
//   5) 客户信用额度账本真实运算：set→available=limit-used；occupy 后 used/available
//      同事务增减（customer_credit_limit.rs:120-121）；超额占用拒绝（:108-114）、
//      超额释放拒绝（:156-161）、非 active 状态占用拒绝（:102-105）、deactivate 落
//      status=inactive（:331，词表 general.rs:56）。金额被拒后**零漂移回读**。
//
// CI 测不到（显式声明）：
//   - 供应商资质审批门（supplier_qualification_gate）与 ES 同步（sync_customer_to_es 为
//     best-effort，ES 失败仅日志）；
//   - 非管理员 PII 脱敏分支（mask_contact_fields_for_role 依赖 role_id!=1 判定，本链只用
//     admin 会话断言非 PII 字段，脱敏契约由 29 权限链专测）；
//   - 客户信用与订单占用的事务内 TOCTOU 联动（P2 3-20 属后端集成测试范畴）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  genName,
  tryCleanup,
  expectBusinessRejection,
  APP_ERROR_CODES,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** AppError 机器码取源 flow/helpers 的 APP_ERROR_CODES（NOT_FOUND 登记依据 utils/error.rs:739） */
const ERR_NOT_FOUND = APP_ERROR_CODES.NOT_FOUND;

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/** Decimal 出参为十进制字符串（rust_decimal serde 默认），按数值比较，字符串形状仍校验 */
function expectDecimal(actual: unknown, expected: number, label: string): void {
  expect(
    typeof actual === 'string' || typeof actual === 'number',
    `${label}：期望 Decimal 序列化（string|number），实际=${JSON.stringify(actual)}`
  ).toBe(true);
  expect(Number(actual), `${label}：数值不符，实际=${JSON.stringify(actual)}`).toBe(expected);
}

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

/** API 建最小合法 PO（DRAFT），返回 id；用于供应商引用守卫 */
async function seedDraftPo(page: Page, supplierId: number): Promise<number> {
  const ctx = getCtx();
  const po = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/orders', {
    supplier_id: supplierId,
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    order_date: todayStr(),
    notes: `E2E-F26-REF-${genCode('PO')}`,
    items: [{ material_id: ctx.productIds[0], quantity_ordered: '10', unit_price: '1.00' }],
  });
  const poId = requireNum(po.id, '建 DRAFT PO');
  CLEANUP.push({ path: `/purchase/orders/${poId}`, label: `purchase_order#${poId}` });
  return poId;
}

test.describe('26 往来主数据契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('26-01 供应商：服务端取号 SUP + 中文默认值逐字符落库回读；名称重复 400 BUSINESS', async ({
    page,
  }) => {
    const name = genName('F26供');
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/suppliers', {
      supplier_name: name,
      supplier_short_name: 'E2F26',
      contact_phone: '13800000026',
    });
    const supplierId = requireNum(created.id, '建供应商');
    CLEANUP.push({ path: `/purchase/suppliers/${supplierId}`, label: `supplier#${supplierId}` });

    // 回读：取号前缀与中文默认值（写入点 supplier_service.rs:42-50/:103-106/:118-121）
    const back = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/suppliers/${supplierId}`
    );
    expect(String(back.supplier_code ?? ''), 'supplier_code 服务端取号前缀 SUP').toMatch(/^SUP/);
    expect(back.supplier_name, '名称回读').toBe(name);
    expect(back.supplier_type, '缺省供应商类型（中文业务 token，禁止被改写/英文化）').toBe(
      '普通供应商'
    );
    expect(back.taxpayer_type, '缺省纳税人类型').toBe('一般纳税人');

    // 负例：名称唯一性 = 业务族（400 BUSINESS_ERROR，非 5xx）
    const dup = await apiCallExpectFail(page, 'POST', '/purchase/suppliers', {
      supplier_name: name,
      supplier_short_name: 'E2F26B',
      contact_phone: '13800000027',
    });
    expect(dup.status, '名称重复应 400').toBe(400);
    expect(failureCode(dup), '重复名称机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(typeof dup.message === 'string' && dup.message.trim().length > 0, '拒绝原因非空').toBe(
      true
    );
  });

  test('26-02 供应商引用守卫：被 PO 引用删除 400 BUSINESS；解除引用后删除成功、回读 404', async ({
    page,
  }) => {
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/suppliers', {
      supplier_name: genName('F26引用'),
      supplier_short_name: 'E2F26R',
      contact_phone: '13800000028',
    });
    const supplierId = requireNum(created.id, '建供应商');
    CLEANUP.push({ path: `/purchase/suppliers/${supplierId}`, label: `supplier#${supplierId}` });

    const poId = await seedDraftPo(page, supplierId);

    const blocked = await apiCallExpectFail(page, 'DELETE', `/purchase/suppliers/${supplierId}`);
    expectBusinessRejection(
      blocked,
      `供应商 ${supplierId} 被 PO ${poId} 引用，删除应 400 业务拒绝`
    );

    // 供应商必须仍然存在（守卫不是"删一半"）
    const still = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/suppliers/${supplierId}`
    );
    expect(Number(still.id), '被拒后供应商仍存在').toBe(supplierId);

    // 解除引用 → 删除成功 → GET 404 NOT_FOUND
    await apiCall(page, 'DELETE', `/purchase/orders/${poId}`);
    await apiCall(page, 'DELETE', `/purchase/suppliers/${supplierId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/purchase/suppliers/${supplierId}`);
    expect(gone.status, '删除后应 404').toBe(404);
    expect(failureCode(gone), '404 机器码').toBe(ERR_NOT_FOUND);
  });

  test('26-03 客户创建契约：CUS 取号 + active/中国/payment_terms=30 默认值回读；非法枚举与格式 VALIDATION', async ({
    page,
  }) => {
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/crm/customers', {
      customer_name: genName('F26客'),
    });
    const customerId = requireNum(created.id, '建客户');
    CLEANUP.push({ path: `/crm/customers/${customerId}`, label: `customer#${customerId}` });

    const back = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}`
    );
    expect(String(back.customer_code ?? ''), 'customer_code 服务端取号前缀 CUS').toMatch(/^CUS/);
    // 词表 models/status/general.rs:53（master_data::ACTIVE，小写）；写入点 customer_ops/update.rs:104-107
    expect(back.status, '新客默认状态 active（小写词表）').toBe('active');
    expect(back.country, '缺省国家（update.rs:95-97）').toBe('中国');
    expect(Number(back.payment_terms), '缺省账期 30（constants.rs:20）').toBe(30);
    expect(back.customer_type, '缺省客户类型 retail（customer_handler.rs:276-278）').toBe('retail');
    expectDecimal(back.credit_limit, 0, '缺省信用额度（handler 未传置 ZERO，:267-274）');

    // 负例 1：customer_type 越界 = 取值族 VALIDATION_ERROR（customer_handler.rs:74-88 白名单）
    const badType = await apiCallExpectFail(page, 'POST', '/crm/customers', {
      customer_name: genName('F26非法'),
      customer_type: 'VIP_GOLD',
    });
    expect(badType.status, '非法客户类型应 400').toBe(400);
    expect(failureCode(badType), '非法枚举机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 负例 2：credit_limit 非数字串 = 取值族（customer_handler.rs:269-274 P2-1）
    const badLimit = await apiCallExpectFail(page, 'POST', '/crm/customers', {
      customer_name: genName('F26额度'),
      credit_limit: '十万',
    });
    expect(badLimit.status, '非法信用额度应 400').toBe(400);
    expect(failureCode(badLimit), '格式非法机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 负例 3：手工重复已存在的编码 = 业务族（customer_ops/crud.rs:47-49）
    const dupCode = await apiCallExpectFail(page, 'POST', '/crm/customers', {
      customer_name: genName('F26重码'),
      customer_code: String(back.customer_code),
    });
    expect(dupCode.status, '重复编码应 400').toBe(400);
    expect(failureCode(dupCode), '重复编码机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('26-04 客户软删除状态机：DELETE 后 status=inactive 可回读，重复 DELETE 被状态门拒绝', async ({
    page,
  }) => {
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/crm/customers', {
      customer_name: genName('F26软删'),
    });
    const customerId = requireNum(created.id, '建客户');

    await apiCall(page, 'DELETE', `/crm/customers/${customerId}`);

    // 软删除契约（customer_ops/crud.rs:154-199）：记录仍在、状态词表值 inactive（general.rs:56）
    const after = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}`
    );
    expect(Number(after.id), '软删除后记录仍可定位（不是 404）').toBe(customerId);
    expect(after.status, '软删除写 status=inactive').toBe('inactive');

    const twice = await apiCallExpectFail(page, 'DELETE', `/crm/customers/${customerId}`);
    expect(twice.status, '重复软删除应 400').toBe(400);
    expect(failureCode(twice), '重复删除机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const stillInactive = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}`
    );
    expect(stillInactive.status, '被拒后仍 inactive（零漂移）').toBe('inactive');
  });

  test('26-05 客户信用额度账本：set/occupy/release/deactivate 金额逐分回读，超限与非法态一律 400 且零漂移', async ({
    page,
  }) => {
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/crm/customers', {
      customer_name: genName('F26信用'),
    });
    const customerId = requireNum(created.id, '建客户');
    CLEANUP.push({ path: `/crm/customers/${customerId}`, label: `customer#${customerId}` });

    // 建额度 10000（POST /crm/customer-credits 走 set_credit_rating，
    // 初始 available=limit、used=0、status=active：customer_credit_limit.rs:57-66 + general.rs:53）
    await apiCall(page, 'POST', '/crm/customer-credits', {
      customer_id: customerId,
      credit_level: 'A',
      credit_limit: '10000',
    });
    let credit = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}/credit`
    );
    expectDecimal(credit.credit_limit, 10000, '初始额度');
    expectDecimal(credit.used_credit, 0, '初始占用');
    expectDecimal(credit.available_credit, 10000, '初始可用');
    expect(credit.status, '初始状态 active（general.rs:53）').toBe('active');

    // occupy 3000 → used/available 同事务变更（customer_credit_limit.rs:120-121）
    await apiCall(page, 'POST', `/crm/customer-credits/${customerId}/occupy`, {
      amount: '3000',
    });
    credit = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}/credit`
    );
    expectDecimal(credit.used_credit, 3000, '占用后 used');
    expectDecimal(credit.available_credit, 7000, '占用后 available');

    // 负例 1：超额占用（>available）= 业务族（:108-114），拒绝后零漂移
    const overOcc = await apiCallExpectFail(
      page,
      'POST',
      `/crm/customer-credits/${customerId}/occupy`,
      { amount: '99999' }
    );
    expect(overOcc.status, '超额占用应 400').toBe(400);
    expect(failureCode(overOcc), '超额占用机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    credit = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}/credit`
    );
    expectDecimal(credit.used_credit, 3000, '超额占用被拒后 used 零漂移');
    expectDecimal(credit.available_credit, 7000, '超额占用被拒后 available 零漂移');

    // 负例 2：释放 > used = 业务族（:156-161 displayable 文案直传，仅断非空）
    const overRel = await apiCallExpectFail(
      page,
      'POST',
      `/crm/customer-credits/${customerId}/release`,
      { amount: '5000' }
    );
    expect(overRel.status, '超额释放应 400').toBe(400);
    expect(failureCode(overRel), '超额释放机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 负例 3：amount 取值非法（validate_amount_range）= VALIDATION_ERROR
    const negAmt = await apiCallExpectFail(
      page,
      'POST',
      `/crm/customer-credits/${customerId}/occupy`,
      { amount: '-1' }
    );
    expect(negAmt.status, '负金额应 400').toBe(400);
    expect(failureCode(negAmt), '负金额机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 释放 3000 → 回到初始账
    await apiCall(page, 'POST', `/crm/customer-credits/${customerId}/release`, { amount: '3000' });
    credit = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}/credit`
    );
    expectDecimal(credit.used_credit, 0, '释放后 used 归零');
    expectDecimal(credit.available_credit, 10000, '释放后 available 复原');

    // deactivate → status=inactive（customer_credit_limit.rs:331，general.rs:56）；
    // 停用后占用被拒（:102-105 非活跃门）
    await apiCall(page, 'POST', `/crm/customer-credits/${customerId}/deactivate`);
    credit = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/crm/customers/${customerId}/credit`
    );
    expect(credit.status, 'deactivate 写 inactive').toBe('inactive');
    const occInactive = await apiCallExpectFail(
      page,
      'POST',
      `/crm/customer-credits/${customerId}/occupy`,
      { amount: '100' }
    );
    expect(occInactive.status, '停用后占用应 400').toBe(400);
    expect(failureCode(occInactive), '停用后占用机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });
});
