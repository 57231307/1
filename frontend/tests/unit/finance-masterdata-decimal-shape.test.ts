/**
 * 财务主数据族（AP/资产/预算/辅助核算）rust_decimal 出参键形状锁（契约三向对锁的前端侧判据）。
 * 功能：把 backend/src/models 与 services/handlers 中金额/数量列的 Rust Decimal 声明、
 * frontend/src/api 中对应接口的 TS 声明、以及本文件钉死清单三者互锁——后端列为 Decimal 的键
 * （rust_decimal 仅启 serde feature、未启 serde-floats，见 backend/Cargo.toml:60，序列化线格式
 * 为十进制字符串）在前端必须声明 string；i32/i64 计数键必须声明 number。任一侧单边改型即判红，
 * 防止"类型谎言"回潮（number 声明会在 .toFixed/算术处运行期崩或算错）。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内源码文本，不执行后端代码、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 * 范围说明：键名与后端真实出参整体错位的接口（APReconciliation/APStatisticsData/
 * APDailyReportData/APMonthlyReportData/APAgingReportData/Budget/FixedAsset 的
 * asset_code/purchase_amount/useful_life_months 等）属独立契约漂移、已另行立案，
 * 其键名不参与本锁的后端对锁；本批已钉为 string 的线格式形态由既有源码注释维持判据。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/** 读仓库内源码并统一行尾为 LF（Windows 检出 CRLF、CI 检出 LF，判据对行尾敏感） */
function readRepoSource(relFromRepoRoot: string): string {
  return readFileSync(path.join(REPO_ROOT, relFromRepoRoot), 'utf8').replace(/\r\n/g, '\n');
}

/** 截取 TS 接口块正文（含字段类型原文）；接口不存在即抛错（防判据静默失去覆盖） */
function tsInterfaceBody(tsSrc: string, name: string): string {
  const start = tsSrc.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`TS 源码中未找到 interface ${name}——类型被删除或改名，形状锁失效`);
  }
  const end = tsSrc.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return tsSrc.slice(start, end);
}

type DecimalContract = {
  /** 前端 api 文件（相对 frontend/） */
  tsFile: string;
  /** 前端接口名 */
  iface: string;
  /** 后端 Rust 声明锚点（相对 backend/src/），逐字子串 */
  rsMarker: string;
  /** 前端接口块内必须逐字存在的 TS 声明（含可空形态） */
  tsDecl: string;
};

/**
 * 钉死清单：后端 Decimal ⇒ 前端 string（出参）。
 * rsMarker 中的行号以 backend/src 当前源码为准（tests 只按逐字子串对锁，不解析行号）。
 */
const DECIMAL_CONTRACTS: DecimalContract[] = [
  // —— AP 应付（GET /ap/invoices 直接序列化 models/ap_invoice.rs 实体） ——
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APInvoice',
    rsMarker: 'pub amount: Decimal',
    tsDecl: '  amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APInvoice',
    rsMarker: 'pub paid_amount: Decimal',
    tsDecl: '  paid_amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APInvoice',
    rsMarker: 'pub unpaid_amount: Decimal',
    tsDecl: '  unpaid_amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APInvoice',
    rsMarker: 'pub tax_amount: Decimal',
    tsDecl: '  tax_amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APVerification',
    rsMarker: 'pub total_amount: Decimal',
    tsDecl: '  total_amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APSupplierSummary',
    rsMarker: 'pub total_invoice_amount: Decimal',
    tsDecl: '  total_invoice_amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APSupplierSummary',
    rsMarker: 'pub total_paid_amount: Decimal',
    tsDecl: '  total_paid_amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APSupplierSummary',
    rsMarker: 'pub total_unpaid_amount: Decimal',
    tsDecl: '  total_unpaid_amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APSupplierSummary',
    rsMarker: 'pub overdue_amount: Decimal',
    tsDecl: '  overdue_amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APInvoiceRelation',
    rsMarker: 'pub amount: Decimal',
    tsDecl: '  amount: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'APAgingItem',
    rsMarker: 'pub total_amount: Decimal',
    tsDecl: '  total_amount: string;',
  },
  // —— 固定资产（GET /fixed-assets 直接序列化 models/fixed_asset.rs 实体） ——
  {
    tsFile: 'src/api/asset.ts',
    iface: 'FixedAsset',
    rsMarker: 'pub accumulated_depreciation: Decimal',
    tsDecl: '  accumulated_depreciation: string;',
  },
  {
    tsFile: 'src/api/asset.ts',
    iface: 'FixedAsset',
    rsMarker: 'pub net_value: Option<Decimal>',
    tsDecl: '  net_value: string | null;',
  },
  {
    tsFile: 'src/api/asset.ts',
    iface: 'FixedAsset',
    rsMarker: 'pub salvage_value: Option<Decimal>',
    tsDecl: '  salvage_value: string | null;',
  },
  // —— 预算（列表/详情/执行/版本均直接序列化实体） ——
  {
    tsFile: 'src/api/budget.ts',
    iface: 'BudgetItem',
    rsMarker: 'pub planned_amount: Decimal',
    tsDecl: '  planned_amount: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'BudgetItemPeriod',
    rsMarker: 'pub planned_amount: Decimal',
    tsDecl: '  planned_amount: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'BudgetItemPeriod',
    rsMarker: 'pub actual_amount: Decimal',
    tsDecl: '  actual_amount: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'BudgetPlan',
    rsMarker: 'pub total_amount: Decimal',
    tsDecl: '  total_amount: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'BudgetExecution',
    rsMarker: 'pub amount: Decimal',
    tsDecl: '  amount: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'BudgetVersion',
    rsMarker: 'pub total_amount: Decimal',
    tsDecl: '  total_amount: string;',
  },
  // —— 辅助核算（handler 出参 DTO） ——
  {
    tsFile: 'src/api/assist-accounting.ts',
    iface: 'AssistRecordResponse',
    rsMarker: 'pub debit_amount: Decimal',
    tsDecl: '  debit_amount: string;',
  },
  {
    tsFile: 'src/api/assist-accounting.ts',
    iface: 'AssistRecordResponse',
    rsMarker: 'pub credit_amount: Decimal',
    tsDecl: '  credit_amount: string;',
  },
  {
    tsFile: 'src/api/assist-accounting.ts',
    iface: 'AssistRecordResponse',
    rsMarker: 'pub quantity_meters: Decimal',
    tsDecl: '  quantity_meters: string;',
  },
  {
    tsFile: 'src/api/assist-accounting.ts',
    iface: 'AssistRecordResponse',
    rsMarker: 'pub quantity_kg: Decimal',
    tsDecl: '  quantity_kg: string;',
  },
  {
    tsFile: 'src/api/assist-accounting.ts',
    iface: 'AssistSummaryResponse',
    rsMarker: 'pub total_debit: Decimal',
    tsDecl: '  total_debit: string;',
  },
  {
    tsFile: 'src/api/assist-accounting.ts',
    iface: 'AssistSummaryResponse',
    rsMarker: 'pub total_credit: Decimal',
    tsDecl: '  total_credit: string;',
  },
  {
    tsFile: 'src/api/assist-accounting.ts',
    iface: 'AssistSummaryResponse',
    rsMarker: 'pub total_quantity_meters: Decimal',
    tsDecl: '  total_quantity_meters: string;',
  },
  {
    tsFile: 'src/api/assist-accounting.ts',
    iface: 'AssistSummaryResponse',
    rsMarker: 'pub total_quantity_kg: Decimal',
    tsDecl: '  total_quantity_kg: string;',
  },
];

/**
 * 钉死清单：请求侧（后端反序列化目标）Decimal 键 ⇒ 前端按十进制字符串下发。
 * rust_decimal 未启 serde-floats，JSON 浮点字面量在反序列化层即被拒绝（400），
 * 整数可过但落库精度语义不稳，本仓口径=两位小数十进制字符串（见各接口注释）。
 */
const REQUEST_DECIMAL_CONTRACTS: DecimalContract[] = [
  {
    tsFile: 'src/api/ap.ts',
    iface: 'CreateAPInvoiceRequest',
    rsMarker: 'pub amount: Option<Decimal>',
    tsDecl: '  amount?: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'CreateAPInvoiceRequest',
    rsMarker: 'pub tax_amount: Option<Decimal>',
    tsDecl: '  tax_amount?: string;',
  },
  {
    tsFile: 'src/api/ap.ts',
    iface: 'ApVerificationItemInput',
    rsMarker: 'pub verify_amount: Decimal',
    tsDecl: '  verify_amount: string;',
  },
  {
    tsFile: 'src/api/asset.ts',
    iface: 'FixedAssetCreateRequest',
    rsMarker: 'pub original_value: Option<rust_decimal::Decimal>',
    tsDecl: '  original_value: string;',
  },
  {
    tsFile: 'src/api/asset.ts',
    iface: 'DisposalRequest',
    rsMarker: 'pub disposal_value: rust_decimal::Decimal',
    tsDecl: '  disposal_value: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'CreateBudgetItemPayload',
    rsMarker: 'pub planned_amount: Decimal',
    tsDecl: '  planned_amount: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'UpdateBudgetItemPayload',
    rsMarker: 'pub planned_amount: Option<Decimal>',
    tsDecl: '  planned_amount?: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'BudgetItemPeriodInput',
    rsMarker: 'pub planned_amount: Decimal',
    tsDecl: '  planned_amount: string;',
  },
  {
    tsFile: 'src/api/budget.ts',
    iface: 'CreateBudgetPlanPayload',
    rsMarker: 'pub total_amount: Option<Decimal>',
    tsDecl: '  total_amount?: string;',
  },
];

/** 后端 Rust 源（相对 backend/src/）——对锁文本来自这些文件 */
const BACKEND_SOURCES = [
  'models/ap_invoice.rs',
  'models/ap_verification.rs',
  'models/ap_verification_item.rs',
  'models/fixed_asset.rs',
  'models/budget_management.rs',
  'models/budget_item_periods.rs',
  'models/budget_plan.rs',
  'models/budget_execution.rs',
  'models/budget_version.rs',
  'models/dto/budget_dto.rs',
  'models/dto/budget_management_dto.rs',
  'services/ap_invoice_ops/types.rs',
  'services/ap_reconciliation_ops/types.rs',
  'services/ap_verification_service.rs',
  'services/budget_management_service.rs',
  'handlers/fixed_asset_handler.rs',
  'handlers/budget_management_handler.rs',
  'handlers/assist_accounting_handler.rs',
];

describe('财务主数据族金额键线格式形状锁（后端 Rust 声明 ↔ 前端 TS 声明 ↔ 钉死清单）', () => {
  const backendText = BACKEND_SOURCES.map(f => readRepoSource(`backend/src/${f}`)).join('\n');

  for (const c of DECIMAL_CONTRACTS) {
    it(`出参对锁：${c.iface}.${c.tsDecl.trim().replace(/;.*/, '')} ⇒ string，后端同键为 Decimal`, () => {
      const ifaceBody = tsInterfaceBody(readRepoSource(`frontend/${c.tsFile}`), c.iface);
      expect(
        ifaceBody,
        `${c.tsFile} 的 interface ${c.iface} 缺少声明 "${c.tsDecl.trim()}"（Decimal 出参被改回 number 即类型谎言回潮）`
      ).toContain(c.tsDecl);
      expect(backendText, `后端源码缺少锚点 "${c.rsMarker}"，需人工复核线格式是否变化`).toContain(
        c.rsMarker
      );
    });
  }

  for (const c of REQUEST_DECIMAL_CONTRACTS) {
    it(`入参对锁：${c.iface}.${c.tsDecl.trim().replace(/;.*/, '')} ⇒ 十进制字符串下发`, () => {
      const ifaceBody = tsInterfaceBody(readRepoSource(`frontend/${c.tsFile}`), c.iface);
      expect(
        ifaceBody,
        `${c.tsFile} 的 interface ${c.iface} 缺少声明 "${c.tsDecl.trim()}"`
      ).toContain(c.tsDecl);
      expect(backendText).toContain(c.rsMarker);
    });
  }

  it('计数/整型键保持 number：后端 i64 ⇒ 前端 number（不得被字符串化矫枉过正）', () => {
    const apTs = readRepoSource('frontend/src/api/ap.ts');
    const summaryBody = tsInterfaceBody(apTs, 'APSupplierSummary');
    for (const k of [
      '  total_invoice_count: number;',
      '  paid_invoice_count: number;',
      '  partial_paid_invoice_count: number;',
      '  overdue_invoice_count: number;',
    ]) {
      expect(summaryBody, `APSupplierSummary 缺少计数键声明 "${k}"`).toContain(k);
    }
    expect(backendText).toContain('pub total_invoice_count: i64');
    const agingBody = tsInterfaceBody(apTs, 'APAgingItem');
    expect(agingBody).toContain('  invoice_count: number;');
    expect(backendText).toContain('pub invoice_count: i64');
    const assistBody = tsInterfaceBody(
      readRepoSource('frontend/src/api/assist-accounting.ts'),
      'AssistSummaryResponse'
    );
    expect(assistBody).toContain('  record_count: number;');
    expect(backendText).toContain('pub record_count: i64');
    const assetBody = tsInterfaceBody(readRepoSource('frontend/src/api/asset.ts'), 'FixedAsset');
    expect(assetBody, 'id 为后端 i32，出参 JSON number').toContain('  id: number;');
    expect(backendText).toContain('pub id: i32');
  });

  it('budget.ts 不再残留 string | number 形态 hedge 声明', () => {
    expect(readRepoSource('frontend/src/api/budget.ts')).not.toMatch(/: string \| number;/);
  });

  it('检测力自证：把 Decimal 键改回 number 声明必被本锁抓到', () => {
    const apTs = readRepoSource('frontend/src/api/ap.ts');
    const mutated = apTs.replace('  amount: string;', '  amount: number;');
    expect(mutated).not.toBe(apTs); // 夹具替换本身必须生效
    const body = tsInterfaceBody(mutated, 'APInvoice');
    expect(body).not.toContain('  amount: string;');
    const mutatedReq = readRepoSource('frontend/src/api/budget.ts').replace(
      '  planned_amount: string;',
      '  planned_amount: number;'
    );
    expect(tsInterfaceBody(mutatedReq, 'BudgetItem')).not.toContain('  planned_amount: string;');
  });
});
