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
 * 追加（盲区口径收紧，逐类可复验）
 *   (e) 形参注解尾注释剥离；(f) 联合类型逐员合并/一员失明整条盲区；
 *   (g) 交叉类型含 Omit 成员的完整展开；(h) 跨文件同名歧义类型的本文件遮蔽还原；
 *   (i) 同名跨文件副本调用点的补录（含幂等）。数组载荷 `T[]`↔`Json<Vec<T>>`、
 *   `data ?? {}`、`{}` 空体均落在 [a] 段断言。
 * 任何一条断言失败 -> 退出码 1（本脚本已接入 CI ci-static-checks，防解析器回归）。
 */
import {
  CFG_BLIND,
  buildLocalTypeIndex,
  compareKeys,
  extractAxiosConfigValue,
  extractorType,
  feKeysOf,
  leafTypeText,
  paramTypeOf,
  resolveTsTypeExpr,
  rustFieldsOf,
  splitTopLevelAmp,
  stripTsComments,
  supplementMissedCallSites,
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
  // 盲区 1+2 联合形态：async 箭头函数 + 数组类型注解载荷,违规必须能报
  const srcArr = `export const receiveItems = async (receiptId: number, data: Partial<WidgetDto>[]): Promise<ApiResponse<null>> =>
  request.post<ApiResponse<null>>(\`/receipts/\${receiptId}/receive\`, data);
`;
  const sigArr = sigForSymbol(srcArr, 0, srcArr.indexOf('request.post'));
  ok(
    sigArr === 'receiptId: number, data: Partial<WidgetDto>[]',
    '(a6) async 箭头 + Promise 返回注解:sig 完整回填(含数组注解)',
    JSON.stringify(sigArr)
  );
  const rArr = feKeysOf({ sig: sigArr }, 'data', tsIndex);
  ok(
    rArr.kind === 'keys' && rArr.keys.every(k => k.optional),
    '(a7) `T[]` 载荷剥壳按元素类型展开（旧实现记“未处理的类型写法”整体跳过）',
    JSON.stringify(
      rArr.kind === 'keys' ? rArr.keys.map(k => k.name + (k.optional ? '?' : '')) : rArr
    )
  );
  const cmpArr = compareKeys(
    rArr.kind === 'keys' ? rArr.keys : [],
    rustFieldsOf('WidgetPayload', structIndex)
  );
  ok(
    cmpArr.status === 'mismatch' && cmpArr.extra.join(',') === 'ghostKey',
    '(a8) 数组载荷的键名漂移(ghostKey 后端不读)能报——盲区补录后该类不再静默',
    `extra=${(cmpArr.extra || []).join(',')}`
  );
  ok(
    extractorType('Json(items): Json<Vec<WidgetPayload>>', 'Json') === 'WidgetPayload',
    '(a9) 后端 Json<Vec<T>> 展开到元素类型 T（旧实现截到 Vec 后整条盲区）',
    extractorType('Json(items): Json<Vec<WidgetPayload>>', 'Json')
  );
  // `data ?? {}` 兜底写法：载荷即 data,全部键置可选（13 处曾整体拒检）
  const rAlt = feKeysOf({ sig: 'id: number, data: NameDto', payload: '' }, 'data ?? {}', tsIndex);
  ok(
    rAlt.kind === 'keys' && rAlt.keys.length === 1 && rAlt.keys[0].optional,
    '(a10) `data ?? {}` 解析为 data 的键集且全部可选（兜底空对象时任何键都可能缺）',
    JSON.stringify(rAlt.kind === 'keys' ? rAlt.keys : rAlt)
  );
  // 空对象载荷：是「确实不发键」,不是「解析不出」
  const rEmpty = feKeysOf({ sig: 'data: NameDto' }, '{}', tsIndex);
  ok(
    rEmpty.kind === 'none',
    '(a11) `{}` 空对象载荷归 none（走“后端必填 vs 前端空体”判据，不再记盲区躲检查）'
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
    r3.kind === 'keys' &&
      r3.keys
        .map(k => k.name)
        .sort()
        .join(',') === 'ghostKey,name',
    '(c6) Omit<T,K> 已支持：展开 T 后删掉 K（看板 #40 口径收紧，原“未支持=>盲区”用例）',
    JSON.stringify(r3.kind === 'keys' ? r3.keys.map(k => k.name) : r3)
  );
  const r4 = feKeysOf({ sig: 'data: keyof WidgetDto' }, 'data', tsIndex);
  ok(
    r4.kind === 'blind' && /未处理的类型写法/.test(r4.why),
    '(c6b) 真·未支持写法仍必须显式盲区（带原文），绝不静默 continue',
    r4.why
  );
  const r5 = feKeysOf({ sig: 'data: Pick<WidgetDto, "qty">' }, 'data', tsIndex);
  ok(
    r5.kind === 'keys' && r5.keys.map(k => k.name).join(',') === 'qty',
    '(c6c) Pick<T,K> 展开：只留列出的键',
    JSON.stringify(r5.kind === 'keys' ? r5.keys.map(k => k.name) : r5)
  );
  const r6 = feKeysOf({ sig: 'data: Omit<WidgetDto, 42>' }, 'data', tsIndex);
  ok(
    r6.kind === 'blind' && /键列表非字符串字面量/.test(r6.why),
    '(c6d) Omit 键列表非字符串字面量 => 显式盲区，不猜',
    r6.why
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

// ---------- (e) 注解尾注释剥离（盲区 2-带注释的类型注解） ----------
console.log('\n[e] 形参注解后的 `//` 说明注释不得再污染类型解析');
{
  const sig =
    'params?: CreateWidgetDto // 后端 handler::list(a, b) 返回 ApiResponse<Vec<Model>> ⇒ 裸数组';
  const pt = paramTypeOf(sig, 'params');
  ok(
    pt && pt.typeText === 'CreateWidgetDto',
    '(e1) 注释含逗号/括号也不会把类型文本拖进参数解析',
    JSON.stringify(pt)
  );
  const r = feKeysOf({ sig }, 'params', tsIndex);
  ok(
    r.kind === 'keys' && r.keys.map(k => k.name).join(',') === 'name,qty',
    '(e2) 注释污染形态的形参载荷现在能进比对',
    JSON.stringify(r.kind === 'keys' ? r.keys.map(k => k.name) : r)
  );
  ok(
    stripTsComments('A & B // 说明 /* x */').replace(/\s+/g, ' ').trim() === 'A & B',
    '(e3) stripTsComments 基本形态'
  );
}

// ---------- (f) 联合类型（复杂类型注解, 盲区 2） ----------
console.log('\n[f] 联合类型：逐员合并,一员失明整条显式盲区');
{
  const r = feKeysOf(
    { sig: "data: { step: 'd1'; memo: string } | { step: 'd2'; memo: string; owner: string }" },
    'data',
    tsIndex
  );
  ok(
    r.kind === 'keys' &&
      r.keys
        .map(k => k.name)
        .sort()
        .join(',') === 'memo,owner,step',
    '(f1) 各成员键集并集参与比对（旧实现“未处理的类型写法”整体跳过）',
    JSON.stringify(r.kind === 'keys' ? r.keys.map(k => k.name + (k.optional ? '?' : '')) : r)
  );
  const owner = r.kind === 'keys' && r.keys.find(k => k.name === 'owner');
  ok(!!owner && owner.optional === true, '(f2) 只在部分成员出现的键必须置可选（缺它不算失配）');
  const memo = r.kind === 'keys' && r.keys.find(k => k.name === 'memo');
  ok(!!memo && memo.optional === false, '(f3) 全成员都必填的键保持必填（后端缺它仍会报）');
  const rb = feKeysOf({ sig: 'data: NameDto | GhostThing' }, 'data', tsIndex);
  ok(
    rb.kind === 'blind' && /GhostThing/.test(rb.why),
    '(f4) 一员解析不出 => 整条显式盲区并点名成员,禁止半解析',
    rb.why
  );
}

// ---------- (g) 交叉类型含 Omit 成员（交叉+复杂类型合并形态） ----------
console.log('\n[g] 交叉类型 x Omit：成员展开后再合并');
{
  const r = feKeysOf({ sig: 'data: NameDto & Omit<WidgetDto, "ghostKey">' }, 'data', tsIndex);
  ok(
    r.kind === 'keys' &&
      r.keys
        .map(k => k.name)
        .sort()
        .join(',') === 'name,qty',
    '(g1) `A & Omit<B,K>` 完整展开合并',
    JSON.stringify(r.kind === 'keys' ? r.keys.map(k => k.name) : r)
  );
  const cmp = compareKeys(
    r.kind === 'keys' ? r.keys : [],
    rustFieldsOf('NameOnlyPayload', structIndex)
  );
  ok(
    cmp.status === 'mismatch' && cmp.extra.join(',') === 'qty',
    '(g2) 该交叉形态下多发的 qty（后端不读）能报——交叉违规被门禁真实抓住',
    `extra=${(cmp.extra || []).join(',')}`
  );
}

// ---------- (h) 跨文件同名歧义类型:本文件定义优先（盲区 2-多定义） ----------
console.log('\n[h] 同名歧义类型:调用方文件自身定义优先');
{
  const ambIndex = new Map([['AmbDto', { kind: 'ambiguous', file: 'other' }]]);
  const localSrc = `export interface AmbDto { own_field: string; other: number }\n`;
  const local = buildLocalTypeIndex(localSrc, '/src/api/fixture.ts');
  const r = feKeysOf({ sig: 'data: AmbDto' }, 'data', ambIndex, local);
  ok(
    r.kind === 'keys' && r.keys.map(k => k.name).join(',') === 'own_field,other',
    '(h1) 全局索引 ambiguous 的类型,本地定义可无歧义还原（user.ts/user-profile.ts 双 ChangePasswordRequest 形态）',
    JSON.stringify(r.kind === 'keys' ? r.keys.map(k => k.name) : r)
  );
  const rNoLocal = feKeysOf({ sig: 'data: AmbDto' }, 'data', ambIndex, null);
  ok(rNoLocal.kind === 'blind', '(h2) 无本地定义时仍显式盲区,不猜别文件的同名类型');
}

// ---------- (i) 被全局去重吞掉的调用点必须补录（盲区 1-静默丢弃） ----------
console.log('\n[i] 同名跨文件副本调用点补录');
{
  const twinSrc = `export const getList = (data: CreateWidgetDto) =>
  request.post('/widgets', data);
`;
  const feFunctions = [
    {
      name: 'getList',
      file: '/src/api/other.ts',
      line: 1,
      call: { method: 'POST', path: '/widgets', args: ["'/widgets'", 'data'] },
      sig: 'data: CreateWidgetDto',
    },
  ];
  const added = supplementMissedCallSites(feFunctions, [{ rel: '/src/api/twin.ts', src: twinSrc }]);
  ok(
    added === 1 && feFunctions.length === 2,
    '(i1) 另一文件的同名同端点副本不再被吞掉',
    `added=${added}`
  );
  const twin = feFunctions[1];
  ok(
    twin.file === '/src/api/twin.ts' && twin.sig === 'data: CreateWidgetDto',
    '(i2) 补录条目带文件与形参签名,可正常参与载荷比对',
    JSON.stringify({ file: twin.file, sig: twin.sig })
  );
  const again = supplementMissedCallSites(feFunctions, [{ rel: '/src/api/twin.ts', src: twinSrc }]);
  ok(again === 0, '(i3) 重复执行不双计（幂等）');
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
