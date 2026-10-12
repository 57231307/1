/**
 * BI 订单钻取形状锁（后端 json! 构造点 ↔ 前端声明 ↔ 消费绑定三向互证）。
 * 功能：
 * 1) 钻取端点（客户→订单 / 产品→订单）的内层载荷在后端
 *    services/bi_analysis_ops/drilldown.rs 以 serde_json::json! 构造，外层是对象
 *    （含 *_id 与 orders 两键）、不是裸数组——消费方必须读 .orders，
 *    把 unwrapBi 结果直接当数组绑定会使钻取表恒不渲染；
 * 2) 前端 bi.ts 的行声明键集必须 ⊆ 对应构造点的真实键集，
 *    且不得回流后端从不输出的历史幽灵键（order_no/order_date/total_amount/status）；
 * 3) 金额/数量经后端 dec_to_f64 出 JSON number、date 为字符串，前端类型逐键钉死；
 * 4) 注入违例夹具必被上述判据抓到（防判据静默失效）。
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
 * 本锁的正则字面量对行尾形态敏感，不归一会让检测力自证被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const BI_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/bi.ts'));
const SALES_VIEW_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/views/bi/SalesAnalysis.vue'));
const DRILLDOWN_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/bi_analysis_ops/drilldown.rs')
);

/** 后端从不输出、前端历史上声明过的幽灵键：任何一侧回流即判红 */
const GHOST_ROW_KEYS = ['order_no', 'order_date', 'total_amount', 'status'] as const;

/** 截取 Rust 公开方法体（从签名到下一个公开方法/文件末尾）；签名不存在即抛错（防判据静默失效） */
function rustFnBody(src: string, fnName: string, nextFnName: string | null): string {
  const start = src.indexOf(`pub async fn ${fnName}(`);
  if (start === -1) {
    throw new Error(`后端源码中未找到 pub async fn ${fnName}——构造点已漂移，锁需同步修订`);
  }
  const end = nextFnName
    ? src.indexOf(`pub async fn ${nextFnName}(`, start + 1)
    : src.lastIndexOf('}');
  if (end === -1) {
    throw new Error(`方法 ${fnName} 的边界无法定位，无法解析`);
  }
  return src.slice(start, end);
}

/** 提取方法体内每个 serde_json::json!({...}) 块的顶层键名（按出现顺序） */
function rustJsonBlockKeys(body: string): string[][] {
  const blocks = [...body.matchAll(/serde_json::json!\(\{([\s\S]*?)\}\)/g)].map(m => m[1]);
  if (blocks.length < 2) {
    throw new Error(`方法体内 json! 构造块少于 2 个（行块+外层信封块）——构造点已漂移`);
  }
  return blocks.map(b => [...b.matchAll(/"([a-z_]+)"\s*:/g)].map(m => m[1]));
}

/** 截取 TS interface 正文；接口被删除或改名即抛错（防判据静默失去覆盖） */
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

/** 提取 TS 接口块内的声明键（含索引签名存在性检测：本锁禁止行接口带索引签名） */
function tsInterfaceKeys(src: string, name: string): string[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}(?:readonly\s+)?([a-z][A-Za-z0-9_]*)\??\s*:/gm)].map(m => m[1]);
}

const customerFnBody = rustFnBody(
  DRILLDOWN_SRC,
  'drilldown_customer_to_order',
  'drilldown_product_to_order'
);
const productFnBody = rustFnBody(DRILLDOWN_SRC, 'drilldown_product_to_order', null);
const [customerRowKeys, customerOuterKeys] = rustJsonBlockKeys(customerFnBody);
const [productRowKeys, productOuterKeys] = rustJsonBlockKeys(productFnBody);

const declaredCustomerRowKeys = tsInterfaceKeys(BI_API_SRC, 'DrilldownCustomerOrderItem');
const declaredProductRowKeys = tsInterfaceKeys(BI_API_SRC, 'DrilldownProductOrderItem');
const declaredCustomerPayloadKeys = tsInterfaceKeys(BI_API_SRC, 'CustomerOrderDrilldown');
const declaredProductPayloadKeys = tsInterfaceKeys(BI_API_SRC, 'ProductOrderDrilldown');

describe('BI 订单钻取形状锁（后端构造点 ↔ 前端声明 ↔ 消费绑定）', () => {
  it('后端事实基准：行键集与外层对象键集按 json! 构造点原文钉死，外层含 orders', () => {
    // 键数地板：解析失效或构造点缩水判红
    expect(customerRowKeys.length).toBeGreaterThanOrEqual(3);
    expect(productRowKeys.length).toBeGreaterThanOrEqual(3);
    // 行键集 = 构造点原文（精确相等：两侧任何单边走偏都要显式修订本锁）
    expect([...customerRowKeys].sort()).toEqual(['amount', 'date', 'order_id']);
    expect([...productRowKeys].sort()).toEqual(['amount', 'order_id', 'quantity']);
    // 外层信封是对象：两键（*_id + orders），不是裸数组
    expect([...customerOuterKeys].sort()).toEqual(['customer_id', 'orders']);
    expect([...productOuterKeys].sort()).toEqual(['orders', 'product_id']);
    // 金额/数量出 JSON number 的载体证据：dec_to_f64；date 出字符串的载体证据：format("%Y-%m-%d")
    expect(customerFnBody).toContain('dec_to_f64(r.amount)');
    expect(customerFnBody).toContain('d.format("%Y-%m-%d")');
    expect(productFnBody).toContain('dec_to_f64(r.quantity)');
    expect(productFnBody).toContain('dec_to_f64(r.amount)');
  });

  it('前端声明：行/载荷键集 ⊆ 后端构造点键集，且零幽灵键、零索引签名', () => {
    expect(declaredCustomerRowKeys.length).toBeGreaterThanOrEqual(3);
    expect(declaredProductRowKeys.length).toBeGreaterThanOrEqual(3);
    for (const key of declaredCustomerRowKeys) {
      expect(customerRowKeys, `DrilldownCustomerOrderItem 声明了后端不输出的键 ${key}`).toContain(
        key
      );
    }
    for (const key of declaredProductRowKeys) {
      expect(productRowKeys, `DrilldownProductOrderItem 声明了后端不输出的键 ${key}`).toContain(
        key
      );
    }
    // 后端真实键必须全部有声明（漏声明=列缺失，同样判红）
    for (const key of customerRowKeys) {
      expect(declaredCustomerRowKeys, `前端漏声明后端真实键 ${key}`).toContain(key);
    }
    for (const key of productRowKeys) {
      expect(declaredProductRowKeys, `前端漏声明后端真实键 ${key}`).toContain(key);
    }
    // 幽灵键双侧都不许出现
    for (const key of GHOST_ROW_KEYS) {
      expect(customerRowKeys.join(',')).not.toContain(key);
      expect(declaredCustomerRowKeys).not.toContain(key);
      expect(declaredProductRowKeys).not.toContain(key);
    }
    // 载荷声明键集 ⊆ 外层构造点键集，且必须声明 orders（外层是对象⇒只能经 .orders 取数组）
    for (const key of declaredCustomerPayloadKeys) {
      expect(customerOuterKeys, `CustomerOrderDrilldown 多出外层不存在的键 ${key}`).toContain(key);
    }
    expect(declaredCustomerPayloadKeys).toContain('orders');
    for (const key of declaredProductPayloadKeys) {
      expect(productOuterKeys, `ProductOrderDrilldown 多出外层不存在的键 ${key}`).toContain(key);
    }
    expect(declaredProductPayloadKeys).toContain('orders');
    // 行接口不得带索引签名（[key: string]: unknown 会把键集失配静默吞掉）
    expect(tsInterfaceBody(BI_API_SRC, 'DrilldownCustomerOrderItem')).not.toContain(
      '[key: string]'
    );
    expect(tsInterfaceBody(BI_API_SRC, 'DrilldownProductOrderItem')).not.toContain('[key: string]');
    // 类型逐键钉死：dec_to_f64 出数 ⇒ number；date 为字符串
    expect(tsInterfaceBody(BI_API_SRC, 'DrilldownCustomerOrderItem')).toMatch(
      /^ {2}amount: number;$/m
    );
    expect(tsInterfaceBody(BI_API_SRC, 'DrilldownCustomerOrderItem')).toMatch(
      /^ {2}date: string;$/m
    );
    expect(tsInterfaceBody(BI_API_SRC, 'DrilldownProductOrderItem')).toMatch(
      /^ {2}quantity: number;$/m
    );
    expect(tsInterfaceBody(BI_API_SRC, 'DrilldownProductOrderItem')).toMatch(
      /^ {2}amount: number;$/m
    );
    // API 函数返回类型必须是对象载荷信封，不得回流数组形态声明
    expect(BI_API_SRC).toContain('BiEnvelope<CustomerOrderDrilldown>');
    expect(BI_API_SRC).toContain('BiEnvelope<ProductOrderDrilldown>');
    expect(BI_API_SRC).not.toMatch(/BiEnvelope<DrilldownOrderItem\[\]>/);
    expect(BI_API_SRC).not.toContain('DrilldownOrderItem');
  });

  it('消费绑定：钻取表数据源必须读 .orders，禁止双形状探测/兜底掩盖', () => {
    expect(SALES_VIEW_SRC).toContain(':data="customerOrderDrill.orders"');
    expect(SALES_VIEW_SRC).toContain(':data="productOrderDrill.orders"');
    // 钻取载荷不得被当数组直接绑定或赋值
    expect(SALES_VIEW_SRC).not.toMatch(/:data="customerOrderDrill(\.length)?"/);
    expect(SALES_VIEW_SRC).not.toMatch(/:data="productOrderDrill(\.length)?"/);
    // 本仓禁区：?? [] / Array.isArray 双形状探测兜底
    expect(SALES_VIEW_SRC).not.toMatch(/unwrapBi\([^)]*\)\s*\?\?\s*\[\]/);
    expect(SALES_VIEW_SRC).not.toMatch(/Array\.isArray\((customerOrderDrill|productOrderDrill)/);
    // 订单钻取列不得回流幽灵键绑定（total_amount 在时间序列表是合法键，单独按列数地板锁形）
    for (const key of ['order_no', 'order_date', 'status']) {
      expect(SALES_VIEW_SRC, `SalesAnalysis.vue 不得再绑定后端不输出的列 ${key}`).not.toContain(
        `prop="${key}"`
      );
    }
    // 订单钻取表列形定锁：order_id 两表同绑、amount 只出现在订单钻取表、date 仅客户表
    expect((SALES_VIEW_SRC.match(/prop="order_id"/g) || []).length).toBe(2);
    expect((SALES_VIEW_SRC.match(/prop="amount"/g) || []).length).toBe(2);
    expect((SALES_VIEW_SRC.match(/prop="date"/g) || []).length).toBe(1);
  });

  it('检测力自证：复现历史失配形态的夹具必被上述判据抓到（防判据静默失效）', () => {
    // 夹具 A：把幽灵键 order_no 加回行声明，子集判据必须抓到
    const mutatedRow = BI_API_SRC.replace(
      '  amount: number;\n  date: string;',
      '  amount: number;\n  order_no: string;\n  date: string;'
    );
    expect(mutatedRow, '夹具替换本身必须生效').not.toBe(BI_API_SRC);
    const mutatedKeys = tsInterfaceKeys(mutatedRow, 'DrilldownCustomerOrderItem');
    expect(mutatedKeys.filter(k => !customerRowKeys.includes(k))).toEqual(['order_no']);
    // 夹具 B：把载荷接口改回数组消费形态（删掉 orders 声明），载荷判据必须抓到
    const mutatedPayload = BI_API_SRC.replace(
      '  orders: DrilldownCustomerOrderItem[];',
      '  items: DrilldownCustomerOrderItem[];'
    );
    expect(mutatedPayload, '夹具替换本身必须生效').not.toBe(BI_API_SRC);
    expect(tsInterfaceKeys(mutatedPayload, 'CustomerOrderDrilldown')).not.toContain('orders');
    // 夹具 C：视图把载荷直接当数组绑定，消费判据必须抓到
    const mutatedView = SALES_VIEW_SRC.replace(
      ':data="customerOrderDrill.orders"',
      ':data="customerOrderDrill"'
    );
    expect(mutatedView, '夹具替换本身必须生效').not.toBe(SALES_VIEW_SRC);
    expect(/:data="customerOrderDrill(\.length)?"/.test(mutatedView)).toBe(true);
    // 夹具 D：消费侧回流 ?? [] 兜底，禁区判据必须抓到
    const mutatedFallback = SALES_VIEW_SRC.replace(
      'customerOrderDrill.value = unwrapBi(res);',
      'customerOrderDrill.value = unwrapBi(res) ?? [];'
    );
    expect(mutatedFallback, '夹具替换本身必须生效').not.toBe(SALES_VIEW_SRC);
    expect(/unwrapBi\([^)]*\)\s*\?\?\s*\[\]/.test(mutatedFallback)).toBe(true);
    // 反向自证：后端构造点整段缺失时解析必须抛错而非静默空集
    expect(() => rustFnBody('fn nothing() {}', 'drilldown_customer_to_order', null)).toThrow();
    expect(() => rustJsonBlockKeys('serde_json::json!({ "only": 1 });')).toThrow();
  });
});
