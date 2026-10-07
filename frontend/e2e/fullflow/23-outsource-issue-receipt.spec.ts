// 交易域全流程契约级 E2E — 23 委外发料→收回→结算（契约级 API 链）
//
// 定位：委外链每步回读后端权威状态（词表逐字符），并钉两条契约要点：
//   A. 明细读取信封键名与后端出参**同源**：orders/receipts/vouchers 列表 = PaginatedResponse
//      {items,total,page,page_size}（utils/response.rs:34，handler 返回类型直书）；
//      而 GET /production/outsourcing-orders/items/by-order/{id} 的 data 是**裸数组**
//      （handlers/outsourcing_handler.rs:234-240 ApiResponse<Vec<Model>>）——前端若按
//      items 键读明细必然恒空，这是"保存后明细显示不出来"的契约级成因之一。
//   B. 收回后库存/成本联动：confirm 收回单 ⇒ 订单推进 received 词值、损耗分类/率、
//      total_cost/unit_cost 计算落列、入库凭证生成、匹记录与质检记录联动（inspection_id 回写）。
// 取数口径：本例自建唯一单——委外域 order_no/receipt_no 由**客户端提供**
//   （services/outsourcing_ops/types.rs:30/:185 非 Option），故用 genCode 唯一码直接做锚点，
//   经 by-no 端点精确回查；afterEach 尽力清理（非 draft 删除被拒仅告警，属预期）。
//
// 契约真值源：
//   订单状态词表（小写）  backend/src/models/status/wage_energy_chemical_business.rs:262-277
//     draft→issued→processing→received→settled→closed（settle 状态门 order.rs:594-597 仅 received，
//     settle 费用门 order.rs:607-611 processing_fee+freight_fee<=0 硬拒 400；
//     issue 门 order.rs:466-470 仅 draft；record_processing 门 :561-565 仅 issued）
//   收回单词表（小写）    同上 :288-295（draft/confirmed/cancelled；confirm 状态门 receipt.rs:326-331，
//     confirm 数量门 :338-342 return_quantity<=0 硬拒；create/update 入口同族数量门 :99-101/:257-261）
//   收回质检结论词表      同上 :328 起 outsourcing_receipt_quality_status：
//     pending/qualified/concession/unqualified（validate_receipt_quality_status receipt.rs:61-70
//     入口拒越界写法——历史上 passed/不合格 混写点）
//   损耗类型词表          同上 :279-285（normal/abnormal）
//   凭证类型词表          同上 :342-348（issue/fee/receipt/loss 小写）
//   收回前置              services/outsourcing_ops/order.rs:42-62 validate_receipt_eligibility
//     （仅 issued/processing 可收回；收回>发出 直接拒）
//   确认落账公式          services/outsourcing_ops/receipt.rs:336-477 + outsourcing_service.rs:51-115
//     loss=issue-return；rate=loss/issue；classify_loss(actual<=standard→normal)；
//     abnormal=超额损耗×单位材料成本；total_cost=material+fee+freight+abnormal；unit_cost=total/return
//   发料匹号门            services/piece_domain_service.rs:152-195 validate_pieces_for_issue
//     （每条明细必须带真实存在且 AVAILABLE 的生产匹号——无明细则整单放行，见 23-02 负例）
//   更新三态拒绝 NOT NULL 显式 null 的入口：order.rs:276-289 + types.rs:53-94
// 诚实标注/回归钉（本波源码修复后应为绿）：
//   settle 语义要求订单携带 processing_fee/freight_fee/tax_amount（order.rs:587 注释：
//   "需在订单更新时填入"），Create/Update DTO（services/outsourcing_ops/types.rs）已补
//   这三个真实键（NOT NULL 列 v15/mod.rs:3247-3249，Update 显式 null 拒清）⇒ 费用经
//   API 可录入回读，FEE 凭证金额取真实值。23-03 由缺陷钉转为回归钉，再红即回归。
//   本批后端新增两道硬拒（commit 0d74c04f）⇒ 前端门控与之同口径，契约级由本例锁定：
//   零费用结算拒（order.rs:607-611，23-04）；零数量收回入口拒（receipt.rs:99-101，23-02-G）。
// PR 实测值三列（weight/width/gram_weight，receipt.rs:76-85/:230/:269-271/:373-383/:523-525）
//   e2e 活体覆盖：23-05 建单落值+回读+三态（键缺席保持/显式 null 清空/有值覆盖）+confirm 出参透传；
//   23-06 值域负例（0/负数建单与更新口一律 400+VALIDATION_ERROR，拒绝零变化）。
//   权限/校验类拒绝文案永久脱敏 ⇒ 只断 status+信封 code，不断任何中文文案（与 23-02 D2 同口径）。
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
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(
    obj[key],
    `${label}：键 "${key}" 期望=${JSON.stringify(expected)} 实际=${JSON.stringify(obj[key])}`
  ).toEqual(expected);
}

function expectDecimal(
  obj: Record<string, unknown>,
  key: string,
  expected: number,
  label: string,
  tol = 0.005
) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：缺数值键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n))
    throw new Error(`${label}：${key} 不可解析，raw=${JSON.stringify(obj[key])}`);
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(tol);
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/**
 * 实测值三列（weight/width/gram_weight）出参类型如实性断言。
 *
 * 类型事实（不许用 Number()/parseFloat() 归一把"类型谎言"洗成通过行）：
 *   - rust_decimal 在本仓仅启用 serde feature（backend/Cargo.toml:60 `features=["serde"]`，
 *     未启用 serde-float），JSON 序列化为**十进制字符串**；
 *   - DECIMAL(18,4) 列（models/outsourcing_receipt.rs:70-78）经 DB RETURNING/回读后
 *     可能按列 scale 补尾零（"1.5"→"1.5000"，同型先例 flow/21a-fabric-sales-order.spec.ts:103）。
 * 因此断言分两层：
 *   ① 键必在场且 typeof === 'string'（值非 null 时）——出参退化成 number、缺键（会被前端
 *      `??` 兜底洗成"已实测"，与标签 fail-closed 口径分裂）在此层必红；
 *   ② 数值按**十进制串逐位归一**（去整数前导零/小数尾零）比较，只容忍尾零形状差异，
 *      不改值域——真值回归（如 21.5→21.6、串被洗成 "0"）仍必红。
 * null 分支用 toBeNull 严格锁定"显式清空后=未补录"：后端若把 null 当"不改"残留旧值、
 * 或把 NULL 洗成 0/''/缺键，该分支与键在场检查都能抓到——这是三态语义唯一活体证明点。
 */
function canonDecimalStr(s: string): string {
  const neg = s.startsWith('-');
  const body = neg ? s.slice(1) : s;
  const [int, frac = ''] = body.split('.');
  const i = int.replace(/^0+/, '') || '0';
  const f = frac.replace(/0+$/, '');
  const unsigned = `${i}${f ? `.${f}` : ''}`;
  return neg && unsigned !== '0' ? `-${unsigned}` : unsigned;
}

function expectMeasured(
  obj: Record<string, unknown>,
  key: 'weight' | 'width' | 'gram_weight',
  expected: string | null,
  label: string
) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(
      `${label}：响应缺实测值键 "${key}"，实际键=${Object.keys(obj).join(',')}` +
        `（收回单端点出参=outsourcing_receipt::Model 直接序列化，三键必须恒在场——缺键会被前端 ?? 兜底掩盖成"已实测"）`
    );
  }
  const v = obj[key];
  if (expected === null) {
    expect(
      v,
      `${label}："${key}" 应为严格 null（未补录），实际=${JSON.stringify(v)}——0/''/旧值残留都不是"清空"`
    ).toBeNull();
    return;
  }
  if (typeof v !== 'string') {
    throw new Error(
      `${label}："${key}" 出参应为 rust_decimal 十进制字符串（Cargo.toml:60 未启用 serde-float），` +
        `实际类型=${v === null ? 'null' : typeof v} raw=${JSON.stringify(v)}——类型回归，禁止 Number() 归一蒙绿`
    );
  }
  expect(
    canonDecimalStr(v),
    `${label}："${key}" 期望=${expected} 实际=${v}（十进制逐位归一比较，仅容忍列 scale 尾零补形）`
  ).toBe(canonDecimalStr(expected));
}

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

/**
 * 自建唯一委外订单（order_type=dyeing，发料 100kg、材料成本 1000、标准损耗率 10%），
 * 全键回读（缺陷①）。初始 draft：total_cost=material_cost、unit_cost=0、return_quantity=0
 * （services/outsourcing_ops/order.rs:185-220 build+create）。
 */
async function seedOutsourcingOrder(page: Page, tag: string): Promise<Record<string, unknown>> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  const orderNo = `E23-${tag}-${genCode('OS')}`;
  const created = await apiCallRaw<Record<string, unknown>>(
    page,
    'POST',
    '/production/outsourcing-orders',
    {
      order_no: orderNo,
      order_type: 'dyeing',
      supplier_id: ctx.supplierId,
      color_no: `E23C${orderNo.slice(-6)}`,
      dye_lot_no: `E23L${orderNo.slice(-6)}`,
      issue_date: todayStr(),
      issue_quantity: '100',
      issue_unit: 'kg',
      material_cost: '1000',
      standard_loss_rate: '0.10',
      remarks: `E2E-F23-${tag}`,
    }
  );
  const id = requireNum(created.id, `建委外订单 ${orderNo}`);
  CLEANUP.push({ path: `/production/outsourcing-orders/${id}`, label: `outsourcing_order#${id}` });

  // by-no 精确锚定回读（handlers/outsourcing_handler.rs:148-154）
  const byNo = await apiCallRaw<Record<string, unknown>>(
    page,
    'GET',
    `/production/outsourcing-orders/by-no/${orderNo}`
  );
  expectKeyValue(byNo, 'id', id, 'by-no 回读');
  expectKeyValue(byNo, 'order_no', orderNo, 'by-no 回读单号');
  expectKeyValue(
    byNo,
    'status',
    'draft',
    '初始词值（wage_energy_chemical_business.rs:264 小写 draft）'
  );
  expectDecimal(byNo, 'issue_quantity', 100, '发料数量回读');
  expectDecimal(byNo, 'material_cost', 1000, '材料成本回读');
  expectDecimal(byNo, 'total_cost', 1000, '建单时 total_cost=material_cost（order.rs:207）');
  expectDecimal(byNo, 'return_quantity', 0, '建单 return_quantity 归零起步（order.rs:197）');
  expectDecimal(byNo, 'unit_cost', 0, '建单 unit_cost=0（order.rs:208）');
  expectKeyValue(byNo, 'voucher_no_issue', null, '未发料无发料凭证号');
  expectDecimal(byNo, 'standard_loss_rate', 0.1, '标准损耗率回读');
  return byNo;
}

test.describe('23 委外发料→收回→结算契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('23-01 主链：建单→发料(issued+OVIS凭证)→收回→确认(received 词值+成本/损耗/凭证/质检联动)→结算→关闭；信封键同源', async ({
    page,
  }) => {
    const order = await seedOutsourcingOrder(page, 'MAIN');
    const orderId = requireNum(order.id, '委外订单');

    // draft 期补录三费（真实业务链：settle 费用门 order.rs:607-611 对 0 费用硬拒 400，
    // 而更新门 order.rs:318-322 仅 draft 可录入 ⇒ 费用必须在发料前就位，received 后补录走不通）
    await apiCall(page, 'PUT', `/production/outsourcing-orders/${orderId}`, {
      processing_fee: 100,
      freight_fee: 50,
      tax_amount: 13,
    });

    // 明细端点信封：data 为**裸数组**（outsourcing_handler.rs:234-240），前端读取键必须同源
    const itemsRaw = await apiCallRaw<unknown>(
      page,
      'GET',
      `/production/outsourcing-orders/items/by-order/${orderId}`
    );
    expect(
      Array.isArray(itemsRaw),
      `items/by-order 出参应为裸数组（后端 ApiResponse<Vec>），实际键=${
        itemsRaw && typeof itemsRaw === 'object' ? Object.keys(itemsRaw).join(',') : typeof itemsRaw
      }`
    ).toBe(true);

    // 列表信封 = PaginatedResponse 四键（utils/response.rs:34）
    const list = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/production/outsourcing-orders?page=1&page_size=5'
    );
    for (const k of ['items', 'total', 'page', 'page_size']) {
      expect(
        Object.prototype.hasOwnProperty.call(list, k),
        `订单列表应含 PaginatedResponse 键 "${k}"，实际键=${Object.keys(list).join(',')}`
      ).toBe(true);
    }
    pickListArray<Record<string, unknown>>(list, 'items', '订单列表');

    // ── 发料（无明细⇒匹号门整单放行，见 23-02 的另一面）──
    const issued = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/outsourcing-orders/${orderId}/issue`
    );
    expectKeyValue(issued, 'status', 'issued', '发料响应词值');
    expect(
      typeof issued.voucher_no_issue === 'string' &&
        (issued.voucher_no_issue as string).startsWith('OVIS'),
      `发料应回写 OVIS 前缀凭证号（order.rs:391-396 generate_no_with_txn("OVIS")），实际=${JSON.stringify(issued.voucher_no_issue)}`
    ).toBe(true);
    const issuedReread = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    expectKeyValue(issuedReread, 'status', 'issued', '发料回查词值');

    // 发料凭证：voucher_type='issue'、金额=材料成本、未过账
    const vouchers1 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '发料后凭证列表'
    );
    const issueVch = vouchers1.find(v => v.voucher_type === 'issue');
    expect(
      issueVch,
      `应存在 issue 凭证，实际=${JSON.stringify(vouchers1.map(v => v.voucher_type))}`
    ).toBeTruthy();
    expectDecimal(issueVch!, 'amount', 1000, '发料凭证金额=material_cost（order.rs:408-410）');
    expectKeyValue(issueVch!, 'is_posted', false, '凭证初始未过账');

    // ── 收回单（draft；quality_status 用词表合法值 qualified）──
    const ctx = getCtx();
    const receiptNo = `E23-R${genCode('RC')}`;
    const rcpt = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: receiptNo,
        outsourcing_order_id: orderId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        color_no: order.color_no,
        dye_lot_no: order.dye_lot_no,
        batch_no: `E23B${receiptNo.slice(-6)}`,
        warehouse_id: ctx.warehouseIds[0],
        return_quantity: '95',
        quality_status: 'qualified',
        grade: '一等品',
        remarks: `E2E-F23-RCV-${receiptNo}`,
      }
    );
    const rcptId = requireNum(rcpt.id, '建收回单');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${rcptId}`,
      label: `outsourcing_receipt#${rcptId}`,
    });
    expectKeyValue(rcpt, 'status', 'draft', '收回单初始词值（receipt.rs:198）');
    expectDecimal(rcpt, 'unit_cost', 0, '建单 unit_cost=0（receipt.rs:180）');
    expectKeyValue(rcpt, 'quality_status', 'qualified', '质检结论按词表原样回读');

    // ── 确认收回 ⇒ received 词值 + 成本/损耗/凭证/质检四联动 ──
    await apiCall(page, 'POST', `/production/outsourcing-receipts/${rcptId}/confirm`);
    const rcptAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-receipts/by-no/${receiptNo}`
    );
    expectKeyValue(
      rcptAfter,
      'status',
      'confirmed',
      '收回单确认词值（finance 域外，wage_energy_chemical_business.rs:288-295）'
    );
    expectDecimal(rcptAfter, 'loss_quantity', 5, '损耗=100-95（receipt.rs:343 calc）');
    expectDecimal(rcptAfter, 'loss_rate', 0.05, '损耗率=5/100');
    expectKeyValue(
      rcptAfter,
      'loss_type',
      'normal',
      '5%<=标准10% ⇒ normal（classify_loss outsourcing_service.rs:91-96）'
    );
    expectDecimal(rcptAfter, 'abnormal_loss_amount', 0, '无超额损耗');
    expectDecimal(
      rcptAfter,
      'total_cost',
      1150,
      'total_cost=material+processing_fee+freight_fee-abnormal=1000+100+50-0（compute_total_cost outsourcing_service.rs:59-66）'
    );
    expectDecimal(
      rcptAfter,
      'unit_cost',
      1150 / 95,
      'unit_cost=total/return=1150/95（compute_unit_cost :69-76）',
      0.01
    );
    expect(
      requireNum(
        rcptAfter.inspection_id,
        '收回单回写质检记录 id（receipt.rs:484-500 trigger_quality_inspection）'
      ),
      'confirm 应联动生成质检记录并回写 inspection_id'
    ).toBeGreaterThan(0);

    const orderAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    expectKeyValue(
      orderAfter,
      'status',
      'received',
      '收回后订单词值 received（词表 :270，"收回"的权威写法）'
    );
    expectDecimal(orderAfter, 'return_quantity', 95, '订单回写收回量');
    expectDecimal(orderAfter, 'loss_quantity', 5, '订单回写损耗量');
    expectDecimal(orderAfter, 'unit_cost', 1150 / 95, '订单成本联动（receipt.rs:470-477）', 0.01);
    expect(
      typeof orderAfter.voucher_no_receipt === 'string' &&
        (orderAfter.voucher_no_receipt as string).startsWith('OVRC'),
      `入库凭证号回写（receipt.rs:358-361 generate_no_with_txn("OVRC")），实际=${JSON.stringify(orderAfter.voucher_no_receipt)}`
    ).toBe(true);

    const vouchers2 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '确认后凭证列表'
    );
    const receiptVch = vouchers2.find(v => v.voucher_type === 'receipt');
    expect(receiptVch, '应存在 receipt 入库凭证').toBeTruthy();
    expectDecimal(receiptVch!, 'amount', 1150, '入库凭证金额=total_cost（receipt.rs:374）');
    expect(
      !vouchers2.some(v => v.voucher_type === 'loss'),
      'normal 损耗不应生成 loss 凭证（receipt.rs:426 条件门）'
    ).toBe(true);

    // 凭证过账端点回读（POST /production/outsourcing-vouchers/{id}/post）
    await apiCall(
      page,
      'POST',
      `/production/outsourcing-vouchers/${requireNum(receiptVch!.id, '入库凭证 id')}/post`
    );
    const vouchers3 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '过账后凭证列表'
    );
    const postedRow = vouchers3.find(v => Number(v.id) === Number(receiptVch!.id));
    expect(postedRow, '过账后凭证列表应仍含该 receipt 凭证').toBeTruthy();
    expectKeyValue(postedRow!, 'is_posted', true, 'post 后过账标记回读');

    // ── 结算：received→settled；FEE 凭证金额=processing_fee+freight_fee，税额单记 tax_amount ──
    const settled = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/outsourcing-orders/${orderId}/settle`
    );
    expectKeyValue(
      settled,
      'status',
      'settled',
      '结算词值（状态门 order.rs:594-597 + 费用门 :607-611）'
    );
    expect(
      typeof settled.voucher_no_fee === 'string' &&
        (settled.voucher_no_fee as string).startsWith('OVFE'),
      `加工费凭证号回写（order.rs:618-628），实际=${JSON.stringify(settled.voucher_no_fee)}`
    ).toBe(true);
    const vouchers4 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '结算后凭证列表'
    );
    const feeVch = vouchers4.find(v => v.voucher_type === 'fee');
    expect(feeVch, '应存在 fee 结算凭证（order.rs:631-656）').toBeTruthy();
    expectDecimal(
      feeVch!,
      'amount',
      150,
      'fee 凭证金额=processing_fee+freight_fee=100+50（order.rs:607/639，草稿期补录见上 PUT）'
    );
    expectDecimal(feeVch!, 'tax_amount', 13, 'fee 凭证税额单记订单 tax_amount（order.rs:640）');

    // ── 关闭：settled→closed ──
    const closed = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/outsourcing-orders/${orderId}/close`
    );
    expectKeyValue(closed, 'status', 'closed', '关闭词值（门 order.rs:705-708）');
  });

  test('23-02 门控与词表负例：重复发料拒、非 issued 收回拒、超量收回拒、零量收回建单拒、越界质检词入口拒、明细带假匹号发料拒，全部锁机器码且无痕', async ({
    page,
  }) => {
    const ctx = getCtx();

    // A) 建单后未发料即建收回并 confirm ⇒ 收回前置门拒（order.rs:42-52 仅 issued/processing）
    const early = await seedOutsourcingOrder(page, 'EARLY');
    const earlyId = requireNum(early.id, 'EARLY 订单');
    const rcv0 = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: `E23-R0${genCode('E')}`,
        outsourcing_order_id: earlyId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        return_quantity: '10',
        quality_status: 'qualified',
      }
    );
    const rcv0Id = requireNum(rcv0.id, '建收回单(early)');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${rcv0Id}`,
      label: `receipt(early)#${rcv0Id}`,
    });
    const f0 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-receipts/${rcv0Id}/confirm`
    );
    expect(
      f0.status,
      `draft 订单收回 confirm 应 400，实际=${f0.status} body=${JSON.stringify(f0)}`
    ).toBe(400);
    expect(failureCode(f0), '收回前置机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const earlyStill = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${earlyId}`
    );
    expectKeyValue(earlyStill, 'status', 'draft', '被拒后订单仍 draft 无痕');
    const rcv0Still = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-receipts/${rcv0Id}`
    );
    expectKeyValue(rcv0Still, 'status', 'draft', '被拒后收回单仍 draft 无痕');

    // B) 质检结论越界词 'passed'（别域写法）⇒ 入口拒（receipt.rs:61-70/:153-154）
    const fQ = await apiCallExpectFail(page, 'POST', '/production/outsourcing-receipts', {
      receipt_no: `E23-RQ${genCode('Q')}`,
      outsourcing_order_id: earlyId,
      receipt_date: todayStr(),
      product_id: ctx.productIds[0],
      return_quantity: '10',
      quality_status: 'passed',
    });
    expect(fQ.status, `越界质检词建单应 400，实际=${fQ.status}`).toBe(400);
    expect(failureCode(fQ), '越界质检词机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // C) 重复发料：draft 门（order.rs:371-376）
    const issued = await seedOutsourcingOrder(page, 'DUP');
    const issuedId = requireNum(issued.id, 'DUP 订单');
    await apiCall(page, 'POST', `/production/outsourcing-orders/${issuedId}/issue`);
    const f1 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${issuedId}/issue`
    );
    expect(f1.status, '重复发料应 400').toBe(400);
    expect(failureCode(f1), '重复发料机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 未结算即关闭 → 拒（close 门仅 settled，order.rs:581-587），且状态不漂移
    const f2 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${issuedId}/close`
    );
    expect(f2.status, 'issued 直接 close 应 400').toBe(400);
    expect(failureCode(f2), '越级 close 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const noDrift = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${issuedId}`
    );
    expectKeyValue(noDrift, 'status', 'issued', '越级动作被拒后仍 issued');

    // D) 超量收回：return>issue 时 confirm 拒（order.rs:56-60）
    const rcvBig = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: `E23-RB${genCode('B')}`,
        outsourcing_order_id: issuedId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        return_quantity: '101',
        quality_status: 'qualified',
      }
    );
    const bigId = requireNum(rcvBig.id, '建超量收回单');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${bigId}`,
      label: `receipt(big)#${bigId}`,
    });
    const f3 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-receipts/${bigId}/confirm`
    );
    expect(f3.status, '收回量>发料量确认应 400').toBe(400);
    expect(failureCode(f3), '超量收回机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // D2) 收回数量取值域门：0 量建单入口拒（receipt.rs:99-101 validation_displayable，
    // VALIDATION_ERROR 族；confirm 侧同族数量门 receipt.rs:338-342 拦存量 0 量草稿）
    const fRz = await apiCallExpectFail(page, 'POST', '/production/outsourcing-receipts', {
      receipt_no: `E23-RZ${genCode('RZ')}`,
      outsourcing_order_id: issuedId,
      receipt_date: todayStr(),
      product_id: ctx.productIds[0],
      return_quantity: '0',
      quality_status: 'qualified',
    });
    expect(fRz.status, `0 量收回建单应 400，实际=${fRz.status} body=${JSON.stringify(fRz)}`).toBe(
      400
    );
    expect(failureCode(fRz), '0 量收回建单机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // E) 单号唯一门：重复 order_no 建单拒（validate_order_no_unique，order.rs:112-120 区段）
    const f4 = await apiCallExpectFail(page, 'POST', '/production/outsourcing-orders', {
      order_no: String(issued.order_no),
      order_type: 'dyeing',
      supplier_id: ctx.supplierId,
      issue_date: todayStr(),
      issue_quantity: '1',
      material_cost: '1',
    });
    expect(f4.status, `重复单号应 400，实际=${f4.status} body=${JSON.stringify(f4)}`).toBe(400);
    expect(failureCode(f4), '重复单号机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // F) 明细引用不存在的生产匹号 ⇒ 发料整单拒（piece_domain_service.rs:186-189），订单仍 draft
    const pieceOrder = await seedOutsourcingOrder(page, 'PIECE');
    const pieceId = requireNum(pieceOrder.id, 'PIECE 订单');
    const fakePieceNo = `E23-NOPE-${genCode('P')}`;
    await apiCall(page, 'POST', '/production/outsourcing-orders/items', {
      outsourcing_order_id: pieceId,
      product_id: ctx.productIds[0],
      quantity: '10',
      unit_cost: '1',
      piece_no: fakePieceNo,
    });
    const f5 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${pieceId}/issue`
    );
    expect(f5.status, '假匹号发料应 400').toBe(400);
    expect(failureCode(f5), '假匹号机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 被拒无痕：明细仍在（裸数组键口径）
    const its = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(
        page,
        'GET',
        `/production/outsourcing-orders/items/by-order/${pieceId}`
      ),
      'bare',
      '假匹号订单明细'
    );
    expect(its.length, '发料被拒不应吞明细').toBe(1);
    expectKeyValue(its[0], 'piece_no', fakePieceNo, '假匹号建单后应原样回读（不被发料门吞键）');
    const noDrift2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${pieceId}`
    );
    expectKeyValue(noDrift2, 'status', 'draft', '假匹号发料被拒后仍 draft');
  });

  test('23-03 回归钉（本波修复后应为绿）：加工费/运费/税额经契约录入回读（FEE 链路数值生效）', async ({
    page,
  }) => {
    // settle 语义要求订单携带 processing_fee/freight_fee/tax_amount（order.rs:587 注释
    // "需在订单更新时填入"）。本波源码修复：UpdateOutsourcingOrderRequest
    // （services/outsourcing_ops/types.rs）补齐三键（NOT NULL 列 v15/mod.rs:3247-3249，
    // 显式 null 拒清），PUT 送键即须可回读；结算 FEE 凭证按真实值计（后端集成测
    // tests/contract_wave5_trade_fields_roundtrip_test.rs 同口径锁定）。
    // 本例再红即缺陷①委外形态回归，禁止放宽成"键缺失容忍"。
    const order = await seedOutsourcingOrder(page, 'FEE');
    const orderId = requireNum(order.id, 'FEE 订单');
    await apiCall(page, 'PUT', `/production/outsourcing-orders/${orderId}`, {
      processing_fee: 100,
    });
    const reread = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    if (Number(reread.processing_fee) !== 100) {
      throw new Error(
        `加工费经契约录入回读回归（缺陷①委外形态）：draft 期 PUT processing_fee=100 后回读=` +
          JSON.stringify(reread.processing_fee) +
          `；应查 backend/src/services/outsourcing_ops/types.rs UpdateOutsourcingOrderRequest 的 ` +
          `processing_fee/freight_fee/tax_amount 三键与 backend/src/services/outsourcing_ops/order.rs ` +
          `update 写入接线（settle 语义 order.rs:587 ⇒ FEE 凭证 order.rs:631-656）。` +
          `实际键=${Object.keys(reread).join(',')}`
      );
    }
  });

  test('23-04 结算费用门：received 态 0 费用硬拒（400/BUSINESS_ERROR）且无痕——前端「结算」按钮禁用与之同口径', async ({
    page,
  }) => {
    const ctx = getCtx();
    // 建单不带费用键 ⇒ 三费落 NOT NULL 列 0 起步（types.rs serde(default)，v15/mod.rs:3247-3249）；
    // 全程不补录，订单推进到 received 后结算只剩费用门（状态门 order.rs:594-597 已通过）。
    const order = await seedOutsourcingOrder(page, 'ZFEE');
    const orderId = requireNum(order.id, 'ZFEE 订单');
    await apiCall(page, 'POST', `/production/outsourcing-orders/${orderId}/issue`);
    const rcpt = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: `E23-RF${genCode('ZF')}`,
        outsourcing_order_id: orderId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        return_quantity: '95',
        quality_status: 'qualified',
      }
    );
    const rcptId = requireNum(rcpt.id, '建收回单(zfee)');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${rcptId}`,
      label: `receipt(zfee)#${rcptId}`,
    });
    await apiCall(page, 'POST', `/production/outsourcing-receipts/${rcptId}/confirm`);
    const received = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    expectKeyValue(received, 'status', 'received', '前置：收回确认已落 received（receipt.rs:510）');
    expectDecimal(received, 'processing_fee', 0, '零费用前提（建单 0 起步回读）');
    expectDecimal(received, 'freight_fee', 0, '零费用前提（建单 0 起步回读）');

    const f = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${orderId}/settle`
    );
    expect(f.status, `0 费用结算应 400，实际=${f.status} body=${JSON.stringify(f)}`).toBe(400);
    expect(failureCode(f), '零费用结算机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 无痕（order.rs 费用门置于取号与事务 begin() 之前，拒绝时零副作用）：
    // 状态不漂移、不产生金额为 0 的空壳 OVFE 凭证、凭证号不落库
    const still = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    expectKeyValue(still, 'status', 'received', '被拒后订单仍 received 不漂移');
    expectKeyValue(still, 'voucher_no_fee', null, '被拒后不应回写加工费凭证号');
    const vouchers = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '零费用结算被拒后凭证列表'
    );
    expect(
      !vouchers.some(v => v.voucher_type === 'fee'),
      '零费用被拒不应留下 fee 凭证（拒绝点见 order.rs:601-611 注释）'
    ).toBe(true);
  });

  test('23-05 #220 实测值三列活体证明：建单落值(串型)→by-no/列表双路回读→三态(键缺席保持/显式null清空/有值覆盖)→confirm 出参逐列透传', async ({
    page,
  }) => {
    const ctx = getCtx();
    // 真实前置：订单经 seedOutsourcingOrder 自建（supplierId/productIds 来自 ensureTestEntities，
    // 与 23-01/23-04 同取值路径，无假 id），发料后收回单才允许 confirm（资格门 order.rs:42-52）
    const order = await seedOutsourcingOrder(page, 'MEAS');
    const orderId = requireNum(order.id, 'MEAS 订单');
    await apiCall(page, 'POST', `/production/outsourcing-orders/${orderId}/issue`);

    const receiptNo = `E23-RM${genCode('RC')}`;
    const created = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: receiptNo,
        outsourcing_order_id: orderId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        return_quantity: '95',
        quality_status: 'qualified',
        // 提交值一律十进制字符串（与后端集成锁 contract_wave7 receipt_body 同口径，不经浮点）；
        // 三值形状刻意不同：一位小数 / 整数 / 两位小数，钉 DECIMAL(18,4) 无损透传
        weight: '21.5',
        width: '185',
        gram_weight: '200.25',
      }
    );
    const rcptId = requireNum(created.id, '建收回单(带实测值)');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${rcptId}`,
      label: `receipt(meas)#${rcptId}`,
    });

    // ① POST 出参三键：串型+值等（前端补录表单即时回显的契约面，receipt.rs:269-271 写入形状）
    expectMeasured(created, 'weight', '21.5', 'POST 建单出参');
    expectMeasured(created, 'width', '185', 'POST 建单出参');
    expectMeasured(created, 'gram_weight', '200.25', 'POST 建单出参');

    // ② by-no 回读＝活库行：证明值真实落库，而非请求层内存回显
    const byNo = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-receipts/by-no/${receiptNo}`
    );
    expectKeyValue(byNo, 'id', rcptId, 'by-no 回读');
    expectMeasured(byNo, 'weight', '21.5', 'by-no 回读');
    expectMeasured(byNo, 'width', '185', 'by-no 回读');
    expectMeasured(byNo, 'gram_weight', '200.25', 'by-no 回读');

    // ③ 列表回读（前端表格渲染源，handler=list_outsourcing_receipts 同一 Model 序列化）：
    //    列不是只在详情端点存在——两条读取路径同源同形
    const listRows = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-receipts?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '收回单列表(实测值)'
    );
    const row = listRows.find(r => Number(r.id) === rcptId);
    expect(
      row,
      `列表应含自建行 id=${rcptId}，实际 ids=${JSON.stringify(listRows.map(r => r.id))}`
    ).toBeTruthy();
    expectMeasured(row!, 'weight', '21.5', '列表回读');
    expectMeasured(row!, 'width', '185', '列表回读');
    expectMeasured(row!, 'gram_weight', '200.25', '列表回读');

    // ④ 三态·键缺席=保持（types.rs:239 RFC 7386 口径）：只 PUT remarks，三键整体缺席 ⇒
    //    三列必须原样。后端若把"键缺席"当"清空"（洗成 NULL）或塞 0，此处即红
    await apiCall(page, 'PUT', `/production/outsourcing-receipts/${rcptId}`, {
      remarks: 'E2E-三态-键缺席探针',
    });
    const afterAbsent = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-receipts/by-no/${receiptNo}`
    );
    expectKeyValue(afterAbsent, 'remarks', 'E2E-三态-键缺席探针', '键缺席探针 PUT 应真实生效');
    expectMeasured(afterAbsent, 'weight', '21.5', '键缺席=保持');
    expectMeasured(afterAbsent, 'width', '185', '键缺席=保持');
    expectMeasured(afterAbsent, 'gram_weight', '200.25', '键缺席=保持');

    // ⑤ 三态·显式 null=清空（本用例核心活体证明点，receipt.rs:373-375 Some(None)=Set(None)）：
    //    后端若退化为"null 当不改"⇒ 回读残留 "21.5"，toBeNull 必红；洗成 0/'' 同样必红。
    //    另两列不得被连带改动——逐列独立性锁
    await apiCall(page, 'PUT', `/production/outsourcing-receipts/${rcptId}`, { weight: null });
    const afterNull = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-receipts/by-no/${receiptNo}`
    );
    expectMeasured(afterNull, 'weight', null, '显式 null=清空');
    expectMeasured(afterNull, 'width', '185', '清空 weight 不得连带改 width');
    expectMeasured(afterNull, 'gram_weight', '200.25', '清空 weight 不得连带改 gram_weight');

    // ⑥ 三态·有值=覆盖（清空后按实补录，NULL 不是一扇门）
    await apiCall(page, 'PUT', `/production/outsourcing-receipts/${rcptId}`, { weight: '22.25' });
    const afterOverwrite = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-receipts/by-no/${receiptNo}`
    );
    expectMeasured(afterOverwrite, 'weight', '22.25', '有值=覆盖（清后补录）');
    expectMeasured(afterOverwrite, 'width', '185', '有值=覆盖不得连带洗邻居');
    expectMeasured(afterOverwrite, 'gram_weight', '200.25', '有值=覆盖不得连带洗邻居');

    // ⑦ confirm 出参逐列透传（handler 返回 final_receipt=DB 行；receipt.rs:523-525 匹行透传
    //    的数据源就是这三列，落匹行侧由后端集成锁 wave7① 真库钉死，此处锁 API 契约面）
    const confirmed = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/outsourcing-receipts/${rcptId}/confirm`
    );
    expectKeyValue(confirmed, 'status', 'confirmed', 'confirm 出参词值');
    expectMeasured(confirmed, 'weight', '22.25', 'confirm 出参三列');
    expectMeasured(confirmed, 'width', '185', 'confirm 出参三列');
    expectMeasured(confirmed, 'gram_weight', '200.25', 'confirm 出参三列');
  });

  test('23-06 #220 实测值值域负例：建单/更新口 0 与负数一律 400+VALIDATION_ERROR，被拒写入零变化（只断 status+信封 code，不断脱敏文案）', async ({
    page,
  }) => {
    const ctx = getCtx();
    const order = await seedOutsourcingOrder(page, 'MZV');
    const orderId = requireNum(order.id, 'MZV 订单');
    await apiCall(page, 'POST', `/production/outsourcing-orders/${orderId}/issue`);

    // A) 建单口逐列点名：weight=0 ⇒ 值域门（receipt.rs:230 validate_measured_values → :65-74，
    //    与 m0075 DB CHECK 逐字同口径）先于 INSERT 拒绝；拒绝必须零落库——by-no 回读 404/NOT_FOUND
    const badNoW = `E23-RW0${genCode('W0')}`;
    const fa = await apiCallExpectFail(page, 'POST', '/production/outsourcing-receipts', {
      receipt_no: badNoW,
      outsourcing_order_id: orderId,
      receipt_date: todayStr(),
      product_id: ctx.productIds[0],
      return_quantity: '10',
      quality_status: 'qualified',
      weight: '0',
    });
    expect(fa.status, `建单 weight=0 应 400，实际=${fa.status} body=${JSON.stringify(fa)}`).toBe(
      400
    );
    expect(failureCode(fa), '建单实测值 0 机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const faGone = await apiCallExpectFail(
      page,
      'GET',
      `/production/outsourcing-receipts/by-no/${badNoW}`
    );
    expect(faGone.status, '被拒建单应零落库（by-no 回读 404）').toBe(404);
    expect(failureCode(faGone), '被拒建单零落库回读机器码').toBe(APP_ERROR_CODES.NOT_FOUND);

    // B) 建单口·width 负数（逐列成组校验缺一列＝该列可被伪造实测值绕过标签 fail-closed）
    const fb = await apiCallExpectFail(page, 'POST', '/production/outsourcing-receipts', {
      receipt_no: `E23-RW1${genCode('W1')}`,
      outsourcing_order_id: orderId,
      receipt_date: todayStr(),
      product_id: ctx.productIds[0],
      return_quantity: '10',
      quality_status: 'qualified',
      width: '-5',
    });
    expect(fb.status, `建单 width=-5 应 400，实际=${fb.status}`).toBe(400);
    expect(failureCode(fb), '建单 width 负数机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // C) 建单口·gram_weight 负小数（-0.5 钉"<=0 拒绝"不是"<0"松门）
    const fc = await apiCallExpectFail(page, 'POST', '/production/outsourcing-receipts', {
      receipt_no: `E23-RW2${genCode('W2')}`,
      outsourcing_order_id: orderId,
      receipt_date: todayStr(),
      product_id: ctx.productIds[0],
      return_quantity: '10',
      quality_status: 'qualified',
      gram_weight: '-0.5',
    });
    expect(fc.status, `建单 gram_weight=-0.5 应 400，实际=${fc.status}`).toBe(400);
    expect(failureCode(fc), '建单 gram_weight 负数机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // D) 对照组：正常带值 draft 收回单（更新口负例的零变化基准）
    const okNo = `E23-RK${genCode('OK')}`;
    const okRcpt = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: okNo,
        outsourcing_order_id: orderId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        return_quantity: '10',
        quality_status: 'qualified',
        weight: '21.5',
        width: '185',
        gram_weight: '200.25',
      }
    );
    const okId = requireNum(okRcpt.id, '对照组收回单');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${okId}`,
      label: `receipt(mzv)#${okId}`,
    });

    // E/F/G) 更新口逐列值域门（receipt.rs:373-384 校验 `?` 前置于 active.update :387 ⇒
    //    拒绝=库内零变化）：每列被拒后回读三列全部原值——既钉"越界没写进去"，
    //    也钉"被拒更新没有半行副作用/没把别的列顺手洗掉"
    for (const [col, bad] of [
      ['weight', '0'],
      ['width', '-1'],
      ['gram_weight', '-0.01'],
    ] as const) {
      const f = await apiCallExpectFail(page, 'PUT', `/production/outsourcing-receipts/${okId}`, {
        [col]: bad,
      });
      expect(
        f.status,
        `更新 ${col}=${bad} 应 400，实际=${f.status} body=${JSON.stringify(f)}`
      ).toBe(400);
      expect(failureCode(f), `更新 ${col} 越界机器码`).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
      const still = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-receipts/by-no/${okNo}`
      );
      expectMeasured(still, 'weight', '21.5', `${col}=${bad} 被拒后零变化`);
      expectMeasured(still, 'width', '185', `${col}=${bad} 被拒后零变化`);
      expectMeasured(still, 'gram_weight', '200.25', `${col}=${bad} 被拒后零变化`);
    }
  });
});
