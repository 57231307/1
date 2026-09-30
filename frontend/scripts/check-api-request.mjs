#!/usr/bin/env node
/**
 * check-api-request.mjs —— 「前端请求载荷 ↔ 后端反序列化结构体」一致性门禁
 *
 * 已接入 CI（.github/workflows/ci-cd.yml · ci-static-checks · 阻断）。
 *    运行：`cd frontend && node scripts/check-api-request.mjs`（退出码 0=无新增失配）。
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
 * 本轮补齐的四类解析盲区（挂账项，逐条对应）：
 *   (a) 箭头函数形式请求签名：parseFrontendApiFunctions 此前只认 `: Promise<...>` 声明式函数
 *       或带泛型的 `request.get<T>(...)`；两种都不满足的箭头函数整条不进清单（真·静默跳过）。
 *       现由 envelope 侧第三趟扫描补录并回填形参签名（见 check-api-envelope.mjs）。
 *   (b) `Partial<T>`/`Required<T>` 包装：resolveTsTypeExpr 剥壳展开内部具名类型，
 *       Partial 使全部键视为可选（PATCH 语义），不再以「不是具名对象」拒收。
 *   (c) 交叉类型 `A & B`：类型捕获正则补 `&`，splitTopLevelAmp 顶层切分后逐员展开合并键集；
 *       任一成员解析不出 -> 显式盲区（写明是哪一员），禁止只解析一半就拿去比对。
 *   (d) 叶子类型字段（`serde_json::Value`/`any`/`Record<string, unknown>`/`HashMap<String,_>`）：
 *       顶层键名仍参与比对，但内层键集**不可比**这一事实必须进「未覆盖清单」逐条输出，
 *       不再当作无声通过；索引签名与不可解析的 axios config 同样显式入盲区。
 *
 * 存量失配过渡机制：scripts/api-request-baseline.json 登记「当前已知、待清零」的失配项
 * （file:line + 端点为键）。命中基线只列示不判负；**不在基线内的新失配仍立即红**。
 * 这是过渡而非放宽——基线只准缩短，清零后应删除该文件恢复全量阻断。
 *
 * 解析器复用：全部从 check-api-envelope.mjs 导入（同一套 nest 前缀还原、宏展开、pub use 转出、
 * TS 具名类型展开），两处各写一份必然漂移——这是本仓库反复栽过的根因之一。
 */
import { existsSync, readFileSync } from 'fs';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';
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
// extractAxiosConfigValue 的三态：null=确实没有该键；CFG_BLIND=有 config 但静态读不懂。
// 旧实现把两者混同为 null，等于「读不懂 => 当作没发」——正是被挂账的静默放行。
const CFG_BLIND = '\u0000__BLIND_CONFIG__\u0000';

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

// 叶子类型判定：名字层面可比，但内层键集静态不可穷举。命中即必须进「未覆盖清单」，
// 不允许「值为 Value/any/Record -> 静默视为通过」。
function leafTypeText(t) {
  let s = String(t || '')
    .replace(/\s+/g, '')
    .replace(/;$/, '');
  for (;;) {
    const om = /^(?:std::)?Option<([\s\S]*)>$/.exec(s);
    if (om) {
      s = om[1];
      continue;
    }
    const bm = /^(?:Box|Vec|Cow)<([\s\S]*)>$/.exec(s);
    if (bm && /^(?:serde_json::)?Value$|^JsonValue$/.test(bm[1])) return 'serde_json::Value';
    break;
  }
  if (/^(any|unknown)$/.test(s)) return s;
  if (/^Record<[\s\S]+>$/.test(s)) return 'Record<...>';
  if (/^(?:serde_json::)?Value$|^JsonValue$/.test(s)) return 'serde_json::Value';
  if (/^(?:std::collections::)?(?:HashMap|BTreeMap|Map)<\s*String,/.test(s))
    return 'Map<String,...>';
  return null;
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
    out.push({
      name: f.name,
      optional: optional || hasDefault,
      renamed: applyCase(f.name, rule),
      type: t, // 保留类型文本供叶子判定(d)；旧实现直接丢弃，Value 字段无声漂过
    });
    if (out[out.length - 1].renamed === null) return null;
  }
  if (/flatten/.test(entry.attrs || '')) return null;
  return out;
}

// ---------- TS 侧：具名类型/内联对象/Partial/交叉类型 -> 键集 ----------

// 顶层 `&` 切分（<> {} [] () 内不切）。返回成员列表；无 & 时即 [t]。
function splitTopLevelAmp(s) {
  const out = [];
  let depth = 0;
  let cur = '';
  for (const c of s) {
    if (c === '<' || c === '{' || c === '[' || c === '(') depth++;
    else if (c === '>' || c === '}' || c === ']' || c === ')') depth--;
    if (c === '&' && depth === 0) {
      out.push(cur);
      cur = '';
      continue;
    }
    cur += c;
  }
  if (cur.trim()) out.push(cur);
  return out.map(x => x.trim()).filter(Boolean);
}

// 任意 TS 类型文本 -> {fields} 或 {blind: 原因}。
// 覆盖：具名 interface/type、`Name<Arg>`、Partial<T>/Required<T>、A & B（交叉）、内联对象。
// 原则：解析不了必须带原因返回 blind，绝不返回「部分解析的结果」冒充完整。
function resolveTsTypeExpr(raw, tsIndex, seen = new Set()) {
  const t = String(raw || '')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/;+$/, '');
  if (!t) return { blind: '类型文本为空' };
  const leaf = leafTypeText(t);
  if (leaf) return { blind: `叶子类型 ${leaf}：键集不可穷举`, leaf };
  const parts = splitTopLevelAmp(t);
  if (parts.length > 1) {
    const merged = [];
    const unresolved = [];
    for (const p of parts) {
      const r = resolveTsTypeExpr(p, tsIndex, new Set(seen));
      if (r.blind) {
        unresolved.push(`${p}（${r.blind}）`);
        continue;
      }
      for (const f of r.fields) if (!merged.some(m => m.name === f.name)) merged.push(f);
    }
    // (c) 关键红线：交叉类型只要有一员解析不出，整条判盲区并点名是谁——
    // 旧实现静默只取首员，剩余成员键集被无声忽略。
    if (unresolved.length) return { blind: '交叉类型含不可解析成员: ' + unresolved.join(' | ') };
    if (!merged.length) return { blind: '交叉类型展开后无字段' };
    return { fields: merged, from: t.slice(0, 60) };
  }
  let mp;
  if ((mp = /^(Partial|Required)<([\s\S]+)>$/.exec(t))) {
    // (b) Partial<T>：T 全部键并入且置为可选（PATCH 语义，缺省不算失配）；
    //     Required<T> 反向。内部可继续是交叉/具名，递归交给同一入口。
    const r = resolveTsTypeExpr(mp[2], tsIndex, seen);
    if (r.blind) return r;
    const isPartial = mp[1] === 'Partial';
    return {
      fields: r.fields.map(f => ({ ...f, optional: isPartial ? true : f.optional })),
      from: `${mp[1]}<${mp[2]}>`.slice(0, 60),
    };
  }
  if (/^[A-Z]\w*$/.test(t)) {
    const f = tsFieldsOf(t, tsIndex, 0, seen);
    if (!f) return { blind: `${t} 无法静态展开（泛型/多定义/索引签名/父类型不可解析）` };
    return { fields: f.fields, from: t };
  }
  if (/^[A-Z]\w*<[A-Za-z_]\w*(\s*,\s*[A-Za-z_]\w*)*>$/.test(t)) {
    const bare = /^([A-Z]\w*)/.exec(t)[1];
    const f = tsFieldsOf(bare, tsIndex, 0, seen);
    if (!f) return { blind: `${bare} 无法静态展开（带类型实参）` };
    return { fields: f.fields, from: t };
  }
  if (t.startsWith('{') && t.endsWith('}')) {
    const f = tsFieldsOfBody(t.slice(1, -1), tsIndex, 0, new Set(seen), '(内联对象类型)');
    if (!f) return { blind: `内联对象类型无法展开: ${t.slice(0, 40)}` };
    return { fields: f.fields, from: '内联对象类型' };
  }
  return { blind: `未处理的类型写法: ${t.slice(0, 60)}` };
}

function tsFieldsOf(name, tsIndex, depth = 0, seen = new Set()) {
  if (depth > 3 || seen.has(name)) return null;
  const ent = tsIndex.get(name);
  if (!ent || ent.kind === 'ambiguous') return null;
  const localSeen = new Set(seen);
  localSeen.add(name);
  if (ent.kind === 'alias') {
    // type X = A & B / type X = Partial<Y> / type X = {..}：统一走一个解析口，
    // 避免 alias 分支只认「{...} 或裸名」而把交叉/Partial 无声退回 null。
    const r = resolveTsTypeExpr(ent.text, tsIndex, localSeen);
    if (r.blind) return null;
    return { fields: r.fields, raw: ent.text, from: name };
  }
  if (ent.kind !== 'object') return null;
  const self = tsFieldsOfBody(ent.body, tsIndex, depth, localSeen, name);
  if (!self) return null;
  const merged = self.fields.slice();
  for (const par of ent.extends || []) {
    const r = resolveTsTypeExpr(par, tsIndex, localSeen);
    if (r.blind) return null; // 父类型不可解析 -> 整条盲区（键集不完整就不比对）
    for (const f of r.fields) if (!merged.some(m => m.name === f.name)) merged.push(f);
  }
  return { fields: merged, raw: ent.body, from: name };
}

function tsFieldsOfBody(body, tsIndex, depth, seen, label) {
  const out = [];
  for (const entry of splitObjFields(body)) {
    const kv = entry.match(/^\s*([A-Za-z_]\w*)(\??)\s*:\s*([\s\S]*)$/);
    if (!kv) {
      // (d) 索引签名（`[key: string]: X`）：键集不可穷举，无论值类型是什么都必须判盲区。
      // 旧实现只拦 `unknown|any` 值，`[key: string]: number` 会走 continue 被无声跳过。
      if (/^\s*\[[^\]]*\]\s*:/.test(entry.trim())) return null;
      if (/\[/.test(entry) && /unknown|any/.test(entry)) return null;
      continue;
    }
    if (/\[\s*\w+\s*:\s*string\s*\]/.test(entry)) return null;
    out.push({ name: kv[1], optional: kv[2] === '?', typeText: kv[3].trim() });
  }
  if (!out.length) return null;
  return { fields: out, raw: body, from: label };
}

// 从 axios config 对象文本里取某个键的值（`{ params }` 简写 -> 'params'）。
// 返回 CFG_BLIND 表示「有第二个实参但静态读不懂」——调用方必须入盲区，禁止当作没有。
function extractAxiosConfigValue(objText, wanted) {
  const t = (objText || '').trim();
  if (!t) return null;
  if (!t.startsWith('{')) return CFG_BLIND; // config 来自变量/表达式：载荷不可静态定位
  const inner = t.slice(1, t.lastIndexOf('}'));
  for (const part of splitObjFields(inner)) {
    // 展开运算符 `{ params, ...extra }`：键集不可穷举，必须判盲区。
    // （`...x` 会被下方键名正则误识为 '..x'，须先显式拦截。）
    if (/^\s*\.\.\./.test(part)) return CFG_BLIND;
    const kv = /^\s*([A-Za-z_]\w*)\s*(?::\s*([\s\S]*))?$/.exec(part.trim());
    if (!kv) return CFG_BLIND; // 不可解析的 config 项（计算键等）-> 盲区，不再伪装成「无 params」
    if (kv[1] === wanted) return kv[2] ? kv[2].trim() : wanted; // 简写：值即变量名
  }
  return null;
}

// 从函数签名文本中取指定形参的类型注解。
// 旧实现用单条正则捕获，字符类容不下 `{...}` 内联对象类型（本仓大量
// `(data: { period: string; product_id?: number }) => ...` 写法），约 56 条
// 落进「形参在签名里找不到类型」。现按顶层逗号拆参数表，逐个取名/型，
// 剥默认值；`?` 可选标记保留为 optional。取不到类型时返回 null（调用方入盲区）。
function paramTypeOf(sig, name) {
  const s = String(sig || '').replace(/\s+/g, ' ');
  if (!s) return null;
  for (const p of splitTopLevelRust(s)) {
    const t = p.trim();
    const m = new RegExp('^' + name + '\\s*(\\??)\\s*:\\s*([\\s\\S]*)$').exec(t);
    if (!m) continue;
    let v = m[2].trim();
    // 剥顶层默认值 `= xxx`（不在 <>{}[]() 内）
    let depth = 0;
    for (let i = 0; i < v.length; i++) {
      const c = v[i];
      if (c === '<' || c === '(' || c === '[' || c === '{') depth++;
      else if (c === '>' || c === ')' || c === ']' || c === '}') depth--;
      else if (c === '=' && depth === 0 && v[i + 1] !== '=' && v[i - 1] !== '=') {
        v = v.slice(0, i).trim();
        break;
      }
    }
    return { typeText: v, optionalMark: m[1] === '?' };
  }
  return null;
}
function feKeysOf(fn, payloadExpr, tsIndex) {
  const e = (payloadExpr || '').replace(/\s+/g, ' ').trim();
  if (!e) return { kind: 'none' };
  if (/^(undefined|null|void)$/.test(e)) return { kind: 'none' };
  if (e.startsWith('{')) {
    // 内联对象：键即字面量键（简写 `page,` 也算）
    const inner = e.replace(/^\{/, '').replace(/\}\s*$/, '');
    const keys = [];
    for (const part of splitObjFields(inner)) {
      const kv = part.match(/^\s*(\.\.\.|[A-Za-z_]\w*)\s*(?::\s*([\s\S]*))?$/);
      if (!kv) return { kind: 'blind', why: '内联对象含不可解析项: ' + part.trim().slice(0, 40) };
      if (kv[1] === '...') return { kind: 'blind', why: '内联对象含展开运算符，键集不可穷举' };
      if (!kv[2] && !/[,:]$/.test(part.trim())) {
        keys.push({ name: kv[1], optional: false, typeText: '' });
        continue;
      }
      keys.push({
        name: kv[1],
        optional: /^\?/.test(part.trim().slice(kv[1].length)),
        typeText: (kv[2] || '').trim(),
      });
    }
    return keys.length ? { kind: 'keys', keys } : { kind: 'blind', why: '内联对象无键' };
  }
  if (/^[A-Za-z_]\w*$/.test(e)) {
    const sig = fn.sig || '';
    if (!sig.trim()) return { kind: 'blind', why: `形参 ${e}：未取到函数签名（解析器未回填 sig）` };
    const pt = paramTypeOf(sig, e);
    if (!pt || !pt.typeText) return { kind: 'blind', why: `形参 ${e} 在签名里找不到类型注解` };
    let t = pt.typeText
      .replace(/\s*\|\s*(?:null|undefined)\s*$/, '')
      .replace(/^\s*(?:null|undefined)\s*\|\s*/, '');
    const r = resolveTsTypeExpr(t, tsIndex);
    if (r.blind) return { kind: 'blind', why: r.blind };
    const keys = r.fields.map(f => (pt.optionalMark ? { ...f, optional: true } : f));
    return { kind: 'keys', keys, from: r.from };
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
  const missingRequired = beFields
    .filter(f => !f.optional && !fe.has(f.name) && !fe.has(f.renamed))
    .map(f => f.name);
  const missingOptional = beFields
    .filter(f => f.optional && !fe.has(f.name) && !fe.has(f.renamed))
    .map(f => f.name);
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

// ---------- 存量失配基线（过渡机制，只准缩短，不准新增豁免理由不明的项） ----------
const HERE = dirname(fileURLToPath(import.meta.url));
const BASELINE_PATH = join(HERE, 'api-request-baseline.json');
function loadBaseline() {
  if (!existsSync(BASELINE_PATH)) return { entries: new Set(), missing: true };
  let doc;
  try {
    doc = JSON.parse(readFileSync(BASELINE_PATH, 'utf-8'));
  } catch (e) {
    // 基线坏了必须硬失败：静默当作空基线会让存量项集体变红是小事，
    // 解析坏了却回退成「无基线」再被顺手重建，才是门禁失守的路径。
    console.error(`FAIL: 基线文件解析失败(${BASELINE_PATH}): ${e.message}`);
    process.exit(2);
  }
  return { entries: new Set((doc.entries || []).map(s => String(s))), missing: false };
}
// 基线键不含行号：存量清单认领修复期间源文件行号会漂移，带行号会把老问题误报成
// 「新增失配」（假红），逼人重新刷基线——那才是基线机制最容易被玩坏的地方。
const mismatchKey = r => `${r.fn.file} ${r.fn.name} ${r.key}`;

function main() {
  const structIndex = buildStructIndex();
  const handlerMods = buildHandlerModules(loadHandlerMacroTemplates());
  const tsIndex = buildTsTypeIndex();
  const { handlers } = walkBackendRoutes();
  const feFunctions = parseFrontendApiFunctions(tsIndex);

  const buckets = {
    ok: [],
    mismatch: [], // 新增失配（不在基线）——判负
    stock: [], // 存量失配（在基线）——只列示，待清零
    blind: [],
    uncovered: [], // (d) 叶子类型：顶层名比对了，内层键集没比对——显式入清单
    noHandler: [],
  };
  let noCall = 0; // 前端调用不可静态定位（URL 解析失败归 check-api-paths 判负）
  let noPayload = 0; // GET 不带任何 config/params：确无载荷可比（汇总计数，不逐条刷噪声）
  for (const fn of feFunctions) {
    if (!fn.call || !fn.call.path) {
      noCall++;
      continue;
    }
    const key = `${fn.call.path} ${fn.call.method}`;
    const h = handlers.get(key);
    if (!h) {
      // 路由是否存在由 check-api-paths 专门判负，这里不重复
      continue;
    }
    const parts = String(h.handler || '')
      .split('::')
      .filter(Boolean);
    if (parts.length < 2) {
      buckets.noHandler.push({ fn, key, handler: h.handler });
      continue;
    }
    const sym = resolveHandlerSymbolPath(handlerMods, parts);
    if (!sym) {
      buckets.noHandler.push({ fn, key, handler: h.handler });
      continue;
    }
    // 比对哪个提取器由「前端实际发了什么」决定，而不是由 HTTP 方法决定：
    // axios 的 DELETE 也会把 { data } 作为请求体发出（取消定制订单带原因就是这么走的），
    // 而 GET 的 { params } 对应 Query<T>。方法名推法会把这两类判反。
    // 注意 args[1] 的双重身份：BODY 方法里它是**载荷本身**（标识符/内联对象都交给
    // feKeysOf 走显式盲区），只有 GET/DELETE 里它才是 axios config —— 对 config 的
    // 静态不可解析（变量/展开/计算键）必须入盲区，禁止伪装成「没发载荷」。
    const cfg = (fn.call.args[1] || '').trim();
    let dvMark = null;
    let pvMark = null;
    if (cfg && BODY_METHODS.has(fn.call.method)) {
      if (/^\{/.test(cfg)) {
        dvMark = extractAxiosConfigValue(cfg, 'data');
        pvMark = extractAxiosConfigValue(cfg, 'params');
        if (dvMark === CFG_BLIND || pvMark === CFG_BLIND) {
          dvMark = null;
          pvMark = null; // 载荷解析统一交给 feKeysOf（展开/不可解析项会在那里显式入盲区）
        }
      }
    } else if (cfg) {
      // GET/DELETE 第二实参
      if (/^\{/.test(cfg)) {
        dvMark = extractAxiosConfigValue(cfg, 'data');
        pvMark = extractAxiosConfigValue(cfg, 'params');
        if (dvMark === CFG_BLIND || pvMark === CFG_BLIND) {
          buckets.blind.push({
            fn,
            key,
            handler: h.handler,
            beType: '(未及比对)',
            why: 'axios config 静态不可解析（展开运算符/计算键/非法规项），查询载荷无法定位',
          });
          continue;
        }
      } else {
        buckets.blind.push({
          fn,
          key,
          handler: h.handler,
          beType: '(未及比对)',
          why: 'GET/DELETE 第二实参为变量/表达式，无法静态确认是否携带 params/data',
        });
        continue;
      }
    }
    const sendsBody =
      BODY_METHODS.has(fn.call.method) || (!!dvMark && QUERY_METHODS.has(fn.call.method));
    const sendsQuery = !BODY_METHODS.has(fn.call.method) && !!pvMark;
    // config 里既无 params 也无 data（只有 responseType/timeout 等）= 没有可比对的载荷
    if (!sendsBody && !sendsQuery && !BODY_METHODS.has(fn.call.method)) {
      noPayload++;
      continue;
    }
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
      const sent = cfg;
      const isEmptyish = !sent || sent === 'null' || sent === 'undefined' || /^\{\s*\}$/.test(sent);
      if (isQuery) {
        buckets.mismatch.push({
          fn,
          key,
          handler: h.handler,
          kind: 'query',
          reason: `后端签名里没有 Query<T> 提取器，前端查询参数被整体忽略：${(pvMark || dvMark || '').slice(0, 60)}`,
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
      if (dvMark) payloadExpr = dvMark; // DELETE-with-data：真正载荷在 config.data 里
    }
    let feExpr = payloadExpr;
    if (isQuery) {
      const p = (payloadExpr || '').trim();
      if (!p) {
        noPayload++;
        continue; // 不带 config 的 GET：无可比对
      }
      // GET/DELETE 的第二实参是 axios config：只有 `params` 键才是查询串，
      // responseType/timeout/signal 等属请求选项，参与比对会产生假阳性。
      // （CFG_BLIND 已在前面拦截；走到这里 pvMark 要么是字符串要么是 null。）
      if (pvMark === null) {
        noPayload++;
        continue; // config 里确实没有 params -> 无查询参数可比
      }
      feExpr = pvMark;
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
          feKeys: [],
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
    // (d) 叶子字段：顶层键名参与比对了，但内层键集从未比对——显式记入未覆盖清单。
    const beLeaves = beFields
      .filter(f => leafTypeText(f.type))
      .map(f => `${f.name}: ${leafTypeText(f.type)}`);
    const feLeaves = fe.keys
      .filter(k => leafTypeText(k.typeText))
      .map(k => `${k.name}: ${leafTypeText(k.typeText)}`);
    if (beLeaves.length || feLeaves.length)
      buckets.uncovered.push({ fn, key, beType: typeText, beLeaves, feLeaves });
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
  }

  const baseline = loadBaseline();
  const fresh = [];
  for (const r of buckets.mismatch) {
    if (!baseline.missing && baseline.entries.has(mismatchKey(r))) buckets.stock.push(r);
    else fresh.push(r);
  }
  const staleBaseline = baseline.missing
    ? []
    : [...baseline.entries].filter(k => !buckets.mismatch.some(r => mismatchKey(r) === k));

  if (process.argv.includes('--json')) {
    // 机器可读输出：给并行修复任务当工单用，避免把清单抄进提示词时抄错或漏项。
    const dump = rs =>
      rs.map(r => ({
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
      }));
    console.log(
      JSON.stringify(
        {
          total: feFunctions.length,
          ok: buckets.ok.length,
          blind: buckets.blind.length,
          uncoveredLeafFields: buckets.uncovered.length,
          stockMismatch: buckets.stock.length,
          mismatches: dump(fresh),
          stockMismatches: dump(buckets.stock),
          baselineStale: staleBaseline,
        },
        null,
        2
      )
    );
    process.exit(fresh.length ? 1 : 0);
  }
  console.log('=== check-api-request: 前端请求载荷 ↔ 后端 Json<T>/Query<T> 字段集 ===');
  console.log(`前端 api 函数总数: ${feFunctions.length}`);
  console.log(`  一致(ok)          : ${buckets.ok.length}`);
  console.log(`  新增失配(mismatch): ${fresh.length}`);
  console.log(`  存量失配(基线待清零): ${buckets.stock.length}`);
  console.log(`  盲区(不可判定)    : ${buckets.blind.length}`);
  console.log(`  叶子字段未覆盖    : ${buckets.uncovered.length}`);
  console.log(`  未定位 handler    : ${buckets.noHandler.length}`);
  console.log(`  无调用点/无载荷(汇总): 不可定位调用 ${noCall}，确无载荷 ${noPayload}`);
  if (baseline.missing)
    console.log(`  [基线] 未找到 api-request-baseline.json -> 所有失配按新增判负（严格模式）`);

  const printMismatch = r => {
    console.log(`  - ${r.fn.file}:${r.fn.line} ${r.fn.name}  ${r.key}  [${r.kind || 'no-body'}]`);
    if (r.beRequired)
      console.log(
        `      后端必填: ${r.beRequired.join(',')}  全部字段: ${(r.beFields || [])
          .map(f => f.name + (f.optional ? '?' : ''))
          .join(',')}`
      );
    console.log(`      后端: ${r.handler} -> ${r.beType}`);
    console.log(
      `      前端: ${r.feFrom || ''} 多余(后端不读)=${((r.cmp && r.cmp.extra) || []).join(',') || '-'}` +
        ` 缺必填=${((r.cmp && r.cmp.missingRequired) || []).join(',') || '-'}` +
        ` 缺可选=${((r.cmp && r.cmp.missingOptional) || []).join(',') || '-'}`
    );
    if (r.reason) console.log(`      说明: ${r.reason}`);
  };

  if (fresh.length) {
    console.log('\n[新增失配 · 请求载荷与后端结构体字段集不一致（判负）]');
    for (const r of fresh) printMismatch(r);
  }
  if (buckets.stock.length) {
    console.log(
      `\n[存量失配 · 已登记 api-request-baseline.json（过渡机制，不判负，待清零；清零项请同步从基线删除）${buckets.stock.length} 条]`
    );
    for (const r of buckets.stock) printMismatch(r);
  }
  if (staleBaseline.length) {
    console.log(
      `\n[基线中已不再复现的存量项 ${staleBaseline.length} 条（请从基线删除，基线只准缩短）]`
    );
    for (const k of staleBaseline.slice(0, 30)) console.log(`  - ${k}`);
  }
  if (buckets.uncovered.length) {
    console.log(
      `\n[未覆盖清单 · 叶子类型字段（内层键集不参与比对，逐条计数不谎称覆盖）${buckets.uncovered.length} 条]`
    );
    for (const r of buckets.uncovered.slice(0, 40))
      console.log(
        `  - ${r.fn.file}:${r.fn.line} ${r.fn.name} ${r.key} -> ${r.beType}` +
          (r.beLeaves.length ? `  后端叶子=${r.beLeaves.join('; ')}` : '') +
          (r.feLeaves.length ? `  前端叶子=${r.feLeaves.join('; ')}` : '')
      );
    if (buckets.uncovered.length > 40) console.log(`  ... 共 ${buckets.uncovered.length} 条`);
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
  if (fresh.length) {
    console.error(
      `FAIL: 新增请求载荷失配 ${fresh.length} 条（存量基线另计 ${buckets.stock.length} 条）`
    );
    process.exit(1);
  }
  console.log(
    `\nOK: 可比对项全部一致或已登记存量基线（盲区 ${buckets.blind.length} 条、叶子未覆盖 ${buckets.uncovered.length} 条已逐条计数，未谎称全覆盖）。`
  );
  void BASE_URL;
  void splitTopLevelRust;
}

const invokedDirectly =
  !!process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) main();

export {
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
  tsFieldsOfBody,
};
