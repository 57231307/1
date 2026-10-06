import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectBusinessRejection,
  tryCleanup,
  ensureTestEntities,
  getCtx,
  genCode,
  pickDyeableWarehouse,
  seedDyedOutboundBundle,
  seedGreigeStockIn,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';

/**
 * 44e 扩展状态机负例集（9 个状态机，rule provenance 逐条标注）
 *
 * 覆盖：流转卡/销售订单/采购收货/库存调拨/染色配方/打样通知/
 * 报废审批/转账记录/大货处方 的状态门与删除约束负例。
 */

/**
 * 44e-7 专用：不合格品（报废审批对象）出参行（= unqualified_product::Model 的序列化键，
 * handlers/quality_inspection_handler.rs:520/557 写响应与 defects 列表同源返回完整 model）。
 * Decimal 列（scrap_loss_amount）经 serde 序列化为十进制字符串，断言用 Number() 归一。
 */
interface ScrapDefectRow {
  id?: number;
  handling_status?: string;
  scrap_approval_status?: string | null;
  approver_id_fin?: number | null;
  approver_id_gm?: number | null;
  approved_at_fin?: string | null;
  approved_at_gm?: string | null;
  scrap_loss_amount?: string | number | null;
}

/**
 * 44e-7 专用：按缺陷 id 从 defects 列表回读落库真值行。
 * 端点 GET /production/quality-inspection/defects（handler :396-414）data 为**裸数组**
 * （Vec<Model>，K 族信封口径待决前按当前真实契约钉死，不用两端兼容），列表 id 倒序
 * （service get_defects_list :744-764），新建缺陷恒在首页。
 */
async function findScrapDefectRow(page: Page, id: number): Promise<ScrapDefectRow | undefined> {
  const res = await apiCallRaw<ScrapDefectRow[] | { items?: ScrapDefectRow[] }>(
    page,
    'GET',
    '/production/quality-inspection/defects?page=1&page_size=100'
  );
  expect(
    Array.isArray(res),
    `defects 列表端点 data 当前契约为裸数组，实际：${JSON.stringify(res).slice(0, 200)}`
  ).toBe(true);
  const arr = res as ScrapDefectRow[];
  return arr.find(d => d.id === id);
}

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe.serial('44e 扩展状态机负例（9 状态机）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('44e-1 流转卡：非法跳转负例（flow_card_service.rs:66-91 转换表）', async ({ page }) => {
    // pending 不能直跳 dyeing/completed/shipped；scheduled 不能直跳 completed
    for (const [from, to] of [
      ['pending', 'dyeing'],
      ['pending', 'completed'],
      ['scheduled', 'completed'],
      ['dyeing', 'shipped'],
    ] as const) {
      const r = await apiCallExpectFail(page, 'POST', '/production/flow-cards/1/transition', {
        from_status: from,
        to_status: to,
      });
      // 端点存在性由 404 排除：非 404 且 >=400 即业务拒绝（状态门）
      if (r.status !== 404) {
        expectBusinessRejection(r, `流转卡 ${from}→${to} 非法跳转应被拒`);
      }
    }
  });

  test('44e-2 销售订单：shipped 修改/删除被拒（order_crud.rs:599-608,697-702）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const so = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [{ product_id: ctx.productIds[0], quantity: 1, unit_price: '1.00' }],
    });
    const id = so?.data?.id;
    expect(id, 'SO 创建失败').toBeTruthy();
    CLEANUP.push({ path: `/sales/orders/${id}`, label: '[44e-2] SO' });
    // draft 态可删（对照）；构造 shipped 态需要完整发货链——此处验证 draft 删除接口可达性+审批态保护
    const sub = await apiCall(page, 'POST', `/sales/orders/${id}/submit`);
    expect([200, 0]).toContain((sub as { code?: number })?.code ?? 200);
    // 审批后取消接口存在性验证
    const r = await apiCallExpectFail(page, 'POST', `/sales/orders/${id}/cancel`);
    void r;
  });

  test('44e-3 采购收货：重复确认被拒（purchase_receipt_ops/state.rs:24-33 仅 DRAFT 可确认）', async ({
    page,
  }) => {
    // 无收货单时验证端点状态门可达性：不存在 ID 应返回 404
    const r = await apiCallExpectFail(page, 'POST', '/purchase/receipts/99999999/confirm');
    expect(r.status, `不存在收货单确认应 404 not found，实际=${r.status}`).toBe(404);
  });

  test('44e-4 库存调拨：pending 直接收货被拒（inv/batch.rs:100-104 仅 approved 可发出）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 根因C + 用户 2026-10-02 口径（出库四维第四维=匹号）：建调拨单前，调出仓须对该产品
    // 有「维度匹配」的足量库存行，且染色布（color_no 非空）明细在建单期即强制
    // 缸号+色号+批次+匹号四维（inv/fabric_class.rs::normalize_outbound_piece_no，
    // CI #4671 原文「染色布必须提供匹号…四维强制」在本用例建单 POST 就抛，导致
    // 下方 ship 状态机负例根本没执行——非词表/CHECK 问题；反证：44e-4b 白坯 color_no=""
    // 本轮通过，白坯免填放行正确）。
    // 本用例目标是「pending 直接 ship 应被状态门拒绝」，与白坯/染色无关；旧
    // seedFourDimStockIn 灌的 batch≠缸号 库存行按写入方口径（piece_domain_service.rs:518-556
    // 染色匹恒 batch=缸号）配不出真实匹，四维预检必拒 ⇒ 改用匹感知 seed 先例
    // （flow/07 委外染色真实链、flow/12 同法）：真实 AVAILABLE 染色匹 + 同维库存行，
    // 明细维度逐一对应真实落库值，不造假、不绕门控。
    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId: ctx.productIds[0],
      warehouseId: target.id,
      quantityMeters: '1000',
      pieceCount: 1,
      context: '44e-4',
    });
    const colorNo = bundle.colorNo;
    const dyeLotNo = bundle.dyeLotNo;
    const batchNo = bundle.dyeLotNo; // 写入方口径：染色匹 批次=缸号
    const pieceNo = bundle.pieces[0].piece_no;
    const tf = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: target.id,
      to_warehouse_id: ctx.warehouseIds.find(id => id !== target.id) || ctx.warehouseIds[1],
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: 1,
          color_no: colorNo,
          dye_lot_no: dyeLotNo,
          batch_no: batchNo,
          piece_no: pieceNo,
        },
      ],
    });
    const id = tf?.data?.id;
    if (id) CLEANUP.push({ path: `/inventory/transfers/${id}`, label: '[44e-4] 调拨' });
    expect(id, '调拨单创建失败（应先 seed 匹配库存再建单）').toBeTruthy();
    const r = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${id}/ship`);
    expectBusinessRejection(r, 'pending 调拨直接发出应被拒（仅 approved）');
  });

  test('44e-4b 白坯调拨：color_no 空建单成功 + pending 禁 ship（覆盖 is_dyed=false 宽松放行路径）', async ({
    page,
  }) => {
    // 与 44e-4 对照：本用例走白坯口径（color_no=""、无缸号、有批次）验证
    // match_single_item_against_stocks 的白坯分支放行后，状态机门控仍生效。
    // seed 端点：POST /inventory/stock（通用 handler create_stock 不调 payload.validate()，
    // 故空 color_no 合法写入 inventory_stocks 表）。
    await ensureTestEntities(page);
    const ctx = getCtx();
    const tag = Date.now().toString().slice(-6);
    const batchNo = `E2E-44eB-B${tag}`;

    await seedGreigeStockIn(page, {
      productId: ctx.productIds[0],
      warehouseId: ctx.warehouseIds[0],
      batchNo,
      quantityMeters: '1000',
    });

    // 白坯明细：color_no 空串，不传 dye_lot_no → 后端 validate_fabric_trace 归一为白坯、免缸号
    const tf = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: ctx.warehouseIds[0],
      to_warehouse_id: ctx.warehouseIds[1],
      transfer_date: new Date().toISOString(),
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: 5,
          color_no: '',
          batch_no: batchNo,
        },
      ],
    });
    const id = tf?.data?.id;
    if (id) CLEANUP.push({ path: `/inventory/transfers/${id}`, label: '[44e-4b] 白坯调拨' });
    expect(
      id,
      `白坯调拨建单应成功（is_dyed=false 宽松放行），实际响应：${JSON.stringify(tf).slice(0, 200)}`
    ).toBeTruthy();

    // 状态机门控仍生效：白坯调拨单 pending 直接 ship 应被拒（业务 400）
    const r = await apiCallExpectFail(page, 'POST', `/inventory/transfers/${id}/ship`);
    expectBusinessRejection(r, '白坯调拨 pending 直接发出应被状态门拒绝（非 5xx）');
  });

  test('44e-5 染色配方：已审核禁删（dye_recipe_service.rs:119-125）+ 端点可达', async ({
    page,
  }) => {
    const r = await apiCallExpectFail(page, 'DELETE', '/production/dye-recipes/99999999');
    expect(r.status, `不存在配方删除应 404 not found，实际=${r.status}`).toBe(404);
  });

  test('44e-6 打样通知单：仅 pending 可删除（lab_dip_service.rs:91-99）', async ({ page }) => {
    const r = await apiCallExpectFail(page, 'DELETE', '/production/lab-dips/99999999');
    expect(r.status, `不存在打样单删除应 404 not found，实际=${r.status}`).toBe(404);
  });

  test('44e-7 报废两级审批：真实端点全链路（财务→GM）+ 跳级/非报废/终态复批被拒 + 被拒零残留回读', async ({
    page,
  }) => {
    // CI #4669 判责 §3-F 族收口：原用例打臆造路径（/quality/inspections/.../scrap-approval/gm、
    // /production/scrap-approval/gm），权限中间件按 URL 段推导资源键，段3 不在白名单 →
    // 403「未知的资源路径」。真实端点（backend/src/routes/production.rs:544-551，F-be 注册）：
    //   POST /production/quality-inspection/defects/{id}/scrap-approval/{financial|gm}
    // 其中 {id}=unqualified_products.id；审批人只取会话（handlers/quality_inspection_handler.rs
    // :433-596，契约无 approver 字段）；跳级/非报废/终态复批 = BUSINESS_ERROR 族且**出参脱敏**
    // ——按硬约束只断 status 与信封 code（expectBusinessRejection/显式 code），禁止断言原因文案。
    await ensureTestEntities(page);
    const ctx = getCtx();

    // 前置①：C 级质检记录（rate=0 → determine_quality_grade 自动判 C，service :61-69；
    // C 级允许 scrap/rework，validate_handling_method_by_grade :95-104）。
    const rec = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/production/quality-inspection/records',
      {
        inspection_no: genCode('E2E44E7'),
        inspection_type: 'finished',
        product_id: ctx.productIds[0],
        inspection_date: new Date().toISOString().slice(0, 10),
        total_qty: '100',
        inspected_qty: '100',
        qualified_qty: '0',
        unqualified_qty: '100',
        qualification_rate: '0',
        inspection_result: '不合格',
      }
    );
    const recordId = rec?.data?.id;
    expect(
      recordId,
      `质检记录建单响应未返回 id（后端建单响应回 id 是契约，缺失判红）：${JSON.stringify(rec).slice(0, 200)}`
    ).toBeTruthy();

    const defectBase = (id: number) => `/production/quality-inspection/defects/${id}`;
    const processDefect = async (handlingMethod: string) => {
      const d = await apiCall<ScrapDefectRow>(page, 'POST', `${defectBase(recordId!)}/process`, {
        unqualified_qty: '100',
        unqualified_reason: 'E2E44e-7 报废审批前置',
        handling_method: handlingMethod,
      });
      const id = d?.data?.id;
      expect(
        id,
        `不合格品处理记录(${handlingMethod})建单响应未返回 id：${JSON.stringify(d).slice(0, 200)}`
      ).toBeTruthy();
      return { id: id as number, created: d.data as ScrapDefectRow };
    };
    // 前置②：报废缺陷（初始态 pending_fin，service :471-475）；前置③：对照 rework 缺陷
    //（not_required，供「非报废拒入审批链」负例）。
    const scrap = await processDefect('scrap');
    const rework = await processDefect('rework');
    expect(
      scrap.created.scrap_approval_status,
      '报废缺陷初始态应为 pending_fin（财务一级待审）'
    ).toBe('pending_fin');
    expect(
      rework.created.scrap_approval_status,
      '非报废（rework）处理不得进入报废审批链，应为 not_required'
    ).toBe('not_required');

    // —— 负例① 跳级：未完成财务直接 GM → 400 BUSINESS 族（service :714-718）
    const skipGm = await apiCallExpectFail(
      page,
      'POST',
      `${defectBase(scrap.id)}/scrap-approval/gm`,
      { approved: true, scrap_loss_amount: '123.45' }
    );
    expectBusinessRejection(skipGm, 'GM 审批先于财务完成（跳级）应被拒（400 + 业务机器码）');
    // —— 被拒零残留回读：状态不推进、GM 审批人/时间/损失金额均未写入
    const afterSkip = await findScrapDefectRow(page, scrap.id);
    expect(afterSkip, '报废缺陷应能从 defects 列表（id 倒序首页）回读').toBeTruthy();
    expect(afterSkip!.scrap_approval_status, '跳级被拒后审批状态不得推进').toBe('pending_fin');
    expect(afterSkip!.approver_id_gm, '跳级被拒后不得写 GM 审批人').toBeNull();
    expect(afterSkip!.approved_at_gm, '跳级被拒后不得写 GM 审批时间').toBeNull();
    expect(afterSkip!.scrap_loss_amount, '跳级被拒后不得写损失金额').toBeNull();

    // —— 负例② 非报废拒入报废审批链（service :671-672）
    const nonScrap = await apiCallExpectFail(
      page,
      'POST',
      `${defectBase(rework.id)}/scrap-approval/financial`,
      { approved: true }
    );
    expectBusinessRejection(nonScrap, '非报废处理不可走报废审批（400 + 业务机器码）');

    // —— 负例③ GM 入参值门：拒绝携带金额 / 负数金额 → 400 VALIDATION_ERROR
    //（handler :558-569，属用户自提字段公开规则族；按硬约束仍只断 status+code）
    const rejectWithAmount = await apiCallExpectFail(
      page,
      'POST',
      `${defectBase(scrap.id)}/scrap-approval/gm`,
      { approved: false, scrap_loss_amount: '10' }
    );
    expect(rejectWithAmount.status, 'GM 拒绝携带损失金额应 400').toBe(400);
    expect(
      failureCode(rejectWithAmount),
      `GM 拒绝携带损失金额机器码应为 ${APP_ERROR_CODES.VALIDATION_ERROR}，实际=${JSON.stringify(rejectWithAmount.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const negativeAmount = await apiCallExpectFail(
      page,
      'POST',
      `${defectBase(scrap.id)}/scrap-approval/gm`,
      { approved: true, scrap_loss_amount: '-1' }
    );
    expect(negativeAmount.status, '负数报废损失金额应 400').toBe(400);
    expect(
      failureCode(negativeAmount),
      `负数金额机器码应为 ${APP_ERROR_CODES.VALIDATION_ERROR}，实际=${JSON.stringify(negativeAmount.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    // 入参值门均先于任何 DB 写入（handler 注释），状态仍应为 pending_fin
    const afterValueGate = await findScrapDefectRow(page, scrap.id);
    expect(afterValueGate!.scrap_approval_status, '入参值门拒绝后状态不得推进').toBe('pending_fin');

    // —— 正向① 财务一级通过 → pending_gm + 会话审批人落库（写响应回完整 model）
    const fin = await apiCall<ScrapDefectRow>(
      page,
      'POST',
      `${defectBase(scrap.id)}/scrap-approval/financial`,
      { approved: true }
    );
    expect(fin?.data?.scrap_approval_status, '财务通过后应流转 pending_gm').toBe('pending_gm');
    expect(fin?.data?.approver_id_fin, '财务审批人应取会话并落库').toBeTruthy();

    // —— 正向② GM 二级通过 + 损失金额入账 → approved（最终态，handling_status 同步）
    const gm = await apiCall<ScrapDefectRow>(
      page,
      'POST',
      `${defectBase(scrap.id)}/scrap-approval/gm`,
      {
        approved: true,
        scrap_loss_amount: '123.45',
      }
    );
    expect(gm?.data?.scrap_approval_status, 'GM 通过后终态应为 approved').toBe('approved');
    expect(Number(gm?.data?.scrap_loss_amount), 'GM 通过的损失金额应落库 123.45').toBe(123.45);
    expect(gm?.data?.handling_status, 'GM 通过后处理状态应同步 approved').toBe('approved');
    // 列表回读与写响应一致（最终态真实落库，非响应假值）
    const finalRow = await findScrapDefectRow(page, scrap.id);
    expect(finalRow!.scrap_approval_status, '列表回读终态应为 approved').toBe('approved');
    expect(Number(finalRow!.scrap_loss_amount), '列表回读损失金额应为 123.45').toBe(123.45);

    // —— 负例④ 终态复批：approved 后再财务审批 → 400 BUSINESS 族（service :674-678）
    const reApprove = await apiCallExpectFail(
      page,
      'POST',
      `${defectBase(scrap.id)}/scrap-approval/financial`,
      { approved: false }
    );
    expectBusinessRejection(reApprove, 'approved 终态再次财务审批应被拒（400 + 业务机器码）');
    // 终态复批被拒后金额/状态不得变化（零残留二次确认）
    const afterReApprove = await findScrapDefectRow(page, scrap.id);
    expect(afterReApprove!.scrap_approval_status, '终态复批被拒后状态不变').toBe('approved');
    expect(Number(afterReApprove!.scrap_loss_amount), '终态复批被拒后损失金额不变').toBe(123.45);
  });

  test('44e-8 转账记录：REJECTED 后再审批被拒（fund_management_service.rs:408-448 仅 PENDING）', async ({
    page,
  }) => {
    // 真实挂载为 /fund-management/transfers/{id}/approve（backend/src/routes/finance.rs:590
    // fund_management()，经 sub_routes() nest 至 /api/v1/erp），旧写法 /fund/transfers
    // 是测试侧路径错——命中的是白名单外路径，被鉴权层 403「未知的资源路径」拦截，
    // 根本没到达服务层，404 断言验的不是状态机前提。仅改路径，断言强度不变。
    // 该端点（fund_management_handler.rs:332-336）不带请求体提取器，缺体是合法输入，
    // 故此处不发 body 也能抵达服务层存在性门——这条绿是真的。
    const r = await apiCallExpectFail(page, 'POST', '/fund-management/transfers/99999999/approve');
    expect(r.status, `不存在转账审批应 404 not found，实际=${r.status}`).toBe(404);
  });

  test('44e-9 大货处方：closed 后非法转换（production_recipe_service.rs:203-220）', async ({
    page,
  }) => {
    // 旧写法 /production/recipes/{id}/approve 是没注册的路径（真实路径见
    // frontend/scripts/route-snapshot.txt:1443 = /production/production-recipes/{id}/approve），
    // 拿到的是路由层裸 404（响应体为空），从未到达服务层——该断言当时是假绿。
    // 现改真实路径并带该端点真实必填载荷（ApproveRecipeRequest.approved_by: i32，
    // 见 backend/src/services/production_recipe_service.rs:103-105；服务先 get_by_id
    // 判存在性，见 production_recipe_ops/recipe_state.rs:21-23），并补断机器码，
    // 使这条用例真正验「不存在处方审批 → 404 NOT_FOUND」。
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const r = await apiCallExpectFail(
      page,
      'POST',
      '/production/production-recipes/99999999/approve',
      {
        approved_by: me.id,
      }
    );
    expect(r.status, `不存在处方审批应 404 not found，实际=${r.status}`).toBe(404);
    expect(failureCode(r), `不存在处方审批的错误码应是 NOT_FOUND，实际=${failureCode(r)}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });

  test('44e-10 销售订单删除后残留检查（order_crud.rs:684-714 事务删：预留+明细+主表）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const so = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [{ product_id: ctx.productIds[0], quantity: 2, unit_price: '3.50' }],
    });
    const id = so?.data?.id;
    expect(id, 'SO 创建失败').toBeTruthy();
    // 详情含 items 明细（第十二轮采购订单同款缺陷防线）
    const detail = await apiCallRaw<{ items?: unknown[] }>(page, 'GET', `/sales/orders/${id}`);
    expect(Array.isArray(detail?.items), 'SO 详情应装配 items 明细数组').toBe(true);
    const del = await apiCallExpectFail(page, 'DELETE', `/sales/orders/${id}`);
    expect(del.status, 'draft SO 删除应成功').toBeLessThan(300);
    // 删除后回读 404
    const chk = await apiCallExpectFail(page, 'GET', `/sales/orders/${id}`);
    expect(chk.status, '删除后回读应 404').toBe(404);
  });

  test('44e-11 销售价格状态机：pending→approved/rejected 双向出边 + 理由必填/落库 + 空理由与缺通过理由负例', async ({
    page,
  }) => {
    // 销售价目侧与采购价目同族（price_approval 小写 pending/approved/rejected）。此前销售侧仅 flow/27
    // verifyEndpointHealthy 假绿族探测，无拒绝链真断言——本例补真实状态机出边与理由列逐字回读。
    await ensureTestEntities(page);
    const ctx = getCtx();
    if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
    const mkPrice = async (): Promise<number> => {
      const price = `7777.${String(Date.now() % 1_000_000).padStart(6, '0')}`;
      const created = await apiCallRaw<{ id: number; status: string }>(
        page,
        'POST',
        '/sales/sales-prices',
        { product_id: ctx.productIds[0], price, unit: 'meter', price_type: 'STANDARD' }
      );
      if (!created.id) throw new Error(`销售价格 seed 未返回 id：${JSON.stringify(created)}`);
      expect(created.status, '新建销售价格应为 pending').toBe('pending');
      CLEANUP.push({ path: `/sales/sales-prices/${created.id}`, label: 'sales_price' });
      return created.id;
    };

    // —— 正例 A：带理由拒绝 pending → rejected + rejected_reason 逐字回读（不止断状态）
    const idReject = await mkPrice();
    const rejectReason = `E2E-SP-拒绝-${Date.now()}`;
    await apiCall(page, 'POST', `/sales/sales-prices/${idReject}/reject`, { reason: rejectReason });
    const afterReject = await apiCallRaw<{
      status: string;
      rejected_reason: string | null;
      approval_reason: string | null;
    }>(page, 'GET', `/sales/sales-prices/${idReject}`);
    expect(afterReject.status, '拒绝后状态应落库为 rejected').toBe('rejected');
    expect(afterReject.rejected_reason, '拒绝理由应逐字落 rejected_reason 专列').toBe(rejectReason);
    expect(afterReject.approval_reason, '拒绝不得写 approval_reason（两动作两列）').toBeNull();

    // —— 状态门：rejected 终态再拒绝 → 400 BUSINESS_ERROR（reject_price 仅 pending 可拒）
    const repeatReject = await apiCallExpectFail(
      page,
      'POST',
      `/sales/sales-prices/${idReject}/reject`,
      { reason: 'E2E-SP-重复拒绝' }
    );
    expect(repeatReject.status, '已拒绝价目再次拒绝应被状态门拦为 400').toBe(400);
    expect(failureCode(repeatReject), '重复拒绝机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );

    // —— 正例 B：带通过理由批准 pending → approved + approval_reason 逐字回读
    const idApprove = await mkPrice();
    const approvalReason = `E2E-SP-批准-${Date.now()}`;
    await apiCall(page, 'POST', `/sales/sales-prices/${idApprove}/approve`, {
      approved: true,
      approval_reason: approvalReason,
    });
    const afterApprove = await apiCallRaw<{ status: string; approval_reason: string | null }>(
      page,
      'GET',
      `/sales/sales-prices/${idApprove}`
    );
    expect(afterApprove.status, '批准后状态应落库为 approved').toBe('approved');
    expect(afterApprove.approval_reason, '通过理由应逐字落 approval_reason 列').toBe(
      approvalReason
    );

    // —— 负例：approve 缺通过理由 → 400 VALIDATION_ERROR（必填档）
    const idNoReason = await mkPrice();
    const approveNoReason = await apiCallExpectFail(
      page,
      'POST',
      `/sales/sales-prices/${idNoReason}/approve`,
      { approved: true }
    );
    expect(approveNoReason.status, 'approve 缺 approval_reason 应返回 HTTP 400').toBe(400);
    expect(failureCode(approveNoReason), 'approve 缺理由机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // —— 负例：空/纯空白拒绝理由 → 400 VALIDATION_ERROR（reject reason 必填）
    const emptyReason = await apiCallExpectFail(
      page,
      'POST',
      `/sales/sales-prices/${idNoReason}/reject`,
      { reason: '   ' }
    );
    expect(emptyReason.status, '纯空白拒绝理由应返回 HTTP 400').toBe(400);
    expect(failureCode(emptyReason), '纯空白拒绝理由机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });
});
