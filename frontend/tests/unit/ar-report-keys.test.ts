/**
 * AR 统计/账龄报表出参键形状锁（契约三向对锁的前端侧判据）。
 * 功能：把 backend/src/services/ar_ops/report.rs 中 build_statistics_response /
 * build_aging_response 的 json! 字面键、frontend/src/api/ar.ts 中
 * ARStatisticsReport / ARAgingReport 的声明键、以及本文件钉死清单三者互锁，
 * 任一侧单边改键即判红。信封门禁对同形状出参（single↔single）判不出键名失配，
 * 本文件与后端 contract 形状锁共同补足该族检测力。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内两份源码文本，不执行后端代码、不发网络请求。
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
 * 而本锁的夹具用字面量文本 `replace`（含 `\n`）构造单边改键的反例，对行尾形态敏感——
 * 不归一则本地 CRLF 下夹具替换静默落空，检测力自证用例被误判成判据失效。
 * 归一只统一输入形态，不改变任何判据语义（键集正则本就不受行尾影响）。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const AR_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/ar.ts'));
const REPORT_RS_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/services/ar_ops/report.rs'));

/** 钉死清单：statistics 聚合对象键（顺序即后端 json! 字面顺序） */
const STATISTICS_KEYS = [
  'total_invoices',
  'total_amount',
  'paid_amount',
  'unpaid_amount',
  'overdue_count',
  'overdue_amount',
  'collection_rate',
] as const;

/** 钉死清单：statistics 中必须为字符串（Decimal.to_string）的金额键 */
const STATISTICS_DECIMAL_KEYS = [
  'total_amount',
  'paid_amount',
  'unpaid_amount',
  'overdue_amount',
] as const;

/** 钉死清单：aging 聚合对象键（顺序即后端 json! 字面顺序） */
const AGING_KEYS = [
  'not_due',
  'bucket_0_30',
  'bucket_31_60',
  'bucket_61_90',
  'bucket_90_plus',
  'total_overdue',
  'invoice_count',
] as const;

/** 钉死清单：aging 中必须为字符串（Decimal.to_string）的金额键 */
const AGING_DECIMAL_KEYS = [
  'not_due',
  'bucket_0_30',
  'bucket_31_60',
  'bucket_61_90',
  'bucket_90_plus',
  'total_overdue',
] as const;

/**
 * 截取 TS 接口块正文（含字段类型原文）。入参：源码文本 + 接口名；找不到即抛错。
 */
function tsInterfaceBody(src: string, name: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`ar.ts 中未找到 interface ${name}——类型被删除或改名，形状锁失效`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/**
 * 提取 TS 接口块内的声明键（跳过 `[key: string]: unknown` 索引签名行）。
 * 入参：源码文本 + 接口名；找不到接口即抛错（防判据静默失去覆盖）。
 */
function tsInterfaceKeys(src: string, name: string): string[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}(?:readonly\s+)?([a-z][A-Za-z0-9_]*)\??\s*:/gm)].map(m => m[1]);
}

/**
 * 提取 Rust 函数段内 json! 的字符串字面键。
 * 入参：report.rs 文本 + 起始/结束标记；找不到标记即抛错（防判据静默失效）。
 */
function rustJsonKeys(src: string, startMarker: string, endMarker: string): string[] {
  const start = src.indexOf(startMarker);
  if (start === -1) {
    throw new Error(`report.rs 中未找到 \`${startMarker}\`——构造函数已漂移，锁需同步修订`);
  }
  const end = src.indexOf(endMarker, start);
  if (end === -1) {
    throw new Error(`report.rs 中未找到终止标记 \`${endMarker}\``);
  }
  const body = src.slice(start, end);
  return [...body.matchAll(/"([a-z_][a-z0-9_]*)"\s*:/g)].map(m => m[1]);
}

/** 取 report.rs 中某构造函数的源码段（正向/反向夹具共用） */
function rustFnBody(src: string, startMarker: string, endMarker: string): string {
  const start = src.indexOf(startMarker);
  const end = src.indexOf(endMarker, start);
  return src.slice(start, end);
}

describe('AR statistics/aging 出参键形状锁（后端源码 ↔ 前端声明 ↔ 钉死清单）', () => {
  it('statistics：后端 json! 键与前端声明键均与钉死清单逐字一致', () => {
    const backendKeys = rustJsonKeys(
      REPORT_RS_SRC,
      'fn build_statistics_response',
      'pub async fn get_daily_report'
    );
    const frontendKeys = tsInterfaceKeys(AR_API_SRC, 'ARStatisticsReport');
    expect(backendKeys).toEqual([...STATISTICS_KEYS]);
    expect(frontendKeys).toEqual([...STATISTICS_KEYS]);
  });

  it('aging：后端 json! 键与前端声明键均与钉死清单逐字一致', () => {
    const backendKeys = rustJsonKeys(
      REPORT_RS_SRC,
      'fn build_aging_response',
      'pub async fn get_aging_by_salesperson'
    );
    const frontendKeys = tsInterfaceKeys(AR_API_SRC, 'ARAgingReport');
    expect(backendKeys).toEqual([...AGING_KEYS]);
    expect(frontendKeys).toEqual([...AGING_KEYS]);
  });

  it('金额键类型形态：后端一律 Decimal.to_string() 字符串出参，前端接口块内声明 string', () => {
    const statsBody = rustFnBody(
      REPORT_RS_SRC,
      'fn build_statistics_response',
      'pub async fn get_daily_report'
    );
    const agingBody = rustFnBody(
      REPORT_RS_SRC,
      'fn build_aging_response',
      'pub async fn get_aging_by_salesperson'
    );
    const statsIface = tsInterfaceBody(AR_API_SRC, 'ARStatisticsReport');
    const agingIface = tsInterfaceBody(AR_API_SRC, 'ARAgingReport');
    for (const key of STATISTICS_DECIMAL_KEYS) {
      expect(statsBody).toContain(`"${key}": ${key}.to_string(),`);
      expect(statsIface).toContain(`  ${key}: string;`);
    }
    for (const key of AGING_DECIMAL_KEYS) {
      expect(agingBody).toContain(`"${key}": ${key}.to_string(),`);
      expect(agingIface).toContain(`  ${key}: string;`);
    }
    // 计数/率值为 JSON number（非字符串）：后端裸值入 json!，前端接口块内声明 number
    expect(statsBody).toContain('"total_invoices": total_invoices,');
    expect(statsBody).toContain('"overdue_count": overdue_count,');
    expect(statsBody).toContain('"collection_rate": collection_rate,');
    expect(agingBody).toContain('"invoice_count": invoice_count,');
    expect(statsIface).toContain('  total_invoices: number;');
    expect(statsIface).toContain('  overdue_count: number;');
    expect(statsIface).toContain('  collection_rate: number;');
    expect(agingIface).toContain('  invoice_count: number;');
  });

  it('信封声明：statistics/aging 均为单对象（无 [] 后缀），与后端单对象构造同源', () => {
    expect(AR_API_SRC).toMatch(/Promise<ApiResponse<ARStatisticsReport>>\s*\{/);
    expect(AR_API_SRC).toMatch(/Promise<ApiResponse<ARAgingReport>>\s*\{/);
  });

  it('检测力自证：单边改键（夹具复现历史编造键形态）必被上述对锁抓到', () => {
    // 负例夹具：把前端声明换成后端不存在的编造键名，对锁必须判红而非静默通过
    const mutated = AR_API_SRC.replace(
      'export interface ARStatisticsReport {\n  total_invoices: number;',
      'export interface ARStatisticsReport {\n  total_invoice_amount: number;'
    );
    expect(mutated).not.toBe(AR_API_SRC); // 夹具替换本身必须生效
    expect(tsInterfaceKeys(mutated, 'ARStatisticsReport')).not.toEqual([...STATISTICS_KEYS]);
    // 反向：后端侧改键的夹具同样破坏三向一致
    const mutatedRs = REPORT_RS_SRC.replace('"not_due": not_due.to_string(),', '"days_0_30": x,');
    expect(
      rustJsonKeys(mutatedRs, 'fn build_aging_response', 'pub async fn get_aging_by_salesperson')
    ).not.toEqual([...AGING_KEYS]);
  });
});
