import { test, expect } from '../diagnose-fixture';
import type { Locator, Page, Response } from '@playwright/test';
import { loginViaUI, apiCall, apiCallRaw, genCode, tryCleanup } from './helpers';
import {
  safeGoto,
  findTableRow,
  findRowAction,
  waitForDialog,
  submitDialog,
  formItemByExactLabel,
} from './ui-helpers';

/**
 * 68 可选字段留空的请求形态：清空后提交 = **省略该键**，且原值既不被空串覆盖、也不被洗成 NULL
 *
 * 本仓两个相反方向的坑（均已在源码注释中留档，本 spec 用真实 UI 提交抓请求体防复发）：
 * ① `Some("")` 触发校验失败：后端 Option + length 校验对"缺省不携带该键"跳过、对 `Some("")` 判长度
 *    （services/supplier_service.rs:1064/1067，validator 对 Option 解包语义），前端若把清空后的
 *    空串直发即 400"参数错误"——本批修复形态：前端在提交前条件展开省略键
 *    （views/supplier/index.vue:246-254；views/sales-contract/composables/useSc.ts:257-265 `|| undefined`）；
 * ② 整表重插把未提交的列洗成 NULL：销售合同 update 对明细走 delete+重插
 *    （services/sales_contract_service.rs:205-321 update 注释），前端 prepareEdit 回源明细并
 *    原样回传真实列（useSc.ts:174-245 buildItemsPayload）。
 * ③ 表头可空列的三态（本批已把合同/部门/付款的更新 DTO 改为 RFC 7386 显式 null 清空）：
 *    键缺席=保持原值、显式 null=清空该列、有值=覆盖（useSc.ts:40 explicitToNull、
 *    sales_contract_service.rs 的 DoubleOption 分支）。因此合同域"清空"的正确形态是
 *    **送 null 并真的清空**，而供应商域（仍是单层 Option 的 if-let-覆盖语义）正确形态是
 *    **省略键并保持原值**——两种语义各自被 68-03 与 68-02 钉住，不得互相套用。
 *
 * 断言形态（不是只看 200）：
 * - 用 page.waitForResponse 抓**前端真实发出的请求体**（postData），逐键断言
 *   `hasOwnProperty(key) === false`——发送空串/null 一律判红；
 * - 响应 HTTP 必须 2xx（族级锚点：留空可选字段提交不能报参数错误）；
 * - 回读断言两向都成立：UI 清空并被省略的键 → 原值保持（不被空串覆盖、不被洗 NULL）；
 *   API 建好、UI 重提交（未触碰）的可选列 → 整表重插后仍为原值。
 *
 * UI 文案与 locales 真实 key 对齐（zh-CN.ts）：
 * - supplier.dialog.label.shortName='供应商简称'、label.creditCode='信用代码'、
 *   button.save='保存'、supplier.list.button.edit='编辑'、supplier.index.button.create='新建供应商'
 *   （views/supplier/SupplierList.vue:116-121、SupplierDialog.vue:47-75/224-229）
 * - salesContract.form.labelDeliveryDate='交货日期'、buttonConfirm='确定'、
 *   salesContract.table.buttonEdit='编辑'（SalesContractTable.vue:110，status==='draft' 渲染，
 *   后端 contract::DRAFT='draft' 小写，models/status/bpm_crm_contract.rs:34-43）
 *
 * 页面路由：/supplier、/sales-contract（router/index.ts:262/982）。
 * 端点（相对 baseURL=/api/v1/erp）：POST/PUT /purchase/suppliers[/{id}]（routes/purchase.rs:313/323）、
 * PUT /sales/sales-contracts/{id} 与 GET /sales/sales-contracts/{id}/items（routes/sales.rs:150/170）。
 */

type Row = Record<string, unknown>;

interface CapturedSubmit {
  resp: Response;
  status: number;
  body: Row;
  bodyRaw: string;
}

/** 触发一次 UI 提交并抓取前端真实请求体 + 响应（waitForResponse：请求与结果一体，免二次猜测） */
async function captureUiSubmit(
  page: Page,
  urlFragment: string,
  method: 'POST' | 'PUT',
  action: () => Promise<void>,
  what: string
): Promise<CapturedSubmit> {
  const respPromise = page.waitForResponse(
    res => res.url().includes(urlFragment) && res.request().method() === method,
    { timeout: 30_000 }
  );
  await action();
  const resp = await respPromise;
  const raw = resp.request().postData();
  expect(
    raw,
    `${what}：未捕获到前端提交请求体（url 含 "${urlFragment}" 的 ${method} 请求 postData 为空）`
  ).not.toBeNull();
  const body = JSON.parse(String(raw)) as Row;
  return { resp, status: resp.status(), body, bodyRaw: String(raw) };
}

function inputOf(root: Locator, labelText: string): Locator {
  return formItemByExactLabel(root, labelText).locator('input').first();
}

async function seedCustomer(page: Page, tag: string): Promise<number> {
  const code = genCode('E2E68C');
  const res = await apiCall<Row>(page, 'POST', '/crm/customers', {
    customer_code: code,
    customer_name: `68号${tag}客户_${code}`,
    contact_phone: '13800006800',
  });
  const id = Number(res.data?.id);
  expect(id, `[68] 客户创建应回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  return id;
}

test.describe.serial('68 可选字段留空形态（省略键 + 原值不被覆盖/不被洗 NULL）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('68-01 UI 新建供应商·可选简称/信用代码留空：请求体省略两键 → 提交 2xx（Some("") 会 400 的对照形态）', async ({
    page,
  }) => {
    const supplierName = `E2E68新建供应商_${genCode('S68A')}`;
    const supplierCode = genCode('E2E68SA');

    await safeGoto(page, '/supplier');
    await page.getByRole('button', { name: '新建供应商' }).first().click();
    const dlg = await waitForDialog(page);

    // 只填前端必填项（SupplierDialog.vue:321-346：编码/名称/联系电话 required）；
    // 供应商简称、信用代码保持空——本用例的主体形态。
    await inputOf(dlg, '供应商编码').fill(supplierCode);
    await inputOf(dlg, '供应商名称').fill(supplierName);
    await inputOf(dlg, '联系电话').fill('13900006801');
    // 显式钉桩：可选项确实是空（未被上次残留预填），否则"省略键"断言失去形态意义
    expect(await inputOf(dlg, '供应商简称').inputValue(), '新建弹窗：简称应为空').toBe('');
    expect(await inputOf(dlg, '信用代码').inputValue(), '新建弹窗：信用代码应为空').toBe('');

    const cap = await captureUiSubmit(
      page,
      '/api/v1/erp/purchase/suppliers',
      'POST',
      () => submitDialog(dlg, /保存/),
      '[68-01] 新建供应商提交'
    );

    expect(
      cap.status,
      `[68-01] 可选字段留空的提交必须被后端接受（族级锚点：HTTP 恰 200——handler Ok(Json(ApiResponse)) 无状态改写；空串直发会撞 Some("")+length 校验报 400 参数错误），实际 status=${cap.status} body=${cap.bodyRaw}`
    ).toBe(200);
    expect(
      Object.prototype.hasOwnProperty.call(cap.body, 'supplier_short_name'),
      `[68-01] 请求体必须省略 supplier_short_name 键（空串直发=复发），实际=${cap.bodyRaw}`
    ).toBe(false);
    expect(
      Object.prototype.hasOwnProperty.call(cap.body, 'credit_code'),
      `[68-01] 请求体必须省略 credit_code 键，实际=${cap.bodyRaw}`
    ).toBe(false);

    const json = (await cap.resp.json()) as { code?: number; data?: Row };
    expect(json.code, `[68-01] 成功信封 code=200，实际 ${cap.bodyRaw}`).toBe(200);
    const id = Number(json.data?.id);
    expect(id, `[68-01] 创建应回 id：${JSON.stringify(json)}`).toBeGreaterThan(0);

    // 创建路径的省略语义 = 落库默认空串（supplier_service.rs:101/106 unwrap_or_default），
    // 如实钉住该契约（区别于 68-02 编辑语义的"保持原值"）
    const detailEp = `/purchase/suppliers/${id}`;
    const row = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(row.supplier_name, '回读：名称=UI 提交值').toBe(supplierName);
    expect(row.supplier_short_name, '回读：创建时省略简称 → 落库默认空串（非 NULL、非报错）').toBe(
      ''
    );
    expect(row.credit_code, '回读：创建时省略信用代码 → 落库默认空串').toBe('');

    await tryCleanup(page, 'DELETE', `/purchase/suppliers/${id}`, '[68-01] 供应商');
  });

  test('68-02 UI 编辑供应商·清空已回填的简称/信用代码：请求体省略两键 → 原值不被空串覆盖也不被洗 NULL', async ({
    page,
  }) => {
    // API 预置一条简称/信用代码/备注齐全且各列值互不相同的供应商（三列分别覆盖
    // "被清空的可选列保持原值"与"未触碰的可选列不被重插洗掉"两个方向）
    const code = genCode('E2E68SB');
    const supplierName = `E2E68编辑回归供应商_${code}`;
    const shortOriginal = 'E2E68原始简称';
    const creditOriginal = '91310000MA1K3AB0X9';
    const remarksOriginal = `E2E68原始备注_${code}`;
    const created = await apiCall<Row>(page, 'POST', '/purchase/suppliers', {
      supplier_name: supplierName,
      supplier_short_name: shortOriginal,
      credit_code: creditOriginal,
      contact_phone: '13900006802',
      remarks: remarksOriginal,
    });
    const supplierId = Number(created.data?.id);
    expect(supplierId, `[68-02] 预置供应商应回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);

    // 「回读 vs 掩码」口径取证（CI §②/§③ 本族收口，先核源码真实出参再断言）
    // 供应商读出口（handlers/supplier_handler.rs:24-110）的打码列集合 = utils/field_mask.rs
    // 权威定义 `mask_contact_fields_for_role` —— **仅手机号/邮箱键集**（138****8888 形态，
    // 且 role_id=1 放行原文）；本用例三列 supplier_short_name / credit_code / remarks
    // 不在掩码键集内，真实契约就是原文回读。另经数据权限行 allowed/hidden 的 filter_fields
    // 是**删键**而非打值（supplier_handler.rs:94-107）。故 UI 前置钉桩之前，先走详情
    // （允许放行的原文入口）把「落库真值 + 键存在性」钉死：
    // - 本步红 → 后端建单未持久化 / 字段权限剥键（源码/配置缺陷，判红交后端，禁止改弱）；
    // - 本步绿而下方 UI 回填红 → 前端编辑回填链路缺陷（属前端，不属本用例放宽范围）。
    const seeded = await apiCallRaw<Row>(page, 'GET', `/purchase/suppliers/${supplierId}`);
    for (const key of ['supplier_short_name', 'credit_code', 'remarks'] as const) {
      expect(
        Object.prototype.hasOwnProperty.call(seeded, key),
        `[68-02] 详情回读必须含 ${key} 键（缺键=数据权限剥掉原文列，掩码集合真值见 utils/field_mask.rs）：${JSON.stringify(seeded).slice(0, 300)}`
      ).toBe(true);
    }
    expect(
      seeded.supplier_short_name,
      `[68-02] 建单落库回读：简称非掩码列，应逐字原文（实际 ${JSON.stringify(seeded.supplier_short_name)}）`
    ).toBe(shortOriginal);
    expect(
      seeded.credit_code,
      `[68-02] 建单落库回读：信用代码非掩码列，应逐字原文（实际 ${JSON.stringify(seeded.credit_code)}）`
    ).toBe(creditOriginal);
    expect(
      seeded.remarks,
      `[68-02] 建单落库回读：备注非掩码列，应逐字原文（实际 ${JSON.stringify(seeded.remarks)}）`
    ).toBe(remarksOriginal);

    await safeGoto(page, '/supplier');
    const row = await findTableRow(page, supplierName, 1, supplierName);
    expect(row, `[68-02] 列表应定位到自建供应商 ${supplierName}`).toBeTruthy();
    await row!.getByRole('button', { name: '编辑' }).first().click();
    const dlg = await waitForDialog(page);

    // 前置钉桩：编辑弹窗确实回填了落库值（否则"清空后省略键"无从谈起）。
    // 若此处红而上方 API 回读绿，定性为前端回填缺陷（回填源=列表行 vs 详情出参的口径差），
    // 保持红并交前端，禁止把断言降级成 != undefined 之类弱判据。
    await expect
      .poll(() => inputOf(dlg, '供应商简称').inputValue(), { timeout: 10_000 })
      .toBe(shortOriginal);
    await expect
      .poll(() => inputOf(dlg, '信用代码').inputValue(), { timeout: 10_000 })
      .toBe(creditOriginal);

    // UI 清空两个可选字段
    await inputOf(dlg, '供应商简称').fill('');
    await inputOf(dlg, '信用代码').fill('');

    const cap = await captureUiSubmit(
      page,
      `/api/v1/erp/purchase/suppliers/${supplierId}`,
      'PUT',
      () => submitDialog(dlg, /保存/),
      '[68-02] 编辑供应商提交'
    );

    expect(
      Object.prototype.hasOwnProperty.call(cap.body, 'supplier_short_name'),
      `[68-02] 清空后的简称必须以"省略键"形态提交（index.vue:252 条件展开；发送 ""= 编辑语义被洗空/撞校验的复发形态），实际=${cap.bodyRaw}`
    ).toBe(false);
    expect(
      Object.prototype.hasOwnProperty.call(cap.body, 'credit_code'),
      `[68-02] 清空后的信用代码必须省略键，实际=${cap.bodyRaw}`
    ).toBe(false);
    expect(
      cap.status,
      `[68-02] 留空可选字段的编辑提交不得报参数错误（HTTP 恰 200——update_supplier handler Ok(Json(ApiResponse)) 无状态改写；3xx/4xx 皆属回归），实际 status=${cap.status} body=${cap.bodyRaw}`
    ).toBe(200);

    // 回读双向断言：被省略的列保持原值（不被 "" 覆盖、不被洗 NULL）；未触碰的列同样原样
    const detailEp = `/purchase/suppliers/${supplierId}`;
    const fresh = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(
      fresh.supplier_short_name,
      `编辑回读：省略键=保持原值（supplier_service.rs:409-410 的 if-let-取-覆盖语义），实际 ${JSON.stringify(fresh.supplier_short_name)}`
    ).toBe(shortOriginal);
    expect(
      fresh.credit_code,
      `编辑回读：信用代码应保持原 18 位值，实际 ${JSON.stringify(fresh.credit_code)}`
    ).toBe(creditOriginal);
    expect(
      fresh.remarks,
      `编辑回读：未触碰的 remarks 不得被整行重存洗掉，实际 ${JSON.stringify(fresh.remarks)}`
    ).toBe(remarksOriginal);

    await tryCleanup(page, 'DELETE', `/purchase/suppliers/${supplierId}`, '[68-02] 供应商');
  });

  test('68-03 UI 编辑销售合同·清空可选交货日期：送显式 null 真的清空该列、未触碰列与明细整表重插不洗列', async ({
    page,
  }) => {
    const customerId = await seedCustomer(page, '合期');
    const contractNo = genCode('E2E68SC');
    const headerDeliveryDate = '2026-10-15';
    const remarkOriginal = `E2E68表头备注_${contractNo}`;
    const itemSpec = 'E2E68/160g/150cm';
    const itemRemarks = 'E2E68行备注保留';
    const itemDeliveryDate = '2026-10-05';

    // API 建一份"明细行所有可选列都非空"的草稿合同（编辑表单并不采集 product_spec/行日期/行备注，
    // 它们全靠 prepareEdit 回源后原样回传——正是"整表重插洗列"坑的靶心）
    const created = await apiCall<Row>(page, 'POST', '/sales/sales-contracts', {
      contract_no: contractNo,
      contract_name: `68号日期回归合同_${contractNo}`,
      customer_id: customerId,
      total_amount: '6170.00',
      payment_terms: 'E2E68 付款条件',
      delivery_date: headerDeliveryDate,
      signed_date: '2026-09-01',
      remark: remarkOriginal,
      items: [
        {
          product_name: 'E2E68 回归布',
          product_spec: itemSpec,
          unit: 'm',
          quantity: '500.00',
          quantity_tolerance_pct: '10.00',
          unit_price: '12.34',
          delivery_date: itemDeliveryDate,
          remarks: itemRemarks,
        },
      ],
    });
    const contractId = Number(created.data?.id);
    expect(contractId, `[68-03] 合同创建应回 id：${JSON.stringify(created)}`).toBeGreaterThan(0);

    await safeGoto(page, '/sales-contract');
    const row = await findTableRow(page, contractNo, 1, contractNo);
    expect(row, `[68-03] 列表应定位到自建合同 ${contractNo}`).toBeTruthy();
    // 操作列 fixed:'right'（SalesContractTable.vue:176-183）被 el-table-v2 拆层渲染：
    // 「编辑」按钮在 .el-table-v2__right 覆盖层行内，主表行作用域永远 0 命中——
    // 行内按钮在固定列覆盖层，主表行文本里没有；用 findRowAction 做
    // "主表序号（含合同号=该行真实存在）↔ 覆盖层同序号行（按钮所在）"对齐定位。
    const editBtn = await findRowAction(page, contractNo, r =>
      r.getByRole('button', { name: '编辑' })
    );
    await editBtn.click();
    const dlg = await waitForDialog(page);

    const dateInput = inputOf(dlg, '交货日期');
    await expect.poll(() => dateInput.inputValue(), { timeout: 10_000 }).toBe(headerDeliveryDate);

    // 清空日期：EP date-picker 文本框可编辑——全选删除后 Enter 提交空值（面板按 Escape 收起）
    await dateInput.click({ clickCount: 3 });
    await dateInput.press('Backspace');
    await dateInput.press('Enter');
    await page.keyboard.press('Escape');
    await expect.poll(() => dateInput.inputValue(), { timeout: 10_000 }).toBe('');

    const cap = await captureUiSubmit(
      page,
      `/api/v1/erp/sales/sales-contracts/${contractId}`,
      'PUT',
      () => submitDialog(dlg, /确定/),
      '[68-03] 编辑销售合同提交'
    );

    expect(
      Object.prototype.hasOwnProperty.call(cap.body, 'delivery_date'),
      `[68-03] 三态契约：清空后的交货日期必须**送显式 null**（useSc.ts:40 explicitToNull；省略键在后端语义是"保持原值"，用户清空却清不掉＝本轮消灭的静默丢弃形态），实际=${cap.bodyRaw}`
    ).toBe(true);
    expect(
      cap.body.delivery_date,
      `[68-03] 清空字段必须送 null（送 ""/省略键 都属塌层，实际 ${JSON.stringify(cap.body.delivery_date)}）`
    ).toBeNull();
    expect(
      cap.status,
      `[68-03] 清空可选日期的提交不得报参数错误（HTTP 恰 200——update_contract handler Ok(Json(ApiResponse)) 无状态改写；3xx/4xx 皆属回归），实际 status=${cap.status} body=${cap.bodyRaw}`
    ).toBe(200);

    // 明细整表重插的随行回传钉桩：表单不可见但已回源的真实列必须原样在请求体里
    expect(
      Array.isArray(cap.body.items),
      `[68-03] 请求体 items 必须为数组，实际=${cap.bodyRaw}`
    ).toBe(true);
    const sentItems = cap.body.items as Row[];
    expect(sentItems.length, '请求体应含 1 条明细').toBe(1);
    expect(sentItems[0].product_spec, '请求体 items[0].product_spec 必须回传原值').toBe(itemSpec);
    expect(sentItems[0].delivery_date, '请求体 items[0].delivery_date 必须回传原值').toBe(
      itemDeliveryDate
    );
    expect(sentItems[0].remarks, '请求体 items[0].remarks 必须回传原值').toBe(itemRemarks);
    expect(
      Number(sentItems[0].quantity_tolerance_pct),
      '请求体 items[0].quantity_tolerance_pct 必须回传原值 10（Number 归一）'
    ).toBe(10);

    // 回读：送显式 null 的 delivery_date 必须**真的被清空**（这是三态语义的另一半——只断请求体
    // 送 null 而回读仍是旧值，说明 service 把 null 当"未提交"塌层了）；未触碰而经回显原样回传的
    // remark 保持原值；明细四个可选真实列不被整表重插洗掉
    const detailEp = `/sales/sales-contracts/${contractId}`;
    const header = await apiCallRaw<Row>(page, 'GET', detailEp);
    expect(
      header.delivery_date,
      `回读：UI 清空并送 null 的 delivery_date 必须落 NULL（仍为 ${headerDeliveryDate} 即 service 把 null 塌成"保持原值"，实际 ${JSON.stringify(header.delivery_date)}）`
    ).toBeNull();
    expect(
      header.remark,
      `回读：未触碰的 remark 由 prepareEdit 回显后原样回传（useSc.ts:212 把出参单数 remark 映射到表单 remarks）→ 不得被洗 NULL/空串，实际 ${JSON.stringify(header.remark)}`
    ).toBe(remarkOriginal);
    expect(header.signed_date, '回读：未触碰的 signed_date 原样').toBe('2026-09-01');

    const itemsEp = `/sales/sales-contracts/${contractId}/items`;
    const itemsRes = await apiCallRaw(page, 'GET', itemsEp);
    expect(
      Array.isArray(itemsRes),
      `[68-03] 明细端点出参必须是裸数组，实际 ${JSON.stringify(itemsRes)}`
    ).toBe(true);
    const items = (itemsRes as Row[]).filter(it => it.product_spec === itemSpec);
    expect(items.length, `明细回读应命中自建行（product_spec=${itemSpec}）`).toBe(1);
    expect(
      Number(items[0].quantity_tolerance_pct),
      `重插后 quantity_tolerance_pct 不得被洗 NULL（洗成 null 时 Number 为 0），实际 ${JSON.stringify(items[0].quantity_tolerance_pct)}`
    ).toBe(10);
    expect(items[0].delivery_date, '重插后行交货日期不得被洗 NULL').toBe(itemDeliveryDate);
    expect(items[0].remarks, '重插后行备注不得被洗 NULL').toBe(itemRemarks);

    await tryCleanup(page, 'DELETE', `/sales/sales-contracts/${contractId}`, '[68-03] 销售合同');
    await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, '[68-03] 客户');
  });
});
