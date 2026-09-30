import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  ensureBudgetPlan,
  getCtx,
  genCode,
  tryCleanup,
  verifyEndpointHealthy,
  failureCode,
  APP_ERROR_CODES,
  type ApiFailureResult,
} from './helpers';

/**
 * 67 业务拒绝的文案对用户可见（提交被拒时，用户必须看到真实拒绝原因）
 *
 * 出参契约（backend/src/utils/error.rs，逐行核实）：
 * - 失败信封唯一形状 {code,message,trace_id,timestamp}（error.rs:431-436 ErrorResponse）；
 * - AppError::business / BusinessErrorDisplayable 的 code 均为 BUSINESS_ERROR（error.rs:475-477），
 *   但 message 只有 `business_displayable` 才外显真实文案（public_message）；
 * - 输入校验族同构：`ValidationError` 出参被脱敏为固定常量 "请求参数验证失败"
 *   （utils/messages.rs:41），而 `ValidationErrorDisplayable`（构造函数
 *   `AppError::validation_displayable`，或由 `impl From<validator::ValidationErrors>` 从
 *   DTO `#[validate]` 结果提取可读文案）code/status 与脱敏形态完全一致
 *   （400 / VALIDATION_ERROR），唯一区别是 message 携带真实拒绝原因；
 * - 两条 displayable 链路的 HTTP 状态同为 400。
 *
 * 本 spec 的判据 = 族级"可见性"锚点：HTTP 恰 400 + code ∈ {BUSINESS_ERROR, VALIDATION_ERROR}
 * （两者都是"显式声明才外显"的白名单构造点）+ message 含**具体拒绝原因**（对过源码中构造该
 * 文案的确切字符串），且必须 ≠ 两个脱敏常量。
 * 若某构造点仍走脱敏链路（`AppError::business` / 手工 `AppError::validation`），
 * **保留断言让它判红**并在交付报告点名 file:line，禁止放宽为"只要 4xx 就绿"。
 *
 * 用例 ↔ 构造点核实：
 * 67-01 信用调整金额 0：POST /crm/customer-credits/{id}/adjust，
 *   amount=0 命中 validate_amount_range"金额必须为正且不超过10亿"（utils/validator.rs:10-16）；
 *   handler `req.validate()?`（customer_credit_handler.rs:225）→ From<ValidationErrors>
 *   → ValidationErrorDisplayable，出参 code=VALIDATION_ERROR + 真实文案 ⇒ 【应判绿】
 *   （修复前该链路整体 to_string 后进脱敏常量，用户只看到"请求参数验证失败"）。
 * 67-02 采购明细允差 150（创建路径）：POST /purchase/orders，行级 [0,100] 校验
 *   "交货允差百分比(quantity_tolerance_pct)必须在0~100之间"（services/po/mod.rs:196-204）；
 *   handler `req.validate()?`（purchase_order_handler.rs:180）→ 同上 ⇒ 【应判绿】。
 * 67-03 采购明细允差 150（更新路径）：PUT /purchase/orders/{id}/items/{item_id} 走
 *   UpdateOrderItemRequest::validate_write → business_displayable 如实外显
 *   （services/po/mod.rs:172-191；handler:419-422）⇒ 【预计判绿】。
 * 67-04 库存直建·染色布缺缸号：POST /inventory/stock，判定权威
 *   services/inv/fabric_class.rs:54-58 "染色布必须提供缸号（color_no=… 但 dye_lot_no 为空）"，
 *   经 handlers/inventory_stock_handler_fabric.rs:146-156 admit_stock_fabric_trace 转
 *   business_displayable ⇒ 【预计判绿】。
 * 67-05 库存直建(fabric)·白坯缺批次：POST /inventory/stock/fabric，batch_no:'' 先撞
 *   DTO length(min=1)（inventory_stock_handler_dto.rs:17-18）。修复前该处手工
 *   `map_err(|e| AppError::validation(e.to_string()))` 把原因压成脱敏常量，永远到不了
 *   fabric_class 的可见文案（"明细缺少批号…批次不得为空"，fabric_class.rs:50-52）；
 *   现改走 `map_err(AppError::from)` 的可读外显链路 ⇒ 【应判绿，断的是 DTO 层文案】。
 * 67-06 供应商资质日期倒挂：POST /purchase/suppliers/{id}/qualifications，
 *   check_qualification_dates → business_displayable("资质「有效期至」不能早于发证日期")
 *   （services/supplier_service.rs:893-903）⇒ 【预计判绿】。
 *
 * 假绿防线：拒绝后一律回读——断言"半行未落库/原值未被改动"（被拒的写必须无痕）；
 * 读回端点 strict verifyEndpointHealthy；数值断言 Number() 归一；自建自流转数据。
 */

/** 两个脱敏常量原文（backend/src/utils/messages.rs:41/43），断言 message 必须≠它们 */
const SANITIZED_BUSINESS = '业务处理失败';
const SANITIZED_VALIDATION = '请求参数验证失败';

function expectVisibleBusinessRejection(
  r: ApiFailureResult,
  phrase: string,
  context: string
): void {
  const detail = `${context}：实际 status=${r.status} code=${failureCode(r) ?? JSON.stringify(r.code)} message=${JSON.stringify(r.message)}`;
  expect(r.status, `${detail}（业务拒绝的正确契约是 HTTP 400，非 5xx 裸崩/非 200 假成功）`).toBe(
    400
  );
  expect(
    [APP_ERROR_CODES.BUSINESS_ERROR, APP_ERROR_CODES.VALIDATION_ERROR],
    `${detail}（code 必须是 BUSINESS_ERROR 或 VALIDATION_ERROR——二者是"显式声明才外显"的
两条白名单构造点 business_displayable / validation_displayable；出现别的 code 说明拒绝既没走
业务外显也没走校验外显，正是"提交报参数错误但用户看不到原因"的族级回归）`
  ).toContain(failureCode(r));
  expect(
    typeof r.message === 'string' && r.message.trim().length > 0,
    `${detail}（message 必须非空）`
  ).toBe(true);
  const msg = String(r.message ?? '');
  expect(msg, `${detail}（message 不得是被脱敏的固定常量"业务处理失败"）`).not.toBe(
    SANITIZED_BUSINESS
  );
  expect(msg, `${detail}（message 不得是被脱敏的固定常量"请求参数验证失败"）`).not.toBe(
    SANITIZED_VALIDATION
  );
  expect(
    msg,
    `${detail}（message 应含具体拒绝原因"${phrase}"——用户看得懂的拒绝理由，禁止放宽此断言）`
  ).toContain(phrase);
}

type Row = Record<string, unknown>;

function requireArray(data: unknown, endpoint: string): Row[] {
  expect(
    Array.isArray(data),
    `${endpoint}：出参 data 必须为数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return data as Row[];
}

function requireItemsEnvelope(data: unknown, endpoint: string): { items: Row[]; total: number } {
  const env = data as { items?: unknown; total?: unknown };
  expect(
    Array.isArray(env?.items),
    `${endpoint}：分页唯一形状 {items,total,page,page_size}，items 必须为数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  expect(typeof env?.total, `${endpoint}：total 必须为数字，实际 ${JSON.stringify(data)}`).toBe(
    'number'
  );
  return { items: env.items as Row[], total: Number(env.total) };
}

async function seedCustomer(page: Page, tag: string): Promise<number> {
  const code = genCode('E2E67C');
  const res = await apiCall<Row>(page, 'POST', '/crm/customers', {
    customer_code: code,
    customer_name: `67号${tag}客户_${code}`,
    contact_phone: '13800006701',
  });
  const id = Number(res.data?.id);
  expect(id, `[67] 客户创建应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

async function seedSupplier(page: Page, tag: string): Promise<number> {
  const code = genCode('E2E67SUP');
  const res = await apiCall<Row>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `67号${tag}供应商_${code}`,
    supplier_short_name: '67供',
    contact_phone: '13800006702',
  });
  const id = Number(res.data?.id);
  expect(id, `[67] 供应商创建应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

async function seedProduct(page: Page, tag: string): Promise<number> {
  const code = genCode('E2E67P');
  const res = await apiCall<Row>(page, 'POST', '/products', {
    code,
    name: `67号${tag}布_${code}`,
    unit: 'm',
  });
  const id = Number(res.data?.id);
  expect(id, `[67] 产品创建应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

const today = (): string => new Date().toISOString().slice(0, 10);

test.describe.serial('67 业务拒绝文案可见性（被拒不等于一片脱敏常量）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('67-01 信用调整金额 0：拒绝原因"金额必须为正"必须对用户可见（校验外显链路）', async ({
    page,
  }) => {
    const customerId = await seedCustomer(page, '额度');
    await apiCall<Row>(page, 'POST', '/crm/customer-credits', {
      customer_id: customerId,
      credit_level: 'A',
      credit_limit: '5000.00',
      credit_days: 30,
    });

    const r = await apiCallExpectFail(page, 'POST', `/crm/customer-credits/${customerId}/adjust`, {
      adjustment_type: 'increase',
      amount: '0',
      reason: 'E2E67 零金额负例',
    });
    expectVisibleBusinessRejection(
      r,
      '金额',
      '67-01 调整额度 amount=0 应外显具体原因（validate_amount_range 真实文案含"金额必须为正"，' +
        '当前 customer_credit_handler.rs:225 `req.validate()?` 走 error.rs:413 脱敏链路——判红即该处源码缺陷，勿放宽）'
    );

    // 被拒的写必须无痕：额度未被半写
    const detailEp = `/crm/customer-credits/${customerId}`;
    await verifyEndpointHealthy(page, detailEp);
    const credit = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(
      Number(credit.credit_limit),
      `67-01 拒绝后回读：credit_limit 应原样保持 5000，实际 ${JSON.stringify(credit.credit_limit)}`
    ).toBe(5000);
    expect(Number(credit.used_credit), '拒绝后回读：used_credit 应仍为 0').toBe(0);

    await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, '[67-01] 客户');
  });

  test('67-02 采购订单创建·明细允差 150：拒绝原因"0~100"必须可见（创建路径与更新路径同源外显）', async ({
    page,
  }) => {
    const ctx = getCtx();
    const supplierId = await seedSupplier(page, '允差创建');
    const productId = await seedProduct(page, '允差');
    const poBody = {
      supplier_id: supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: today(),
      items: [
        {
          material_id: productId,
          quantity_ordered: '10.00',
          unit_price: '5.00',
          quantity_tolerance_pct: '150',
        },
      ],
    };
    const r = await apiCallExpectFail(page, 'POST', '/purchase/orders', poBody);
    expectVisibleBusinessRejection(
      r,
      '0~100',
      '67-02 PO 创建行允差 150 应外显"交货允差百分比(quantity_tolerance_pct)必须在0~100之间"（po/mod.rs:196-204），' +
        '当前 purchase_order_handler.rs:180 `req.validate()?` 走脱敏 validation 信封——判红即该处源码缺陷，勿放宽'
    );
    // 拒绝发生在校验层（service 之前），不应有任何半写：允许列表回读仅断 total 不增长不做（并发分片下共享列表），
    // 改为仅断本请求未返回 id 这一事实已由 status=400 覆盖。
    await tryCleanup(page, 'DELETE', `/purchase/suppliers/${supplierId}`, '[67-02] 供应商');
    await tryCleanup(page, 'DELETE', `/products/${productId}`, '[67-02] 产品');
  });

  test('67-03 采购订单明细更新·允差 150：validate_write 已外显真实原因（预计判绿锚点）+ 被拒不改动原值', async ({
    page,
  }) => {
    const ctx = getCtx();
    expect(ctx.warehouseIds.length, '前置：仓库').toBeGreaterThanOrEqual(1);
    expect(ctx.departmentIds.length, '前置：部门').toBeGreaterThanOrEqual(1);
    await ensureBudgetPlan(page);
    const supplierId = await seedSupplier(page, '允差更新');
    const productId = await seedProduct(page, '允差更新');
    const po = await apiCall<Row>(page, 'POST', '/purchase/orders', {
      supplier_id: supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: today(),
      items: [
        {
          material_id: productId,
          quantity_ordered: '300.00',
          unit_price: '20.00',
          quantity_tolerance_pct: '8.00',
        },
      ],
    });
    const poId = Number(po.data?.id);
    expect(poId, `[67-03] PO 创建应回 id：${JSON.stringify(po)}`).toBeGreaterThan(0);

    const itemsEp = `/purchase/orders/${poId}/items`;
    await verifyEndpointHealthy(page, itemsEp);
    const items = requireArray(await apiCallRaw(page, 'GET', itemsEp), itemsEp);
    expect(items.length, '前置：PO 应有 1 条明细').toBe(1);
    const itemId = Number(items[0].id);
    expect(itemId, '前置：明细 id').toBeGreaterThan(0);

    const r = await apiCallExpectFail(page, 'PUT', `/purchase/orders/${poId}/items/${itemId}`, {
      quantity_tolerance_pct: '150',
    });
    expectVisibleBusinessRejection(
      r,
      '交货允差百分比',
      '67-03 明细更新允差 150：validate_write→business_displayable（po/mod.rs:172-191）应外显完整原因'
    );
    const msg03 = String(r.message ?? '');
    expect(msg03, '拒绝原因同时应包含范围词 0~100（与创建路径同一校验函数文案）').toContain(
      '0~100'
    );

    // 被拒的更新不得改动既有落库值
    const after = requireArray(await apiCallRaw(page, 'GET', itemsEp), itemsEp);
    const row = after.find(it => Number(it.id) === itemId);
    expect(row, `回读：明细 id=${itemId} 应仍存在`).toBeTruthy();
    expect(
      Number(row?.quantity_tolerance_pct),
      `被拒后回读：quantity_tolerance_pct 必须仍=8.00（半写/置空即缺陷），实际 ${JSON.stringify(row?.quantity_tolerance_pct)}`
    ).toBe(8);

    await tryCleanup(page, 'DELETE', `/purchase/orders/${poId}`, '[67-03] 采购订单');
    await tryCleanup(page, 'DELETE', `/purchase/suppliers/${supplierId}`, '[67-03] 供应商');
  });

  test('67-04 库存直建·染色布缺缸号：外显"染色布必须提供缸号（color_no 回显）"且拒绝无痕（预计判绿）', async ({
    page,
  }) => {
    const ctx = getCtx();
    expect(ctx.warehouseIds.length, '前置：仓库').toBeGreaterThanOrEqual(1);
    const productId = ctx.productIds[0];
    expect(productId, '前置：产品（ensureTestEntities 产物）').toBeTruthy();
    const batchNo = genCode('E2E67BN');
    const colorNo = 'E2E67CN001';

    // 染色布判定唯一权威 fabric_class（color_no 非空 ⇒ 缸号必填）；缸号显式发空串覆盖
    // "trim 后按空处理"形态——无论哪种形态都必须在写库前被拒且原因可见。
    const r = await apiCallExpectFail(page, 'POST', '/inventory/stock', {
      warehouse_id: ctx.warehouseIds[0],
      product_id: productId,
      batch_no: batchNo,
      color_no: colorNo,
      dye_lot_no: '',
      grade: '一等品',
      quantity_meters: '100.00',
    });
    expectVisibleBusinessRejection(
      r,
      '缸号',
      `67-04 染色布缺缸号：admit_stock_fabric_trace 已转 business_displayable（inventory_stock_handler_fabric.rs:146-156），原因应回显提交的色号 color_no=${colorNo}`
    );
    const msg04 = String(r.message ?? '');
    expect(msg04, '拒绝文案应回显用户提交的色号（fabric_class.rs:55-58 的 echo 契约）').toContain(
      colorNo
    );

    // 拒绝无痕：按四维筛选回读，该批次不得存在库存行
    const listEp = `/inventory/stock?product_id=${productId}&batch_no=${encodeURIComponent(batchNo)}&page=1&page_size=10`;
    await verifyEndpointHealthy(page, listEp);
    const listed = requireItemsEnvelope(await apiCallRaw(page, 'GET', listEp), listEp);
    expect(
      listed.total,
      `被拒的库存直建不得半落库：batch_no=${batchNo} 应 0 行，实际 ${JSON.stringify(listed.items)}`
    ).toBe(0);
  });

  test('67-05 库存直建(fabric)·白坯缺批次：应外显 DTO 的"批次号长度必须在1-50个字符之间"', async ({
    page,
  }) => {
    const ctx = getCtx();
    expect(ctx.warehouseIds.length, '前置：仓库').toBeGreaterThanOrEqual(1);
    const productId = ctx.productIds[0];
    expect(productId, '前置：产品').toBeTruthy();

    // 白坯（色号空）批次仍必填——DTO 权威文案"批次号长度必须在1-50个字符之间"
    // （inventory_stock_handler_dto.rs:17）；该处原为手工 `map_err(|e| AppError::validation(e.to_string()))`
    // 把原因压成脱敏常量，现改走 `map_err(AppError::from)`（inventory_stock_handler_fabric.rs:71）
    // 的可读外显链路——回退成脱敏即判红，勿放宽。
    const r = await apiCallExpectFail(page, 'POST', '/inventory/stock/fabric', {
      warehouse_id: ctx.warehouseIds[0],
      product_id: productId,
      batch_no: '',
      color_no: '',
      grade: '一等品',
      quantity_meters: '50.00',
    });
    expectVisibleBusinessRejection(r, '批次', '67-05 白坯直建缺批次应外显四维准入原因');
  });

  test('67-06 供应商资质日期倒挂：外显"不能早于发证日期"且资质列表无痕（预计判绿）', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, '资质倒挂');
    const qualName = `E2E67_${genCode('QUAL')}_营业执照`;

    const r = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/suppliers/${supplierId}/qualifications`,
      {
        qualification_name: qualName,
        qualification_type: 'business_license',
        qualification_no: `XK-E2E67-${Date.now()}`,
        issuing_authority: 'E2E67 市场监督管理局',
        issue_date: '2026-12-31',
        valid_until: '2026-01-01',
        need_annual_check: false,
      }
    );
    expectVisibleBusinessRejection(
      r,
      '不能早于发证日期',
      '67-06 资质有效期早于发证日应外显（supplier_service.rs:893-903 business_displayable 真实文案）'
    );

    // 拒绝无痕：该资质不得出现在列表中
    const listEp = `/purchase/suppliers/${supplierId}/qualifications`;
    await verifyEndpointHealthy(page, listEp);
    const quals = requireArray(await apiCallRaw(page, 'GET', listEp), listEp);
    expect(
      quals.some(q => q.qualification_name === qualName),
      `被拒资质不得半落库，实际列表=${JSON.stringify(quals)}`
    ).toBe(false);

    await tryCleanup(page, 'DELETE', `/purchase/suppliers/${supplierId}`, '[67-06] 供应商');
  });
});
