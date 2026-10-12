/**
 * CRM 赢率真实列与幽灵声明形状锁（后端源码 ↔ 前端源码 ↔ 本文件断言三向互证）。
 * 功能：
 * 1) crm_opportunity 模型只有 win_probability（Option<Decimal>，线上为 JSON 字符串或 null）、
 *    不存在 probability 字段——前端 Opportunity 接口不得保留 probability 键，
 *    商机列表/详情赢率展示不得用 ?? 兜底把无值显示成 0（无值一律 '-' 占位）；
 * 2) /crm/customers/{id}/summary 出参键 = 后端 CustomerRelationSummary 七键
 *    （金额键名是 total_order_amount，不是 total_amount）——前端不得再保留按其旧漂移键名
 *    （customer_name/total_amount/last_order_date/credit_limit/credit_used）声明的摘要类型；
 * 3) 商机表单不得以固定占位值上送赢率——显式上送会覆盖后端按阶段给赢率的口径
 *    （default_win_probability_by_stage：未传时创建/阶段流转均由后端填充），
 *    仅用户显式拨动滑块才如实上送，编辑回填按 Decimal→number 归一范式。
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

const CRM_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/crm.ts'));
const OPP_VIEW_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/crm/opportunities/index.vue')
);
const OPP_FORM_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/crm/opportunities/tabs/OpportunityFormTab.vue')
);
const OPP_MODEL_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/models/crm_opportunity.rs'));
const CRM_SERVICE_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/services/crm/mod.rs'));
const OPP_SERVICE_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/services/crm/opp.rs'));

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

/** 截取 Rust struct 正文；结构体改名/删除即抛错（防判据静默失去覆盖） */
function rustStructBody(src: string, name: string): string {
  const start = src.indexOf(`pub struct ${name} {`);
  if (start === -1) {
    throw new Error(`后端源码中未找到 pub struct ${name}——构造点已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`pub struct ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** interface 块内是否存在幽灵键 probability 的字段声明（win_probability 不算） */
function declaresBareProbabilityKey(ifaceBody: string): boolean {
  return /^ {2}probability\??\s*:/m.test(ifaceBody);
}

describe('CRM 赢率真实列 ↔ 前端幽灵键/兜底/占位上送 形状锁', () => {
  it('后端事实基准：模型列只有 win_probability（Option<Decimal>），无 probability 字段', () => {
    const modelBody = rustStructBody(OPP_MODEL_SRC, 'Model');
    expect(modelBody).toContain('pub win_probability: Option<Decimal>,');
    expect(/pub\s+probability\s*:/.test(OPP_MODEL_SRC)).toBe(false);
    // 阶段默认赢率是后端唯一权威（前端复制该映射即漂移源，故前端只省略键、不算默认值）
    expect(OPP_SERVICE_SRC).toContain('fn default_win_probability_by_stage');
  });

  it('前端 Opportunity 接口：无 probability 幽灵键，win_probability 保持 string|null 形态', () => {
    const body = tsInterfaceBody(CRM_API_SRC, 'Opportunity');
    expect(declaresBareProbabilityKey(body), 'Opportunity 内不得再声明 probability 键').toBe(false);
    expect(body).toContain('win_probability?: string | null;');
  });

  it('漂移键名的客户摘要前端声明已删除（后端出参键为 total_order_amount 族）', () => {
    expect(CRM_API_SRC).not.toContain('export interface CustomerSummary');
    expect(CRM_API_SRC).not.toContain('getCustomerSummary');
    // 后端真实形状仍在（锁后端侧证据，防两侧同时漂移把删除误固化为"无此端点"）
    const summaryBody = rustStructBody(CRM_SERVICE_SRC, 'CustomerRelationSummary');
    expect(summaryBody).toContain('pub total_order_amount: Option<rust_decimal::Decimal>,');
    expect(/pub\s+total_amount\s*:/.test(summaryBody)).toBe(false);
    expect(summaryBody).toContain('pub last_interaction_at:');
    expect(/pub\s+last_order_date\s*:/.test(summaryBody)).toBe(false);
  });

  it('商机列表页：赢率展示无 ?? 兜底、无幽灵键读取，无值经统一函数出占位', () => {
    expect(OPP_VIEW_SRC).not.toContain('viewData.probability');
    expect(OPP_VIEW_SRC).not.toMatch(/\?\?\s*0\s*}\s*%/);
    expect(OPP_VIEW_SRC).toContain('displayWinProbability');
    // 无值（null/缺失/伪形）显示 '-'，有值才拼百分号——不得把无值显示成 0%
    expect(OPP_VIEW_SRC).toMatch(
      /const displayWinProbability = \(row: OpportunityRow\): string =>/
    );
  });

  it('商机表单：无上送固定占位赢率的形态；仅显式拨动才下发，回填走数值归一', () => {
    expect(OPP_FORM_SRC).not.toContain('win_probability: 50');
    // 无条件把控件值放进提交载荷的字面量形态即本锁要禁的"恒上送"，必须消失
    expect(OPP_FORM_SRC).not.toContain('win_probability: formData.win_probability,');
    expect(OPP_FORM_SRC).toContain('winProbabilityTouched');
    expect(OPP_FORM_SRC).toContain('@change="winProbabilityTouched = true"');
    expect(OPP_FORM_SRC).toContain('normalizePercent(props.rowData.win_probability)');
    expect(OPP_FORM_SRC).toContain('formData.win_probability = undefined;');
  });

  it('检测力自证：复现历史谎报形态的夹具必被上述判据抓到（防判据静默失效）', () => {
    // 正例夹具 A：把幽灵键 probability 加回 Opportunity，键声明判据必须抓到
    const mutatedIface = CRM_API_SRC.replace(
      '  expected_close_date: string;\n  description: string;',
      '  expected_close_date: string;\n  probability: number;\n  description: string;'
    );
    expect(mutatedIface).not.toBe(CRM_API_SRC); // 夹具替换本身必须生效
    expect(declaresBareProbabilityKey(tsInterfaceBody(mutatedIface, 'Opportunity'))).toBe(true);
    // 正例夹具 B：把 ?? 兜底展示写回列表页，兜底判据必须抓到
    const mutatedView = OPP_VIEW_SRC.replace(
      'displayWinProbability(viewData)',
      'viewData.win_probability ?? viewData.probability ?? 0 }}%'
    );
    expect(mutatedView).not.toBe(OPP_VIEW_SRC);
    expect(mutatedView).toContain('viewData.probability');
    // 正例夹具 C：把恒上送占位值写回表单，占位判据必须抓到
    const mutatedForm = OPP_FORM_SRC.replace(
      'win_probability: winProbabilityToSend,',
      'win_probability: formData.win_probability,'
    );
    expect(mutatedForm).not.toBe(OPP_FORM_SRC);
    expect(mutatedForm).toContain('win_probability: formData.win_probability,');
  });
});
