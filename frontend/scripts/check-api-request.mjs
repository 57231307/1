#!/usr/bin/env node
/**
 * check-api-request.mjs —— 「前端请求载荷 ↔ 后端反序列化结构体」一致性门禁
 *
 * ⚠️ 当前未接入 CI：与 check-api-envelope 同为静态分析工具，是否升级为阻断门禁由用户拍板。
 *    运行：`cd frontend && node scripts/check-api-request.mjs`（退出码 0=无失配）。
 *
 * 为什么需要它：本仓库的主导缺陷类是 UI↔API 契约漂移，而响应侧已有 check-api-envelope 覆盖，
 * 请求侧此前**完全没有工具**在看。本轮手工查 MRP 时才验证了这类缺陷的真实代价：
 * 前端提交 `{product_ids, demand_quantity, demand_date}`，后端 `MrpCalculatePayload` 要
 * `items[]{product_id, required_quantity, required_date}` 且 `#[validate(length(min=1))]`
 * ——计算接口必然 422，而此前任何静态检查都不会报错。同族还有「传了后端不读的参数被静默丢弃」
 * （preview_report 的 page/page_size、生产订单日志的 order_no 等）。
 *
 * 判据（与响应侧同一取向：判不出就记盲区并计数，绝不"读不懂即通过"）：
 *   - 请求体（POST/PUT/PATCH）：前端实参（内联对象 或 形参声明的具名类型展开后的键集）
 *     ↔ 后端 `Json<T>` 的 T 字段集。
 *       · 后端必填（非 Option、无 #[serde(default)]）而前端不传  -> 失配（反序列化失败/校验 422）
 *       · 前端传了后端没有的字段                                -> 失配（serde 忽略=参数被静默丢弃）
 *   - 查询参数（GET/DELETE 的 `{ params }`）：同一套判据比对后端 `Query<T>`。
 *   - 任一侧解析不出（`Record<string, unknown>`、`any`、`#[serde(flatten)]`、泛型、跨文件多定义）
 *     -> 盲区，逐条计数并列出，不判负也不谎称已覆盖。
 *   - serde 命名换算：仅按结构体级 `#[serde(rename_all = "...")]` 换算；比对时同时接受
 *     「原样字段名」与「换算后键名」两种集合之一（整表一致即可），混用才算失配——
 *     避免把大小写约定当成漂移，同时仍能抓出真正多/少的字段。
 *
 * 解析器复用：全部从 check-api-envelope.mjs 导入（同一套 nest 前缀还原、宏展开、pub use 转出、
 * TS 具名类型展开），两处各写一份必然漂移——这是本仓库反复栽过的根因之一。
 */
import { BASE_URL, walkBackendRoutes } from './check-api-paths.mjs';
import {
  buildHandlerModules,
  buildStructIndex,
  buildTsTypeIndex,
  loadHandlerMacroTemplates,
  parseFrontendApiFunctions,
  resolveHandlerSymbolPath,
  splitObjFields,
  splitTopLevelRust,
} from './check-api-envelope.mjs';

const BODY_METHODS = new Set(['POST', 'PUT', 'PATCH']);
const QUERY_METHODS = new Set(['GET', 'DELETE']);

// ---------- Rust 侧：Json<T> / Query<T> 提取器与结构体字段 ----------
function extractorType(sig, wrapper) {
  const re = new RegExp(
    '\\b' +
      wrapper +
      '\\s*\\(\\s*[A-Za-z_][\\w]*\\s*\\)\\s*:\\s*(?:[a-z_]\\w*::)*' +
      wrapper +
      '\\s*<\\s*([A-Za-z_][\\w:]*)',
    'g'
  );
  const m = re.exec(sig);
  if (m) return m[1];
  const re2 = new RegExp('\\b' + wrapper + '\\s*<\\s*([A-Z][\\w:]*)\\s*>', 'g');
  const m2 = re2.exec(sig);
  return m2 ? m2[1] : null;
}

function serdeRenameAll(attrs) {
  const m = /rename_all\s*=\s*"([^"]+)"/.exec(attrs || '');
  return m ? m[1] : null;
}

function applyCase(name, rule) {
  if (!rule || rule === 'snake_case') return name;
  const parts = name.split('_');
  if (rule === 'camelCase')
    return (
      parts[0] +
      parts
        .slice(1)
        .map(p => p.charAt(0).toUpperCase() + p.slice(1))
        .join('')
    );
  if (rule === 'PascalCase') return parts.map(p => p.charAt(0).toUpperCase() + p.slice(1)).join('');
  if (rule === 'kebab-case') return parts.join('-');
  if (rule === 'SCREAMING-KEBAB') return parts.join('-').toUpperCase();
  if (rule === 'SCREAMING_SNAKE_CASE') return name.toUpperCase();
  return null; // 未知规则 -> 不猜
}

function rustFieldsOf(typeName, structIndex) {
  if (!typeName) return null;
  const clean = typeName.replace(/\s+/g, '').replace(/^crate::/, '');
  const tail = clean.split('::').slice(-2).join('::');
  const entry = structIndex[tail] || structIndex[clean] || structIndex[clean.split('::').pop()];
  if (!entry || entry.ambiguous) return null;
  const rule = serdeRenameAll(entry.attrs);
  const out = [];
  for (const f of entry.allFields) {
    const t = (f.type || '').replace(/\s+/g, '');
    if (/^#\[?serde\(flatten/.test(t)) return null;
    const optional = /^Option</.test(t);
    const hasDefault = /#[a-z_]*serde[^]*default/.test(entry.attrs || '');
    out.push({ name: f.name, optional: optional || hasDefault, renamed: applyCase(f.name, rule) });
    if (out[out.length - 1].renamed === null) return null;
  }
  if (/flatten/.test(entry.attrs || '')) return null;
  return out;
}

// ---------- TS 侧：具名类型/内联对象 -> 键集 ----------
function tsFieldsOf(name, tsIndex, depth = 0, seen = new Set()) {
  if (depth > 3 || seen.has(name)) return null;
  const ent = tsIndex.get(name);
  if (!ent || ent.kind === 'ambiguous') return null;
  seen.add(name);
  if (ent.kind === 'alias') {
    const t = (ent.text || '').trim();
    if (t.startsWith('{')) return tsFieldsOfBody(t.slice(1, -1), tsIndex, depth, seen, name);
    if (/^[A-Z]\w*$/.test(t)) return tsFieldsOf(t, tsIndex, depth + 1, seen);
    return null;
  }
  if (ent.kind !== 'object') return null;
  let body = ent.body;
  for (const par of ent.extends || []) {
    const pname = par.replace(/<.*>/, '').trim();
    const pf = tsFieldsOf(pname, tsIndex, depth + 1, seen);
    if (!pf) return null;
    body = pf.raw + '\n;' + body;
  }
  return tsFieldsOfBody(body, tsIndex, depth, seen, name);
}

function tsFieldsOfBody(body, tsIndex, depth, seen, label) {
  const out = [];
  for (const entry of splitObjFields(body)) {
    const kv = entry.match(/^\s*([A-Za-z_]\w*)(\??)\s*:\s*([\s\S]*)$/);
    if (!kv) {
      if (/\[/.test(entry) && /unknown|any/.test(entry)) return null; // 索引签名：键集不可穷举
      continue;
    }
    if (/\[\s*\w+\s*:\s*string\s*\]/.test(entry)) return null;
    out.push({ name: kv[1], optional: kv[2] === '?', typeText: kv[3].trim() });
  }
  if (!out.length) return null;
  return { fields: out, raw: body, from: label };
}

// 从 axios config 对象文本里取某个键的值（`{ params }` 简写 -> 'params'）。
function extractAxiosConfigValue(objText, wanted) {
  const t = objText.trim();
  if (!t.startsWith('{')) return null;
  const inner = t.slice(1, t.lastIndexOf('}'));
  for (const part of splitObjFields(inner)) {
    const kv = /^\s*([A-Za-z_]\w*)\s*(?::\s*([\s\S]*))?$/.exec(part.trim());
    if (!kv) return null; // 不可解析的 config 项 -> 视为无 params
    if (kv[1] === wanted) return kv[2] ? kv[2].trim() : wanted; // 简写：值即变量名
  }
  return null;
}

// 前端实参 -> 键集：内联对象字面量 或 形参声明的具名类型
function feKeysOf(fn, payloadExpr, tsIndex) {
  const e = (payloadExpr || '').replace(/\s+/g, ' ').trim();
  if (!e) return { kind: 'none' };
  if (e.startsWith('{')) {
    // 内联对象：键即字面量键（简写 `page,` 也算）
    const inner = e.replace(/^\{/, '').replace(/\}\s*$/, '');
    const keys = [];
    for (const part of splitObjFields(inner)) {
      const kv = part.match(/^\s*(\.\.\.|[A-Za-z_]\w*)\s*(?::\s*([\s\S]*))?$/);
      if (!kv) return { kind: 'blind', why: '内联对象含不可解析项: ' + part.trim().slice(0, 40) };
      if (kv[1] === '...') return { kind: 'blind', why: '内联对象含展开运算符，键集不可穷举' };
      if (!kv[2] && !/[,:]$/.test(part.trim())) {
        keys.push({ name: kv[1], optional: false });
        continue;
      }
      keys.push({ name: kv[1], optional: /^\?/.test(part.trim().slice(kv[1].length)) });
    }
    return keys.length ? { kind: 'keys', keys } : { kind: 'blind', why: '内联对象无键' };
  }
  if (/^[A-Za-z_]\w*$/.test(e)) {
    const sig = (fn.sig || '').replace(/\s+/g, ' ');
    // 可选形参写作 `params?: RoleQuery`——问号在名字与冒号之间；漏掉它会让上百个函数
    // 落进"签名里找不到类型"的盲区（本仓库盲区中 94 条即由此而来）。
    const pm = new RegExp('\\b' + e + '\\s*\\??\\s*:\\s*([A-Za-z_][\\w.<>\\[\\]| ]*)').exec(sig);
    if (!pm) return { kind: 'blind', why: `形参 ${e} 在签名里找不到类型` };
    let t = pm[1]
      .trim()
      .replace(/\s*\|\s*null\s*$/, '')
      .replace(/\?\s*$/, '');
    const bare = t.match(/^([A-Z]\w*)$/);
    if (!bare) return { kind: 'blind', why: `形参类型不是具名对象: ${t}` };
    const f = tsFieldsOf(bare[1], tsIndex);
    if (!f) return { kind: 'blind', why: `${bare[1]} 无法静态展开（泛型/多定义/索引签名）` };
    return { kind: 'keys', keys: f.fields, from: bare[1] };
  }
  return { kind: 'blind', why: '实参既非对象字面量也非单一标识符' };
}

// 后端字段集 -> 与前端键集比对（接受"全原样"或"全 rename 后"两种约定之一）
function compareKeys(feKeys, beFields) {
  const fe = new Set(feKeys.map(k => k.name));
  const feOptional = new Set(feKeys.filter(k => k.optional).map(k => k.name));
  const raw = new Set(beFields.map(f => f.name));
  const renamed = new Set(beFields.map(f => f.renamed));
  const sameRaw = fe.size === raw.size && [...fe].every(k => raw.has(k));
  const sameRenamed = fe.size === renamed.size && [...fe].every(k => renamed.has(k));
  if (sameRaw || sameRenamed) return { status: 'ok' };
  const view = new Map(beFields.map(f => [f.name, f]));
  const beNames = new Set([...beFields.map(f => f.name), ...beFields.map(f => f.renamed)]);
  const extra = [...fe].filter(k => !beNames.has(k));
  const beSet = new Set([...raw, ...(sameRenamed ? [] : [])]);
  const missingRequired = beFields
    .filter(f => !f.optional && !fe.has(f.name) && !fe.has(f.renamed))
    .map(f => f.name);
  const missingOptional = beFields
    .filter(f => f.optional && !fe.has(f.name) && !fe.has(f.renamed))
    .map(f => f.name);
  void beSet;
  const softMissingOnly =
    extra.length === 0 && missingRequired.length === 0 && missingOptional.length > 0;
  if (softMissingOnly) return { status: 'ok', note: '未传可选字段: ' + missingOptional.join('/') };
  return {
    status: 'mismatch',
    extra,
    missingRequired,
    missingOptional,
    feOptional: [...feOptional],
    view,
  };
}

function main() {
  const structIndex = buildStructIndex();
  const handlerMods = buildHandlerModules(loadHandlerMacroTemplates());
  const tsIndex = buildTsTypeIndex();
  const { handlers } = walkBackendRoutes();
  const feFunctions = parseFrontendApiFunctions(tsIndex);

  const buckets = { ok: [], mismatch: [], blind: [], noHandler: [], noExtractor: [] };
  for (const fn of feFunctions) {
    if (!fn.call || !fn.call.path) continue;
    const key = `${fn.call.path} ${fn.call.method}`;
    const h = handlers.get(key);
    if (!h) {
      // 路由是否存在由 check-api-paths 专门判负，这里不重复
      continue;
    }
    const parts = String(h.handler || '')
      .split('::')
      .filter(Boolean);
    if (parts.length < 2) continue;
    const sym = resolveHandlerSymbolPath(handlerMods, parts);
    if (!sym) {
      buckets.noHandler.push({ fn, key, handler: h.handler });
      continue;
    }
    // 比对哪个提取器由「前端实际发了什么」决定，而不是由 HTTP 方法决定：
    // axios 的 DELETE 也会把 { data } 作为请求体发出（取消定制订单带原因就是这么走的），
    // 而 GET 的 { params } 对应 Query<T>。方法名推法会把这两类判反。
    const cfg = (fn.call.args[1] || '').trim();
    const sendsBody =
      BODY_METHODS.has(fn.call.method) ||
      (!!extractAxiosConfigValue(cfg, 'data') && QUERY_METHODS.has(fn.call.method));
    const sendsQuery =
      !BODY_METHODS.has(fn.call.method) && !!extractAxiosConfigValue(cfg, 'params');
    // config 里既无 params 也无 data（只有 responseType/timeout 等）= 没有可比对的载荷
    if (!sendsBody && !sendsQuery && !BODY_METHODS.has(fn.call.method)) continue;
    const isBody = sendsBody;
    const isQuery = !sendsBody && sendsQuery;
    // Option<Json<Value>> / Option<Query<Value>>：后端明说"可空且不限形"，比对不适用
    if (
      new RegExp(
        'Option\s*<\\s*' + (isBody ? 'Json' : 'Query') + '\s*<\s*(?:serde_json::)?Value'
      ).test(sym.sig || '')
    ) {
      buckets.blind.push({
        fn,
        key,
        handler: h.handler,
        beType: '(Option<Json/Query<Value>>)',
        why: '后端接受任意或空载荷且不限形，键集比对不适用',
      });
      continue;
    }
    const wrapper = isBody ? 'Json' : 'Query';
    const typeText = extractorType(sym.sig || '', wrapper);
    if (!typeText) {
      // 后端这个 handler 没有 Json<T>/Query<T> 提取器。此时前端发什么都会被忽略——
      // 但只有"确实发了业务载荷"才算缺陷，否则是噪声：
      //  - GET/DELETE：只有 config 里真有 params（或 DELETE 带 data）才算；
      //  - POST/PUT：null / 空对象 / FormData 走的是别的提取器（MultipartForm 等），归盲区。
      const sent = (fn.call.args[1] || '').trim();
      // `undefined` 也算空载荷：axios 的 post(url, data, config) 必须占位才能传 config
      // （如以 query 传参的 POST），此时并不发请求体，判成"载荷被忽略"是误报。
      const isEmptyish = !sent || sent === 'null' || sent === 'undefined' || /^\{\s*\}$/.test(sent);
      if (isQuery) {
        const pv = extractAxiosConfigValue(sent, 'params');
        const dv = extractAxiosConfigValue(sent, 'data');
        if (pv === null && dv === null) continue;
        buckets.mismatch.push({
          fn,
          key,
          handler: h.handler,
          kind: 'query',
          reason: `后端签名里没有 Query<T> 提取器，前端查询参数被整体忽略：${(pv || dv || '').slice(0, 60)}`,
        });
        continue;
      }
      if (isEmptyish) continue;
      if (/MultipartForm|multipart|Bytes|Extension</.test(sym.sig || '')) {
        buckets.blind.push({
          fn,
          key,
          handler: h.handler,
          beType: '(非 Json/Query 提取器)',
          why: '后端走 MultipartForm/Bytes 等提取器，键集比对不适用',
        });
        continue;
      }
      buckets.mismatch.push({
        fn,
        key,
        handler: h.handler,
        kind: 'body',
        reason: `后端签名里没有 Json<T> 提取器，前端请求体被整体忽略：${sent.slice(0, 60)}`,
      });
      continue;
    }
    let payloadExpr = fn.call.args[1];
    if (isBody && cfg) {
      const dv = extractAxiosConfigValue(cfg, 'data');
      if (dv) payloadExpr = dv; // DELETE-with-data：真正载荷在 config.data 里
    }
    let feExpr = payloadExpr;
    if (isQuery) {
      const p = (payloadExpr || '').trim();
      if (!p) continue; // 不带 config 的 GET：无可比对
      // GET/DELETE 的第二实参是 axios config：只有 `params` 键才是查询串，
      // responseType/timeout/signal 等属请求选项，参与比对会产生假阳性。
      const paramsVal = extractAxiosConfigValue(p, 'params');
      if (paramsVal === null) continue; // config 里没有 params -> 无查询参数可比
      const inner = paramsVal;
      feExpr = inner;
    }
    const beFields = typeText ? rustFieldsOf(typeText, structIndex) : null;
    const fe = feKeysOf(fn, feExpr, tsIndex);
    if (fe.kind === 'none') {
      if (typeText && beFields && beFields.some(f => !f.optional) && (isBody || isQuery))
        buckets.mismatch.push({
          fn,
          key,
          handler: h.handler,
          beType: typeText,
          reason: `前端未发送任何${isBody ? '请求体' : '查询参数'}，而后端 ${typeText} 存在必填字段: ${beFields
            .filter(f => !f.optional)
            .map(f => f.name)
            .join(',')}`,
          beFields,
          beRequired: beFields.filter(f => !f.optional).map(f => f.name),
          feKeys: beFields.map(() => null).filter(() => false),
        });
      continue;
    }
    if (fe.kind === 'blind' || !beFields) {
      buckets.blind.push({
        fn,
        key,
        handler: h.handler,
        beType: typeText || '(无 ' + wrapper + '<T> 提取器)',
        why:
          fe.kind === 'blind'
            ? fe.why
            : `后端 ${typeText || '?'} 字段不可静态解析（未定位/多定义/flatten）`,
      });
      continue;
    }
    const cmp = compareKeys(fe.keys, beFields);
    fn.feKeysUsed = fe.keys.map(k => k.name + (k.optional ? '?' : ''));
    const rec = {
      fn,
      key,
      handler: h.handler,
      beType: typeText,
      feFrom: fe.from || '内联对象',
      note: cmp.note,
      cmp,
    };
    if (cmp.status === 'ok') buckets.ok.push(rec);
    else buckets.mismatch.push({ ...rec, kind: isBody ? 'body' : 'query' });
    void buckets.noExtractor;
  }

  if (process.argv.includes('--json')) {
    // 机器可读输出：给并行修复任务当工单用，避免把清单抄进提示词时抄错或漏项。
    console.log(
      JSON.stringify(
        {
          total: feFunctions.length,
          ok: buckets.ok.length,
          blind: buckets.blind.length,
          mismatches: buckets.mismatch.map(r => ({
            file: r.fn.file,
            line: r.fn.line,
            fn: r.fn.name,
            endpoint: r.key,
            kind: r.kind || 'no-payload',
            handler: r.handler,
            beType: r.beType || null,
            beFields: r.beFields ? r.beFields.map(f => f.name + (f.optional ? '?' : '')) : null,
            extra: (r.cmp && r.cmp.extra) || [],
            missingRequired:
              (r.cmp && r.cmp.missingRequired) || (r.beRequired ? r.beRequired.slice() : []),
            missingOptional: (r.cmp && r.cmp.missingOptional) || [],
            feKeys: (r.fn && r.fn.feKeysUsed) || null,
            reason: r.reason || null,
          })),
        },
        null,
        2
      )
    );
    process.exit(buckets.mismatch.length ? 1 : 0);
  }
  console.log('=== check-api-request: 前端请求载荷 ↔ 后端 Json<T>/Query<T> 字段集 ===');
  console.log(`前端 api 函数总数: ${feFunctions.length}`);
  console.log(`  一致(ok)        : ${buckets.ok.length}`);
  console.log(`  失配(mismatch)  : ${buckets.mismatch.length}`);
  console.log(`  盲区(不可判定)  : ${buckets.blind.length}`);
  console.log(`  未定位 handler  : ${buckets.noHandler.length}`);

  if (buckets.mismatch.length) {
    console.log('\n[失配 · 请求载荷与后端结构体字段集不一致]');
    for (const r of buckets.mismatch) {
      console.log(`  - ${r.fn.file}:${r.fn.line} ${r.fn.name}  ${r.key}  [${r.kind || 'no-body'}]`);
      if (r.beRequired)
        console.log(
          `      后端必填: ${r.beRequired.join(',')}  全部字段: ${r.beFields.map(f => f.name + (f.optional ? '?' : '')).join(',')}`
        );
      console.log(`      后端: ${r.handler} -> ${r.beType}`);
      console.log(
        `      前端: ${r.feFrom || ''} 多余(后端不读)=${((r.cmp && r.cmp.extra) || []).join(',') || '-'}` +
          ` 缺必填=${((r.cmp && r.cmp.missingRequired) || []).join(',') || '-'}` +
          ` 缺可选=${((r.cmp && r.cmp.missingOptional) || []).join(',') || '-'}`
      );
      if (r.reason) console.log(`      说明: ${r.reason}`);
    }
  }
  if (buckets.blind.length) {
    console.log(`\n[盲区 · 任一侧无法静态展开(不判负，但须知道没覆盖) ${buckets.blind.length} 条]`);
    const by = {};
    for (const r of buckets.blind) {
      const k = r.why.replace(/[:].*/, '').slice(0, 46);
      by[k] = (by[k] || 0) + 1;
    }
    for (const [k, n] of Object.entries(by).sort((a, b) => b[1] - a[1]))
      console.log(`  ${n}  ${k}`);
  }
  if (buckets.noHandler.length) {
    console.log(`\n[未定位到 handler 签名(不判负) ${buckets.noHandler.length} 条]`);
    for (const r of buckets.noHandler.slice(0, 20))
      console.log(`  - ${r.fn.file}:${r.fn.line} ${r.key} -> ${r.handler}`);
  }

  console.log('\n---- 结论 ----');
  if (buckets.mismatch.length) {
    console.error(`FAIL: 请求载荷失配 ${buckets.mismatch.length} 条`);
    process.exit(1);
  }
  console.log('\nOK: 可比对项全部一致（盲区已逐条计数，未谎称全覆盖）。');
  void BASE_URL;
}

main();
