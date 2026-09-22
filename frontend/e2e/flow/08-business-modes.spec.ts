import type { Page } from '@playwright/test';
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  getCtx,
  genCode,
  ensureTestEntities,
  getBusinessModeByCode,
  getProcessSteps,
  tryCleanup,
} from './helpers';
import type { BusinessModeFlowStepRow } from './helpers';

/**
 * 多业务模式（v14 批次 431，依据 §6 业务模式）。
 *
 * 数据前提：6 种模式是 backend `validate_mode_code` 的封闭词表，由 v15 迁移种子写入
 * business_mode_config（模式代码不允许自定义，任何 spec 都不该再去建/删模式本身）。
 * 流程节点、业务规则、单据关联是模式的子表，backend 不预置，由本 spec 通过 API 真实写入再回读。
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 与 backend business_mode_service.rs check_module_consistency 的 6 个分支一一对应 */
const CANONICAL_MODES: Array<{
  mode_code: string;
  material_source: string;
  settlement_method: string;
  require_purchase: boolean;
  require_production: boolean;
  require_outsourcing: boolean;
  require_sales: boolean;
}> = [
  {
    mode_code: 'grey_trading',
    material_source: 'purchase',
    settlement_method: 'sale_settlement',
    require_purchase: true,
    require_production: false,
    require_outsourcing: false,
    require_sales: true,
  },
  {
    mode_code: 'finished_trading',
    material_source: 'purchase',
    settlement_method: 'sale_settlement',
    require_purchase: true,
    require_production: true,
    require_outsourcing: false,
    require_sales: true,
  },
  {
    mode_code: 'dyeing_processing',
    material_source: 'customer_provided',
    settlement_method: 'processing_fee_settlement',
    require_purchase: false,
    require_production: true,
    require_outsourcing: false,
    require_sales: false,
  },
  {
    mode_code: 'self_weave_dye',
    material_source: 'purchase',
    settlement_method: 'sale_settlement',
    require_purchase: true,
    require_production: true,
    require_outsourcing: false,
    require_sales: true,
  },
  {
    mode_code: 'outsourcing',
    material_source: 'self_made',
    settlement_method: 'sale_settlement',
    require_purchase: false,
    require_production: true,
    require_outsourcing: true,
    require_sales: true,
  },
  {
    mode_code: 'toll_processing',
    material_source: 'toll',
    settlement_method: 'processing_fee_settlement',
    require_purchase: false,
    require_production: true,
    require_outsourcing: false,
    require_sales: false,
  },
];

interface ChainStep {
  step_code: string;
  step_name: string;
  module_name: string;
}

/** 染整加工：客供坯布进厂 → 车间加工 → 成品交付 → 加工费结算（无采购、无销售出库单） */
const DYEING_CHAIN: ChainStep[] = [
  { step_code: 'inventory_in', step_name: '客供坯布入库', module_name: 'inventory' },
  { step_code: 'production', step_name: '染整加工', module_name: 'production' },
  { step_code: 'inventory_out', step_name: '成品出库', module_name: 'inventory' },
  { step_code: 'settlement', step_name: '加工费结算', module_name: 'finance' },
];

/** 来料加工：来料入库 → 代工 → 成品出库 → 加工费结算（同样不含采购节点） */
const TOLL_CHAIN: ChainStep[] = [
  { step_code: 'inventory_in', step_name: '来料入库', module_name: 'inventory' },
  { step_code: 'production', step_name: '代工生产', module_name: 'production' },
  { step_code: 'inventory_out', step_name: '成品出库', module_name: 'inventory' },
  { step_code: 'settlement', step_name: '加工费结算', module_name: 'finance' },
];

/**
 * 写入流程链并回读：已存在的 step_code 跳过（同一 CI 库重跑不产生脏数据），
 * step_no 从库里已有的最大值往后排，避开「同模式内步骤序号唯一」约束。
 */
async function seedFlowChain(
  page: Page,
  modeCode: string,
  chain: ChainStep[]
): Promise<BusinessModeFlowStepRow[]> {
  const mode = await getBusinessModeByCode(page, modeCode);
  const existing = await getProcessSteps(page, modeCode);
  const knownCodes = new Set(existing.map(s => s.step_code));
  let nextNo = existing.reduce((max, s) => Math.max(max, s.step_no), 0);
  for (const step of chain) {
    if (knownCodes.has(step.step_code)) continue;
    nextNo += 1;
    const created = await apiCallRaw<{ id: number }>(
      page,
      'POST',
      '/production/business-modes/flow-steps',
      {
        mode_id: mode.id,
        step_no: nextNo,
        step_code: step.step_code,
        step_name: step.step_name,
        module_name: step.module_name,
        is_required: true,
        description: `E2E ${modeCode} 流程链`,
      }
    );
    expect(
      created?.id,
      `[seedFlowChain] ${modeCode}/${step.step_code} 创建后应返回 id`
    ).toBeTruthy();
    CLEANUP.push({
      path: `/production/business-modes/flow-steps/${created.id}`,
      label: `[${modeCode}] 流程节点 ${step.step_code}`,
    });
  }
  return getProcessSteps(page, modeCode);
}

test.describe.serial('扩展: 业务模式测试（染整加工/来料加工/委外加工）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.beforeEach(async ({ page }) => {
    await ensureTestEntities(page);
  });

  let outsourcingOrderId = 0;
  let outsourcingOrderNo = '';

  test('M1-1 验证业务模式种子（6 种模式齐全且模块开关与后端一致性规则相符）', async ({ page }) => {
    const modes = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      '/production/business-modes?page=1&page_size=50'
    );
    expect(
      Array.isArray(modes?.items),
      `业务模式列表应返回 items 数组，实际：${JSON.stringify(modes).slice(0, 200)}`
    ).toBe(true);
    const byCode = new Map(
      modes.items.map(m => [String(m.mode_code), m] as [string, Record<string, unknown>])
    );
    for (const want of CANONICAL_MODES) {
      const row = byCode.get(want.mode_code);
      expect(
        row,
        `模式种子缺少 ${want.mode_code}，库里现有：${[...byCode.keys()].join(',')}`
      ).toBeTruthy();
      for (const key of [
        'material_source',
        'settlement_method',
        'require_purchase',
        'require_production',
        'require_outsourcing',
        'require_sales',
      ] as const) {
        expect(row![key], `模式 ${want.mode_code} 的 ${key} 与后端一致性规则不符`).toBe(want[key]);
      }
      expect(String(row!.mode_name ?? ''), `模式 ${want.mode_code} 缺少名称`).not.toBe('');
    }
  });

  test('M1-2 染整加工模式流程链（写入 → 按模式回读）', async ({ page }) => {
    const steps = await seedFlowChain(page, 'dyeing_processing', DYEING_CHAIN);
    const chainCodes = DYEING_CHAIN.map(s => s.step_code);
    const own = steps
      .filter(s => chainCodes.includes(s.step_code))
      .sort((a, b) => a.step_no - b.step_no);
    expect(
      own.map(s => s.step_code),
      `染整加工流程链应按序落库，实际：${JSON.stringify(steps)}`
    ).toEqual(chainCodes);
    for (const step of own) {
      expect(step.is_required, `节点 ${step.step_code} 应标记为必需`).toBe(true);
      expect(
        ['inventory', 'production', 'finance'],
        `节点 ${step.step_code} 的 module_name 超出预期：${step.module_name}`
      ).toContain(step.module_name);
    }
    // 染整加工是客供模式，不应出现采购节点
    expect(
      own.map(s => s.step_code),
      '染整加工流程链不应包含采购节点'
    ).not.toContain('purchase');
  });

  test('M1-3 来料加工模式流程链（不含采购/委外节点）', async ({ page }) => {
    const steps = await seedFlowChain(page, 'toll_processing', TOLL_CHAIN);
    const chainCodes = TOLL_CHAIN.map(s => s.step_code);
    const own = steps
      .filter(s => chainCodes.includes(s.step_code))
      .sort((a, b) => a.step_no - b.step_no);
    expect(
      own.map(s => s.step_code),
      `来料加工流程链应按序落库，实际：${JSON.stringify(steps)}`
    ).toEqual(chainCodes);
    const allCodes = steps.map(s => s.step_code);
    expect(allCodes, '来料加工不应包含采购节点').not.toContain('purchase');
    expect(allCodes, '来料加工不应包含委外节点').not.toContain('outsourcing');
  });

  test('M1-4 创建委外加工订单', async ({ page }) => {
    const ctx = getCtx();
    outsourcingOrderNo = genCode('OUT');
    const result = await apiCall<{ id?: number }>(page, 'POST', '/production/outsourcing-orders', {
      order_no: outsourcingOrderNo,
      order_type: 'dyeing',
      supplier_id: ctx.supplierId || 1,
      // CreateOutsourcingOrderRequest 必填 issue_date/issue_quantity/material_cost；
      // 交期字段名是 expected_return_date（原发的 expected_delivery_date 不在契约内，
      // 会被 serde 忽略，而缺失必填字段直接 422）
      issue_date: new Date().toISOString().split('T')[0],
      expected_return_date: new Date(Date.now() + 14 * 86400000).toISOString().split('T')[0],
      issue_quantity: '100',
      material_cost: '0',
      remarks: 'E2E 委外加工订单',
    });
    expect(result.data?.id, '委外订单创建应返回 id').toBeTruthy();
    outsourcingOrderId = result.data!.id!;
  });

  test('M1-5 委外加工订单状态流转', async ({ page }) => {
    // 必须在 M1-4（describe.serial）之后跑：按刚建单据号精确回读，避免「列表为空也通过」的空断言
    expect(outsourcingOrderId, 'M1-4 未创建委外订单').toBeGreaterThan(0);
    const detail = await apiCallRaw<{ id: number; status: string }>(
      page,
      'GET',
      `/production/outsourcing-orders/${outsourcingOrderId}`
    );
    expect(detail.id, '委外订单详情 id 应与创建返回一致').toBe(outsourcingOrderId);
    // 取值域见 backend models/status 委外订单状态机：draft → issued → processing → received → settled/closed
    expect(
      ['draft', 'issued', 'processing', 'received', 'settled', 'closed', 'cancelled'],
      `委外订单 ${outsourcingOrderId} 状态在取值域外：${detail.status}`
    ).toContain(String(detail.status ?? '').toLowerCase());
  });

  test('M1-6 验证委外加工模式规则', async ({ page }) => {
    // GET /production/business-modes/rules 只有 POST 路由（无全局列表端点），
    // 规则的真实查询入口是按模式取回：/business-modes/rules/by-mode/{mode_id}。
    const mode = await getBusinessModeByCode(page, 'outsourcing');
    const ruleCode = genCode('BM-RULE');
    const created = await apiCallRaw<{ id: number }>(
      page,
      'POST',
      '/production/business-modes/rules',
      {
        mode_id: mode.id,
        rule_code: ruleCode,
        rule_name: '委外订单必须指定加工厂',
        rule_type: 'required',
        module_name: 'outsourcing',
        validation_logic: { field: 'supplier_id', operator: 'required' },
        description: 'E2E 委托加工规则',
        is_active: true,
      }
    );
    expect(created?.id, '规则创建应返回 id').toBeTruthy();
    CLEANUP.push({
      path: `/production/business-modes/rules/${created.id}`,
      label: `规则 ${ruleCode}`,
    });

    const rules = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/production/business-modes/rules/by-mode/${mode.id}`
    );
    expect(
      Array.isArray(rules),
      `按模式查询规则应返回数组，实际：${JSON.stringify(rules).slice(0, 200)}`
    ).toBe(true);
    const mine = rules.find(r => r.id === created.id);
    expect(mine, `新建规则 ${ruleCode} 未出现在 by-mode 回读里`).toBeTruthy();
    for (const rule of rules) {
      expect(rule.mode_id, `规则混入了其他模式的行：${JSON.stringify(rule)}`).toBe(mode.id);
      expect(String(rule.rule_code ?? ''), `规则缺少 rule_code：${JSON.stringify(rule)}`).not.toBe(
        ''
      );
      // rule_type 取值域见 backend validate_rule_type：required / optional / forbidden
      expect(
        ['required', 'optional', 'forbidden'],
        `规则 ${rule.rule_code} 的类型在取值域外：${rule.rule_type}`
      ).toContain(String(rule.rule_type));
    }
  });

  test('M1-7 验证业务模式快照（mode_snapshot）', async ({ page }) => {
    const mode = await getBusinessModeByCode(page, 'outsourcing');
    expect(outsourcingOrderId, 'M1-4 未创建委外订单，无法验证单据关联').toBeGreaterThan(0);

    const link = await apiCallRaw<{ id: number; mode_id: number; document_no: string }>(
      page,
      'POST',
      '/production/business-mode-links',
      {
        mode_id: mode.id,
        document_type: 'outsourcing_order',
        document_id: outsourcingOrderId,
        document_no: outsourcingOrderNo,
        mode_snapshot: {
          mode_code: mode.mode_code,
          mode_name: mode.mode_name,
          settlement_method: mode.settlement_method,
        },
      }
    );
    expect(link?.id, '单据-模式关联创建应返回 id').toBeTruthy();
    CLEANUP.push({ path: `/production/business-mode-links/${link.id}`, label: '单据-模式关联' });

    // 按单据回读：关联是「一单据一模式」的强约束入口
    const byDoc = await apiCallRaw<{
      id: number;
      mode_id: number;
      document_type: string;
      mode_snapshot: Record<string, unknown>;
    }>(
      page,
      'GET',
      `/production/business-mode-links/by-document/outsourcing_order/${outsourcingOrderId}`
    );
    expect(byDoc?.id, `按单据回读应命中刚建的关联，实际：${JSON.stringify(byDoc)}`).toBe(link.id);
    expect(byDoc.mode_id, '关联应挂在 outsourcing 模式上').toBe(mode.id);
    expect(byDoc.document_type).toBe('outsourcing_order');
    expect(
      byDoc.mode_snapshot?.mode_code,
      `mode_snapshot 应固化关联时的模式代码，实际：${JSON.stringify(byDoc.mode_snapshot)}`
    ).toBe('outsourcing');

    // 列表入口（PaginatedResponse：items/total）也应能按模式过滤出这一条
    const links = await apiCallRaw<{
      items: Array<{ id: number; mode_id: number }>;
      total: number;
    }>(page, 'GET', `/production/business-mode-links?page=1&page_size=50&mode_id=${mode.id}`);
    expect(Array.isArray(links?.items), `关联列表应返回 items 数组`).toBe(true);
    expect(
      links.items.some(l => l.id === link.id),
      `按模式过滤的关联列表缺少 id=${link.id}`
    ).toBe(true);
  });
});
