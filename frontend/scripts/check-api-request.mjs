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
 *   (multipart 双向形态判定 · 任务板 #249) 旧实现只按后端**签名**认 multipart，且命中即整条
 *       记盲区、从不比对前端实发形态 ⇒ 「后端收 multipart、前端发 JSON」与「前端发 FormData、
 *       后端只收 Json<T>」两个方向的契约错都落盲区（实测各 1 条真错藏身）。现判定为双向：
 *         · 后端证据不再只看签名——签名提取器 `: Multipart`/函数体 `<Multipart as
 *           FromRequest<_>>::from_request`/`.next_field()`/`.get_file()`/`*_from_multipart()`
 *           任一命中即认定（detectBackendMultipart；错误文案里的裸"multipart"字样不算证据）；
 *         · 前端证据——调用点实参内联 `new FormData()`、函数体窗口内 `x = new FormData` 赋值、
 *           形参类型含 FormData，三者任一认定；内联对象/JSON.stringify/非 FormData 具名实参判 json；
 *         · 后端 multipart + 前端 JSON/空体 ⇒ 失配；前端 FormData + 后端 `Json<T>` ⇒ 失配；
 *           两侧同为 multipart ⇒ 记 multipart-ok（键集仍不可比，属残余盲区）；
 *           任一侧形态判不出仍走盲区，禁止猜。`--self-test` 夹具对上述两方向各做正/负自证。
 *
 * 看板 #40 再补的两类（同一红线：判不出=必须显式，绝不能「该报的没报」）：
 *   (e) 箭头函数签名的残余静默丢弃：parseFrontendApiFunctions 的判重键（函数名, path）是
 *       **跨文件全局**的——同名同端点的跨文件副本（如 inventory.ts 与 inventory-transfer.ts
 *       各有一个 approveInventoryTransfer）只会有一个进检查面，另一个无声消失。副本可以
 *       各自漂移（一边修了一边没修），漏掉任何一个就是假绿。现按（文件, path, method）口径
 *       补录（supplementMissedCallSites，实测找回 3 条）。
 *   (f) 复杂类型注解：注解尾注释（拖脏类型文本）、`T[]`/`Array<T>` 数组载荷（含后端
 *       `Json<Vec<T>>` 元素展开）、`Omit/Pick`、联合 `A | B`（逐员合并、必填=各员交集）、
 *       跨文件同名歧义类型（本文件定义优先）、`data ?? {}` 兜底与 `{}` 空体（归 none 走
 *       必填判据）——此前全部落「未处理的类型写法」盲区躲过比对，现已纳入或显式盲区。
 *
 * 存量失配过渡机制：scripts/api-request-baseline.json 登记「当前已知、待清零」的失配项
 * （file:line + 端点为键）。命中基线只列示不判负；**不在基线内的新失配仍立即红**。
 * 这是过渡而非放宽——基线只准缩短，清零后应删除该文件恢复全量阻断。
 *
 * 解析器复用：全部从 check-api-envelope.mjs 导入（同一套 nest 前缀还原、宏展开、pub use 转出、
 * TS 具名类型展开），两处各写一份必然漂移——这是本仓库反复栽过的根因之一。
 */
import { spawnSync } from 'child_process';
import { existsSync, readFileSync, readdirSync } from 'fs';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';
import {
  BASE_URL,
  FRONTEND,
  constStringMap,
  normalizePath,
  resolveFrontendUrl,
  walkBackendRoutes,
} from './check-api-paths.mjs';
import {
  buildHandlerModules,
  buildStructIndex,
  buildTsTypeIndex,
  captureBalanced,
  loadHandlerMacroTemplates,
  parseFrontendApiFunctions,
  readUntilStatementEnd,
  resolveHandlerSymbolPath,
  sigForSymbol,
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
  // Json<Vec<T>> / Query<Vec<T>>：批量端点的真实形状是元素类型。必须先于泛型壳正则试——
  // 否则外层 `([A-Za-z_][\w:]*)` 会把 `Vec` 本身当类型名截走，rustFieldsOf 查不到名为
  // Vec 的 struct -> 整条落「后端字段不可静态解析」盲区，前端 `T[]` 载荷与后端元素字段集
  // 的漂移因此从未被比对（看板 #40 盲区 2-数组形态）。
  const reVec = new RegExp(
    '\\b' +
      wrapper +
      '\\s*(?:\\(\\s*[A-Za-z_][\\w]*\\s*\\)\\s*:\\s*(?:[a-z_]\\w*::)*' +
      wrapper +
      '\\s*)?<\\s*(?:std::vec::)?(?:Vec|Box)\\s*<\\s*([A-Z][\\w:]*)\\s*>\\s*>',
    'g'
  );
  const mv = reVec.exec(sig);
  if (mv) return mv[1];
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

/// 请求体提取器的类型实参。本仓有两种：axum 的 `Json<T>`（体必填）与
/// `utils::optional_json::OptionalJson<T>`（缺体归 None、有体照常字段校验）。
/// 两者比对语义相同——载荷键集都必须等于 T，故都要认；漏认 OptionalJson 会把
/// "前端发了后端不读的键"误判成"后端无体提取器、载荷被整体忽略"。
/// `\b` 使 `Json` 不会命中 `OptionalJson` 的词内子串，两个 wrapper 各自独立匹配。
function jsonBodyType(sig) {
  return (
    extractorType(sig || '', 'Json') ||
    extractorType(sig || '', 'OptionalJson') ||
    untypedJsonBodyType(sig)
  );
}

/// 不限形请求体：`Json<serde_json::Value>` / `OptionalJson<serde_json::Value>` 这类
/// "后端收 JSON 但不声明字段集"的形态。泛型壳正则只认大写开头的类型名，小写路径
/// `serde_json::Value` 会被漏掉，进而被误判成"后端无体提取器、前端载荷被整体忽略"
/// 的失配（print_handler.rs:1521/:1561 实证）。这里显式识别它，交由下游按
/// "字段不可静态解析"归盲区并留原因，不判负也不判通过。
function untypedJsonBodyType(sig) {
  return /(?:\bJson|\bOptionalJson)\s*<\s*(?:std::)?(?:serde_json::)?Value\s*>/.test(sig || '')
    ? 'serde_json::Value'
    : null;
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

// 剥 `//…` 行注释与 `/*…*/` 块注释（字符串字面量内的不误伤）。
// 看板 #40 盲区 2：本仓大量形参注解后拖着一行 `// 后端 xxx_handler::yyy` 说明注释，
// 类型文本被注释污染后走「未处理的类型写法」盲区（fund.ts:50、purchase-price.ts:73 实锤）；
// 更危险的是注释里含 `,`/`)` 会把参数表切歪，连累同一签名里其它可解析形参。
function stripTsComments(s) {
  const out = [];
  let i = 0;
  let q = null; // 当前字符串引号
  while (i < s.length) {
    const c = s[i];
    if (q) {
      out.push(c);
      if (c === '\\') {
        out.push(s[++i] || '');
      } else if (c === q) q = null;
      i++;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') {
      q = c;
      out.push(c);
      i++;
      continue;
    }
    if (c === '/' && s[i + 1] === '/') {
      while (i < s.length && s[i] !== '\n') i++;
      out.push(' ');
      continue;
    }
    if (c === '/' && s[i + 1] === '*') {
      i += 2;
      while (i < s.length && !(s[i] === '*' && s[i + 1] === '/')) i++;
      i += 2;
      out.push(' ');
      continue;
    }
    out.push(c);
    i++;
  }
  return out.join('');
}

// 顶层 `|` 切分（<> {} [] () 与字符串内不切）。用于联合类型逐员展开。
function splitTopLevelBar(s) {
  const out = [];
  let depth = 0;
  let cur = '';
  let q = null;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (q) {
      cur += c;
      if (c === q) q = null;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') {
      q = c;
      cur += c;
      continue;
    }
    if (c === '<' || c === '{' || c === '[' || c === '(') depth++;
    else if (c === '>' || c === '}' || c === ']' || c === ')') depth--;
    if (c === '|' && depth === 0 && s[i + 1] !== '|' && s[i - 1] !== '|') {
      out.push(cur);
      cur = '';
      continue;
    }
    cur += c;
  }
  if (cur.trim()) out.push(cur);
  return out.map(x => x.trim()).filter(Boolean);
}

// 顶层 `,` 切分（<> {} [] () 与字符串内不切）。Omit/Pick 的「基础类型, 键列表」拆分用。
function splitTopLevelCommas(s) {
  const out = [];
  let depth = 0;
  let cur = '';
  let q = null;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (q) {
      cur += c;
      if (c === q) q = null;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') {
      q = c;
      cur += c;
      continue;
    }
    if (c === '<' || c === '{' || c === '[' || c === '(') depth++;
    else if (c === '>' || c === '}' || c === ']' || c === ')') depth--;
    if (c === ',' && depth === 0) {
      out.push(cur.trim());
      cur = '';
      continue;
    }
    cur += c;
  }
  if (cur.trim()) out.push(cur.trim());
  return out.filter(Boolean);
}

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
// 覆盖：具名 interface/type、`Name<Arg>`、Partial<T>/Required<T>、A & B（交叉）、
//       A | B（联合，逐员合并、必填=各员交集）、T[]/Array<T>（剥壳取元素）、
//       Omit<T,K>/Pick<T,K>（展开基础类型后删/留键）、内联对象。
// 原则：解析不了必须带原因返回 blind，绝不返回「部分解析的结果」冒充完整。
// localIndex（看板 #40）：全局 tsIndex 把跨文件同名不同体的类型标为 ambiguous 后整条拒解；
// 调用方所在文件自身的定义会遮蔽 import 的同名类型，故本文件定义优先。
function resolveTsTypeExpr(raw, tsIndex, seen = new Set(), localIndex = null) {
  const t = stripTsComments(String(raw || ''))
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/;+$/, '')
    .trim()
    .replace(/;+$/, '');
  if (!t) return { blind: '类型文本为空' };
  const leaf = leafTypeText(t);
  if (leaf) return { blind: `叶子类型 ${leaf}：键集不可穷举`, leaf };
  const barParts = splitTopLevelBar(t);
  if (barParts.length > 1) {
    // 联合类型：全体员必须可解析；合并键集 = 各员并集，某键只在部分成员出现
    // （或成员内本就可选）时一律置可选——前端只会发其中一个形态，缺键不判失配，
    // 但「后端完全不认识的多余键」仍抓得住。一员解析不出 -> 显式盲区并点名成员。
    const resolved = barParts.map(p => ({
      p,
      r: resolveTsTypeExpr(p, tsIndex, new Set(seen), localIndex),
    }));
    const unresolved = resolved.filter(x => x.r.blind).map(x => `${x.p}（${x.r.blind}）`);
    if (unresolved.length) return { blind: '联合类型含不可解析成员: ' + unresolved.join(' | ') };
    const merged = [];
    for (const x of resolved)
      for (const f of x.r.fields) if (!merged.some(m => m.name === f.name)) merged.push({ ...f });
    if (!merged.length) return { blind: '联合类型展开后无字段' };
    const fields = merged.map(f => {
      const appearsInAll = resolved.every(x => x.r.fields.some(v => v.name === f.name));
      const optInSome = resolved.some(x => x.r.fields.some(v => v.name === f.name && v.optional));
      return { ...f, optional: f.optional || !appearsInAll || optInSome };
    });
    return { fields, from: t.slice(0, 60) };
  }
  const parts = splitTopLevelAmp(t);
  if (parts.length > 1) {
    const merged = [];
    const unresolved = [];
    for (const p of parts) {
      const r = resolveTsTypeExpr(p, tsIndex, new Set(seen), localIndex);
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
    const r = resolveTsTypeExpr(mp[2], tsIndex, seen, localIndex);
    if (r.blind) return r;
    const isPartial = mp[1] === 'Partial';
    return {
      fields: r.fields.map(f => ({ ...f, optional: isPartial ? true : f.optional })),
      from: `${mp[1]}<${mp[2]}>`.slice(0, 60),
    };
  }
  if ((mp = /^(Omit|Pick)<([\s\S]+)>$/.exec(t))) {
    // Omit<T, 'a' | 'b'> / Pick<T, 'a' | 'b'>：展开 T 后按字面量键删/留。
    // 键列表里出现非字符串字面量（计算值、keyof 表达式）-> 显式盲区，不猜。
    const inner = splitTopLevelCommas(mp[2]);
    if (inner.length < 2) return { blind: `未处理的类型写法: ${t.slice(0, 60)}` };
    const r = resolveTsTypeExpr(inner[0], tsIndex, seen, localIndex);
    if (r.blind) return r;
    const keyText = inner.slice(1).join(',');
    const lits = [...keyText.matchAll(/(['"])((?:\\\1|(?!\1)[^\\])*)\1/g)].map(x => x[2]);
    if (keyText.replace(/\s/g, '').length && !lits.length)
      return { blind: `${mp[1]} 的键列表非字符串字面量，不可静态展开: ${keyText.slice(0, 40)}` };
    const set = new Set(lits);
    const fields =
      mp[1] === 'Omit'
        ? r.fields.filter(f => !set.has(f.name))
        : r.fields.filter(f => set.has(f.name));
    if (mp[1] === 'Pick' && set.size !== fields.length)
      return {
        blind: `Pick 列出的键与 ${r.from} 字段对不上（列 ${set.size} 留 ${fields.length}），不猜`,
      };
    return { fields, from: `${mp[1]}<${inner[0]}>`.slice(0, 60) };
  }
  if ((mp = /^(?:readonly\s+)?(?:Array|ReadonlyArray)<([\s\S]+)>$/.exec(t))) {
    // 数组载荷（批量端点的 Array<T> 写法）：剥壳比对**元素**键集。
    const r = resolveTsTypeExpr(mp[1], tsIndex, seen, localIndex);
    if (r.blind) return r;
    return { fields: r.fields, from: `Array<${mp[1]}>`.slice(0, 60), arrayElement: true };
  }
  if (/(?:<|\{|\[|\()$/.test(t)) return { blind: `未处理的类型写法: ${t.slice(0, 60)}` };
  if ((mp = /^([\s\S]+)\[\]$/.exec(t))) {
    // `T[]`：同上按元素比对；顶层 [] 判定在配平尾部检查之后，避免吃掉 `A[] & B` 之类
    // 已在交叉分支处理过的写法。
    const r = resolveTsTypeExpr(mp[1], tsIndex, seen, localIndex);
    if (r.blind) return r;
    return { fields: r.fields, from: `${mp[1]}[]`.slice(0, 60), arrayElement: true };
  }
  if (/^[A-Z]\w*$/.test(t)) {
    const f = tsFieldsOf(t, tsIndex, 0, seen, localIndex);
    if (!f) return { blind: `${t} 无法静态展开（泛型/多定义/索引签名/父类型不可解析）` };
    return { fields: f.fields, from: t };
  }
  if (/^[A-Z]\w*<[A-Za-z_]\w*(\s*,\s*[A-Za-z_]\w*)*>$/.test(t)) {
    const bare = /^([A-Z]\w*)/.exec(t)[1];
    const f = tsFieldsOf(bare, tsIndex, 0, seen, localIndex);
    if (!f) return { blind: `${bare} 无法静态展开（带类型实参）` };
    return { fields: f.fields, from: t };
  }
  if (t.startsWith('{') && t.endsWith('}')) {
    const f = tsFieldsOfBody(
      t.slice(1, -1),
      tsIndex,
      0,
      new Set(seen),
      '(内联对象类型)',
      localIndex
    );
    if (!f) return { blind: `内联对象类型无法展开: ${t.slice(0, 40)}` };
    return { fields: f.fields, from: '内联对象类型' };
  }
  return { blind: `未处理的类型写法: ${t.slice(0, 60)}` };
}

function tsFieldsOf(name, tsIndex, depth = 0, seen = new Set(), localIndex = null) {
  if (depth > 3 || seen.has(name)) return null;
  // 本文件定义优先：同名类型跨文件冲突时全局索引会标 ambiguous 整条拒解，
  // 但调用方文件自己声明的那份是无歧义的（TS 的词义作用域就是它）。
  const ent = (localIndex && localIndex.get(name)) || tsIndex.get(name);
  if (!ent || ent.kind === 'ambiguous') return null;
  const localSeen = new Set(seen);
  localSeen.add(name);
  if (ent.kind === 'alias') {
    // type X = A & B / type X = Partial<Y> / type X = {..}：统一走一个解析口，
    // 避免 alias 分支只认「{...} 或裸名」而把交叉/Partial 无声退回 null。
    const r = resolveTsTypeExpr(ent.text, tsIndex, localSeen, localIndex);
    if (r.blind) return null;
    return { fields: r.fields, raw: ent.text, from: name };
  }
  if (ent.kind !== 'object') return null;
  const self = tsFieldsOfBody(ent.body, tsIndex, depth, localSeen, name, localIndex);
  if (!self) return null;
  const merged = self.fields.slice();
  for (const par of ent.extends || []) {
    const r = resolveTsTypeExpr(par, tsIndex, localSeen, localIndex);
    if (r.blind) return null; // 父类型不可解析 -> 整条盲区（键集不完整就不比对）
    for (const f of r.fields) if (!merged.some(m => m.name === f.name)) merged.push(f);
  }
  return { fields: merged, raw: ent.body, from: name };
}

function tsFieldsOfBody(body, tsIndex, depth, seen, label, localIndex = null) {
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
  // 先剥注释再拆参数表：形参注解后的 `// ...` 说明注释含 `,`/`(` 会把参数切歪，
  // 连累同签名内本可解析的形参（看板 #40 盲区 2）。
  const s = stripTsComments(String(sig || '')).replace(/\s+/g, ' ');
  if (!s.trim()) return null;
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
function feKeysOf(fn, payloadExpr, tsIndex, localIndex = null) {
  const e0 = stripTsComments(String(payloadExpr || ''))
    .replace(/\s+/g, ' ')
    .trim();
  if (!e0) return { kind: 'none' };
  if (/^(undefined|null|void)$/.test(e0)) return { kind: 'none' };
  // `data ?? {}` / `data || {}`：本仓 13 处这种「可空载荷兜底空对象」写法，旧实现按
  // 「既非对象字面量也非单一标识符」整体拒检。语义上等价于「发 data 或什么都不发」，
  // 因此解析 data 本身的类型，并把其全部键置可选（兜底 {} 时任何一个键都可能缺）。
  const alt = /^([A-Za-z_]\w*)\s*(?:\?\?|\|\|)\s*\{\s*\}$/.exec(e0);
  const e = alt ? alt[1] : e0;
  const forceOptional = !!alt;
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
    // `{}` 空对象载荷：不是「解析不出」，而是「确实不发任何键」——按 none 走
    // 「后端有必填却前端空体」判据（看板 #40：旧实现记盲区，5 处从未被检查）。
    return keys.length ? { kind: 'keys', keys } : { kind: 'none' };
  }
  if (/^[A-Za-z_]\w*$/.test(e)) {
    const sig = fn.sig || '';
    if (!sig.trim()) return { kind: 'blind', why: `形参 ${e}：未取到函数签名（解析器未回填 sig）` };
    const pt = paramTypeOf(sig, e);
    if (!pt || !pt.typeText) return { kind: 'blind', why: `形参 ${e} 在签名里找不到类型注解` };
    let t = pt.typeText
      .replace(/\s*\|\s*(?:null|undefined)\s*$/, '')
      .replace(/^\s*(?:null|undefined)\s*\|\s*/, '');
    const r = resolveTsTypeExpr(t, tsIndex, new Set(), localIndex);
    if (r.blind) return { kind: 'blind', why: r.blind };
    const keys = r.fields.map(f =>
      pt.optionalMark || forceOptional ? { ...f, optional: true } : f
    );
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

// ---------- multipart 双向形态判定（任务板 #249） ----------

// 后端 multipart 证据源（本仓实测全部形态，命中任一即认定端点只收 multipart/form-data）：
//  (1) 参数签名提取器 `multipart: Multipart`/`MultipartForm`/`multipart::Form`
//      （crm_handler.rs:558、import_export_handler.rs:705、product_handler.rs:694、
//       system_update_handler.rs:187、user_handler.rs:224）；
//  (2) 签名收 `Request` + 函数体 `<Multipart as FromRequest<_>>::from_request(...)`
//      （supplier_handler.rs:804/:814，手构造是为把拒绝收进 AppError 信封并覆写 body limit）；
//  (3) 函数体字段/文件读取 `.next_field()`/`.get_file()`（(1)(2) 的后果证据，兜底识别）；
//  (4) 经辅助函数间接取 multipart：`extract_*_from_multipart(...)`（system_update_handler.rs:192→:212）。
// 红线：只匹配类型位/调用位，不匹配裸词——supplier_handler.rs:822/826 的中文拒绝文案含
// "multipart" 字样，按词面判会把纯 Json 端点误判成 multipart。
function detectBackendMultipart(sig, body) {
  const s = String(sig || '');
  const b = String(body || '');
  if (
    /(?:^|[,(]\s*)(?:mut\s+)?[a-z_]\w*\s*:\s*(?:axum::extract::)?Multipart\b/.test(s) ||
    /\bMultipartForm\b/.test(s) ||
    /\bmultipart::(?:Form|Multipart)\b/.test(s)
  )
    return '参数签名 Multipart 提取器';
  if (/<\s*Multipart\s+as\s+FromRequest\s*<[^>]*>\s*>::from_request/.test(b))
    return '函数体 FromRequest 构造';
  if (/\.next_field\s*\(/.test(b) || /\.get_file\s*\(/.test(b))
    return '函数体 next_field/get_file 字段读取';
  if (/\b[a-z_]\w*from_multipart\s*\(/.test(b)) return '函数体 multipart 辅助函数调用';
  return '';
}

// 调用点所在 api 文件源码缓存（fePayloadForm 需要函数体窗口判定 `x = new FormData` 赋值）。
const FE_SRC_CACHE = new Map();
function readApiFileSrc(file) {
  if (!FE_SRC_CACHE.has(file)) {
    let txt = null;
    try {
      txt = readFileSync(join(FRONTEND, ...String(file).split('/').filter(Boolean)), 'utf-8');
    } catch {
      txt = null; // 读不到即返回 null，调用方判 unknown——不猜
    }
    FE_SRC_CACHE.set(file, txt);
  }
  return FE_SRC_CACHE.get(file);
}

// 前端实发载荷形态：multipart（FormData 实发）/ json（对象字面量、JSON.stringify、
// 非 FormData 的具名实参）/ none（根本没发）/ unknown（静态判不出，调用方必须落盲区）。
// 证据形态与本仓实测对齐：调用点内联 new FormData()；函数体窗口内 `const x = new FormData()`
// （crm.ts:315、data-import.ts:101、product.ts:275、sku-mapping.ts:146、supplier.ts:320、
//  user-profile.ts:58 全部是这一形态）；形参类型直接标注 FormData。
// 注意：判 json 不依赖 config 的 Content-Type 覆写——手标 multipart 头而载荷是对象，
// 浏览器实际发出的仍是 JSON 文本（无 boundary），后端 multipart 解析照样失败，
// 按载荷本体形态判失配才是对的，头覆写只能当旁证、不能当免检。
function fePayloadForm(fn, payloadExpr, readSrc = readApiFileSrc) {
  const p = stripTsComments(String(payloadExpr || ''))
    .replace(/\s+/g, ' ')
    .trim();
  if (!p || /^(undefined|null|void)$/.test(p)) return { form: 'none', why: '未携带请求体' };
  if (/new\s+FormData\s*\(/.test(p))
    return { form: 'multipart', why: '调用点实参内联 new FormData()' };
  if (/^JSON\.stringify\b/.test(p)) return { form: 'json', why: 'JSON.stringify 显式序列化' };
  if (p.startsWith('{')) return { form: 'json', why: '内联对象字面量' };
  const id = /^[A-Za-z_]\w*$/.exec(p);
  if (!id) return { form: 'unknown', why: '载荷表达式静态不可定位' };
  // 形参优先：若该标识符是带类型注解的形参，作用域铁定属于本函数，先按类型判，
  // 避免同文件他处同名局部变量把形态带偏。
  const pt = paramTypeOf(fn.sig || '', id[0]);
  if (pt && pt.typeText) {
    if (/\bFormData\b/.test(pt.typeText))
      return { form: 'multipart', why: `形参 ${id[0]}: ${pt.typeText.slice(0, 40)}` };
    if (!leafTypeText(pt.typeText) && !/^(any|unknown)$/.test(pt.typeText.trim()))
      return { form: 'json', why: `形参 ${id[0]}: ${pt.typeText.slice(0, 40)}（非 FormData）` };
  }
  // 非形参标识符：在其所属导出函数的体窗口内找 `const x = new FormData` 赋值。
  // 窗口按「函数名声明 → 配平闭括号」定位，不按行号猜——feFunctions 的 fn.line 在三条
  // 解析趟里分别是声明行/调用行（实测 crm.ts:314 与 supplier.ts:322 分属不同趟），
  // 以行为窗会漏掉声明趟函数体内的赋值，把 FormData 实发误判成 unknown。
  const src = readSrc(fn.file);
  if (typeof src === 'string' && fn.name) {
    const decl = new RegExp(
      'export\\s+(?:async\\s+)?(?:function|const|let)\\s+' + fn.name + '\\b'
    ).exec(src);
    if (decl) {
      const brace = src.indexOf('{', decl.index);
      const cap = brace >= 0 ? captureBalanced(src, brace, '{', '}') : null;
      // captureBalanced 返回 [闭括号下标, 括号内文本]，函数体窗口 = 声明起点 → 闭括号
      const win = cap ? src.slice(decl.index, brace + cap[0] + 1) : '';
      if (
        new RegExp('(?:const|let|var)\\s+' + id[0] + '\\s*(?::[^=]*)?=\\s*new\\s+FormData\\b').test(
          win
        )
      )
        return { form: 'multipart', why: `函数体内 ${id[0]} = new FormData()` };
    }
  }
  return { form: 'unknown', why: `载荷标识符 ${id[0]} 形态静态判不出` };
}

// 双向判定（main 与 --self-test 共用同一实现，禁止两处各写一份口径——本仓库反复栽过的根因）：
//  - 后端 multipart + 前端 FormData            -> multipart-ok（键集仍不可比，另列盲区账）
//  - 后端 multipart + 前端 JSON/空体           -> mismatch-backend-expects-multipart（真错）
//  - 前端 FormData + 后端 Json<T>（非 multipart）-> mismatch-frontend-sends-multipart（真错，必 415）
//  - 其余                                       -> n/a（沿用既有 Json/Query/盲区路径）
function multipartContractVerdict(sym, fn, payloadExpr, readSrc = readApiFileSrc) {
  const beMp = detectBackendMultipart(sym.sig, sym.body);
  const fm = fePayloadForm(fn, payloadExpr, readSrc);
  if (beMp && fm.form === 'multipart')
    return { verdict: 'multipart-ok', evidence: `后端证据：${beMp}；前端证据：${fm.why}` };
  if (beMp && (fm.form === 'json' || fm.form === 'none'))
    return { verdict: 'mismatch-backend-expects-multipart', beMp, fm };
  if (!beMp && fm.form === 'multipart') {
    const jt = jsonBodyType(sym.sig || '');
    if (jt) return { verdict: 'mismatch-frontend-sends-multipart', beType: jt, fm };
  }
  return { verdict: 'n/a', beMp, form: fm.form };
}

// ---------- 看板 #40 盲区补录 ----------

// 本文件局部类型索引：全局 buildTsTypeIndex 把「跨文件同名且体不同」的类型标 ambiguous,
// 之后整条拒解（user.ts 与 user-profile.ts 各自定义 ChangePasswordRequest 即此形态,
// 两处 changePassword 的载荷比对因此从未发生）。TS 的语义是本地声明遮蔽 import,
// 所以按「调用方所在文件的定义优先」重建一份文件内索引即可无歧义还原。
function buildLocalTypeIndex(src, rel) {
  const index = new Map();
  for (const m of src.matchAll(
    /\b(?:export\s+)?interface\s+([A-Z]\w*)\s*(<[^{>]*>)?\s*(?:extends\s+([A-Za-z_][\w<>,.\s'"]*?))?\s*\{/g
  )) {
    const open = src.indexOf('{', m.index + m[0].length - 1);
    const cap = open >= 0 ? captureBalanced(src, open, '{', '}') : null;
    if (!cap) continue;
    if (!index.has(m[1]))
      index.set(m[1], {
        kind: 'object',
        body: cap[1],
        extends: m[3] ? splitObjFields(m[3]).map(s => s.trim()) : [],
        file: rel,
      });
  }
  for (const m of src.matchAll(/\b(?:export\s+)?type\s+([A-Z]\w*)\s*(<[^=]*>)?\s*=\s*/g)) {
    if (m[2]) continue; // 泛型 alias 不可实例化，与全局索引同口径
    const text = readUntilStatementEnd(src, m.index + m[0].length) || '';
    const t = text.trim().replace(/;\s*$/, '');
    if (!t) continue;
    if (!index.has(m[1])) index.set(m[1], { kind: 'alias', text: t, file: rel });
  }
  return index;
}

// 被全局去重吞掉的调用点补录：parseFrontendApiFunctions 的判重键是（函数名, path）且跨文件
// 共享——inventory.ts / inventory-transfer.ts 各有一个 approveInventoryTransfer 打同一端点时,
// 后扫到的那个被静默丢弃（看板 #40 实测 3 处：approveInventoryTransfer、purchase.ts 的
// getPurchaseReceiptList/createPurchaseReceipt 与 purchase-receipt.ts 同名同路径副本）。
// 副本之间可以各自漂移（一边修了一边没修），漏掉任何一个就是「该报的没报」。
// 判据用（文件, path, method）：同文件同端点视为已覆盖；跨文件同名同端点必须各自进检查面。
const API_SRCDIR = join(FRONTEND, 'src', 'api');
function collectApiTsFiles(dir) {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) out.push(...collectApiTsFiles(p));
    else if (e.name.endsWith('.ts') && !e.name.endsWith('.d.ts')) out.push(p);
  }
  return out;
}
function supplementMissedCallSites(feFunctions, sources = null) {
  const covered = new Map();
  const mark = f => {
    if (!f.call || !f.call.path) return;
    if (!covered.has(f.file)) covered.set(f.file, new Set());
    covered.get(f.file).add(f.call.path + ' ' + f.call.method);
  };
  feFunctions.forEach(mark);
  let added = 0;
  const files = sources || collectApiTsFiles(API_SRCDIR).map(p => ({ abs: p }));
  for (const item of files) {
    const src = item.src != null ? item.src : readFileSync(item.abs, 'utf-8');
    const consts = constStringMap(src);
    const rel = item.rel || item.abs.replace(FRONTEND, '').replace(/\\/g, '/');
    let set = covered.get(rel);
    if (!set) {
      set = new Set();
      covered.set(rel, set);
    }
    const re = /request\.(get|post|put|delete|patch)\s*[<(]/g;
    let m;
    while ((m = re.exec(src))) {
      const paren = src.indexOf('(', m.index);
      const argCap = paren >= 0 ? captureBalanced(src, paren, '(', ')') : null;
      if (!argCap) continue;
      const argTexts = splitTopLevelRust(argCap[1]).map(a => a.trim());
      const url = resolveFrontendUrl((argTexts[0] || '').trim(), consts);
      if (!url) continue; // URL 不可静态还原：路由存在性由 check-api-paths 专门判负
      let path = url;
      if (!path.startsWith(BASE_URL)) path = BASE_URL + (path.startsWith('/') ? path : '/' + path);
      const call = { method: m[1].toUpperCase(), path: normalizePath(path), args: argTexts };
      if (set.has(call.path + ' ' + call.method)) continue;
      const head = src.slice(0, m.index);
      const nm = [
        ...head.matchAll(/export\s+(?:async\s+)?(?:function|const|let)\s+([A-Za-z_]\w*)/g),
      ].pop();
      if (!nm) continue; // 无归属符号（现仓 0 处）：不猜名字，留给后续挂账
      const name = nm[1];
      const line = head.split('\n').length;
      // 响应侧信封判定不属于本门禁（且该副本的孪生条目已在 envelope 门禁面上），只补载荷侧。
      feFunctions.push({
        name,
        file: rel,
        line,
        feShape: { kind: 'opaque', raw: '(补录条目:仅请求载荷侧参与比对)' },
        retType: '',
        call,
        sig: sigForSymbol(src, nm.index, m.index),
      });
      set.add(call.path + ' ' + call.method);
      added++;
    }
  }
  return added;
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
  const supplemented = supplementMissedCallSites(feFunctions);
  // 本文件类型索引按需构建并缓存（每个 api 文件至多解析一次）
  const localIdxCache = new Map();
  const localIndexOf = file => {
    if (!localIdxCache.has(file)) {
      let srcTxt = '';
      try {
        srcTxt = readFileSync(join(FRONTEND, ...String(file).split('/').filter(Boolean)), 'utf-8');
      } catch {
        srcTxt = '';
      }
      localIdxCache.set(file, srcTxt ? buildLocalTypeIndex(srcTxt, file) : new Map());
    }
    return localIdxCache.get(file);
  };

  const buckets = {
    ok: [],
    mismatch: [], // 新增失配（不在基线）——判负
    stock: [], // 存量失配（在基线）——只列示，待清零
    blind: [],
    uncovered: [], // (d) 叶子类型：顶层名比对了，内层键集没比对——显式入清单
    noHandler: [],
    multipartOk: [], // 后端 multipart + 前端 FormData：形态一致（键集仍不可比，见 #249 残余盲区）
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
    // ---- multipart 双向形态判定（任务板 #249 闭盲区） ----
    // 旧实现只在「后端无 Json/Query 提取器」分支把 multipart 端点整条记盲区，
    // 前端实发形态从不比对 ⇒ 「后端收 multipart、前端发 JSON」与「前端发 FormData、
    // 后端只收 Json<T>」两个方向的契约错都以盲区形态假绿（实测两方向各 1 条真错）。
    // 判据实现与 --self-test 共用 multipartContractVerdict，两处口径不可能漂移。
    if (isBody) {
      const mpv = multipartContractVerdict(sym, fn, dvMark || fn.call.args[1] || '');
      if (mpv.verdict === 'multipart-ok') {
        buckets.multipartOk.push({ fn, key, handler: h.handler, evidence: mpv.evidence });
        continue;
      }
      if (mpv.verdict === 'mismatch-backend-expects-multipart') {
        buckets.mismatch.push({
          fn,
          key,
          handler: h.handler,
          kind: 'body',
          beType: '(Multipart)',
          reason:
            `后端按 multipart/form-data 接收（证据：${mpv.beMp}），前端实发` +
            (mpv.fm.form === 'none' ? '未携带任何请求体' : `非 FormData 载荷（${mpv.fm.why}）`) +
            '——Content-Type 不符，运行期必 415/400',
        });
        continue;
      }
      if (mpv.verdict === 'mismatch-frontend-sends-multipart') {
        buckets.mismatch.push({
          fn,
          key,
          handler: h.handler,
          kind: 'body',
          beType: mpv.beType,
          reason: `前端以 FormData 发送 multipart/form-data（${mpv.fm.why}），后端 ${mpv.beType} 走 axum Json 提取器（仅接受 application/json），运行期必 415`,
        });
        continue;
      }
      // n/a：两侧任一形态静态判不出等，继续走下方既有键集比对/盲区路径（行为不劣化）。
    }
    const wrapper = isBody ? 'Json' : 'Query';
    const typeText = isBody ? jsonBodyType(sym.sig) : extractorType(sym.sig || '', wrapper);
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
      // multipart/字节流端点的证据不一定落在参数位：本仓存在 `request: Request` +
      // 函数体内 `<Multipart as FromRequest<_>>::from_request(...)` 的形态
      // （supplier_handler.rs:795/:805，为把"请求形态非法"的拒绝收进 AppError 信封、
      // 并按全局同值覆写请求体上限）。原先只扫签名 ⇒ 该形态被误判成"前端载荷被整体忽略"
      // 的失配；现两侧都看，且把证据位置写进盲区条目，便于复核而非吞掉。
      const beMultipart = /MultipartForm|[Mm]ultipart|Bytes|Extension</.test(sym.sig || '')
        ? '参数签名'
        : /\bMultipart\b[\s\S]{0,120}from_request|from_request[\s\S]{0,120}\bMultipart\b/.test(
              sym.body || ''
            )
          ? '函数体内的 FromRequest 构造'
          : '';
      if (beMultipart) {
        buckets.blind.push({
          fn,
          key,
          handler: h.handler,
          beType: '(非 Json/Query 提取器)',
          why: `后端走 MultipartForm/Bytes 等提取器（multipart 证据：${beMultipart}），键集比对不适用`,
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
    const fe = feKeysOf(fn, feExpr, tsIndex, localIndexOf(fn.file));
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
          multipartOk: buckets.multipartOk.length,
          multipartOkDetails: buckets.multipartOk.map(r => ({
            file: r.fn.file,
            line: r.fn.line,
            fn: r.fn.name,
            endpoint: r.key,
            evidence: r.evidence,
          })),
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
  console.log(
    `前端 api 函数总数: ${feFunctions.length}（其中补录被全局去重吞掉的调用点 ${supplemented} 条）`
  );
  console.log(`  一致(ok)          : ${buckets.ok.length}`);
  console.log(`  multipart形态一致 : ${buckets.multipartOk.length}`);
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

// ---------- --self-test：multipart 双向判据自证夹具（任务板 #249） ----------
// 四条判据：
//  ① 后端 multipart + 前端 JSON（具名实参/内联对象两种写法）必须被抓；
//  ② 后端 multipart + 前端 FormData（签名提取器形态 与 函数体 FromRequest 形态）必须不被抓；
//  ③ 前端 FormData + 后端只收 Json<T> 必须被抓；后端既无 Multipart 又无 Json（如仅 State）
//     且函数体只在错误文案里出现「multipart」裸词时不得被抓（防词面误报）；
//  ④ 真实仓库回归：既有 ok/stock 计数不得因收紧而变化，新增失配必须逐条落在已派修的
//     真错清单（PENDING_TRUE_ERRORS）内——清单之外的新增一律判 FAIL，防误报混入。
const PENDING_TRUE_ERRORS = [
  'batchImportFabrics /api/v1/erp/products/import POST',
  'importSkuMappings /api/v1/erp/purchase/sku-mappings/import POST',
];
function runSelfTest() {
  const results = [];
  const check = (name, pass, detail) => {
    results.push(pass);
    console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? '  [' + detail + ']' : ''}`);
  };
  const FIX = '/src/api/__self_test__.ts';
  const readSrc = () =>
    [
      'export function uploadForm(file: File) {', // 1
      '  const formData = new FormData();', // 2
      "  formData.append('file', file);", // 3
      "  return request.post('/x/import', formData);", // 4
      '}',
      'export function batchJson(data: Item[]) {', // 6
      "  return request.post('/x/import', data);", // 7
      '}',
      'export function postInline() {', // 8
      "  return request.post('/x/import', { a: 1 });", // 9
      '}',
    ].join('\n');
  // sig 采用 sigForSymbol 同款形态：不带外层括号的参数表文本（实测 fabric.ts 条目即如此）
  const fnForm = { name: 'uploadForm', file: FIX, line: 4, sig: 'file: File' };
  const fnJson = { name: 'batchJson', file: FIX, line: 7, sig: 'data: Item[]' };
  const fnInline = { name: 'postInline', file: FIX, line: 9, sig: '' };
  const symMpSig = {
    sig: 'pub async fn import_x(State(state): State<AppState>, auth: AuthContext, mut multipart: Multipart) -> Result<Json<ApiResponse<X>>, AppError>',
    body: 'let field = multipart.next_field().await;',
  };
  const symMpBody = {
    sig: 'pub async fn upload_att(Path(p): Path<(i32, i32)>, State(s): State<AppState>, auth: AuthContext, request: Request) -> Result<Json<ApiResponse<X>>, AppError>',
    body: 'let mut m = <Multipart as FromRequest<_>>::from_request(request, &()).await;',
  };
  const symJson = {
    sig: 'pub async fn import_mappings(State(s): State<AppState>, auth: AuthContext, Json(req): Json<ImportMappingsRequest>) -> Result<Json<ApiResponse<V>>, AppError>',
    body: 'info!("共 {} 条", req.rows.len());',
  };
  const symWordOnly = {
    // 反例：函数体只在中文拒绝文案里出现 "multipart" 裸词，既无提取器也无字段读取，
    // 且没有 Json 提取器——任何方向都不许判。
    sig: 'pub async fn ping(State(s): State<AppState>, auth: AuthContext) -> Json<ApiResponse<V>>',
    body: 'return Err(AppError::bad_request("上传请求必须是合法的 multipart/form-data（需包含 boundary）"));',
  };
  const v = (sym, fn, expr) => multipartContractVerdict(sym, fn, expr, readSrc).verdict;
  check(
    '① 后端multipart(签名形态)+前端JSON具名实参 -> 必抓',
    v(symMpSig, fnJson, 'data') === 'mismatch-backend-expects-multipart'
  );
  check(
    '① 后端multipart(签名形态)+前端内联对象 -> 必抓',
    v(symMpSig, fnInline, '{ a: 1 }') === 'mismatch-backend-expects-multipart'
  );
  check(
    '① 后端multipart(函数体FromRequest形态)+前端JSON -> 必抓',
    v(symMpBody, fnJson, 'data') === 'mismatch-backend-expects-multipart'
  );
  check(
    '② 后端multipart(签名形态)+前端FormData -> 不抓',
    v(symMpSig, fnForm, 'formData') === 'multipart-ok'
  );
  check(
    '② 后端multipart(函数体FromRequest形态)+前端FormData -> 不抓',
    v(symMpBody, fnForm, 'formData') === 'multipart-ok'
  );
  check(
    '③ 前端FormData+后端Json<T> -> 必抓',
    v(symJson, fnForm, 'formData') === 'mismatch-frontend-sends-multipart'
  );
  check(
    '③负 仅错误文案含"multipart"裸词且无Json提取器 -> 不许判',
    v(symWordOnly, fnForm, 'formData') === 'n/a' &&
      detectBackendMultipart(symWordOnly.sig, symWordOnly.body) === ''
  );
  check('③负 同反例+前端JSON载荷 -> 不许判', v(symWordOnly, fnJson, 'data') === 'n/a');
  // ⑤ 请求体提取器识别覆盖面：本仓"理由选填的动作端点"统一走 OptionalJson<T>
  //    （utils/optional_json.rs）。判定器只认 Json<T> 时，这 29 处会被误报成
  //    "后端无体提取器、前端载荷被整体忽略"的失配（#4677 后实测 26 条假红）。
  check(
    '⑤ OptionalJson<T> 具名载荷 -> 认作体提取器并解出 T',
    jsonBodyType(
      'pub async fn a(State(s): State<AppState>, auth: AuthContext, payload: OptionalJson<InventoryOptimizationRequest>) -> X'
    ) === 'InventoryOptimizationRequest'
  );
  check(
    '⑤ OptionalJson 元组解构形态 -> 认作体提取器并解出 T',
    jsonBodyType(
      'pub async fn b(Path(id): Path<i32>, OptionalJson(req): OptionalJson<CancelInstanceRequest>) -> X'
    ) === 'CancelInstanceRequest'
  );
  check(
    '⑤ OptionalJson<serde_json::Value> 不限形体 -> 认出提取器（不得退化成"无体提取器"判负）',
    jsonBodyType(
      'pub async fn c(Path(id): Path<i32>, body: OptionalJson<serde_json::Value>) -> X'
    ) === 'serde_json::Value'
  );
  check(
    '⑤负 只收 State/Path 且返回类型含 Json<ApiResponse<V>> -> 必须仍为 null',
    jsonBodyType(
      'pub async fn d(State(s): State<AppState>, auth: AuthContext) -> Result<axum::Json<ApiResponse<V>>, AppError>'
    ) === null
  );
  // ④ 真实仓库回归（子进程跑 --json，判据与门禁本体同源）
  const rp = spawnSync(process.execPath, [fileURLToPath(import.meta.url), '--json'], {
    encoding: 'utf-8',
    maxBuffer: 64 * 1024 * 1024,
  });
  let ok4 = false;
  let detail4 = '子进程无输出';
  try {
    const j = JSON.parse(rp.stdout);
    const unknownFresh = j.mismatches
      .map(m => `${m.fn} ${m.endpoint}`)
      .filter(s => !PENDING_TRUE_ERRORS.includes(s));
    ok4 = j.ok >= 500 && j.stockMismatch >= 0 && j.multipartOk >= 5 && unknownFresh.length === 0;
    detail4 = `ok=${j.ok} multipartOk=${j.multipartOk} stock=${j.stockMismatch} fresh=${j.mismatches.length} 判据外新增=${unknownFresh.join('|') || '-'}`;
  } catch (e) {
    detail4 = 'JSON 解析失败: ' + e.message;
  }
  check('④ 真实仓库：存量不劣化且新增失配全部在已派修真错清单内', ok4, detail4);
  const failed = results.filter(x => !x).length;
  console.log(
    failed
      ? `\nSELF-TEST FAIL: ${failed}/${results.length} 条未过`
      : `\nSELF-TEST OK: ${results.length}/${results.length} 条通过`
  );
  process.exit(failed ? 1 : 0);
}

const invokedDirectly =
  !!process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  if (process.argv.includes('--self-test')) runSelfTest();
  else main();
}

export {
  CFG_BLIND,
  buildLocalTypeIndex,
  compareKeys,
  detectBackendMultipart,
  extractorType,
  extractAxiosConfigValue,
  feKeysOf,
  fePayloadForm,
  leafTypeText,
  multipartContractVerdict,
  paramTypeOf,
  resolveTsTypeExpr,
  rustFieldsOf,
  splitTopLevelAmp,
  splitTopLevelBar,
  splitTopLevelCommas,
  stripTsComments,
  supplementMissedCallSites,
  tsFieldsOf,
  tsFieldsOfBody,
};
