/**
 * CRM 商机分析出参金额/赢率列类型形状锁（Decimal→JSON 字符串族的前端侧判据）。
 * 功能：把 backend/src/services/crm/opp.rs 中 SalesFunnelReport / WeightedForecastResult /
 * WeightedForecastItem / ForecastAccuracyResult 四个 Serialize 结构体、
 * backend/src/models/crm_opportunity.rs 的 win_probability 列、
 * frontend/src/api/crm.ts 中同名接口的声明类型、以及本文件钉死清单三者互锁：
 * Decimal 列必须声明 string（可空列 string|null），f64/i64/i32/u32 列必须保持 number，
 * 任一侧单边改类型（含把 f64 误改成 string）即判红。
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
 * 夹具的字面量 replace（含 `\n`）对行尾形态敏感，不归一会让检测力自证用例被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const CRM_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/crm.ts'));
const OPP_RS_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/services/crm/opp.rs'));
const OPP_MODEL_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/models/crm_opportunity.rs'));

/** 截取 TS interface 正文（含字段类型原文）；接口被删除或改名即抛错（防判据静默失去覆盖） */
function tsInterfaceBody(src: string, name: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`crm.ts 中未找到 interface ${name}——类型被删除或改名，形状锁失效`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 取 TS interface 内某键的声明类型原文（去可选标记）；键不存在返回 undefined */
function tsFieldType(body: string, key: string): string | undefined {
  const re = new RegExp(`^ {2}${key}\\??\\s*:\\s*([^;]+);`, 'm');
  const m = body.match(re);
  return m ? m[1].trim() : undefined;
}

/** 截取 Rust pub struct 正文；结构体改名/删除即抛错 */
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

/** 取 Rust 结构体内某字段的类型原文 */
function rustFieldType(body: string, key: string): string | undefined {
  const re = new RegExp(`^\\s{4}pub ${key}: ([^,]+),`, 'm');
  const m = body.match(re);
  return m ? m[1].trim() : undefined;
}

/** Decimal→字符串族钉死清单：[接口名, Rust 结构体名, 键, 前端应声明类型] */
const DECIMAL_LOCKS: ReadonlyArray<readonly [string, string, string, string]> = [
  ['SalesFunnelReport', 'SalesFunnelReport', 'opportunity_amount', 'string'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'won_amount', 'string'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'order_amount', 'string'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'collected_amount', 'string'],
  ['WeightedForecastResult', 'WeightedForecastResult', 'total_estimated_amount', 'string'],
  ['WeightedForecastResult', 'WeightedForecastResult', 'total_weighted_amount', 'string'],
  ['WeightedForecastItem', 'WeightedForecastItem', 'estimated_amount', 'string | null'],
  ['WeightedForecastItem', 'WeightedForecastItem', 'weighted_amount', 'string | null'],
  ['WeightedForecastItem', 'WeightedForecastItem', 'win_probability', 'string'],
  ['ForecastAccuracyResult', 'ForecastAccuracyResult', 'forecast_amount', 'string'],
  ['ForecastAccuracyResult', 'ForecastAccuracyResult', 'actual_amount', 'string'],
];

/** f64/i64/i32/u32→number 族钉死清单（防"见金额就改 string"的过修正） */
const NUMBER_LOCKS: ReadonlyArray<readonly [string, string, string, string]> = [
  ['SalesFunnelReport', 'SalesFunnelReport', 'lead_count', 'number'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'opportunity_count', 'number'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'quotation_count', 'number'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'won_count', 'number'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'order_count', 'number'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'lead_to_opp_rate', 'number'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'opp_to_quotation_rate', 'number'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'opp_to_order_rate', 'number'],
  ['SalesFunnelReport', 'SalesFunnelReport', 'order_to_collection_rate', 'number'],
  ['WeightedForecastResult', 'WeightedForecastResult', 'total_opportunities', 'number'],
  ['ForecastAccuracyResult', 'ForecastAccuracyResult', 'forecast_count', 'number'],
  ['ForecastAccuracyResult', 'ForecastAccuracyResult', 'won_count', 'number'],
  ['ForecastAccuracyResult', 'ForecastAccuracyResult', 'accuracy_rate', 'number'],
  ['ForecastAccuracyResult', 'ForecastAccuracyResult', 'year', 'number'],
  ['ForecastAccuracyResult', 'ForecastAccuracyResult', 'month', 'number'],
];

/** 后端 Decimal 类型原文（Option<Decimal> 与裸 Decimal 两种形态） */
function rustIsDecimal(ty: string | undefined): boolean {
  return (
    ty === 'rust_decimal::Decimal' ||
    ty === 'Decimal' ||
    ty === 'Option<rust_decimal::Decimal>' ||
    ty === 'Option<Decimal>'
  );
}

/** 后端 JSON number 类型原文（f64/整数族） */
function rustIsJsonNumber(ty: string | undefined): boolean {
  return ty !== undefined && ['f64', 'i64', 'i32', 'u32', 'i16'].includes(ty);
}

describe('CRM 商机分析出参 Decimal→string / f64→number 类型形状锁', () => {
  it('Decimal 族：后端列为 Decimal，前端逐键声明字符串形态', () => {
    for (const [iface, structName, key, expected] of DECIMAL_LOCKS) {
      const rsTy = rustFieldType(rustStructBody(OPP_RS_SRC, structName), key);
      expect(rustIsDecimal(rsTy), `${structName}.${key} 后端类型应为 Decimal，实际 ${rsTy}`).toBe(
        true
      );
      expect(tsFieldType(tsInterfaceBody(CRM_API_SRC, iface), key)).toBe(expected);
    }
  });

  it('number 族：f64/i64/i32/u32 列前端保持 number（禁止误改 string）', () => {
    for (const [iface, structName, key, expected] of NUMBER_LOCKS) {
      const rsTy = rustFieldType(rustStructBody(OPP_RS_SRC, structName), key);
      expect(
        rustIsJsonNumber(rsTy),
        `${structName}.${key} 后端类型应为 number 族，实际 ${rsTy}`
      ).toBe(true);
      expect(tsFieldType(tsInterfaceBody(CRM_API_SRC, iface), key)).toBe(expected);
    }
  });

  it('商机行赢率：模型列 Option<Decimal> 整行直出，前端 Opportunity 声明 string|null', () => {
    const rsTy = rustFieldType(rustStructBody(OPP_MODEL_SRC, 'Model'), 'win_probability');
    expect(rsTy).toBe('Option<Decimal>');
    expect(tsFieldType(tsInterfaceBody(CRM_API_SRC, 'Opportunity'), 'win_probability')).toBe(
      'string | null'
    );
    // 列表/详情出参即 crm_opportunity::Model 整行序列化（services/crm/opp.rs list_opportunities）
    expect(OPP_RS_SRC).toContain('let items: Vec<crm_opportunity::Model> = paginator');
    expect(OPP_RS_SRC).toContain('"data": items,');
  });

  it('检测力自证：单边改类型必被对锁抓到', () => {
    // 前端夹具：把 won_amount 回退成谎报形态 number，DECIMAL_LOCKS 判据必须失配
    const mutatedTs = CRM_API_SRC.replace('  won_amount: string;', '  won_amount: number;');
    expect(mutatedTs).not.toBe(CRM_API_SRC); // 夹具替换本身必须生效
    expect(tsFieldType(tsInterfaceBody(mutatedTs, 'SalesFunnelReport'), 'won_amount')).not.toBe(
      'string'
    );
    // 后端夹具：把 accuracy_rate 从 f64 改成 Decimal（模拟列类型漂移），number 族锁必须失配
    const mutatedRs = OPP_RS_SRC.replace(
      'pub accuracy_rate: f64,',
      'pub accuracy_rate: rust_decimal::Decimal,'
    );
    expect(mutatedRs).not.toBe(OPP_RS_SRC);
    const driftTy = rustFieldType(
      rustStructBody(mutatedRs, 'ForecastAccuracyResult'),
      'accuracy_rate'
    );
    expect(rustIsJsonNumber(driftTy)).toBe(false);
  });
});
