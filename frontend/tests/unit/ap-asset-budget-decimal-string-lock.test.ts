/**
 * AP/Asset/Budget 出参 Decimal→JSON 字符串金额键形状锁（契约三向对锁的前端侧判据）。
 * 功能：把 backend/src/models/{ap_invoice,ap_verification,fixed_asset,budget_management,
 * budget_item_periods,budget_plan,budget_execution,budget_version}.rs 与
 * services/ap_reconciliation_ops/types.rs 中金额列的 Rust 类型（Decimal / Option<Decimal>）、
 * frontend/src/api/{ap,asset,budget}.ts 对应接口的字段声明（string / string | null）、以及本文件
 * 钉死清单三者互锁；任一侧单边把金额键退回 number 或混入 string | number 联合即判红。
 * 背景判据：backend/Cargo.toml 的 rust_decimal 仅启用 serde feature（未启 serde-floats），
 * 出参恒为十进制字符串；前端声明 number 会在 .toFixed/求和/比较处运行期崩或算错。
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
 * 归一只统一输入形态，不改变任何判据语义（键匹配不受行尾影响）。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const AP_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/ap.ts'));
const ASSET_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/asset.ts'));
const BUDGET_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/budget.ts'));

const AP_INVOICE_RS = readRepoSource(path.join(REPO_ROOT, 'backend/src/models/ap_invoice.rs'));
const AP_VERIFICATION_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/ap_verification.rs')
);
const AP_SUMMARY_TYPES_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/ap_reconciliation_ops/types.rs')
);
const FIXED_ASSET_RS = readRepoSource(path.join(REPO_ROOT, 'backend/src/models/fixed_asset.rs'));
const BUDGET_ITEM_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/budget_management.rs')
);
const BUDGET_PERIOD_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/budget_item_periods.rs')
);
const BUDGET_PLAN_RS = readRepoSource(path.join(REPO_ROOT, 'backend/src/models/budget_plan.rs'));
const BUDGET_EXEC_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/budget_execution.rs')
);
const BUDGET_VERSION_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/budget_version.rs')
);

/** 截取 TS 接口块正文（含字段类型原文）。找不到接口即抛错，防判据静默失去覆盖。 */
function tsInterfaceBody(src: string, name: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`未找到 interface ${name}——类型被删除或改名，形状锁失效`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 断言接口块内某键声明为 `: string;`，且未回退成 number、未用 string|number 联合掩盖 */
function expectFieldString(body: string, key: string, iface: string): void {
  expect(body, `${iface}.${key} 应声明为 string`).toContain(`  ${key}: string;`);
  expect(body, `${iface}.${key} 不得声明为 number`).not.toContain(`  ${key}: number;`);
  expect(body, `${iface}.${key} 不得用 string | number 联合掩盖`).not.toContain(
    `  ${key}: string | number;`
  );
}

/** 断言接口块内某可空键声明为 `: string | null;`（后端 Option<Decimal>） */
function expectFieldStringOrNull(body: string, key: string, iface: string): void {
  expect(body, `${iface}.${key} 应声明为 string | null`).toContain(`  ${key}: string | null;`);
}

/** 断言 Rust 实体/DTO 中该字段确为 Decimal（optional=true 则为 Option<Decimal>） */
function expectRustDecimal(rsSrc: string, field: string, optional: boolean): void {
  const expected = optional ? `pub ${field}: Option<Decimal>` : `pub ${field}: Decimal`;
  expect(rsSrc, `后端 ${field} 应为 ${expected}`).toContain(expected);
}

describe('AP 域金额键：后端 Decimal ↔ 前端 string 对锁', () => {
  it('APInvoice：amount/paid_amount/unpaid_amount/tax_amount 后端 Decimal、前端 string', () => {
    const body = tsInterfaceBody(AP_API_SRC, 'APInvoice');
    for (const key of ['amount', 'paid_amount', 'unpaid_amount', 'tax_amount'] as const) {
      expectFieldString(body, key, 'APInvoice');
      expectRustDecimal(AP_INVOICE_RS, key, false);
    }
  });

  it('APVerification：total_amount 后端 Decimal、前端 string', () => {
    expectFieldString(
      tsInterfaceBody(AP_API_SRC, 'APVerification'),
      'total_amount',
      'APVerification'
    );
    expectRustDecimal(AP_VERIFICATION_RS, 'total_amount', false);
  });

  it('APSupplierSummary：四个金额键后端 Decimal、前端 string；计数键仍 number', () => {
    const body = tsInterfaceBody(AP_API_SRC, 'APSupplierSummary');
    for (const key of [
      'total_invoice_amount',
      'total_paid_amount',
      'total_unpaid_amount',
      'overdue_amount',
    ] as const) {
      expectFieldString(body, key, 'APSupplierSummary');
      expectRustDecimal(AP_SUMMARY_TYPES_RS, key, false);
    }
    for (const key of [
      'total_invoice_count',
      'paid_invoice_count',
      'overdue_invoice_count',
    ] as const) {
      expect(body, `APSupplierSummary.${key} 应为 number`).toContain(`  ${key}: number;`);
    }
  });

  it('APInvoiceRelation：amount 后端 Decimal、前端 string', () => {
    expectFieldString(
      tsInterfaceBody(AP_API_SRC, 'APInvoiceRelation'),
      'amount',
      'APInvoiceRelation'
    );
    expectRustDecimal(AP_SUMMARY_TYPES_RS, 'amount', false);
  });
});

describe('Asset 域金额键：后端 Decimal / Option<Decimal> ↔ 前端 string / string|null 对锁', () => {
  it('FixedAsset：accumulated_depreciation 非空 Decimal→string；net_value/salvage_value Option<Decimal>→string|null', () => {
    const body = tsInterfaceBody(ASSET_API_SRC, 'FixedAsset');
    expectFieldString(body, 'accumulated_depreciation', 'FixedAsset');
    expectRustDecimal(FIXED_ASSET_RS, 'accumulated_depreciation', false);
    expectFieldStringOrNull(body, 'net_value', 'FixedAsset');
    expectRustDecimal(FIXED_ASSET_RS, 'net_value', true);
    expectFieldStringOrNull(body, 'salvage_value', 'FixedAsset');
    expectRustDecimal(FIXED_ASSET_RS, 'salvage_value', true);
  });
});

describe('Budget 域金额键：后端 Decimal ↔ 前端 string 对锁', () => {
  it('BudgetItem / BudgetItemPeriod / BudgetPlan / BudgetExecution / BudgetVersion 金额键均 string', () => {
    const item = tsInterfaceBody(BUDGET_API_SRC, 'BudgetItem');
    expectFieldString(item, 'planned_amount', 'BudgetItem');
    expectRustDecimal(BUDGET_ITEM_RS, 'planned_amount', false);

    const period = tsInterfaceBody(BUDGET_API_SRC, 'BudgetItemPeriod');
    expectFieldString(period, 'planned_amount', 'BudgetItemPeriod');
    expectFieldString(period, 'actual_amount', 'BudgetItemPeriod');
    expectRustDecimal(BUDGET_PERIOD_RS, 'planned_amount', false);
    expectRustDecimal(BUDGET_PERIOD_RS, 'actual_amount', false);

    const plan = tsInterfaceBody(BUDGET_API_SRC, 'BudgetPlan');
    expectFieldString(plan, 'total_amount', 'BudgetPlan');
    expectRustDecimal(BUDGET_PLAN_RS, 'total_amount', false);

    const exec = tsInterfaceBody(BUDGET_API_SRC, 'BudgetExecution');
    expectFieldString(exec, 'amount', 'BudgetExecution');
    expectRustDecimal(BUDGET_EXEC_RS, 'amount', false);

    const version = tsInterfaceBody(BUDGET_API_SRC, 'BudgetVersion');
    expectFieldString(version, 'total_amount', 'BudgetVersion');
    expectRustDecimal(BUDGET_VERSION_RS, 'total_amount', false);
  });
});

describe('检测力自证：单边把金额键退回 number 必被上述锁抓到', () => {
  it('前端夹具：APInvoice.amount 改回 number 后 string 断言判红', () => {
    const mutated = AP_API_SRC.replace('  amount: string;', '  amount: number;');
    expect(mutated).not.toBe(AP_API_SRC); // 夹具替换本身必须生效
    const body = tsInterfaceBody(mutated, 'APInvoice');
    expect(body).not.toContain('  amount: string;');
    expect(body).toContain('  amount: number;');
  });

  it('后端夹具：FixedAsset.accumulated_depreciation 不再是 Decimal 则实体侧断言判红', () => {
    const mutated = FIXED_ASSET_RS.replace(
      'pub accumulated_depreciation: Decimal',
      'pub accumulated_depreciation: f64'
    );
    expect(mutated).not.toBe(FIXED_ASSET_RS);
    expect(mutated).not.toContain('pub accumulated_depreciation: Decimal');
  });
});
