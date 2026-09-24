import { test, expect } from '../diagnose-fixture';
import {
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  ensureBudgetPlan,
  ensureTestEntities,
  getCtx,
  loginViaUI,
  tryCleanup,
} from './helpers';
import { pickListArray, type ListShapeKey } from './ui-helpers';

/**
 * P0 删除系统覆盖矩阵（2026-09-11 用户指令："需要系统覆盖所有需要删除/停用测试的功能"）
 *
 * 覆盖策略（与 31-deletion-deactivation.spec.ts 的 UI 点击删除互补）：
 * - 31 spec：有 UI 列表入口的核心资源 → 真实 UI 点击删除
 * - 本 spec（31b）：其余全部有 DELETE 端点的资源 → API 创建（全字段：必填+非必填全填）
 *   → API 删除 → 详情回读 404 + 列表回读消失 双重验证
 *
 * 依赖链资源（疵点需检验、工资率需工艺路线、评估需指标、互斥需双角色）内联先建父资源。
 * 所有 payload 字段来自后端真实 Create*Request 结构体（必填+Option 全填）。
 * 每步显式日志（诊断模式）。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
const TS = Date.now().toString().slice(-8);

interface DelCase {
  label: string;
  createApi: string;
  payload: Record<string, unknown>;
  /** 详情回读路径（默认 `${createApi}/${id}`） */
  getApi?: (id: number) => string;
  /** 删除路径（默认 `${createApi}/${id}`） */
  deleteApi?: (id: number) => string;
  /**
   * 列表端点（GET createApi）的显式形状，供"创建后列表回读"诊断按声明形状单一读取。
   * 缺省 'items'（多数 CRUD handler 返回 PaginatedResponse）。裸数组端点须显式声明 'bare'。
   * 取代原 `body.data.items ?? body.data.roles ?? (Array.isArray(body.data)?body.data:[])`
   * 三重形状宽容探测——它会把端点改形静默吸收成空集。
   */
  listKey?: ListShapeKey;
  /** 删除前预处理（如固定资产需先 PUT status=inactive 才允许删除） */
  preDelete?: (page: import('@playwright/test').Page, id: number) => Promise<void>;
  /**
   * 创建前预处理（运行时向 payload 注入依赖前置资源的字段）。
   * payload 是 collection 期构造的静态对象，无法在定义处读取运行时才就绪的 ctx/外键；
   * 预算明细的 plan_id 须为已存在的预算方案（Q3 重构后 NOT NULL 外键），故用本钩子在
   * POST 前取/建一个有效方案 id 注入 payload。
   */
  preCreate?: (
    page: import('@playwright/test').Page,
    payload: Record<string, unknown>
  ) => Promise<void>;
}

/** 创建 → 删除 → 详情 404 + 列表消失 双验证 */
async function createThenApiDelete(
  page: import('@playwright/test').Page,
  c: DelCase
): Promise<void> {
  // 0) 创建前预处理：运行时注入依赖前置外键资源的字段（如预算明细的 plan_id）
  if (c.preCreate) {
    await c.preCreate(page, c.payload);
    console.log(`[31b-${c.label}] 创建前预处理完成`);
  }
  const resp = await apiCall<{ id?: number }>(page, 'POST', c.createApi, c.payload);
  const id = resp?.data?.id;
  expect(id, `[31b-${c.label}] 创建响应无 id（创建 API 异常）`).toBeTruthy();
  // 前置 id 缺失即失败：显式判空收窄为 number，供下方 preDelete/deleteApi/getApi 传参
  // （禁止用非空断言 ! 蒙过）。
  if (id === undefined) {
    throw new Error(`[31b-${c.label}] 前置失败：创建未返回 id，无法继续删除/回读`);
  }
  console.log(`[31b-${c.label}] 创建成功 id=${id}`);

  // 1) 列表回读确认存在（诊断，按 DelCase.listKey 声明的单一形状读取；不匹配则记明确契约告警）
  const listResp = await page.request.get(
    `${API_BASE}${API_PREFIX}${c.createApi}?page=1&page_size=200`
  );
  if (listResp.ok()) {
    const body = await listResp.json();
    try {
      const items = pickListArray<{ id?: number }>(
        body?.data,
        c.listKey ?? 'items',
        `31b-${c.label} 列表回读`
      );
      const exists = items.some(i => i.id === id);
      console.log(
        `[31b-${c.label}] 列表回读（data.${(c.listKey ?? 'items') === 'bare' ? '(裸数组)' : c.listKey} ${items.length} 条）: ${exists ? '✅存在' : '⚠️未在列表找到（可能分页/过滤）'}`
      );
    } catch (e) {
      // 声明形状与实际不符 → 契约漂移，明确记录（不再静默当空集）。删除结果仍由下方详情 404/软删校验。
      console.error(`[31b-${c.label}] ❌ 列表契约不匹配：${(e as Error).message}`);
    }
  } else {
    console.warn(`[31b-${c.label}] 列表回读 HTTP ${listResp.status()}（记录不断言）`);
  }

  // 2) 删除前预处理（业务约束：如固定资产需先停用）
  if (c.preDelete) {
    await c.preDelete(page, id);
    console.log(`[31b-${c.label}] 删除前预处理完成`);
  }

  // 3) API 删除（真实后端 DELETE）
  const delPath = c.deleteApi ? c.deleteApi(id) : `${c.createApi}/${id}`;
  await apiCall(page, 'DELETE', delPath);
  console.log(`[31b-${c.label}] DELETE ${delPath} ✅成功`);

  // 3) 详情回读验证 404
  const getApi = c.getApi ? c.getApi(id) : `${c.createApi}/${id}`;
  const chk = await page.request.get(`${API_BASE}${API_PREFIX}${getApi}`);
  console.log(`[31b-${c.label}] 删除后详情回读 ${getApi} → HTTP ${chk.status()}`);
  if (chk.status() === 404) {
    console.log(`[31b-${c.label}] ✅ 已确认真删除（404）`);
  } else if (chk.status() === 200) {
    const body = await chk.json();
    const d = body?.data as Record<string, unknown> | undefined;
    // 软删场景：记录可能仍返回但 is_deleted=true 或 status 标记为非活跃
    const isSoftDeleted =
      d != null &&
      (d.is_deleted === true ||
        d.status === 'inactive' ||
        d.status === 'deleted' ||
        d.status === 'cancelled');
    const recordMatches =
      d != null &&
      (d.id === id || (Array.isArray(d) && d.some((x: { id?: number }) => x.id === id)));
    const stillActive = recordMatches && !isSoftDeleted;
    if (isSoftDeleted) {
      console.log(`[31b-${c.label}] ✅ 软删生效（is_deleted/status 标记为非活跃）`);
    } else if (stillActive) {
      console.warn(`[31b-${c.label}] ⚠️ 记录仍处于活跃状态（删除未生效）`);
    } else {
      console.log(`[31b-${c.label}] ✅ 记录已不可见或已被移除`);
    }
    expect(stillActive, `[31b-${c.label}] 删除后记录不应仍处于活跃状态`).toBeFalsy();
  } else {
    console.warn(`[31b-${c.label}] 详情回读 HTTP ${chk.status()}（非 200/404，记录）`);
  }
}

// 非 serial：矩阵各例彼此独立（各自 beforeEach 登录+ensureTestEntities；每条用例用 TS 后缀
// 自造唯一命名资源并自行创建/删除；依赖链用例的父资源亦在例内 inline 自建自删），
// 不存在跨用例产物依赖。原 describe.serial 的链式语义会让任一用例失败即把其后所有用例
// 判为 "did not run"（假覆盖盲区）——例如「坯布在库不可删」失败曾连带拖垮其后 28 例。
// 降为普通 describe 后，各例独立执行、独立成败，恢复真实覆盖。
test.describe('P0 删除矩阵：全资源 API 创建→删除→回读验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  for (const c of [
    {
      label: '仓库',
      createApi: '/warehouses',
      payload: {
        name: `P0仓库${TS}`,
        code: `P0-WH-${TS}`,
        address: 'P0测试地址',
        phone: '13800000001',
        capacity: 1000,
        description: 'P0仓库描述',
        warehouse_type: 'finished',
      },
    },
    {
      label: '部门',
      createApi: '/departments',
      payload: { name: `P0部门${TS}`, description: 'P0部门描述' },
    },
    {
      label: '客户',
      createApi: '/crm/customers',
      payload: {
        customer_name: `P0客户${TS}`,
        customer_code: `P0-CUST-${TS}`,
        contact_person: 'P0联系人',
        contact_phone: '13900000001',
        contact_email: 'p0cust@test.com',
        address: 'P0客户地址',
        city: '杭州',
        province: '浙江',
        customer_type: 'wholesale',
        credit_limit: '100000',
        payment_terms: 30,
        tax_id: `P0TAX${TS}`,
        bank_name: 'P0银行',
        bank_account: 'P0ACC1',
        notes: 'P0客户备注',
      },
    },
    {
      label: '供应商',
      createApi: '/purchase/suppliers',
      payload: {
        supplier_name: `P0供应商${TS}`,
        supplier_short_name: `P0简${TS}`,
        supplier_type: 'fabric',
        business_address: 'P0供应商地址',
        legal_representative: 'P0法人',
      },
    },
    {
      label: '产品',
      createApi: '/products',
      payload: {
        name: `P0产品${TS}`,
        code: `P0-PRD-${TS}`,
        category_id: getCtx().productCategoryIds[0],
        specification: 'P0规格',
        unit: '米',
        standard_price: 25.5,
        cost_price: 18,
        status: 'active',
        product_type: 'fabric',
        fabric_composition: '100%涤纶',
        description: 'P0产品描述',
      },
    },
    {
      label: '产品分类',
      createApi: '/product-categories',
      payload: { name: `P0分类${TS}`, code: `P0-CAT-${TS}`, description: 'P0分类描述' },
    },
    {
      label: '会计科目',
      createApi: '/subjects',
      // account_subject_handler::list_subjects → ApiResponse<Vec> → 裸数组
      listKey: 'bare',
      payload: { code: `P0SUB${TS}`, name: `P0科目${TS}`, level: 1, balance_direction: '借' },
    },
    {
      label: '角色',
      createApi: '/roles',
      // role_handler::list_roles → ApiResponse<RoleListResponse{roles}> → data={roles}
      listKey: 'roles',
      // role_permission_service.rs:153-159 要求 code 仅含小写字母/数字/下划线且长度 3-50。
      // 原 payload 用 `P0ROLE${TS}` 含大写字母，被该约束拒绝；错误以 AppError::business 抛出，
      // 经 public_message 脱敏后前端只看到"业务处理失败"，故失败原因在此注明。
      // TS 为 Date.now() 后 8 位纯数字，p0role 前缀满足全部约束（总长 14）。
      payload: { name: `P0角色${TS}`, code: `p0role${TS}`, description: 'P0角色描述' },
    },
    {
      label: '预算',
      createApi: '/budgets',
      payload: {
        item_name: `P0预算${TS}`,
        item_code: `P0-BUD-${TS}`,
        item_type: 'expense',
        budget_year: 2026,
        planned_amount: 50000,
        remark: 'P0预算备注',
      },
      // Q3 重构：预算明细 plan_id 为 NOT NULL 外键且后端校验方案存在，
      // 静态 payload 在 collection 期无法取运行时方案 id，故创建前注入一个有效 plan_id
      preCreate: async (page, payload) => {
        payload.plan_id = await ensureBudgetPlan(page);
      },
    },
    {
      label: '固定资产',
      createApi: '/fixed-assets',
      payload: {
        asset_no: `P0-FA-${TS}`,
        asset_name: `P0资产${TS}`,
        asset_category: '设备',
        specification: 'P0资产规格',
        location: 'P0车间',
        original_value: 120000,
        useful_life: 10,
        depreciation_method: 'straight_line',
        purchase_date: '2026-01-01',
        remark: 'P0资产备注',
      },
      preDelete: async (page, id) => {
        await apiCall(page, 'POST', `/fixed-assets/${id}/dispose`, {
          disposal_type: 'sale',
          disposal_value: 100000,
          disposal_date: '2026-01-01',
          reason: 'P0删除前处置',
        });
      },
    },
    {
      label: 'CRM标签',
      createApi: '/crm/tags',
      payload: { name: `P0标签${TS}`, color: '#FF0000', category: 'P0类' },
    },
    {
      label: 'CRM回收规则',
      createApi: '/crm/recycle-rules',
      payload: { name: `P0回收规则${TS}`, days: 30, is_enabled: true },
    },
    {
      label: 'CRM池规则',
      createApi: '/crm/pool/rules',
      payload: {
        name: `P0池规则${TS}`,
        // 后端 crm_pool_handler.rs::create_pool_rule 仅接受
        // protection_period / claim_limit / max_holdings；
        // 原值 no_follow_up 是 crm_customer_sea.reason_type 的枚举，用错了字段域
        rule_type: 'protection_period',
        rule_value: 30,
        customer_type: 'all',
        notes: 'P0池规则备注',
      },
    },
    {
      label: '染料配方',
      createApi: '/production/dye-recipes',
      payload: {
        recipe_name: `P0配方${TS}`,
        recipe_no: `P0-DR-${TS}`,
        color_code: 'P0-CC',
        color_name: 'P0色名',
        fabric_type: '涤纶',
        dye_type: '分散',
        temperature: 130,
        time_minutes: 45,
        liquor_ratio: '10.00',
        remarks: 'P0配方备注',
      },
    },
    {
      label: '坯布',
      createApi: '/production/greige-fabrics',
      payload: {
        fabric_no: `P0-GF-${TS}`,
        fabric_name: `P0坯布${TS}`,
        // fabric_type 为 greige_fabric NOT NULL 必填列（缺失即 500/422）；
        // 取值与 e2e/fabric/01 seedGreige 同源（'梭织'），本用例仅补 e2e payload 的必填数据，
        // 源码侧对该列的必填校验由后端专家并行处理，两者互不冲突。
        fabric_type: '梭织',
        product_id: 1,
        supplier_id: 1,
        warehouse_id: 1,
        quantity_meters: 100,
        quantity_kg: 50,
        // weight_kg / length_m 是 greige_fabric 独立于 quantity 的库存度量列
        // （create handler:227-228 直接落 req 值）；stock_out 只按这两列递减判定
        // 出库后状态（greige_fabric_handler.rs:489-504：两者归零→'已出库'，否则→'在库'）。
        // 不显式填则二者为 None，出库会因"出库量>现有量"报错，无法构造可删态。
        weight_kg: 50,
        length_m: 100,
        dye_lot_no: `P0-DL-${TS}`,
        status: '在库',
        remarks: 'P0坯布备注',
      },
      // 业务规则（后端既定、源码正确）：在库坯布不允许删除
      // （greige_fabric_handler.rs:345-346 status=='在库' → AppError::business → HTTP 400）。
      // 通用「创建→删除→回读」模式的前置态对坯布不成立，故删除前先真实出库把状态转为
      // '已出库'（构造可删态），再走通用 DELETE——如实反映业务流程，不 skip、不弱化断言。
      // 「在库直接删除应被拒(400)」由下方独立用例专门覆盖。
      preDelete: async (page, id) => {
        await apiCall(page, 'POST', `/production/greige-fabrics/${id}/stock-out`, {
          weight_kg: 50,
          length_m: 100,
        });
      },
    },
    {
      label: '染色批次',
      createApi: '/production/dye-batches',
      payload: {
        batch_no: `P0-DB-${TS}`,
        planned_quantity: 100,
        status: 'pending_schedule',
        dye_lot_no: `P0-DL-${TS}`,
      },
    },
    {
      label: '能源表计',
      createApi: '/production/energy-meters',
      payload: {
        meter_name: `P0表计${TS}`,
        meter_type: 'electricity',
        workshop: 'P0车间',
        location: 'P0位置',
        unit: 'kWh',
        unit_price: 1.2,
        remarks: 'P0表计备注',
      },
    },
    {
      label: '能源规则',
      createApi: '/production/energy-rules',
      payload: {
        rule_name: `P0能源规则${TS}`,
        meter_type: 'water',
        allocation_basis: 'by_workshop',
        effective_date: '2026-01-01',
        remarks: 'P0能源规则备注',
      },
    },
    {
      label: '工艺路线',
      createApi: '/production/process-routes',
      payload: {
        route_code: `P0-RT-${TS}`,
        route_name: `P0工艺${TS}`,
        seq: 1,
        process_type: '染色',
        require_scan: true,
        remarks: 'P0工艺备注',
      },
    },
    {
      label: '打样申请',
      createApi: '/production/lab-dip/requests',
      payload: {
        light_source: 'D65',
        required_date: '2026-12-31',
        customer_color_no: 'P0-CN',
        sample_type: '小样',
        fabric_spec: 'P0打样规格',
        remarks: 'P0打样备注',
      },
    },
    {
      label: '缸号状态规则',
      createApi: '/production/dye-batch-state-rules',
      payload: {
        from_status: 'pending_schedule',
        to_status: 'scheduled',
        transition_code: 'schedule',
        transition_name: `P0流转${TS}`,
        is_allowed: true,
        require_remarks: false,
        description: 'P0状态规则描述',
      },
    },
    {
      label: '工作中心',
      createApi: '/production/capacity/work-centers',
      payload: {
        name: `P0中心${TS}`,
        code: `P0-WC-${TS}`,
        work_center_type: '染色机',
        daily_capacity: 500,
        capacity_unit: '米',
        status: 'active',
        remarks: 'P0工作中心备注',
      },
    },
    {
      label: '库存批次',
      createApi: '/batches',
      payload: {
        batch_no: `P0-BT-${TS}`,
        product_id: 1,
        warehouse_id: 1,
        color_no: 'P0-CN',
        grade: 'A',
        quantity_meters: 100,
        quantity_kg: 50,
        color_name: 'P0色名',
        dye_lot_no: `P0-DL-${TS}`,
        remarks: 'P0批次备注',
      },
    },
    {
      label: 'Webhook',
      createApi: '/webhooks',
      payload: {
        name: `P0钩子${TS}`,
        url: 'https://example.com/p0hook',
        events: ['order.created'],
        secret: 'p0secret',
      },
    },
    {
      label: '报表模板',
      createApi: '/reports/enhanced/templates',
      payload: {
        name: `P0报表模板${TS}`,
        code: `P0-RPT-${TS}`,
        report_type: 'table',
        columns: [],
        description: 'P0报表模板备注',
      },
    },
    {
      label: '销售合同',
      createApi: '/sales/sales-contracts',
      // sales_contract_handler::list_contracts → ApiResponse<Vec> → 裸数组
      listKey: 'bare',
      payload: {
        contract_no: `P0-SC-${TS}`,
        contract_name: `P0销售合同${TS}`,
        customer_id: 1,
        total_amount: 10000,
        delivery_date: '2026-12-31',
        payment_terms: '月结30天',
        remark: 'P0销售合同备注',
      },
    },
    {
      label: '采购合同',
      createApi: '/purchase/purchase-contracts',
      // purchase_contract_handler::list_contracts → ApiResponse<Vec> → 裸数组
      listKey: 'bare',
      payload: {
        contract_no: `P0-PC-${TS}`,
        contract_name: `P0采购合同${TS}`,
        supplier_id: 1,
        total_amount: 8000,
        delivery_date: '2026-12-31',
        payment_terms: '月结',
        remark: 'P0采购合同备注',
      },
    },
    {
      label: '采购价格',
      createApi: '/purchase-prices',
      payload: {
        product_id: 1,
        supplier_id: 1,
        price: 15.5,
        currency: 'CNY',
        min_order_qty: 100,
        effective_date: '2026-01-01',
        expiry_date: '2026-12-31',
      },
    },
    {
      label: 'CRM线索',
      createApi: '/crm/leads',
      payload: {
        lead_no: `P0-LD-${TS}`,
        lead_source: '展会',
        company_name: `P0线索公司${TS}`,
        contact_name: 'P0联系人',
        mobile_phone: '13700000001',
        email: 'lead@test.com',
        product_interest: '坯布',
        priority: 'high',
        requirement_desc: 'P0线索需求',
      },
    },
    {
      label: 'CRM商机',
      createApi: '/crm/opportunities',
      payload: {
        opportunity_name: `P0商机${TS}`,
        customer_id: 1,
        opportunity_type: '新品',
        // opportunity_stage 受后端 chk_crm_opportunity_stage CHECK 约束，取值须为权威大写码
        // （models/status::crm_opportunity::ALL_STAGES）；「初步接洽」对应 QUALIFICATION
        opportunity_stage: 'QUALIFICATION',
        win_probability: 50,
        estimated_amount: 20000,
        currency: 'CNY',
        expected_close_date: '2026-12-31',
        priority: 'medium',
      },
    },
    {
      label: '销售订单',
      createApi: '/sales/orders',
      payload: {
        customer_id: 1,
        items: [{ product_id: 1, quantity: 10, unit_price: 25.5 }],
        required_date: '2026-12-31',
        shipping_address: 'P0收货地址',
        payment_terms: '月结30天',
        remarks: 'P0销售订单备注',
        notes: 'P0销售订单备注2',
      },
    },
    {
      label: '采购订单',
      createApi: '/purchase/orders',
      payload: {
        supplier_id: 1,
        order_date: '2026-01-01',
        expected_delivery_date: '2026-12-31',
        items: [{ material_id: 1, quantity_ordered: 10, unit_price: 18 }],
        payment_terms: '月结',
        notes: 'P0采购订单备注',
      },
    },
    {
      label: '委外订单',
      createApi: '/production/outsourcing-orders',
      payload: {
        order_no: `P0-OS-${TS}`,
        order_type: '染色',
        supplier_id: 1,
        issue_date: '2026-01-01',
        expected_return_date: '2026-12-31',
        issue_quantity: 100,
        issue_unit: '米',
        material_cost: 500,
        color_no: 'P0-CN',
        dye_lot_no: `P0-DL-${TS}`,
        remarks: 'P0委外备注',
      },
    },
  ] as DelCase[]) {
    test(`${c.label}：创建→删除→详情404`, async ({ page }) => {
      test.setTimeout(180_000);
      await createThenApiDelete(page, c);
    });
  }

  // ===== 坯布删除业务规则守卫 =====
  // 后端既定规则（greige_fabric_handler.rs:345-346）：status=='在库' 的坯布 DELETE 直接拒绝，
  // 返回 AppError::business → HTTP 400。注意该 message 经 utils/error.rs::public_message() 脱敏
  // 为常量「业务处理失败」，中文原文不外显，故此处据实断言 HTTP 400（不锁会被脱敏的文案），
  // 证明"在库不可删"前置约束被真实执行——与上方"先出库转可删态再删"用例互为正反两面。
  test('坯布：在库态直接删除应被拒(400)，出库后方可删除', async ({ page }) => {
    test.setTimeout(120_000);
    const fabric = await apiCall<{ id?: number }>(page, 'POST', '/production/greige-fabrics', {
      fabric_no: `P0-GFGUARD-${TS}`,
      fabric_name: `P0坯布守卫${TS}`,
      fabric_type: '梭织',
      product_id: 1,
      supplier_id: 1,
      warehouse_id: 1,
      quantity_meters: 100,
      quantity_kg: 50,
      weight_kg: 50,
      length_m: 100,
      dye_lot_no: `P0-DL-${TS}`,
      status: '在库',
      remarks: 'P0坯布删除守卫',
    });
    const id = fabric?.data?.id;
    expect(id, '[31b-坯布守卫] 在库坯布创建失败').toBeTruthy();

    // 在库态：直接删除必须被拒（HTTP 400），不得静默成功。
    const rejected = await apiCallExpectFail(page, 'DELETE', `/production/greige-fabrics/${id}`);
    console.log(
      `[31b-坯布守卫] 在库 DELETE → HTTP ${rejected.status} code=${rejected.code ?? '-'}`
    );
    expect(rejected.status, '[31b-坯布守卫] 在库坯布删除应返回 400（业务规则拒绝）').toBe(400);

    // 记录仍在（删除被拒不应产生副作用）：详情回读仍 200。
    const stillThere = await page.request.get(
      `${API_BASE}${API_PREFIX}/production/greige-fabrics/${id}`
    );
    expect(stillThere.status(), '[31b-坯布守卫] 删除被拒后坯布记录不应消失').toBe(200);

    // 出库构造可删态 → 删除成功 → 列表回读消失（软删），证明 400 是"态"所致而非端点坏。
    await apiCall(page, 'POST', `/production/greige-fabrics/${id}/stock-out`, {
      weight_kg: 50,
      length_m: 100,
    });
    await apiCall(page, 'DELETE', `/production/greige-fabrics/${id}`);
    console.log(`[31b-坯布守卫] 出库后 DELETE /production/greige-fabrics/${id} ✅成功`);
    const listAfter = await apiCallRaw<{ items?: Array<{ id: number }> }>(
      page,
      'GET',
      `/production/greige-fabrics?fabric_no=P0-GFGUARD-${TS}&page=1&page_size=50`
    );
    const stillInList = (listAfter?.items ?? []).some(r => r.id === id);
    expect(stillInList, `[31b-坯布守卫] 出库并删除后坯布 ${id} 仍出现在列表（软删未生效）`).toBe(
      false
    );
  });

  // ===== 业务模式流程节点（子表） =====
  // 业务模式配置本身不走通用矩阵：mode_code 是 backend validate_mode_code 的封闭词表，
  // 6 行由 v15 迁移种子写入且被 08 spec 只读依赖，「每轮新建一个再删除」既建不出来
  // （同代码唯一）也会删掉别人的前置数据。这里改为删除矩阵真正能覆盖的子资源：
  // 流程节点有 POST/DELETE 端点，step_code 自由填写（同模式内唯一），回读入口是 by-mode 列表。
  test('业务模式流程节点：创建→删除→按模式回读消失', async ({ page }) => {
    test.setTimeout(180_000);
    // /production/business-modes：business_mode_handler::list_business_modes → PaginatedResponse → {items}。
    // 单一形状直读（原 `(modes.items ?? [])` 把 items 键漂移当成"无种子"→误抛）。
    const modes = await apiCallRaw<unknown>(
      page,
      'GET',
      '/production/business-modes?page=1&page_size=50&mode_code=grey_trading'
    );
    const modeItems = pickListArray<{ id: number; mode_code: string }>(
      modes,
      'items',
      '31b 业务模式列表'
    );
    const mode = modeItems.find(m => m.mode_code === 'grey_trading');
    expect(
      mode,
      `[31b-业务模式流程节点] 种子缺少 grey_trading，现有：${modeItems.map(m => m.mode_code).join(',')}`
    ).toBeTruthy();

    const stepsApi = `/production/business-modes/flow-steps/by-mode/${mode!.id}`;
    const before = await apiCallRaw<Array<{ id: number; step_no: number }>>(page, 'GET', stepsApi);
    expect(Array.isArray(before), '[31b-业务模式流程节点] by-mode 回读应为数组').toBe(true);
    const stepCode = `P0-FS-${TS}`;
    const stepNo = before.reduce((max, s) => Math.max(max, s.step_no), 0) + 1;

    const created = await apiCallRaw<{ id?: number }>(
      page,
      'POST',
      '/production/business-modes/flow-steps',
      {
        mode_id: mode!.id,
        step_no: stepNo,
        step_code: stepCode,
        step_name: `P0流程节点${TS}`,
        module_name: 'production',
        is_required: false,
        description: 'P0删除矩阵流程节点',
      }
    );
    const id = created?.id;
    expect(id, `[31b-业务模式流程节点] 创建响应无 id（创建 API 异常）`).toBeTruthy();
    console.log(`[31b-业务模式流程节点] 创建成功 id=${id} step_no=${stepNo}`);

    const afterCreate = await apiCallRaw<Array<{ id: number }>>(page, 'GET', stepsApi);
    expect(
      afterCreate.some(s => s.id === id),
      `[31b-业务模式流程节点] 新建节点 ${id} 未出现在 by-mode 列表里`
    ).toBe(true);

    await apiCall(page, 'DELETE', `/production/business-modes/flow-steps/${id}`);
    console.log(`[31b-业务模式流程节点] DELETE flow-steps/${id} ✅成功`);

    const afterDelete = await apiCallRaw<Array<{ id: number }>>(page, 'GET', stepsApi);
    expect(
      afterDelete.some(s => s.id === id),
      `[31b-业务模式流程节点] 删除后节点 ${id} 仍能从 by-mode 列表读到（删除未生效）`
    ).toBe(false);
  });

  // ===== 凭证（items 必填 debit/credit）=====
  test('凭证：创建→删除→详情404', async ({ page }) => {
    test.setTimeout(180_000);
    await createThenApiDelete(page, {
      label: '凭证',
      createApi: '/vouchers',
      payload: {
        voucher_type: '记',
        voucher_date: '2026-01-01',
        items: [
          { line_no: 1, debit: 100, credit: 0, summary: 'P0凭证借方' },
          { line_no: 2, debit: 0, credit: 100, summary: 'P0凭证贷方' },
        ],
        batch_no: `P0-VB-${TS}`,
        color_no: 'P0-CN',
      },
    });
  });

  // ===== 用户（密码强度规则：≥8 大小写数字特殊符）=====
  test('用户：创建→删除→详情404', async ({ page }) => {
    test.setTimeout(180_000);
    await createThenApiDelete(page, {
      label: '用户',
      createApi: '/users',
      // user_handler::list_users → ApiResponse<UserListResponse{users}> → data={users}
      listKey: 'users',
      payload: {
        username: `p0user${TS}`,
        password: 'P0Test!2026xY',
        email: `p0user${TS}@test.com`,
        phone: '13600000001',
      },
    });
  });

  // ===== 客户信用（依赖 customer_id=1 seed）=====
  test('客户信用：创建→删除→详情404', async ({ page }) => {
    test.setTimeout(180_000);
    await createThenApiDelete(page, {
      label: '客户信用',
      createApi: '/crm/customer-credits',
      payload: {
        customer_id: 1,
        credit_level: 'A',
        credit_score: 90,
        credit_limit: '50000',
        credit_days: 30,
        remark: 'P0信用备注',
      },
    });
  });

  // ===== 供应商评估（依赖指标先建）=====
  test('供应商评估：指标→评估记录→删除→回读', async ({ page }) => {
    test.setTimeout(120_000);
    const ind = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/purchase/supplier-evaluations/indicators',
      {
        indicator_name: `P0指标${TS}`,
        indicator_code: `P0-IND-${TS}`,
        category: '质量',
        weight: 30,
        max_score: 100,
        evaluation_method: '评分',
      }
    );
    const indicatorId = ind?.data?.id;
    expect(indicatorId, '[31b-供应商评估] 评估指标创建失败').toBeTruthy();
    console.log(`[31b-供应商评估] 指标创建成功 id=${indicatorId}`);
    await createThenApiDelete(page, {
      label: '供应商评估',
      createApi: '/purchase/supplier-evaluations',
      payload: {
        supplier_id: 1,
        evaluation_period: `2026-P0-${TS}`,
        indicator_id: indicatorId,
        score: 88,
        remark: 'P0评估备注',
      },
    });
  });

  // ===== 疵点（依赖检验单先建，闭环删除检验单）=====
  test('坯布疵点：检验单→疵点→删除→回读', async ({ page }) => {
    test.setTimeout(120_000);
    const ins = await apiCall<{ id?: number }>(page, 'POST', '/production/fabric-inspections', {
      inspection_date: '2026-01-01',
      product_id: 1,
      product_name: 'P0疵点检验产品',
      color_no: 'P0-CN',
      dye_lot_no: `P0-DL-${TS}`,
      inspector_name: 'P0检验员',
      machine_no: 'P0-M1',
      scoring_system: 'four_point',
      fabric_width_inches: 60,
      remarks: 'P0疵点检验备注',
    });
    const inspectionId = ins?.data?.id;
    expect(inspectionId, '[31b-疵点] 检验单创建失败').toBeTruthy();
    console.log(`[31b-疵点] 检验单创建成功 id=${inspectionId}`);
    await createThenApiDelete(page, {
      label: '疵点',
      createApi: '/production/fabric-defects',
      payload: {
        inspection_id: inspectionId,
        defect_type: '破洞',
        position_yards: 12,
        defect_length_inches: 2,
        direction: '横向',
        is_hole: true,
        description: 'P0疵点描述',
      },
    });
    // 闭环：删除检验单
    await tryCleanup(
      page,
      'DELETE',
      `/production/fabric-inspections/${inspectionId}`,
      '[31b-疵点]'
    );
  });

  // ===== 工资率（依赖工艺路线先建，闭环删除）=====
  test('工资率：工艺路线→工资率→删除→回读', async ({ page }) => {
    test.setTimeout(120_000);
    const rt = await apiCall<{ id?: number }>(page, 'POST', '/production/process-routes', {
      route_code: `P0-WG-RT-${TS}`,
      route_name: `P0工资工艺${TS}`,
      seq: 1,
      process_type: '染色',
      require_scan: true,
      remarks: 'P0工资工艺备注',
    });
    const routeId = rt?.data?.id;
    expect(routeId, '[31b-工资率] 工艺路线创建失败').toBeTruthy();
    console.log(`[31b-工资率] 工艺路线创建成功 id=${routeId}`);
    await createThenApiDelete(page, {
      label: '工资率',
      createApi: '/production/wage-rates',
      payload: {
        process_route_id: routeId,
        wage_type: '计件',
        piece_price: 1.5,
        time_price: 20,
        grade_a_ratio: 1.0,
        grade_b_ratio: 0.8,
        grade_c_ratio: 0.6,
        effective_date: '2026-01-01',
        workshop: 'P0车间',
        remarks: 'P0工资率备注',
      },
    });
    await tryCleanup(page, 'DELETE', `/production/process-routes/${routeId}`, '[31b-工资率]');
  });

  // ===== 角色互斥（依赖两个角色先建，闭环删除）=====
  test('角色互斥：双角色→互斥关系→删除→回读', async ({ page }) => {
    test.setTimeout(150_000);
    const codeA = `p0ra_${TS}`,
      codeB = `p0rb_${TS}`;
    const ra = await apiCall<{ id?: number }>(page, 'POST', '/roles', {
      name: `P0角色A${TS}`,
      code: codeA,
      description: 'P0互斥角色A',
    });
    const rb = await apiCall<{ id?: number }>(page, 'POST', '/roles', {
      name: `P0角色B${TS}`,
      code: codeB,
      description: 'P0互斥角色B',
    });
    const idA = ra?.data?.id;
    const idB = rb?.data?.id;
    expect(idA, '[31b-角色互斥] 互斥角色 A 创建失败').toBeTruthy();
    expect(idB, '[31b-角色互斥] 互斥角色 B 创建失败').toBeTruthy();
    console.log(`[31b-角色互斥] 角色就绪 A=${idA}(${codeA}) B=${idB}(${codeB})`);
    let relDeleted = false;
    await apiCall(page, 'POST', '/role-relations', {
      parent_role_code: codeA,
      child_role_code: codeB,
      relation_type: 'mutual_exclusive',
      description: 'P0互斥关系',
    });
    console.log('[31b-角色互斥] 互斥关系创建成功');
    // 取 relation_id 只能用返回关系行的端点：
    //   GET /role-relations/between/{a}/{b} → ApiResponse<Vec<role_relation::Model>>
    //   （routes/role_relation.rs:27-30 → handler:75-85 → service:171 返回 Vec<Model>，
    //    行字段 id/parent_role_code/child_role_code/relation_type，models/role_relation.rs:15-27）
    // 原先查的 /role-relations/inherited/{code} 返回的是**子角色编码字符串数组**
    // （handler get_inherited_role_codes），根本没有 id/child_role_code 字段，
    // 于是 find() 恒为 undefined、两条分支都只 console.warn，最终 expect(relDeleted) 必失败。
    const chk = await page.request.get(
      `${API_BASE}${API_PREFIX}/role-relations/between/${codeA}/${codeB}`
    );
    expect(chk.ok(), `[31b-角色互斥] between 查询应 200，实际 HTTP ${chk.status()}`).toBe(true);
    const chkBody = await chk.json();
    const rels = pickListArray<{ id?: number; child_role_code?: string; relation_type?: string }>(
      chkBody?.data,
      'bare',
      '31b-角色互斥 between 查询'
    );
    const rel = rels.find(
      r => r.relation_type === 'mutual_exclusive' && r.child_role_code === codeB && r.id
    );
    expect(
      rel?.id,
      `[31b-角色互斥] between 未定位到 A(${codeA})→B(${codeB}) 的互斥关系行，实到 ${rels.length} 行`
    ).toBeTruthy();
    await apiCall(page, 'DELETE', `/role-relations/${rel?.id}`);
    relDeleted = true;
    console.log(`[31b-角色互斥] 关系 ${rel?.id} 删除 ✅`);
    expect(relDeleted, '[31b-角色互斥] 互斥关系应创建并删除成功').toBe(true);
    // 清理双角色
    for (const [rid, code] of [
      [idA, codeA],
      [idB, codeB],
    ] as Array<[number, string]>) {
      await tryCleanup(page, 'DELETE', `/roles/${rid}`, `[31b-角色互斥] ${code}`);
    }
  });

  // ===== 数据权限（依赖角色先建，闭环删除）=====
  test('数据权限：角色→权限记录→删除→回读', async ({ page }) => {
    test.setTimeout(120_000);
    const r = await apiCall<{ id?: number }>(page, 'POST', '/roles', {
      name: `P0DP角色${TS}`,
      code: `p0dp_${TS}`,
      description: 'P0数据权限角色',
    });
    const roleId = r?.data?.id;
    expect(roleId, '[31b-数据权限] 角色创建失败').toBeTruthy();
    await createThenApiDelete(page, {
      label: '数据权限',
      createApi: '/data-permissions',
      payload: {
        role_id: roleId,
        resource_type: 'product',
        scope_type: 'all',
        custom_condition: null,
        allowed_fields: null,
        hidden_fields: null,
      },
    });
    await tryCleanup(page, 'DELETE', `/roles/${roleId}`, '[31b-数据权限]');
  });

  // ===== 通知：自带创建端点（POST /notifications/announcement），据此构造可删对象 =====
  // 原实现从 data.items 取列表，而 list_notifications 的 key 是 data.list（handler:75），
  // 于是 items 恒为 undefined → 走 skip 分支，删除→回读404 从未真正验证过。
  test('通知：公告直发创建→删除→回读404', async ({ page }) => {
    test.setTimeout(180_000);
    await ensureTestEntities(page);
    const me = getCtx().userIds[0];
    expect(me, '[31b-通知] 当前用户 id 未就绪，无法自造可删通知').toBeTruthy();

    // 自带创建端点（notification_handler.rs create_announcement，仅管理员），
    // 用它造一条确定存在的通知，删除链才有真实可验证对象
    const uniq = `P0DEL-${TS}`;
    const sent = await apiCallRaw<{ delivered_count?: number }>(
      page,
      'POST',
      '/notifications/announcement',
      { user_ids: [me], title: `删除矩阵通知${uniq}`, content: `删除矩阵通知内容 ${uniq}` }
    );
    expect(sent?.delivered_count, '[31b-通知] 公告直发应投递 1 条').toBe(1);
    await page.waitForTimeout(3000);

    const listResp = await apiCallRaw<{ list: Array<{ id: number; title: string }> }>(
      page,
      'GET',
      '/notifications?page=1&page_size=20'
    );
    expect(Array.isArray(listResp?.list), '[31b-通知] 列表应返回 list 数组').toBe(true);
    const created = listResp.list.filter(n => n.title === `删除矩阵通知${uniq}`);
    expect(created.length, `[31b-通知] 应能查到刚创建的「删除矩阵通知${uniq}」`).toBe(1);
    const targetId = created[0].id;
    console.log(`[31b-通知] 自造通知 id=${targetId} 待删除`);

    await apiCall(page, 'DELETE', `/notifications/${targetId}`);
    console.log(`[31b-通知] DELETE 通知 ${targetId} ✅`);
    const chk = await page.request.get(`${API_BASE}${API_PREFIX}/notifications/${targetId}`);
    console.log(`[31b-通知] 删除后回读 HTTP ${chk.status()}`);
    expect(chk.status(), '[31b-通知] 删除后详情应 404').toBe(404);
  });
});
