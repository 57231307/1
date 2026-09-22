#!/usr/bin/env node
/**
 * check-api-envelope.mjs —— 「前端响应信封形状 ↔ 后端 handler 实际载荷」一致性门禁
 *
 * ⚠️ 当前未接入 CI：本脚本仅作为静态分析工具存在，是否升级为阻断门禁由用户拍板。
 *    运行：`cd frontend && node scripts/check-api-envelope.mjs`（退出码 0=无失配/无未分类）。
 *
 * 背景（本仓库反复出现的假绿缺陷类）：
 *   前端把某列表接口声明为 `ApiResponse<{ items: X[]; total: number }>`，
 *   但后端 handler 实际返回 `Json<ApiResponse<Vec<Model>>>`（data 是裸数组）。
 *   于是视图读 `res.data.items` 恒为 undefined → 列表恒空 → 依赖行的按钮永不渲染，
 *   全链路既不报错也不变红。仓库里约 45 处 `{items|list|data: X[]; ...}` 形态声明，
 *   逐个手工核对不现实，故做成门禁。
 *
 * 解析复用：路由 nest 前缀还原 / method 匹配 / 路径归一 复用 check-api-paths.mjs 的导出函数
 *   （walkBackendRoutes / loadFrontendEndpoints / normalizePath / constStringMap /
 *    resolveFrontendUrl 等），不重复造轮子。注意：路由在 routes/*.rs 里注册时不带
 *    routes/mod.rs 前缀，按整路径 grep 找不到 ≠ 不存在，故必须走 nest 展开后的 handlers 映射。
 *
 * 分类映射（前端声明的数据载荷 ↔ 后端真实载荷）：
 *   - Vec<T>                          → 前端必须是裸数组 ApiResponse<T[]>
 *   - PaginatedResponse<T>{items,total,page,page_size} → 前端 {items,total} 合法
 *   - PageResponse<T>{data,total,...}  → 前端 {data,total} 合法
 *   - RoleListResponse{roles}/UserListResponse{users}/CountListResponse{counts}
 *     及其它「含数组字段」的自定义 struct → 按承载数组的键名比对
 *   - serde_json::Value / HashMap / 泛型 T / 无法在源码定位字段定义的自定义 struct
 *     → 一律「未分类」并让门禁失败（禁止“解析不到就跳过”的静默放行），
 *     除非在下方 EXEMPTIONS 显式登记且写明原因。
 */
import { readFileSync, readdirSync } from 'fs';
import { join } from 'path';
import {
  FRONTEND,
  BACKEND,
  BASE_URL,
  normalizePath,
  constStringMap,
  resolveFrontendUrl,
  collectRsFiles,
  walkBackendRoutes,
} from './check-api-paths.mjs';

const SRCDIR = join(FRONTEND, 'src', 'api');

// 承载数组的“列表信封”键名（前端内联对象类型里出现这些键、且值为数组，即视为列表声明）
const LIST_KEYS = ['items', 'list', 'data', 'roles', 'users', 'counts', 'results'];

// ---------- 后端：handler 函数返回类型索引 ----------
// 从 handler 源码里按 `fn NAME(...)` 提取其返回类型文本（`->` 与函数体 `{` 之间）。
function extractReturnTypes(src) {
  const out = {};
  const re = /\bfn\s+([A-Za-z_]\w*)\s*[(<]/g;
  let m;
  while ((m = re.exec(src))) {
    const name = m[1];
    // 先跳到参数列表的匹配右括号
    let i = m.index + m[0].length - 1; // 指向 '(' 或 '<'
    if (src[i] !== '(') continue; // 带泛型的 fn 少见，这里只处理普通签名
    let depth = 0;
    let inStr = null;
    for (; i < src.length; i++) {
      const c = src[i];
      if (inStr) {
        if (c === '\\') i++;
        else if (c === inStr) inStr = null;
        continue;
      }
      if (c === '"' || c === "'") inStr = c;
      else if (c === '(') depth++;
      else if (c === ')') {
        depth--;
        if (depth === 0) break;
      }
    }
    // i 指向参数右括号；找紧随其后的 `->`
    const after = src.slice(i + 1);
    const arrow = after.match(/^\s*(?:async\s+)?->/);
    if (!arrow) continue;
    let j = i + 1 + arrow[0].length; // 指向 `->` 之后
    // 读到第一个「非嵌套」的 `{`（函数体起点）或 `;`（trait 声明式 fn），angle/paren/bracket 计数
    let ret = '';
    let ang = 0,
      par = 0,
      bkt = 0;
    inStr = null;
    for (; j < src.length; j++) {
      const c = src[j];
      if (inStr) {
        ret += c;
        if (c === '\\') {
          ret += src[++j];
        } else if (c === inStr) inStr = null;
        continue;
      }
      if (c === '"' || c === "'") {
        inStr = c;
        ret += c;
        continue;
      }
      if (c === '<') ang++;
      else if (c === '>') ang--;
      else if (c === '(') par++;
      else if (c === ')') par--;
      else if (c === '[') bkt++;
      else if (c === ']') bkt--;
      else if (c === '{' && ang <= 0 && par <= 0 && bkt <= 0) break;
      else if (c === ';' && ang <= 0 && par <= 0 && bkt <= 0) break;
      ret += c;
    }
    ret = ret.trim();
    if (ret) out[name] = ret;
  }
  return out;
}

function buildHandlerReturnIndex() {
  const byFile = {}; // base -> {fn -> returnTypeStr}
  for (const f of collectRsFiles(join(BACKEND, 'src', 'handlers'))) {
    const base = f.split(/[\\/]/).pop().replace('.rs', '');
    byFile[base] = extractReturnTypes(readFileSync(f, 'utf-8'));
  }
  return byFile;
}

// ---------- 后端：struct 字段索引（判定自定义返回体里承载数组的键名）----------
// 扫遍 backend/src 里所有 `pub struct NAME { ... }` / `struct NAME<T> { ... }`，
// 记录每个 struct 的字段 (name -> type) 与其数组字段集合。
function buildStructIndex() {
  const index = {}; // name -> { vecFields: [{name, elem}], allFields: [{name,type}] }
  for (const f of collectRsFiles(join(BACKEND, 'src'))) {
    const src = readFileSync(f, 'utf-8');
    const re = /\bpub\s+struct\s+([A-Za-z_]\w*)(?:<[^{]*?>)?\s*\{/g;
    let m;
    while ((m = re.exec(src))) {
      const name = m[1];
      let i = m.index + m[0].length - 1; // 指向 '{'
      let depth = 0;
      let inStr = null;
      let body = '';
      for (; i < src.length; i++) {
        const c = src[i];
        if (inStr) {
          body += c;
          if (c === '\\') body += src[++i];
          else if (c === inStr) inStr = null;
          continue;
        }
        if (c === '"' || c === "'" || c === '`') {
          inStr = c;
          body += c;
          continue;
        }
        if (c === '{') depth++;
        else if (c === '}') {
          depth--;
          if (depth === 0) {
            body += c;
            break;
          }
        }
        body += c;
      }
      const fields = parseStructFields(body.slice(1, -1));
      const vecFields = fields
        .filter(fld => /^\s*(?:Vec|std::vec::Vec)\s*</.test(fld.type))
        .map(fld => ({ name: fld.name, elem: genericArg(fld.type) }));
      index[name] = { vecFields, allFields: fields };
    }
  }
  return index;
}

// 解析 struct body 里的 `pub name: Type,` 字段（忽略嵌套泛型里的逗号）
function parseStructFields(body) {
  const fields = [];
  const re = /(?:pub\s+)?([A-Za-z_]\w*)\s*:\s*/g;
  let m;
  while ((m = re.exec(body))) {
    const name = m[1];
    let i = m.index + m[0].length;
    let depth = 0;
    let type = '';
    for (; i < body.length; i++) {
      const c = body[i];
      if (c === '<') depth++;
      else if (c === '>') depth--;
      else if ((c === ',' || c === ';') && depth === 0) break;
      type += c;
    }
    type = type.trim();
    // 跳过明显不是字段的误命中（如方法体里的标识符）：类型需以合法类型字符起头
    if (/^[A-Za-z_(]/.test(type)) fields.push({ name, type });
  }
  return fields;
}

// 取泛型第一个实参：`Vec<RoleResponse>` -> `RoleResponse`
function genericArg(t) {
  const i = t.indexOf('<');
  if (i < 0) return t;
  let depth = 0;
  for (let j = i; j < t.length; j++) {
    if (t[j] === '<') depth++;
    else if (t[j] === '>') {
      depth--;
      if (depth === 0) return t.slice(i + 1, j);
    }
  }
  return t.slice(i + 1);
}

// ---------- 后端返回类型 -> 载荷分类 ----------
// 逐层剥掉 `Result< X , E>`、`Json< X >`、`ApiResponse< X >`，得到真正载荷 X。
function stripWrappers(t) {
  let s = t.trim();
  let changed = true;
  while (changed) {
    changed = false;
    // Result<INNER, Err>
    let mm = s.match(/^Result<([\s\S]*)>$/);
    if (mm) {
      const inner = splitTopLevel(mm[1])[0];
      if (inner) {
        s = inner.trim();
        changed = true;
        continue;
      }
    }
    // Json<INNER> / Json<...>
    mm = s.match(/^(?:axum::)?Json<([\s\S]*)>$/);
    if (mm) {
      s = mm[1].trim();
      changed = true;
      continue;
    }
    // ApiResponse<INNER> / utils::response::ApiResponse<INNER>
    mm = s.match(/(?:[A-Za-z_]\w*::)*ApiResponse<([\s\S]*)>$/);
    if (mm) {
      s = mm[1].trim();
      changed = true;
      continue;
    }
    // Option<INNER>（可空载荷，剥壳看内核）
    mm = s.match(/^(?:std::option::)?Option<([\s\S]*)>$/);
    if (mm) {
      s = mm[1].trim();
      changed = true;
      continue;
    }
  }
  return s;
}

function splitTopLevel(s) {
  const out = [];
  let depth = 0;
  let cur = '';
  for (const c of s) {
    if (c === '<') depth++;
    else if (c === '>') depth--;
    if (c === ',' && depth === 0) {
      out.push(cur);
      cur = '';
      continue;
    }
    cur += c;
  }
  if (cur.trim()) out.push(cur);
  return out;
}

// TS 对象类型字段分隔符可为 `,` 或 `;`；按顶层（<> {} [] () 深度为 0）切分。
function splitObjFields(s) {
  const out = [];
  let depth = 0;
  let cur = '';
  for (const c of s) {
    if (c === '<' || c === '{' || c === '[' || c === '(') depth++;
    else if (c === '>' || c === '}' || c === ']' || c === ')') depth--;
    if ((c === ',' || c === ';') && depth === 0) {
      out.push(cur);
      cur = '';
      continue;
    }
    cur += c;
  }
  if (cur.trim()) out.push(cur);
  return out;
}

const OPAQUE_TYPES = new Set(['Value', 'JsonValue', 'serde_json::Value']);
const SCALAR_TYPES = new Set([
  'String',
  'str',
  '& str',
  'bool',
  'i8',
  'i16',
  'i32',
  'i64',
  'u8',
  'u16',
  'u32',
  'u64',
  'f32',
  'f64',
  'NaiveDate',
  'NaiveDateTime',
]);

// 返回 { kind: 'array'|'wrapper'|'single'|'empty'|'opaque'|'unknown', carrier?, raw }
function classifyBackendReturn(retStr, structIndex) {
  if (!retStr) return { kind: 'unknown', raw: '' };
  const payload = stripWrappers(retStr);
  return classifyPayload(payload, structIndex);
}

function classifyPayload(payload, structIndex) {
  let p = payload.replace(/\s+/g, ' ').trim();
  if (p === '' || p === '()' || p === 'unit' || /^ApiResponse<\s*>\s*$/.test(p))
    return { kind: 'empty', raw: p };

  // 数组
  if (/^(?:std::vec::)?Vec\s*</.test(p) || /^\[/.test(p)) return { kind: 'array', raw: p };

  // 已知的分页封装
  if (/PaginatedResponse\s*</.test(p)) return { kind: 'wrapper', carrier: 'items', raw: p };
  if (/PageResponse\s*</.test(p)) return { kind: 'wrapper', carrier: 'data', raw: p };

  // 动态 / 不透明
  const head = p.split(/[\s<(:]/)[0];
  if (OPAQUE_TYPES.has(head) || OPAQUE_TYPES.has(p)) return { kind: 'opaque', raw: p };
  if (/^(?:std::collections::)?(?:Hash|BTree)Map\s*</.test(p)) return { kind: 'opaque', raw: p };
  if (/^HashMap$/.test(p)) return { kind: 'opaque', raw: p };

  if (SCALAR_TYPES.has(head)) return { kind: 'single', raw: p };

  // 具名 struct：仅当存在「规范列表键」(items/data/list/roles/users/counts/results)
  // 的数组字段时，才视为列表信封；否则（如 UserInfo{permissions}、ImportResult{errors}、
  // ProcessTimeline{nodes} 这类「附带数组字段的单对象」）判为单对象，避免误报为信封失配。
  const structName = head.replace(/^crate::.*::/, '');
  const info = structIndex[structName];
  if (info) {
    const canon = info.vecFields.find(v => LIST_KEYS.includes(v.name));
    if (canon) return { kind: 'wrapper', carrier: canon.name, raw: p };
    if (info.vecFields.length > 1 && !canon)
      return { kind: 'single', raw: p + ' (含非列表数组字段,按单对象处理)' };
    return { kind: 'single', raw: p };
  }

  // 其余：无法归类（泛型 T / 未定位到定义的类型）
  return { kind: 'unknown', raw: p };
}

// ---------- 前端：解析 api 函数的「声明信封形状」+ 调用的 (method,path) ----------
// 复用 check-api-paths 的 resolveFrontendUrl / normalizePath / constStringMap。
function parseFrontendApiFunctions() {
  const list = [];
  // 目录里所有 .ts（含非 api 子目录? 仅 src/api）
  const files = [];
  (function walk(dir) {
    for (const e of readdirSyncLocal(dir)) files.push(e);
  })(SRCDIR);
  for (const f of files) {
    const src = readFileSync(f, 'utf-8');
    const consts = constStringMap(src);
    const rel = f.replace(FRONTEND, '').replace(/\\/g, '/');
    // 逐个 `export function NAME(...) : Promise<...> { body }`
    const fnRe =
      /export\s+(?:async\s+)?function\s+([A-Za-z_]\w*)\s*\(([^)]*)\)\s*:\s*Promise<([\s\S]*?)>\s*\{/g;
    let m;
    while ((m = fnRe.exec(src))) {
      const name = m[1];
      const retType = m[3].trim();
      // 函数体（配平大括号）
      let i = m.index + m[0].length;
      let depth = 1;
      let inStr = null;
      let bodyStart = i;
      for (; i < src.length; i++) {
        const c = src[i];
        if (inStr) {
          if (c === '\\') i++;
          else if (c === inStr) inStr = null;
          continue;
        }
        if (c === "'" || c === '"' || c === '`') inStr = c;
        else if (c === '{') depth++;
        else if (c === '}') {
          depth--;
          if (depth === 0) break;
        }
      }
      const body = src.slice(bodyStart, i);
      // 行号
      const line = src.slice(0, m.index).split('\n').length;
      // (method, url)：取函数体内第一个 request.<method>(...)
      const call = extractFirstCall(body, consts);
      const feShape = classifyFrontendReturn(retType);
      list.push({ name, file: rel, line, feShape, retType, call });
    }
  }
  return list;
}

function extractFirstCall(body, consts) {
  const re = /request\.(get|post|put|delete|patch)(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\s*\(\s*/g;
  const m = re.exec(body);
  if (!m) return null;
  const method = m[1].toLowerCase();
  const start = m.index + m[0].length;
  let i = start;
  let arg = '';
  let quote = null;
  let braceDepth = 0;
  while (i < body.length) {
    const c = body[i];
    if (quote) {
      arg += c;
      if (c === quote) quote = null;
      else if (c === '$' && body[i + 1] === '{' && (quote === '`' || quote === '"')) {
        arg += '{';
        i += 2;
        let d = 1;
        while (i < body.length && d > 0) {
          if (body[i] === '{') d++;
          else if (body[i] === '}') d--;
          arg += body[i];
          i++;
        }
        continue;
      }
      i++;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') {
      quote = c;
      arg += c;
      i++;
      continue;
    }
    if (c === '{') braceDepth++;
    if (c === '}') braceDepth--;
    if ((c === ',' || c === ')') && braceDepth === 0) break;
    arg += c;
    i++;
  }
  const url = resolveFrontendUrl(arg.trim(), consts);
  if (!url) return null;
  let path = url;
  if (!path.startsWith(BASE_URL)) path = BASE_URL + (path.startsWith('/') ? path : '/' + path);
  return { method: method.toUpperCase(), path: normalizePath(path) };
}

// 前端返回类型 -> 载荷分类（复用后端同款分类语义，方便比对）
function classifyFrontendReturn(retType) {
  // 取 Promise<ApiResponse< INNER >> 的 INNER
  const mm = retType.match(/ApiResponse<([\s\S]*)>\s*$/);
  let inner;
  if (mm) inner = mm[1].trim();
  else {
    const pm = retType.match(/^Promise<([\s\S]*)>\s*$/);
    inner = pm ? pm[1].trim() : retType;
  }
  return classifyFrontendPayload(inner);
}

function classifyFrontendPayload(inner) {
  let p = inner.replace(/\s+/g, ' ').trim();
  if (/^(void|null|undefined|never)$/.test(p)) return { kind: 'empty', raw: p };
  if (/^(any|unknown)$/.test(p)) return { kind: 'opaque', raw: p };
  // 裸数组：T[] 或 Array<T>
  if (/\[\]$/.test(p) || /^Array</.test(p) || /^ReadonlyArray</.test(p))
    return { kind: 'array', raw: p };
  // 前端导入的 PaginatedResponse<T>（items/total）/ PageResponse<T>（data/total）
  if (/^PaginatedResponse\b/.test(p)) return { kind: 'wrapper', carrier: 'items', raw: p };
  if (/^PageResponse\b/.test(p)) return { kind: 'wrapper', carrier: 'data', raw: p };
  // 内联对象类型 { items: X[]; total: number } —— 找承载数组的顶层键
  if (/^\{/.test(p)) {
    const top = splitObjFields(p.replace(/^\{/, '').replace(/\}\s*$/, ''));
    const vecKey = [];
    for (const entry of top) {
      const kv = entry.match(/^\s*([A-Za-z_]\w*)\s*\??\s*:\s*([\s\S]*)$/);
      if (!kv) continue;
      const [, key, val] = kv;
      const vt = val.trim();
      const isArrayVal = /\[\]$/.test(vt) || /^Array</.test(vt) || /^\[\s*\]/.test(vt);
      if (isArrayVal) vecKey.push(key);
    }
    if (vecKey.length === 1) return { kind: 'wrapper', carrier: vecKey[0], raw: p };
    if (vecKey.length > 1) {
      const canon = vecKey.find(k => LIST_KEYS.includes(k));
      if (canon) return { kind: 'wrapper', carrier: canon, raw: p };
      return { kind: 'wrapper', carrier: vecKey[0], raw: p + ' (多数组键,取首个)' };
    }
    return { kind: 'single', raw: p };
  }
  // 具名 interface（单对象）或 Record<...> 等：
  //   - Record<...> / 动态映射 -> opaque
  //   - 其余「前端引用了具名 TS 类型」(如 SalesContract / AuditLogListResponse) -> 'named'：
  //     本门禁不展开 TS interface 定义，无法静态判定其是否列表信封，记为盲区（不判负、单独统计）。
  if (/^Record\s*</.test(p)) return { kind: 'opaque', raw: p };
  return { kind: 'named', raw: p };
}

// 需要目录遍历（与 check-api-paths 的 collectTsFiles 同语义：跳过 .d.ts）
function readdirSyncLocal(dir) {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) out.push(...readdirSyncLocal(p));
    else if (e.name.endsWith('.ts') && !e.name.endsWith('.d.ts')) out.push(p);
  }
  return out;
}

// ---------- 显式豁免清单（每条必须写原因；命中即降级为提示而非失败）----------
// 用途：后端确为动态 JSON / 有意裸数组等「静态不可判定但经人工确认无缺陷」的端点。
// 禁止整片前缀/方法批量塞入以掩盖真实漂移。
const EXEMPTIONS = new Map([
  // 例： `${BASE_URL}/xxx GET` -> '原因：后端返回 serde_json::Value 动态结构，前端按 any 消费，已人工确认无 items/data 误读',
]);

// ---------- 比对逻辑 ----------
function describeShape(fe) {
  if (fe.kind === 'array') return '裸数组 X[]';
  if (fe.kind === 'wrapper') return `{${fe.carrier}: X[]}`;
  if (fe.kind === 'single') return '内联非列表对象';
  if (fe.kind === 'named') return '具名 TS 类型(盲区)';
  if (fe.kind === 'empty') return '空';
  if (fe.kind === 'opaque') return '动态/any';
  return '未知';
}
function describeBe(be) {
  if (be.kind === 'array') return '裸数组 Vec<T>';
  if (be.kind === 'wrapper') return `信封 {${be.carrier}}`;
  if (be.kind === 'single') return '单对象 struct';
  if (be.kind === 'empty') return '空';
  if (be.kind === 'opaque') return '动态 JSON (Value/HashMap)';
  if (be.kind === 'unknown') return `未分类 (${be.raw})`;
  return '未知';
}
// 建议修法：以「后端真实载荷」为准绳
function suggestion(fe, be) {
  if (be.kind === 'array') {
    if (fe.kind === 'wrapper')
      return `前端应改为 ApiResponse<X[]>（后端 data 为裸数组），或视图改读 res.data`;
    return `后端返回裸数组，前端消费点对齐数组`;
  }
  if (be.kind === 'wrapper') {
    if (fe.kind === 'array')
      return `前端应改为 ApiResponse<{${be.carrier}: X[]; total: number}>，视图改读 res.data.${be.carrier}`;
    return `前端信封键应为 ${be.carrier}（与后端 struct 字段一致）`;
  }
  if (be.kind === 'single') return '后端返回单对象，前端不应按列表消费';
  if (be.kind === 'opaque') return '后端为动态 JSON，需人工确认 data 结构后补类型或登记豁免';
  return '后端返回类型无法静态归类，需人工核对并补 EXEMPTIONS 或修正声明';
}

// fe 与 be 均为「是否列表」的抽象：array=裸数组, wrapper=带键信封, single=非列表
function isListLike(k) {
  return k === 'array' || k === 'wrapper';
}

function compare(fe, be) {
  // 前端为具名 TS 类型/动态映射/空：本门禁不展开其定义 -> 记为盲区（skip，不判负）。
  if (fe.kind === 'named' || fe.kind === 'opaque' || fe.kind === 'empty') return { status: 'skip' };

  // 前端声明为内联「非列表对象」(single)：后端却是列表 -> 前端按对象读 res.data.x 会落空 -> 失配
  if (!isListLike(fe.kind)) {
    if (isListLike(be.kind)) return { status: 'mismatch' };
    return { status: 'ok' }; // 后端也非列表 -> 一致
  }

  // 前端声明为列表/裸数组：
  if (be.kind === 'unknown') return { status: 'unclassified' };
  if (be.kind === 'opaque') return { status: 'unclassified' }; // 静态不可判定 -> 按未分类处理（除非豁免）
  if (fe.kind === 'array') {
    if (be.kind === 'array') return { status: 'ok' };
    return { status: 'mismatch' };
  }
  // fe wrapper：
  if (be.kind === 'array') return { status: 'mismatch' }; // 核心缺陷：后端裸数组 vs 前端 {items}
  if (be.kind === 'wrapper')
    return fe.carrier === be.carrier ? { status: 'ok' } : { status: 'mismatch' };
  if (be.kind === 'single' || be.kind === 'empty') return { status: 'mismatch' };
  return { status: 'ok' };
}

// ---------- 主流程 ----------
function main() {
  const handlerByFile = buildHandlerReturnIndex();
  const structIndex = buildStructIndex();
  const { handlers } = walkBackendRoutes();
  const feFunctions = parseFrontendApiFunctions();

  const resolveHandlerReturn = handlerRef => {
    if (!handlerRef) return null;
    const parts = handlerRef.split('::').filter(Boolean);
    const fn = parts[parts.length - 1];
    const mod = parts.length >= 2 ? parts[parts.length - 2].replace(/^crate::.*::/, '') : null;
    if (mod && handlerByFile[mod] && handlerByFile[mod][fn]) return handlerByFile[mod][fn];
    // 裸引用或模块名不匹配：在所有 handler 文件里找该 fn
    for (const base of Object.keys(handlerByFile)) {
      if (handlerByFile[base][fn]) return handlerByFile[base][fn];
    }
    return null;
  };

  const isListLikeKind = k => k === 'array' || k === 'wrapper';

  const results = [];
  for (const fn of feFunctions) {
    if (!fn.call) {
      results.push({ fn, status: 'no-call' });
      continue;
    }
    const key = `${fn.call.path} ${fn.call.method}`;
    const h = handlers.get(key);
    if (!h) {
      // 路由不存在（由 check-api-paths 专门覆盖，这里不重复判负）
      results.push({ fn, status: 'route-not-found', key });
      continue;
    }
    const ret = resolveHandlerReturn(h.handler);
    if (ret == null) {
      // 无法定位后端返回类型：前端若按列表消费则不可验证 -> 未分类（失败）；否则盲区 skip。
      const listLike = isListLikeKind(fn.feShape.kind);
      results.push({
        fn,
        status: listLike ? 'unclassified' : 'skip',
        handler: h.handler,
        key,
        feShape: fn.feShape,
        reason: listLike
          ? '前端按列表消费，但未在 handlers 目录定位到返回类型'
          : '未定位到返回类型(前端非列表,盲区)',
      });
      continue;
    }
    const be = classifyBackendReturn(ret, structIndex);
    const cmp = compare(fn.feShape, be);
    results.push({ fn, status: cmp.status, handler: h.handler, key, be, ret, feShape: fn.feShape });
  }

  // ---------- 统计与输出 ----------
  const buckets = {
    ok: [],
    mismatch: [],
    unclassified: [],
    skip: [],
    'no-call': [],
    'route-not-found': [],
  };
  for (const r of results) buckets[r.status].push(r);

  // 分类分布（后端载荷形态）
  const beDist = {};
  for (const r of results) {
    if (!r.be) continue;
    beDist[r.be.kind] = (beDist[r.be.kind] || 0) + 1;
  }
  const feDist = {};
  for (const r of results) {
    const k = r.fn && r.fn.feShape ? r.fn.feShape.kind : 'n/a';
    feDist[k] = (feDist[k] || 0) + 1;
  }

  console.log('=== check-api-envelope: 前端信封声明 ↔ 后端 handler 载荷 ===');
  console.log(`前端 api 函数总数(含返回注解): ${feFunctions.length}`);
  console.log(
    `  其中调用后端已注册路由可比对: ${buckets.ok.length + buckets.mismatch.length + buckets.unclassified.length}`
  );
  console.log(`分类统计：`);
  console.log(`  OK(一致)          : ${buckets.ok.length}`);
  console.log(`  失配(mismatch)    : ${buckets.mismatch.length}`);
  console.log(`  未分类(unclassified): ${buckets.unclassified.length}`);
  console.log(`  盲区(skip,不判负) : ${buckets.skip.length}`);
  console.log(
    `  无比对目标: 无调用 ${buckets['no-call'].length} / 路由未注册 ${buckets['route-not-found'].length}`
  );
  console.log(`后端载荷形态分布: ${JSON.stringify(beDist)}`);
  console.log(`前端声明形态分布: ${JSON.stringify(feDist)}`);

  const fmtFE = r => `${r.fn.file}:${r.fn.line} ${r.fn.name}`;
  const exemptionHit = key => EXEMPTIONS.get(key);

  if (buckets.mismatch.length) {
    console.log('\n[失配 · 前端信封与后端真实载荷不一致 —— 需核对/修复]');
    for (const r of buckets.mismatch) {
      console.log(`  - ${fmtFE(r)}`);
      console.log(`      端点     : ${r.key}`);
      console.log(`      后端 handler: ${r.handler}  ->  ${r.ret}`);
      console.log(`      前端声明 : ${describeShape(r.feShape)}   (${r.fn.retType})`);
      console.log(`      后端载荷 : ${describeBe(r.be)}`);
      console.log(`      建议修法 : ${suggestion(r.feShape, r.be)}`);
    }
  }

  if (buckets.unclassified.length) {
    console.log(
      '\n[未分类 · 后端载荷无法静态归类/动态 JSON，或前端按列表消费却无法验证 —— 默认失败，除非显式豁免]'
    );
    for (const r of buckets.unclassified) {
      const ex = exemptionHit(r.key);
      const tag = ex ? '已豁免' : '未豁免';
      console.log(`  - [${tag}] ${fmtFE(r)}  端点: ${r.key}`);
      if (r.handler)
        console.log(`      后端 handler: ${r.handler}${r.ret ? '  ->  ' + r.ret : ''}`);
      if (r.feShape && r.be)
        console.log(`      前端声明 : ${describeShape(r.feShape)}  后端载荷: ${describeBe(r.be)}`);
      else if (r.feShape) console.log(`      前端声明 : ${describeShape(r.feShape)}`);
      if (r.reason) console.log(`      原因     : ${r.reason}`);
      if (ex) console.log(`      豁免说明 : ${ex}`);
    }
  }

  if (buckets.skip.length) {
    // 盲区：前端用具名 TS 类型/动态映射消费，本门禁不展开其定义，无法静态判定信封形状。
    // 不判负，但逐条列出以显式承覆盖边界（禁止把未覆盖说成已覆盖）。
    console.log(
      `\n[盲区 · 前端用具名类型/动态结构消费，未静态展开(不判负): ${buckets.skip.length} 条]`
    );
    const byReason = {};
    for (const r of buckets.skip) {
      const kk = r.feShape ? describeShape(r.feShape) : r.reason || 'skip';
      byReason[kk] = (byReason[kk] || 0) + 1;
    }
    for (const [k, n] of Object.entries(byReason)) console.log(`  ${k}: ${n}`);
  }

  const failingUnclassified = buckets.unclassified.filter(r => !exemptionHit(r.key));

  console.log('\n---- 结论 ----');
  console.log(
    `失配: ${buckets.mismatch.length}  |  未分类(未豁免): ${failingUnclassified.length}  |  已豁免: ${buckets.unclassified.length - failingUnclassified.length}`
  );
  if (buckets.mismatch.length || failingUnclassified.length) {
    console.error(
      `\nFAIL: 失配 ${buckets.mismatch.length} 条 + 未分类(未豁免) ${failingUnclassified.length} 条`
    );
    process.exit(1);
  }
  console.log(
    '\nOK: 无可比对失配、无未豁免未分类（仅统计，不代表覆盖全部消费点，见脚本头盲区说明）。'
  );
}

main();
