#!/usr/bin/env node
/**
 * check-api-request-fixture-selftest.mjs —— 请求契约解析器「四类盲区」fixture 自测
 *
 * 目的：check-api-request.mjs 的历史问题不是「报错太少」，而是**解析不到时静默跳过**，
 * 于是「前端提交键名与后端 DTO 不一致 → 用户看到参数错误」整类缺陷穿过门禁。
 * 本自测用内联 fixture（不依赖真实前后端仓库源码、不跑任何服务）逐类验证：
 *   (a) 箭头函数形式请求签名 —— sig 能回填、形参类型能解析、失配能被抓；
 *   (b) Partial<T> 包装 —— 键集展开为 T 的全集且置可选；后端多余/缺失仍照判；
 *   (c) 交叉类型 A & B —— 两侧键集合并参与比对（禁止半解析）；一员不可解析则整条显式盲区；
 *   (d) 叶子类型（serde_json::Value / any / Record<string, unknown> / 索引签名 / 不可解析 config）
 *       —— 必须显式返回 blind/叶子标记，绝不允许无声当作通过。
 * 任何一条断言失败 -> 退出码 1（本脚本已接入 CI ci-static-checks，防解析器回归）。
 */
import {
  CFG_BLIND,
  compareKeys,
  extractAxiosConfigValue,
  feKeysOf,
  leafTypeText,
  paramTypeOf,
  resolveTsTypeExpr,
  rustFieldsOf,
  splitTopLevelAmp,
  tsFieldsOf,
} from './check-api-request.mjs';
import { sigForSymbol } from './check-api-envelope.mjs';

let pass = 0;
let fail = 0;
const ok = (cond, name, detail) => {
  if (cond) {
    pass++;
    console.log(`  PASS  ${name}${detail ? '  [' + detail + ']' : ''}`);
  } else {
    fail++;
    console.log(`  FAIL  ${name}${detail ? '  -> ' + detail : ''}`);
  }
};

// ---------- fixture：具名 TS 类型索引（与 buildTsTypeIndex 同构的 Map 条目） ----------
const tsIndex = new Map([
  [
    'CreateWidgetDto',
    { kind: 'object', body: 'name: string; qty: number', extends: [], file: 'fixture' },
  ],
  ['NameDto', { kind: 'object', body: 'name: string', extends: [], file: 'fixture' }],
  ['NoteDto', { kind: 'object', body: 'note: string', extends: [], file: 'fixture' }],
  [
    'WidgetDto',
    {
      kind: 'object',
      body: 'name: string; qty: number; ghostKey: string',
      extends: [],
      file: 'fixture',
    },
  ],
  [
    'MapDto',
    {
      kind: 'object',
      body: 'rows: Record<string, unknown>; total: number',
      extends: [],
      file: 'fixture',
    },
  ],
  [
    'DynDto',
    { kind: 'object', body: 'page: number; [key: string]: unknown', extends: [], file: 'fixture' },
  ],
  // alias：交叉 + Partial（(b)(c) 的声明形态）
  ['CrossDto', { kind: 'alias', text: 'NameDto & NoteDto', file: 'fixture' }],
  ['BrokenCrossDto', { kind: 'alias', text: 'NameDto & GhostThing', file: 'fixture' }],
  ['WidgetPatch', { kind: 'alias', text: 'Partial<WidgetDto>', file: 'fixture' }],
]);

// ---------- fixture：Rust 结构体索引（与 buildStructIndex 同构的普通对象） ----------
const structIndex = {
  CreateWidgetPayload: {
    attrs: '',
    allFields: [
      { name: 'title', type: 'String' },
      { name: 'qty', type: 'i32' },
    ],
  },
  WidgetPayload: {
    attrs: '',
    allFields: [
      { name: 'name', type: 'String' },
      { name: 'qty', type: 'i32' },
    ],
  },
  NameOnlyPayload: { attrs: '', allFields: [{ name: 'name', type: 'String' }] },
  OrderPayload: {
    attrs: '',
    allFields: [
      { name: 'name', type: 'String' },
      { name: 'meta', type: 'serde_json::Value' },
    ],
  },
  StrictPayload: {
    attrs: '',
    allFields: [
      { name: 'name', type: 'String' },
      { name: 'qty', type: 'i32' },
      { name: 'note', type: 'String' },
    ],
  },
};

console.log('=== fixture 自测：check-api-request 四类盲区解析器 ===');

// ---------- (a) 箭头函数形式请求签名 ----------
console.log('\n[a] 箭头函数签名：sig 回填 + 形参类型解析 + 失配识别');
{
  const src = `export const createWidget = (data: CreateWidgetDto) =>
  request.post('/widgets', data);
`;
  const callIdx = src.indexOf('request.post');
  const sig = sigForSymbol(src, 0, callIdx);
  ok(sig === 'data: CreateWidgetDto', '(a1) sigForSymbol 回填箭头函数参数表', JSON.stringify(sig));
  const r = feKeysOf({ sig }, 'data', tsIndex);
  ok(
    r.kind === 'keys' && r.keys.map(k => k.name).join(',') === 'name,qty',
    '(a2) 箭头签名形参类型可解析为键集',
    JSON.stringify(r)
  );
  const cmp = compareKeys(
    r.kind === 'keys' ? r.keys : [],
    rustFieldsOf('CreateWidgetPayload', structIndex)
  );
  ok(
    cmp.status === 'mismatch' &&
      cmp.extra.join(',') === 'name' &&
      cmp.missingRequired.join(',') === 'title',
    '(a3) 键名漂移被识别为失配(前端 name ↔ 后端 title)',
    `extra=${(cmp.extra || []).join(',')} missingRequired=${(cmp.missingRequired || []).join(',')}`
  );
  // 多行内联对象注解（旧正则字符类容不下 `{`，曾整批落盲区）
  const sig2 = 'data: { period: string; product_id?: number }';
  const r2 = feKeysOf({ sig: sig2 }, 'data', tsIndex);
  ok(
    r2.kind === 'keys' &&
      r2.keys
        .map(k => k.name)
        .sort()
        .join(',') === 'period,product_id',
    '(a4) 内联对象类型注解可直接展开',
    JSON.stringify(r2.kind === 'keys' ? r2.keys.map(k => k.name) : r2)
  );
  ok(
    paramTypeOf('id: number, data?: WidgetPatch = {}', 'data').typeText === 'WidgetPatch',
    '(a5) 顶层默认值剥除后取到类型注解',
    JSON.stringify(paramTypeOf('id: number, data?: WidgetPatch = {}', 'data'))
  );
}

// ---------- (b) Partial<T> 包装 ----------
console.log('\n[b] Partial<T>：键集展开 + 可选化，后端漂移仍照判');
{
  const r = feKeysOf({ sig: 'data: Partial<WidgetDto>' }, 'data', tsIndex);
  ok(
    r.kind === 'keys' && r.keys.every(k => k.optional) && r.keys.length === 3,
    '(b1) Partial 展开为全集且全部可选',
    JSON.stringify(r.kind === 'keys' ? r.keys.map(k => k.name + (k.optional ? '?' : '')) : r)
  );
  const cmp = compareKeys(
    r.kind === 'keys' ? r.keys : [],
    rustFieldsOf('WidgetPayload', structIndex)
  );
  ok(
    cmp.status === 'mismatch' && cmp.extra.join(',') === 'ghostKey',
    '(b2) Partial 展开后多余键 ghostKey 仍被抓出',
    `extra=${(cmp.extra || []).join(',')}`
  );
  const r2 = feKeysOf({ sig: 'data: Partial<NameDto>' }, 'data', tsIndex);
  const cmp2 = compareKeys(
    r2.kind === 'keys' ? r2.keys : [],
    rustFieldsOf('StrictPayload', structIndex)
  );
  ok(
    cmp2.status === 'mismatch' && cmp2.missingRequired.join(',') === 'qty,note',
    '(b3) Partial 不给后端必填(qty/note) => 判缺必填，不被“可选”掩盖',
    `missingRequired=${(cmp2.missingRequired || []).join(',')}`
  );
  const r3 = feKeysOf({ sig: 'data: WidgetPatch' }, 'data', tsIndex); // alias: Partial<WidgetDto>
  ok(r3.kind === 'keys' && r3.keys.length === 3, '(b4) type X = Partial<T> alias 同样展开');
}

// ---------- (c) 交叉类型 A & B ----------
console.log('\n[c] 交叉类型：两侧键集必须合并；半解析/不可解析成员必须显式盲区');
{
  ok(
    splitTopLevelAmp('NameDto & NoteDto').join('|') === 'NameDto|NoteDto',
    '(c0) 顶层 & 切分',
    splitTopLevelAmp('NameDto & NoteDto').join('|')
  );
  ok(
    splitTopLevelAmp('Record<string, "a & b"> & NameDto').join('|') ===
      'Record<string, "a & b">|NameDto',
    '(c0b) 泛型/字符串内的 & 不误切'
  );
  const r = feKeysOf({ sig: 'data: NameDto & NoteDto' }, 'data', tsIndex);
  ok(
    r.kind === 'keys' &&
      r.keys
        .map(k => k.name)
        .sort()
        .join(',') === 'name,note',
    '(c1) A & B 键集完整合并（旧实现只截到 A，B 被静默丢弃）',
    JSON.stringify(r.kind === 'keys' ? r.keys.map(k => k.name) : r)
  );
  const cmp = compareKeys(
    r.kind === 'keys' ? r.keys : [],
    rustFieldsOf('NameOnlyPayload', structIndex)
  );
  ok(
    cmp.status === 'mismatch' && cmp.extra.join(',') === 'note',
    '(c2) 交叉类型多发的 note（后端不读）被抓出',
    `extra=${(cmp.extra || []).join(',')}`
  );
  const rb = feKeysOf({ sig: 'data: NameDto & GhostThing' }, 'data', tsIndex);
  ok(
    rb.kind === 'blind' && /GhostThing/.test(rb.why),
    '(c3) 一员不可解析 => 整条显式盲区并点名成员，禁止半解析后照常比对',
    rb.why
  );
  const ra = tsFieldsOf('CrossDto', tsIndex);
  ok(
    !!ra &&
      ra.fields
        .map(f => f.name)
        .sort()
        .join(',') === 'name,note',
    '(c4) alias 交叉展开'
  );
  const rba = tsFieldsOf('BrokenCrossDto', tsIndex);
  ok(rba === null, '(c5) alias 含不可解析成员 => 判 null 走盲区（不返回部分结果）');
  const r3 = feKeysOf({ sig: 'data: Omit<WidgetDto, "qty">' }, 'data', tsIndex);
  ok(
    r3.kind === 'blind' && /未处理的类型写法/.test(r3.why),
    '(c6) 未支持写法显式盲区（带原文），绝不静默 continue',
    r3.why
  );
}

// ---------- (d) 叶子类型 / 索引签名 / 不可解析 config ----------
console.log('\n[d] 叶子类型：可比部分照常比，不可比部分必须显式入清单');
{
  const fields = rustFieldsOf('OrderPayload', structIndex);
  ok(!!fields && fields.length === 2, '(d0) 后端字段类型文本保留（旧实现直接丢弃）');
  const meta = fields.find(f => f.name === 'meta');
  ok(
    meta && leafTypeText(meta.type) === 'serde_json::Value',
    '(d1) 后端 serde_json::Value 字段被标记为叶子',
    meta && meta.type
  );
  const r = feKeysOf({ sig: 'data: MapDto' }, 'data', tsIndex);
  ok(
    r.kind === 'keys' &&
      leafTypeText(r.keys.find(k => k.name === 'rows')?.typeText) === 'Record<...>',
    '(d2) 前端 Record<string, unknown> 值被标记为叶子（顶层名仍参与比对）',
    JSON.stringify(
      r.kind === 'keys' ? r.keys.map(k => k.name + ':' + (leafTypeText(k.typeText) || '-')) : r
    )
  );
  const rd = feKeysOf({ sig: 'data: DynDto' }, 'data', tsIndex);
  ok(
    rd.kind === 'blind' && /索引签名|无法静态展开/.test(rd.why),
    '(d3) 索引签名 [key: string]: X => 显式盲区（旧实现按值类型放行/静默跳过）',
    rd.why
  );
  ok(
    leafTypeText('any') === 'any' &&
      leafTypeText('Option<serde_json::Value>') === 'serde_json::Value',
    '(d4) any / Option<Value> 叶子判定'
  );
  const rl = resolveTsTypeExpr('Record<string, unknown>', tsIndex);
  ok(rl.blind && /叶子类型/.test(rl.blind), '(d5) Record 整体作载荷 => 盲区且写明叶子', rl.blind);
  // config 三态：确实没有(null) vs 读不懂(CFG_BLIND) —— 后者旧实现被当成前者（静默放行）
  ok(extractAxiosConfigValue('', 'params') === null, '(d6a) 无第二实参 => null(无载荷)');
  ok(
    extractAxiosConfigValue('{ ...q, params }', 'params') === CFG_BLIND,
    '(d6b) config 含前置展开运算符 => CFG_BLIND，不再伪装成“无 params”'
  );
  ok(
    extractAxiosConfigValue('{ params: { a }, ...rest }', 'params') === '{ a }',
    '(d6b2) 展开在目标键之后：params 自身仍可解析（保守放行顶层键）'
  );
  ok(
    extractAxiosConfigValue('buildCfg()', 'params') === CFG_BLIND,
    '(d6c) 变量 config => CFG_BLIND'
  );
  ok(
    extractAxiosConfigValue('{ params: { a }, timeout: 1 }', 'params') === '{ a }',
    '(d6d) 正常 config 解析'
  );
}

console.log('\n---- fixture 自测结论 ----');
console.log(`PASS: ${pass}  FAIL: ${fail}`);
if (fail) {
  console.error('FAIL: 契约解析器盲区回归（四类挂账项之一重新失明），check-api-request 门禁不可信');
  process.exit(1);
}
console.log(
  'OK: (a)箭头签名 (b)Partial (c)交叉 (d)叶子/索引签名/config 三态 —— 全部按“判不出必须显式”工作。'
);
