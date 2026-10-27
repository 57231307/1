/**
 * 供应商 360 入口形状锁（后端查询参数载体 ↔ 候选数据源实发参数 ↔ 视图控件绑定三向互证）。
 * 功能：
 * 1) 钉住供应商入口的已选主键绑定在远程搜索下拉上，界面上不再出现要求手填数字主键的输入项；
 *    页内评估打分仍是数字输入（打分本就是数值），因此判据只认主键绑定，不按控件类型全文禁字段；
 * 2) 钉住候选数据源实发的键名与后端 SupplierQueryParams 逐键一致，且单页取值不超过后端
 *    page_size 的 clamp 上界，退化成 500 或 1000 即被判据抓住（本仓已知的静默截断家族缺陷）；
 * 3) 钉住候选与加载状态按当前关键字独立持有：换关键字先丢弃上一批候选，后发请求为代表，
 *    旧关键字的响应与失败一律不得改写新关键字的状态；
 * 4) 钉住取数失败既留日志又给界面提示，以及选定后带出的派生字段只读呈现。
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
 * 本锁的字面量夹具与逐行正则对行尾形态敏感，不归一会让检测力自证被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const ENHANCED_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/supplier/enhanced/index.vue')
);
const PICKER_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/composables/useSupplierPicker.ts'));
const API_SUPPLIER_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/supplier.ts'));
const SUPPLIER_SERVICE_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/supplier_service.rs')
);

/** 后端 supplier_service.rs 对列表单页的上界，选择器取值不得超过它 */
const BACKEND_PAGE_SIZE_CAP = 100;

/**
 * 被禁的步进数字录入控件标签名，只供检测力自证的夹具 A 构造「退回手填数字主键」的假源码。
 * 分段拼接避免本锁源码自身携带这个字面量，口径与静态判定器一致。
 */
const FORBIDDEN_STEPPER_TAG = ['el-input-', 'number'].join('');

/** 截取 Rust struct 正文；结构体改名或删除即抛错（防判据静默失去覆盖） */
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

/** 提取 Rust struct 字段名（该结构无 rename_all，序列化键名即字段名） */
function rustStructFields(body: string): string[] {
  return [...body.matchAll(/pub\s+([a-z_][a-z0-9_]*)\s*:/g)].map(m => m[1]);
}

/** 截取 TS interface 正文；接口被删除或改名即抛错（防判据静默失效） */
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

/** 提取 TS 接口块内声明的键 */
function tsInterfaceKeys(src: string, name: string): string[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}(?:readonly\s+)?([a-z][A-Za-z0-9_]*)\??\s*:/gm)].map(m => m[1]);
}

/** 提取候选数据源向列表端点实发的参数对象键 */
function sentQueryKeys(src: string): string[] {
  const call = src.match(/getSupplierList\(\{([\s\S]*?)\}\)/);
  if (call === null) {
    throw new Error('候选数据源里找不到向列表端点实发的参数对象——调用形态已漂移');
  }
  return [...call[1].matchAll(/^\s*([a-z_][a-z0-9_]*)\s*:/gm)].map(m => m[1]);
}

/** 提取候选单页取值字面量 */
function pickerPageSize(src: string): number {
  const hit = src.match(/const PICKER_PAGE_SIZE = (\d+);/);
  if (hit === null) {
    throw new Error('候选单页常量不存在或改名——page_size 判据失去抓手');
  }
  return Number(hit[1]);
}

/** 视图上绑定供应商主键的控件标签名；判据只认这一处绑定，不禁页内其它数字输入 */
function supplierIdBoundTag(src: string): string {
  const hit = src.match(/<([a-z][a-z0-9-]*)[^>]*?v-model="supplierId"/);
  if (hit === null) {
    throw new Error('视图里没有任何控件绑定 supplierId——主键入口形态已漂移');
  }
  return hit[1];
}

/** 取绑定主键的那个控件开标签全文，用于核它带的是远程搜索属性而不是本地过滤属性 */
function supplierIdTagOpen(src: string): string {
  const hit = src.match(/<[a-z][a-z0-9-]*[^>]*?v-model="supplierId"[^>]*>/);
  if (hit === null) {
    throw new Error('绑定 supplierId 的控件开标签解析失败——判据失去抓手');
  }
  return hit[0];
}

/** 绑定主键的控件是否真的是按关键字向服务端取候选的远程搜索下拉（本地过滤即判红） */
function isRemoteSearchPicker(tag: string): boolean {
  return (
    tag.includes('filterable') &&
    /(^|\s)remote(\s|=|>)/.test(tag) &&
    tag.includes(':remote-method="searchSuppliers"') &&
    tag.includes(':loading="pickerLoading"') &&
    !tag.includes(':filter-method=')
  );
}

/** 候选与加载状态按当前关键字独立持有 */
function holdsStatePerKeyword(src: string): boolean {
  const hasSeqCounter = /const seq = \+\+searchSeq;/.test(src);
  const dropsPreviousCandidates = /candidates\.value = \[\];\s*\n\s*loading\.value = true;/.test(
    src
  );
  const guardedSuccess =
    /if \(seq !== searchSeq\) return;\s*\n\s*candidates\.value = res\.data\.items;/.test(src);
  const guardedFailure = /if \(seq !== searchSeq\) return;\s*\n\s*candidates\.value = \[\];/.test(
    src
  );
  const loadingReleasedByLatest = /if \(seq === searchSeq\) loading\.value = false;/.test(src);
  return (
    hasSeqCounter &&
    dropsPreviousCandidates &&
    guardedSuccess &&
    guardedFailure &&
    loadingReleasedByLatest
  );
}

/** 实发参数键是否与前端声明、后端载体逐键对齐，且三个必要键都在场 */
function sentKeysAlign(sent: string[], declared: string[], backend: string[]): boolean {
  return (
    ['page', 'page_size', 'keyword'].every(key => sent.includes(key)) &&
    sent.every(key => declared.includes(key) && backend.includes(key))
  );
}

/** 候选取数失败是否既留日志又给界面提示（静默失败判红） */
function failureIsVisible(src: string): boolean {
  return /logAuxLoadFailure\(/.test(src) && /msg\.error\(/.test(src);
}

const backendKeys = rustStructFields(rustStructBody(SUPPLIER_SERVICE_SRC, 'SupplierQueryParams'));
const declaredKeys = tsInterfaceKeys(API_SUPPLIER_SRC, 'SupplierQueryParams');
const sentKeys = sentQueryKeys(PICKER_SRC);

describe('供应商 360 入口选择器形状锁（后端载体 ↔ 候选数据源 ↔ 视图绑定）', () => {
  it('后端事实基准：keyword 与 page、page_size 是列表查询的真实键，单页确有 clamp 上界', () => {
    expect(backendKeys.length).toBeGreaterThanOrEqual(10);
    for (const key of ['page', 'page_size', 'keyword']) {
      expect(backendKeys, `后端 SupplierQueryParams 应含键 ${key}`).toContain(key);
    }
    // clamp 现场：超过上界的取值会被静默削短，本锁第 2 条据此设上界
    expect(SUPPLIER_SERVICE_SRC).toMatch(/page_size\.unwrap_or\(20\)\.clamp\(1, 100\)/);
  });

  it('实发参数键与后端逐键一致，单页取值不超过 clamp 上界', () => {
    expect(sentKeysAlign(sentKeys, declaredKeys, backendKeys)).toBe(true);
    const size = pickerPageSize(PICKER_SRC);
    expect(size).toBeGreaterThan(0);
    expect(size).toBeLessThanOrEqual(BACKEND_PAGE_SIZE_CAP);
  });

  it('视图入口是远程搜索下拉，手填数字主键的输入项已消失', () => {
    expect(supplierIdBoundTag(ENHANCED_SRC)).toBe('el-select');
    expect(isRemoteSearchPicker(supplierIdTagOpen(ENHANCED_SRC))).toBe(true);
    // 主键入口的文案键不得回流
    expect(ENHANCED_SRC).not.toContain('supplier.enhanced.idPlaceholder');
  });

  it('候选与加载状态按当前关键字独立持有，旧响应不得改写新关键字的状态', () => {
    expect(holdsStatePerKeyword(PICKER_SRC)).toBe(true);
  });

  it('取数失败既留日志又给界面提示，不静默；声明侧不把后端必出键写成可选', () => {
    expect(failureIsVisible(PICKER_SRC)).toBe(true);
    // 候选行的名称与编码是后端 NOT NULL 列，前端声明不得标可选来掩盖缺键
    const supplierBody = tsInterfaceBody(API_SUPPLIER_SRC, 'Supplier');
    expect(supplierBody).toMatch(/^ {2}supplier_name: string;$/m);
    expect(supplierBody).toMatch(/^ {2}supplier_code: string;$/m);
  });

  it('选定后带出的派生字段只读呈现，供应商主键以文本而不是输入项出现', () => {
    expect(ENHANCED_SRC).not.toMatch(/v-model="supplier\./);
    const idItem = ENHANCED_SRC.match(
      /<el-descriptions-item :label="t\('supplier\.enhanced\.descriptions\.id'\)">\s*\n\s*\{\{ supplier\.id \}\}/
    );
    expect(idItem, '供应商主键必须在派生信息区以只读文本呈现').not.toBeNull();
  });

  it('检测力自证：退回手填输入框、放大单页、换键名与改本地过滤都必被判据抓到', () => {
    // 夹具 A：把选择器退回手填数字主键输入框，主键绑定判据必须抓到
    const revertedToManualInput = ENHANCED_SRC.replace(
      '<el-select\n            v-model="supplierId"',
      `<${FORBIDDEN_STEPPER_TAG}\n            v-model="supplierId"`
    );
    expect(revertedToManualInput, '夹具替换本身必须生效').not.toBe(ENHANCED_SRC);
    expect(supplierIdBoundTag(revertedToManualInput)).not.toBe('el-select');
    // 夹具 B：把单页取值放大到后端 clamp 之外，上界判据必须抓到
    const oversizedPage = PICKER_SRC.replace(
      'const PICKER_PAGE_SIZE = 50;',
      'const PICKER_PAGE_SIZE = 500;'
    );
    expect(oversizedPage).not.toBe(PICKER_SRC);
    expect(pickerPageSize(oversizedPage)).toBeGreaterThan(BACKEND_PAGE_SIZE_CAP);
    // 夹具 C：把关键字键名改成后端不接受的写法，逐键一致判据必须抓到
    const renamedKeyword = PICKER_SRC.replace(
      'keyword: keyword.trim(),',
      'search_text: keyword.trim(),'
    );
    expect(renamedKeyword).not.toBe(PICKER_SRC);
    expect(backendKeys, '夹具用的键后端确实从不接受，否则对齐判据是空转').not.toContain(
      'search_text'
    );
    expect(sentKeysAlign(sentQueryKeys(renamedKeyword), declaredKeys, backendKeys)).toBe(false);
    // 夹具 D：把远程搜索退化成本地过滤，控件绑定判据必须抓到
    const localFilter = ENHANCED_SRC.replace(
      '            filterable\n            remote\n            clearable\n            :remote-method="searchSuppliers"',
      '            filterable\n            clearable\n            :filter-method="filterSuppliers"'
    );
    expect(localFilter).not.toBe(ENHANCED_SRC);
    expect(isRemoteSearchPicker(supplierIdTagOpen(localFilter))).toBe(false);
    // 夹具 E：去掉序号守卫（竞态回流），独立持有判据必须抓到
    const unguarded = PICKER_SRC.replace(
      'if (seq !== searchSeq) return;\n      candidates.value = res.data.items;',
      'candidates.value = res.data.items;'
    );
    expect(unguarded).not.toBe(PICKER_SRC);
    expect(holdsStatePerKeyword(unguarded)).toBe(false);
    // 夹具 F：把失败提示删成静默，失败可见判据必须抓到
    const silenced = PICKER_SRC.replace("      msg.error('loadFailed');\n", '');
    expect(silenced).not.toBe(PICKER_SRC);
    expect(failureIsVisible(silenced)).toBe(false);
    // 反向自证：后端结构块改名时解析必须抛错而不是静默空集
    expect(() => rustStructFields(rustStructBody('pub Other {}', 'SupplierQueryParams'))).toThrow();
    expect(() => sentQueryKeys('const x = 1;')).toThrow();
    expect(() => pickerPageSize('export const nothing = 1;')).toThrow();
  });
});
