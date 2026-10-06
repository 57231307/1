/**
 * 增强客户列表幽灵聚合键形状锁（后端出参载体 ↔ 前端声明 ↔ 视图绑定三向互证）。
 * 功能：
 * 1) GET /crm/customers/enhanced 的行载荷是后端 crm_lead 模型整行直序列化
 *    （services/crm/lead.rs::list_leads 组 json!{"data": items}，出口仅做掩码/删键
 *    不补键），出参键集恒 ⊆ backend/src/models/crm_lead.rs 的 Model 字段名集；
 * 2) 前端 CustomerWithTags 不得声明后端该端点从不输出的聚合键
 *    （total_amount/total_orders/last_follow_up/contacts），消费视图
 *    （CustomerListTab/RfmTab）不得再绑定它们，打印表头/行单元格列数必须对齐；
 * 3) 视图列仍在绑定的历史漂移键以钉死清单显式登记，差集只允许随视图重绑批
 *    同步收缩，不允许静默扩大。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内源码文本，不执行后端代码、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、CI 检出为 LF，
 * 本锁的字面量夹具 replace 与逐行正则对行尾形态敏感，不归一会让检测力自证被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const CRM_ENHANCED_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/crm-enhanced.ts'));
const CUSTOMER_LIST_TAB_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/crm/tabs/CustomerListTab.vue')
);
const RFM_TAB_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/views/crm/tabs/RfmTab.vue'));
const LEAD_MODEL_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/models/crm_lead.rs'));
const LEAD_SERVICE_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/services/crm/lead.rs'));
const ENHANCED_HANDLER_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/handlers/crm_customer_handler.rs')
);

/** 后端该端点从不输出的聚合键：已从声明与视图中删除，出现即判红 */
const GHOST_AGGREGATE_KEYS = [
  'total_amount',
  'total_orders',
  'last_follow_up',
  'contacts',
] as const;

/**
 * 钉死清单：视图列当前仍在绑定、但 crm_lead 出参不存在的历史漂移键。
 * 这些键属"视图重绑到真实 lead 字段"批的收口对象，本锁只冻结其不再扩大；
 * 任一键被收口删除后须同步修订本清单（差集精确相等，静默漂移即判红）。
 */
const FROZEN_VIEW_DRIFT_KEYS = [
  'customer_code',
  'customer_name',
  'contact_person',
  'phone',
  'customer_type',
  'status',
] as const;

/** 截取 TS interface 正文（含字段类型原文）；接口被删除或改名即抛错（防判据静默失去覆盖） */
function tsInterfaceBody(src: string, name: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`TS 源码中未找到 interface ${name}——结构已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 提取 TS 接口块内的声明键（跳过索引签名行） */
function tsInterfaceKeys(src: string, name: string): string[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}(?:readonly\s+)?([a-z][A-Za-z0-9_]*)\??\s*:/gm)].map(m => m[1]);
}

/** 截取 Rust struct 正文；结构体改名/删除即抛错（防判据静默失效） */
function rustStructBody(src: string, name: string): string {
  const start = src.indexOf(`pub struct ${name} {`);
  if (start === -1) {
    throw new Error(`后端源码中未找到 pub struct ${name}——载体已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`pub struct ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 提取 Rust struct 的字段名（= serde snake_case 序列化键名，该模型无 rename_all） */
function rustStructFields(body: string): string[] {
  return [...body.matchAll(/pub\s+([a-z_][a-z0-9_]*)\s*:/g)].map(m => m[1]);
}

const backendKeys = rustStructFields(rustStructBody(LEAD_MODEL_SRC, 'Model'));
const frontendKeys = tsInterfaceKeys(CRM_ENHANCED_SRC, 'CustomerWithTags');

describe('增强客户列表幽灵聚合键形状锁（后端载体 ↔ 前端声明 ↔ 视图绑定）', () => {
  it('后端事实基准：端点行载荷 = crm_lead::Model 整行直序列化，出口不补键', () => {
    // 服务层：分页元素是模型本体，信封 data 键直接装 Vec<crm_lead::Model>
    expect(LEAD_SERVICE_SRC).toContain('let items: Vec<crm_lead::Model> = paginator');
    expect(LEAD_SERVICE_SRC).toMatch(/"data": items,/);
    // 路由处理层：增强列表复用同一 list_leads，之后仅套字段级掩码（只删键/改值，不补键）
    expect(ENHANCED_HANDLER_SRC).toContain('service.list_leads(query, Some(&data_scope_ctx))');
    expect(ENHANCED_HANDLER_SRC).toContain('apply_customer_field_permission');
    // 模型键集地板：解析结果非空且规模合理（空集/解析失效判红）
    expect(backendKeys.length).toBeGreaterThanOrEqual(30);
    // 真实跟进/金额载体键在模型侧存在（证明删的是"无此出参键"，不是解析失效）
    expect(backendKeys).toContain('last_follow_up_date');
    expect(backendKeys).toContain('estimated_amount');
    // 幽灵键在后端模型侧确实不存在
    for (const key of GHOST_AGGREGATE_KEYS) {
      expect(backendKeys, `crm_lead 模型不应含键 ${key}`).not.toContain(key);
    }
    // owner_name 是模型真实字段：它是保留键，不得被误当成幽灵键一并删
    expect(backendKeys).toContain('owner_name');
  });

  it('前端声明：CustomerWithTags 不含幽灵聚合键，键集合 ⊆ 后端出参键 ∪ 钉死漂移清单', () => {
    // 键数地板：空集或解析失效判红
    expect(frontendKeys.length).toBeGreaterThanOrEqual(13);
    for (const key of GHOST_AGGREGATE_KEYS) {
      expect(frontendKeys, `CustomerWithTags 不得声明后端不输出的键 ${key}`).not.toContain(key);
    }
    const drift = frontendKeys.filter(k => !backendKeys.includes(k)).sort();
    // 精确相等：漂移只允许收缩且须同步修订清单，不允许扩大或静默换名
    expect(drift).toEqual([...FROZEN_VIEW_DRIFT_KEYS].sort());
    // 保留键双侧同在（防"两侧同时漂移把删除误固化"）
    for (const key of [
      'id',
      'email',
      'owner_id',
      'owner_name',
      'tags',
      'created_at',
      'updated_at',
    ]) {
      expect(frontendKeys, `声明缺键 ${key}`).toContain(key);
      expect(backendKeys, `后端模型已无键 ${key}，本锁需同步修订`).toContain(key);
    }
  });

  it('消费视图：绑定侧零幽灵键引用，且无 formatCurrency 假展示残留', () => {
    for (const [viewName, viewSrc] of [
      ['CustomerListTab.vue', CUSTOMER_LIST_TAB_SRC],
      ['RfmTab.vue', RFM_TAB_SRC],
    ] as const) {
      for (const key of GHOST_AGGREGATE_KEYS) {
        expect(viewSrc, `${viewName} 不得再绑定后端不输出的键 ${key}`).not.toContain(key);
      }
      // 恒空金额列的展示载体函数不得回流（两视图的金额单元格已整体删除）
      expect(viewSrc, `${viewName} 不应再引用 formatCurrency`).not.toContain('formatCurrency');
    }
  });

  it('列数对齐：主表列数、打印表头/行单元格数与删除后的定形一致', () => {
    // CustomerListTab：11 列删 total_amount/last_follow_up 两列 → 9
    const listColumns = (CUSTOMER_LIST_TAB_SRC.match(/<el-table-column/g) || []).length;
    expect(listColumns).toBe(9);
    // 打印表格：th 与 td 行数必须相等（删 total_amount 后 8→7），且不低于地板
    const printTh = (CUSTOMER_LIST_TAB_SRC.match(/<th>/g) || []).length;
    const printTd = (CUSTOMER_LIST_TAB_SRC.match(/<td>/g) || []).length;
    expect(printTh).toBe(printTd);
    expect(printTh).toBe(7);
    // RfmTab：6 列删 total_amount/total_orders 两列 → 4
    const rfmColumns = (RFM_TAB_SRC.match(/<el-table-column/g) || []).length;
    expect(rfmColumns).toBe(4);
  });

  it('检测力自证：复现历史幽灵形态的夹具必被上述判据抓到（防判据静默失效）', () => {
    // 正例夹具 A：把幽灵键 total_amount 加回 CustomerWithTags，声明判据必须抓到
    const mutatedIface = CRM_ENHANCED_SRC.replace(
      '  owner_id: number;\n  owner_name: string;',
      '  owner_id: number;\n  total_amount: number;\n  owner_name: string;'
    );
    expect(mutatedIface, '夹具替换本身必须生效').not.toBe(CRM_ENHANCED_SRC);
    const mutatedKeys = tsInterfaceKeys(mutatedIface, 'CustomerWithTags');
    expect(mutatedKeys).toContain('total_amount');
    expect(
      GHOST_AGGREGATE_KEYS.some(k => mutatedKeys.includes(k)),
      '幽灵键回流必须被声明判据抓到'
    ).toBe(true);
    // 正例夹具 B：漂移键换名回流（编造一个新漂移键），差集精确相等判据必须抓到
    const mutatedDrift = CRM_ENHANCED_SRC.replace(
      '  owner_id: number;',
      '  owner_id: number;\n  deal_amount_sum: number;'
    );
    const driftMutated = tsInterfaceKeys(mutatedDrift, 'CustomerWithTags').filter(
      k => !backendKeys.includes(k)
    );
    expect(driftMutated.sort()).not.toEqual([...FROZEN_VIEW_DRIFT_KEYS].sort());
    // 正例夹具 C：把金额单元格写回视图，绑定判据必须抓到
    const mutatedView = CUSTOMER_LIST_TAB_SRC.replace(
      '<el-table-column prop="owner_name"',
      '{{ row.total_amount }}<el-table-column prop="owner_name"'
    );
    expect(mutatedView, '夹具替换本身必须生效').not.toBe(CUSTOMER_LIST_TAB_SRC);
    expect(mutatedView).toContain('total_amount');
    // 反向自证：后端模型块被改名时解析必须抛错而非静默空集
    expect(() =>
      rustStructFields(rustStructBody('export const nothing = 1;\n', 'Model'))
    ).toThrow();
  });
});
