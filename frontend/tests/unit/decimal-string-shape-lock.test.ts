/**
 * 库存/生产/委外出参 Decimal=字符串形状锁（契约三向对锁的前端侧判据）。
 * 功能：把 backend 出参结构体（handlers/inventory_stock_handler_dto.rs、
 * handlers/production_order_handler.rs、models/production_recipe.rs、
 * models/outsourcing_order.rs、models/outsourcing_order_item.rs、
 * models/outsourcing_receipt.rs）中钉死字段的 Rust `Decimal`/`Option<Decimal>` 声明、
 * frontend/src/api/{inventory,production,production-recipe,outsourcing}.ts 对应接口的
 * `string` / `string | null` 声明、以及本文件钉死清单三者互锁；任一侧单边回退成
 * number/f64 即判红。
 * 依据：backend/Cargo.toml 的 rust_decimal 仅启用 `serde` feature（未启用 serde-float），
 * 该配置下 rust_decimal 的 Serialize 走 serialize_str，Decimal 出参恒为 JSON 字符串；
 * 本文件同时钉死该 feature 前提，前提被改动即判红。
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
 * 正则夹具含 `\n` 边界，对行尾形态敏感——不归一则本地 CRLF 下断言静默落空。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

function feSrc(rel: string): string {
  return readRepoSource(path.join(FRONTEND_ROOT, 'src/api', rel));
}
function beSrc(rel: string): string {
  return readRepoSource(path.join(REPO_ROOT, 'backend/src', rel));
}

const INVENTORY_SRC = feSrc('inventory.ts');
const PRODUCTION_SRC = feSrc('production.ts');
const RECIPE_SRC = feSrc('production-recipe.ts');
const OUTSOURCING_SRC = feSrc('outsourcing.ts');
const STOCK_DTO_RS = beSrc('handlers/inventory_stock_handler_dto.rs');
const PROD_ORDER_HANDLER_RS = beSrc('handlers/production_order_handler.rs');
const RECIPE_MODEL_RS = beSrc('models/production_recipe.rs');
const OUTSORDER_MODEL_RS = beSrc('models/outsourcing_order.rs');
const OUTSITEM_MODEL_RS = beSrc('models/outsourcing_order_item.rs');
const OUTSRECEIPT_MODEL_RS = beSrc('models/outsourcing_receipt.rs');
const CARGO_TOML = readRepoSource(path.join(REPO_ROOT, 'backend/Cargo.toml'));

/** TS 接口块正文（含字段类型原文）；找不到即抛错，防判据静默失去覆盖 */
function tsInterfaceBody(src: string, name: string, fileLabel: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`${fileLabel} 中未找到 interface ${name}——类型被删除或改名，形状锁失效`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) throw new Error(`${fileLabel}: interface ${name} 块未闭合，无法解析`);
  return src.slice(start, end);
}

/** Rust 结构体正文；找不到即抛错（后端结构漂移时锁必须显式失败而非空转） */
function rustStructBody(src: string, structMarker: string, fileLabel: string): string {
  const start = src.indexOf(structMarker);
  if (start === -1) {
    throw new Error(`${fileLabel} 中未找到 \`${structMarker}\`——结构已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) throw new Error(`${fileLabel}: ${structMarker} 结构未闭合`);
  return src.slice(start, end);
}

/** 后端字段必须声明为 Decimal（含 rust_decimal:: 全限定名）；Option 形态由 nullable 循环单独锁 */
function expectRustDecimal(body: string, key: string, fileLabel: string): void {
  const re = new RegExp(`pub ${key}: (Option<)?(rust_decimal::)?Decimal`);
  expect(body, `${fileLabel} 字段 ${key} 应为 Decimal/Option<Decimal}`).toMatch(re);
}

/**
 * 前端出参键声明锁：非空键须为 `key: string;`，可空键须为 `key: string | null;`
 * 或 `key?: string | null;`；两种形态都禁止声明里再混入 number（类型谎言回潮）。
 */
function expectTsString(body: string, key: string, nullable: boolean, fileLabel: string): void {
  const typeText = nullable ? 'string \\| null' : 'string';
  const re = new RegExp(`^  ${key}\\??: ${typeText};$`, 'm');
  expect(
    body,
    `${fileLabel} 接口字段 ${key} 应声明为 ${nullable ? 'string | null' : 'string'}`
  ).toMatch(re);
  const bad = new RegExp(`^  ${key}\\??:[^;]*number[^;]*;$`, 'm');
  expect(body, `${fileLabel} 接口字段 ${key} 声明混入 number（类型谎言回潮）`).not.toMatch(bad);
}

interface DecimalLock {
  label: string;
  feSrcText: string;
  feInterface: string;
  beSrcText: string;
  beFileLabel: string;
  beMarker: string;
  /** 后端非 Option Decimal（出参裸 string） */
  required: string[];
  /** 后端 Option<Decimal>（出参 string | null） */
  nullable: string[];
}

const LOCKS: DecimalLock[] = [
  {
    label: 'StockResponse（GET/PUT/POST /inventory/stock 出参）',
    feSrcText: INVENTORY_SRC,
    feInterface: 'StockResponse',
    beSrcText: STOCK_DTO_RS,
    beFileLabel: 'inventory_stock_handler_dto.rs',
    beMarker: 'pub struct StockResponse {',
    required: [
      'quantity_on_hand',
      'quantity_available',
      'quantity_reserved',
      'reorder_point',
      'max_stock_point',
    ],
    nullable: [],
  },
  {
    label: 'StockFabricResponse（/inventory/stock/fabric 出参）',
    feSrcText: INVENTORY_SRC,
    feInterface: 'StockFabricResponse',
    beSrcText: STOCK_DTO_RS,
    beFileLabel: 'inventory_stock_handler_dto.rs',
    beMarker: 'pub struct StockFabricResponse {',
    required: [
      'quantity_on_hand',
      'quantity_available',
      'quantity_reserved',
      'quantity_meters',
      'quantity_kg',
    ],
    nullable: ['gram_weight', 'width'],
  },
  {
    label: 'TransactionResponse（出入库流水出参）',
    feSrcText: INVENTORY_SRC,
    feInterface: 'TransactionResponse',
    beSrcText: STOCK_DTO_RS,
    beFileLabel: 'inventory_stock_handler_dto.rs',
    beMarker: 'pub struct TransactionResponse {',
    required: [
      'quantity_meters',
      'quantity_kg',
      'quantity_before_meters',
      'quantity_before_kg',
      'quantity_after_meters',
      'quantity_after_kg',
    ],
    nullable: [],
  },
  {
    label: 'InventorySummaryRow（/inventory/stock/summary 出参）',
    feSrcText: INVENTORY_SRC,
    feInterface: 'InventorySummaryRow',
    beSrcText: STOCK_DTO_RS,
    beFileLabel: 'inventory_stock_handler_dto.rs',
    beMarker: 'pub struct InventorySummaryItem {',
    required: ['total_quantity_meters', 'total_quantity_kg'],
    nullable: [],
  },
  {
    label: 'ProductionOrder（生产订单出参）',
    feSrcText: PRODUCTION_SRC,
    feInterface: 'ProductionOrder',
    beSrcText: PROD_ORDER_HANDLER_RS,
    beFileLabel: 'production_order_handler.rs',
    beMarker: 'pub struct ProductionOrderResponse {',
    required: ['planned_quantity'],
    nullable: ['actual_quantity'],
  },
  {
    label: 'ProductionRecipe（大货处方出参=模型直序列化）',
    feSrcText: RECIPE_SRC,
    feInterface: 'ProductionRecipe',
    beSrcText: RECIPE_MODEL_RS,
    beFileLabel: 'production_recipe.rs',
    beMarker: 'pub struct Model {',
    required: ['fabric_weight'],
    nullable: [
      'fabric_width',
      'gram_weight',
      'bath_volume',
      'adjustment_factor',
      'total_dye_cost',
      'total_auxiliary_cost',
    ],
  },
  {
    label: 'RecipeMaterialItem（处方明细出参）',
    feSrcText: RECIPE_SRC,
    feInterface: 'RecipeMaterialItem',
    beSrcText: RECIPE_MODEL_RS,
    beFileLabel: 'production_recipe.rs',
    beMarker: 'pub struct RecipeMaterialItem {',
    required: ['amount'],
    nullable: ['concentration'],
  },
  {
    label: 'OutsourcingOrder（委外订单出参=模型直序列化）',
    feSrcText: OUTSOURCING_SRC,
    feInterface: 'OutsourcingOrder',
    beSrcText: OUTSORDER_MODEL_RS,
    beFileLabel: 'outsourcing_order.rs',
    beMarker: 'pub struct Model {',
    required: [
      'issue_quantity',
      'return_quantity',
      'loss_quantity',
      'material_cost',
      'processing_fee',
      'freight_fee',
      'tax_amount',
      'abnormal_loss_amount',
      'total_cost',
      'unit_cost',
    ],
    nullable: ['loss_rate', 'standard_loss_rate'],
  },
  {
    label: 'OutsourcingOrderItem（委外发料明细出参）',
    feSrcText: OUTSOURCING_SRC,
    feInterface: 'OutsourcingOrderItem',
    beSrcText: OUTSITEM_MODEL_RS,
    beFileLabel: 'outsourcing_order_item.rs',
    beMarker: 'pub struct Model {',
    required: ['quantity', 'unit_cost', 'total_cost', 'processing_fee', 'freight_fee'],
    nullable: [],
  },
  {
    label: 'OutsourcingReceipt（委外收回单出参）',
    feSrcText: OUTSOURCING_SRC,
    feInterface: 'OutsourcingReceipt',
    beSrcText: OUTSRECEIPT_MODEL_RS,
    beFileLabel: 'outsourcing_receipt.rs',
    beMarker: 'pub struct Model {',
    required: [
      'return_quantity',
      'loss_quantity',
      'unit_cost',
      'total_cost',
      'abnormal_loss_amount',
    ],
    nullable: ['loss_rate', 'weight', 'width', 'gram_weight'],
  },
];

describe('Decimal=字符串出参形状锁（后端 Rust 声明 ↔ 前端 TS 声明 ↔ 钉死清单）', () => {
  it('rust_decimal serde 前提：仅启用 serde feature（未启用 serde-float）⇒ 序列化为字符串', () => {
    const m = CARGO_TOML.match(/^rust_decimal = \{[^\n]*\}$/m);
    if (!m) {
      throw new Error('backend/Cargo.toml 未找到 rust_decimal 声明行——serde 字符串前提锁失效');
    }
    expect(m[0]).toContain('features = ["serde"]');
    expect(m[0]).not.toContain('serde-float');
  });

  it.each(LOCKS)('$label', lock => {
    const feBody = tsInterfaceBody(lock.feSrcText, lock.feInterface, `api/${lock.feInterface}`);
    const beBody = rustStructBody(lock.beSrcText, lock.beMarker, lock.beFileLabel);
    for (const key of lock.required) {
      expectRustDecimal(beBody, key, lock.beFileLabel);
      expectTsString(feBody, key, false, lock.feInterface);
    }
    for (const key of lock.nullable) {
      expect(beBody, `${lock.beFileLabel} 字段 ${key} 应为 Option<Decimal}`).toMatch(
        new RegExp(`pub ${key}: Option<(rust_decimal::)?Decimal>`)
      );
      expectTsString(feBody, key, true, lock.feInterface);
    }
  });

  it('请求/响应共用类型已拆分：createProductionOrder 不再复用出参类型', () => {
    expect(PRODUCTION_SRC).toMatch(
      /export function createProductionOrder\(\s*data: CreateProductionOrderPayload/
    );
    expect(PRODUCTION_SRC).not.toContain('Partial<ProductionOrder>');
    // 创建载荷里 Decimal 入参按 number 提交（rust_decimal 反序列化接受 JSON number）
    const payloadBody = tsInterfaceBody(
      PRODUCTION_SRC,
      'CreateProductionOrderPayload',
      'production.ts'
    );
    expect(payloadBody).toMatch(/^ {2}planned_quantity\??: number;$/m);
  });

  it('请求/响应共用类型已拆分：处方明细入参独立成 RecipeMaterialItemInput', () => {
    const inputBody = tsInterfaceBody(
      RECIPE_SRC,
      'RecipeMaterialItemInput',
      'production-recipe.ts'
    );
    expect(inputBody).toMatch(/^ {2}amount: number;$/m);
    expect(inputBody).toMatch(/^ {2}concentration\??: number \| null;$/m);
    expect(RECIPE_SRC).toMatch(/recipe_detail\?: RecipeMaterialItemInput\[\];/);
    expect(RECIPE_SRC).toMatch(/items: RecipeMaterialItemInput\[\];/);
    expect(RECIPE_SRC).not.toMatch(/recipe_detail\?: RecipeMaterialItem\[\];/);
  });

  it('检测力自证：把钉死字段回退成 number/f64 的夹具必被上述锁判红', () => {
    // 前端侧夹具：planned_quantity 回退成 number —— string 锁与 anti-number 锁都须失败
    const mutatedFe = PRODUCTION_SRC.replace(
      '  planned_quantity: string;',
      '  planned_quantity: number;'
    );
    expect(mutatedFe).not.toBe(PRODUCTION_SRC); // 夹具替换本身必须生效
    const feBody = tsInterfaceBody(mutatedFe, 'ProductionOrder', 'production.ts');
    expect(feBody).not.toMatch(/^ {2}planned_quantity: string;$/m);
    expect(feBody).toMatch(/^ {2}planned_quantity: number;$/m);
    // 后端侧夹具：ProductionOrderResponse 的 Decimal 改 f64 —— 后端声明锁须失败。
    // 注意只在出参结构体内替换：CreateProductionOrderPayload 里同名声明不参与本锁，
    // 全文 String.replace 只会命中最先出现的一处，夹具必须打在正确的结构上。
    const respBody = rustStructBody(
      PROD_ORDER_HANDLER_RS,
      'pub struct ProductionOrderResponse {',
      'production_order_handler.rs'
    );
    const mutatedRespBody = respBody.replace(
      'pub planned_quantity: Decimal,',
      'pub planned_quantity: f64,'
    );
    expect(mutatedRespBody).not.toBe(respBody); // 夹具替换本身必须生效
    expect(mutatedRespBody).not.toMatch(/pub planned_quantity: (Option<)?(rust_decimal::)?Decimal/);
    // 联盟夹具：`string | number` 混形态（掩盖谎言的旧声明）也须被 anti-number 锁抓住
    const mutatedUnion = RECIPE_SRC.replace(
      '  fabric_weight: string;',
      '  fabric_weight: string | number;'
    );
    expect(mutatedUnion).not.toBe(RECIPE_SRC);
    const unionBody = tsInterfaceBody(mutatedUnion, 'ProductionRecipe', 'production-recipe.ts');
    expect(unionBody).toMatch(/^ {2}fabric_weight: string \| number;$/m);
    expect(unionBody).not.toMatch(/^ {2}fabric_weight: string;$/m);
  });
});
