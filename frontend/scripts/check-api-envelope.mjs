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
 *   - serde_json::Value / JsonValue 标注：**不是「判不出」的同义词**，见下「动态载荷还原」。
 *   - 泛型 T / 无法在源码定位字段定义的自定义 struct → 一律「未分类」并让门禁失败
 *     （禁止“解析不到就跳过”的静默放行），除非在下方 EXEMPTIONS 显式登记且写明原因。
 *
 * 动态载荷还原（本轮新增，把 28 条「未分类」全部判完）：
 *   handler 标注 `ApiResponse<serde_json::Value>` 时，进函数体按三种构造形态还原真实载荷——
 *   ① `serde_json::to_value(具体类型)`；② `json!({ "items": .., "total": .. })` 的顶层键；
 *   ③ 变量回溯（`let v = ..`、`let (items, total) = service.list(..)` 元组解构、类型标注 Vec<T>），
 *   必要时按「接收者类型 + impl 归属」跳进 service 函数体继续还原。
 *   handler 符号解析覆盖三种真实写法：本文件 fn、`define_crud_handlers!`/`define_tuple_crud_handlers!`
 *   宏展开（含文件内 `mod generated` + `pub use`）、`pub use crate::handlers::other::*` 转出。
 *
 * 两条不许退让的判责口径（都对应本轮真实踩过的坑）：
 *   1) **禁止按函数名跨文件回退**。曾有 `department_handler::list`（宏生成）被回退匹配到别的模块的
 *      `list`，把返回类型安错，产出 4 条假「失配」；`params.get(..)` 亦曾被当成某 `service::get`，
 *      把 `"page": page` 判成数组键。故：模块内解析不到 → 报 handler 符号未定义（编译期即失败级别）；
 *      方法调用而接收者类型未知 → 直接放弃该项，不猜。
 *   2) **解析不出 = 判负**，不写兜底、不静默跳过；确属动态 JSON 的须进 EXEMPTIONS 并写明人工核对依据。
 *
 * 追溯：`ENVELOPE_DEBUG=<前端 api 函数名>` 打印逐步还原过程；`ENVELOPE_INDEX=Type#fn,..` 探索引。
 *
 * 已知盲区（不判负、逐条计数）：前端把 data 声明为具名 TS interface（269 条）时本门禁不展开其定义，
 * 因此「0 失配」只覆盖内联声明可比对的 101 条，不代表全部消费点已核对。
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

// 「带数组字段」不等于「列表信封」：UserInfo{roles: string[]}、ImportResult{errors: string[]}
// 都是单对象，采购合同详情更是自带 items: 明细行[]。故前后端共用一条判据：
// 数组键是规范列表名或对象带分页标记(total/count)，且对象没有主键 id（有 id 即实体本身）。
const TOTAL_KEYS = ['total', 'count', 'total_count', 'totalCount'];

function shapeOfArrayKeys({ vecKey, hasTotal, hasId, label }) {
  if (!vecKey.length) return { kind: 'single', raw: label };
  if (hasId) return { kind: 'single', raw: `${label} (含 id，按实体处理)` };
  const canon = vecKey.find(k => LIST_KEYS.includes(k));
  if (!canon && !hasTotal)
    return { kind: 'single', raw: `${label} (数组键 ${vecKey.join('/')} 非规范列表名且无分页标记)` };
  const carrier = canon || vecKey[0];
  return {
    kind: 'wrapper',
    carrier,
    raw: label,
    note: vecKey.length > 1 ? `多数组键 ${vecKey.join('/')}` : '',
  };
}

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

// ---------- 动态 JSON（Value / JsonValue）载荷还原 ----------
// 详见文件头「动态载荷还原」：to_value(具体类型) / json! 顶层键 / 变量与元组回溯 / service 递归。
// 解析不出来就保持「未分类 → 判负」，禁止把「读不懂」当成「没问题」。

// 通用：从 idx 处的 open 括号起配平，返回 [闭括号下标, 内部文本]（跳过字符串字面量）
function captureBalanced(src, idx, open, close) {
  if (src[idx] !== open) return null;
  let depth = 0;
  let inStr = null;
  for (let i = idx; i < src.length; i++) {
    const c = src[i];
    if (inStr) {
      if (c === '\\') {
        i++;
        continue;
      }
      if (c === inStr) inStr = null;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      inStr = c;
      continue;
    }
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return [i, src.slice(idx + 1, i)];
    }
  }
  return null;
}

// 按 fn 名提取「函数体文本」（配平大括号），供动态载荷回溯使用
function extractFnBodies(src) {
  const out = {};
  const re = /\bfn\s+([A-Za-z_]\w*)\s*[(<]/g;
  let m;
  while ((m = re.exec(src))) {
    const name = m[1];
    let i = m.index + m[0].length - 1;
    if (src[i] !== '(') continue;
    const cp = captureBalanced(src, i, '(', ')');
    if (!cp) continue;
    // 签名里可能带 `-> Foo { ... }`：函数体起点 = 参数右括号之后的第一个「顶层」'{'
    let j = cp[0] + 1;
    let ang = 0;
    let bkt = 0;
    while (j < src.length) {
      const c = src[j];
      if (c === '<') ang++;
      else if (c === '>') ang--;
      else if (c === '[') bkt++;
      else if (c === ']') bkt--;
      else if (c === '{' && ang <= 0 && bkt <= 0) break;
      else if (c === ';' && ang <= 0 && bkt <= 0) break; // trait 声明，无函数体
      j++;
    }
    if (src[j] !== '{') continue;
    const body = captureBalanced(src, j, '{', '}');
    if (body) out[name] = body[1];
  }
  return out;
}

// 全局 fn 索引：fnName -> [{file, struct, ret, body}]
// struct = 该 fn 所属 `impl Type` 的类型名（自由函数为 null）；用于按接收者类型精确解析调用，
// 避免「跨文件同名函数」误归因（本仓库 `list`/`list_payments` 这类名字遍布几十个模块）。
function buildGlobalFnIndex() {
  const byName = new Map();
  const byStruct = new Map();
  const dirs = [join(BACKEND, 'src', 'handlers'), join(BACKEND, 'src', 'services')];
  for (const d of dirs) {
    for (const f of collectRsFiles(d)) {
      const src = readFileSync(f, 'utf-8');
      const rets = extractReturnTypes(src);
      const bodies = extractFnBodies(src);
      const rel = f.replace(BACKEND + '/', '').replace(/\\/g, '/');
      const impls = findImplRanges(src);
      for (const name of Object.keys(bodies)) {
        const pos = findFnPos(src, name);
        const struct = enclosingImpl(impls, pos);
        const entry = { file: rel, struct, ret: rets[name] || '', body: bodies[name] };
        if (!byName.has(name)) byName.set(name, []);
        byName.get(name).push(entry);
        if (struct) {
          const k = struct + '#' + name;
          if (!byStruct.has(k)) byStruct.set(k, []);
          byStruct.get(k).push(entry);
        }
      }
    }
  }
  return { byName, byStruct };
}

// `impl Foo {` / `impl<T> Foo<T> {` 的文本区间
function findImplRanges(src) {
  const out = [];
  const re = /\bimpl(?:\s*<[^{>]*>)?\s+((?:[A-Za-z_]\w*\s*::\s*)*[A-Z]\w*)(?:\s*<[^{>]*>)?\s*\{/g;
  let m;
  while ((m = re.exec(src))) {
    const open = src.indexOf('{', m.index + m[0].length - 1);
    const cap = captureBalanced(src, open, '{', '}');
    if (cap) out.push({ start: open, end: cap[0], name: m[1].replace(/\s+/g, '') });
    re.lastIndex = cap ? cap[0] : re.lastIndex;
  }
  return out;
}
function findFnPos(src, name) {
  const m = src.match(new RegExp('\\bfn\\s+' + name + '\\s*[(<]'));
  return m ? m.index : -1;
}
function enclosingImpl(impls, pos) {
  if (pos < 0) return null;
  for (const r of impls) if (pos > r.start && pos < r.end) return r.name.split('::').pop();
  return null;
}

// json!({...}) 对象字面量 -> 顶层键集合（arr: true/false/null=确证数组/确证非数组/判不出）
function parseJsonMacroKeys(objText, body, ctx, depth) {
  const parts = splitJsonMacroEntries(objText);
  const keys = [];
  for (const part of parts) {
    const kv = part.match(/^\s*(?:"([^"]+)"|'([^']+)'|([A-Za-z_]\w*))\s*:\s*([\s\S]*)$/);
    if (!kv) continue;
    const key = kv[1] || kv[2] || kv[3];
    const val = kv[4].trim();
    keys.push({ key, val, arr: exprIsArray(val, body, ctx, (depth || 0) + 1) });
  }
  return keys;
}

function splitJsonMacroEntries(objText) {
  const out = [];
  let depth = 0;
  let cur = '';
  let inStr = null;
  for (let i = 0; i < objText.length; i++) {
    const c = objText[i];
    if (inStr) {
      cur += c;
      if (c === '\\') {
        cur += objText[++i];
      } else if (c === inStr) inStr = null;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      inStr = c;
      cur += c;
      continue;
    }
    if (c === '{' || c === '[' || c === '(') depth++;
    else if (c === '}' || c === ']' || c === ')') depth--;
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

// 表达式文本归一：压缩空白 + 去掉点号两侧空白（Rust 链式调用常跨行书写，`service\n .list()` 必须先归一）
function normExpr(s) {
  return (s || '')
    .replace(/^#\s*/, '')
    .replace(/\s+/g, ' ')
    .replace(/\s*\.\s*/g, '.')
    .trim()
    .replace(/\?+$/, '')
    .replace(/,\s*$/, '') // 多行构造常有尾逗号：`to_value(\n  aging_data,\n)`
    .replace(/\.await(?![\w(])/g, '');
}

// 一个表达式序列化后是否为 JSON 数组：true / false / null(判不出=不许猜)
function exprIsArray(expr, body, ctx, depth) {
  if (!expr || depth > MAX_RESOLVE_DEPTH) return null;
  const e = normExpr(expr);
  if (!e) return null;
  if (/^\[\s*(?:[A-Za-z_\d"{]|$)/.test(e) || /^vec!\s*\(/.test(e)) return true;
  if (/^(?:std::vec::)?Vec\s*</.test(e)) return true;
  // `iter().map(..).collect()`：map 链的 collect 目标是 Vec（HashMap 需 tuple 迭代器，写法不同）
  if (/collect::<\s*(?:std::)?Vec\s*</.test(e)) return true;
  if (
    /\.collect\(\)$/.test(e) &&
    /\.(?:map|filter|into_iter|iter|values|keys|cloned|copied)\s*[.(]/.test(e)
  )
    return true;
  if (/\.collect\(\)$/.test(e) && /map\s*\(/.test(e)) return true;
  if (/^([A-Za-z_][\w:]*::)*to_value\s*\(/.test(e) || /^serde_json::to_value\s*\(/.test(e)) {
    const open = e.indexOf('(');
    const inner = captureBalanced(e, open, '(', ')');
    return inner ? exprIsArray(inner[1], body, ctx, depth + 1) : null;
  }
  const jm = e.match(/^(?:serde_json::)?json!\s*\(/);
  if (jm) {
    const open = e.indexOf('(');
    const inner = captureBalanced(e, open, '(', ')');
    if (!inner) return null;
    const arg = inner[1].trim();
    if (arg.startsWith('[')) return true;
    if (arg.startsWith('{')) return false;
    return exprIsArray(arg, body, ctx, depth + 1);
  }
  if (/^"[^"]*"$/.test(e) || /^-?\d+$/.test(e) || e === 'true' || e === 'false') return false;
  if (/\.len\(\)$/.test(e)) return false;
  const id = e.match(/^[A-Za-z_]\w*$/);
  if (id) {
    const b = findBinding(body, id[0]);
    if (!b) return null;
    if (b.annotation) {
      if (/^(?:std::vec::)?Vec\s*</.test(b.annotation)) return true;
      if (/^(?:std::collections::)?(?:Hash|BTree)Map\s*</.test(b.annotation)) return false;
    }
    if (b.tupleIdx != null) {
      const ty = callReturnType(b.expr, body, ctx, depth + 1);
      const el = ty ? tupleElemType(stripWrappers(ty), b.tupleIdx) : null;
      return el ? typeIsArray(el) : null;
    }
    return exprIsArray(b.expr, body, ctx, depth + 1);
  }
  const recvMethod = e.match(/^([A-Za-z_]\w*)(?:::<[^>]*>)?\.([A-Za-z_]\w*)\s*\(/);
  const staticCall = e.match(/^(?:[A-Za-z_][\w:]*::)?([A-Z][\w]*)::([A-Za-z_]\w*)\s*\(/);
  const bareCall = e.match(/^([a-z_]\w*)\s*\(/);
  let callee = null;
  if (recvMethod)
    callee = {
      name: recvMethod[2],
      recvType: varType(body, recvMethod[1]),
      methodLike: true,
    };
  else if (staticCall) callee = { name: staticCall[2], recvType: staticCall[1] };
  else if (bareCall) callee = { name: bareCall[1], recvType: null };
  if (!callee) return null;
  const sh = resolveFnShape(callee, ctx, depth + 1);
  if (!sh) return null;
  if (sh.kind === 'array') return true;
  if (sh.kind === 'wrapper' || sh.kind === 'single' || sh.kind === 'empty') return false;
  return null;
}

// `let (A, B, _) = EXPR` 或 `let A: T = EXPR` -> {expr, tupleIdx?, annotation?}；取最后一次绑定
function findBinding(body, name) {
  if (!body) return null;
  // 元组解构
  const tre = new RegExp('\\blet\\s*\\(([^)]*)\\)\\s*=\\s*', 'g');
  let tm;
  let tupleHit = null;
  while ((tm = tre.exec(body))) {
    const names = tm[1].split(',').map(s => s.trim());
    const idx = names.indexOf(name);
    if (idx >= 0) {
      const start = tm.index + tm[0].length;
      const expr = readUntilStatementEnd(body, start);
      if (expr) tupleHit = { expr, tupleIdx: idx };
    }
  }
  const nre = new RegExp(
    '\\blet\\s+(?:mut\\s+)?' + name + '\\s*(?::\\s*([^=;\\n]{1,140}))?\\s*=\\s*',
    'g'
  );
  let nm;
  let hit = null;
  while ((nm = nre.exec(body))) {
    const start = nm.index + nm[0].length;
    const expr = readUntilStatementEnd(body, start);
    if (expr) hit = { expr, annotation: nm[1] ? nm[1].trim() : null };
  }
  return hit || tupleHit;
}

function readUntilStatementEnd(body, start) {
  let depth = 0;
  let inStr = null;
  let expr = '';
  for (let i = start; i < body.length; i++) {
    const c = body[i];
    if (inStr) {
      expr += c;
      if (c === '\\') {
        expr += body[++i];
      } else if (c === inStr) inStr = null;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      inStr = c;
      expr += c;
      continue;
    }
    if (c === '(' || c === '[' || c === '{' || c === '<') depth++;
    else if (c === ')' || c === ']' || c === '}' || c === '>') depth--;
    else if (c === ';' && depth <= 0) break;
    expr += c;
  }
  return expr.trim() || null;
}

// `let service = <DeptService>::new(..)` / `let svc = DeptService::new(..)` -> 类型名
function varType(body, varName) {
  if (!body) return null;
  // 类型路径各段大小写都要允许：`crate::services::ar_service::ArService::new(..)` 是仓库常见写法
  const re = new RegExp(
    '\\blet\\s+(?:mut\\s+)?' +
      varName +
      '\\s*(?::[^=\\n]{0,80})?\\s*=\\s*<?([A-Za-z_][\\w]*(?:::[A-Za-z_][\\w]*)*)>?\\s*::\\s*new\\s*\\(',
    'g'
  );
  let m;
  let last = null;
  while ((m = re.exec(body))) last = m[1].split('::').pop();
  return last;
}

// 调用表达式的返回类型文本（用于元组元素类型判定）
function callReturnType(expr, body, ctx, depth) {
  if (!expr || depth > MAX_RESOLVE_DEPTH) return null;
  const e = normExpr(expr);
  const recvMethod = e.match(/^([A-Za-z_]\w*)(?:::<[^>]*>)?\.([A-Za-z_]\w*)\s*\(/);
  const staticCall = e.match(/^(?:[A-Za-z_][\w:]*::)?([A-Z][\w]*)::([A-Za-z_]\w*)\s*\(/);
  let name = null;
  let recvType = null;
  let methodLike = false;
  if (recvMethod) {
    name = recvMethod[2];
    recvType = varType(body, recvMethod[1]);
    methodLike = true;
  } else if (staticCall) {
    name = staticCall[2];
    recvType = staticCall[1];
  } else return null;
  if (methodLike && !recvType) return null; // 接收者类型未知 -> 不猜
  const scoped = recvType ? ctx.fnIndex.byStruct.get(recvType + '#' + name) : null;
  if (recvType && !(scoped && scoped.length)) return null; // 类型已知但方法不属它 -> 不猜
  const cands = scoped || ctx.fnIndex.byName.get(name);
  if (!cands || !cands.length) return null;
  const rets = new Set(cands.map(c => stripWrappers(c.ret || '')).filter(Boolean));
  if (rets.size !== 1) return null;
  return [...rets][0];
}

// 在函数体里找 `let [mut ]NAME = EXPR`（含 `let NAME: Type = EXPR`）；找不到返回 null
// （历史实现已由 findBinding 取代：它额外支持元组解构与类型标注）

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
      // SeaORM 实体每个文件都有一个 `pub struct Model`：只按裸名查会把别的实体字段安上来
      const base = f.split(/[\/]/).pop().replace('.rs', '');
      addStructEntry(index, `${base}::${name}`, { vecFields, allFields: fields });
      addStructEntry(index, name, { vecFields, allFields: fields });
    }
  }
  return index;
}

function addStructEntry(index, key, entry) {
  const prev = index[key];
  if (!prev) {
    index[key] = entry;
    return;
  }
  if (prev.ambiguous) return;
  const sig = e => JSON.stringify(e.allFields.map(x => [x.name, x.type]));
  if (sig(prev) !== sig(entry)) index[key] = { vecFields: [], allFields: [], ambiguous: true };
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
    // 圆/方/花括号同样构成嵌套：`Result<(Vec<X>, u64), E>` 若只跟踪 <> 会在元组内部误切
    if (c === '<' || c === '(' || c === '[' || c === '{') depth++;
    else if (c === '>' || c === ')' || c === ']' || c === '}') depth--;
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
  const qualified = head.replace(/^crate::/, '');
  const tail = qualified.split('::').slice(-2).join('::');
  const info = structIndex[tail] || structIndex[qualified] || structIndex[qualified.split('::').pop()];
  if (info && info.ambiguous)
    return { kind: 'unknown', raw: `${p} (同名 struct 多处定义，判不出)` };
  if (info) {
    return shapeOfArrayKeys({
      vecKey: info.vecFields.map(v => v.name),
      hasTotal: info.allFields.some(f => TOTAL_KEYS.includes(f.name)),
      hasId: info.allFields.some(f => f.name === 'id'),
      label: p,
    });
  }

  // 其余：无法归类（泛型 T / 未定位到定义的类型）
  return { kind: 'unknown', raw: p };
}

// ---------- handler 模块符号表（含宏生成与 pub use 转出）----------
// 路由写的是 `department_handler::list`，而 `list` 可能来自 ①本文件 fn ②文件内 `mod generated`
// 里的 define_*_handlers! 宏展开 ③`pub use crate::handlers::other::*` 转出。
// 三者都要能解析；解析不到就报「符号未定义」——那是编译期即失败的 P0，绝不能回退去按名字全局找同名 fn
// （按名回退会把别的模块的返回类型安到本路由头上，制造假判定）。
function loadHandlerMacroTemplates() {
  const p = join(BACKEND, 'src', 'utils', 'crud_macro.rs');
  let src;
  try {
    src = readFileSync(p, 'utf-8');
  } catch {
    return {};
  }
  const out = {};
  const re = /macro_rules!\s+([a-z_]\w*)\s*\{/g;
  let m;
  while ((m = re.exec(src))) {
    const name = m[1];
    const open = src.indexOf('{', m.index + m[0].length - 1);
    const cap = captureBalanced(src, open, '{', '}');
    if (!cap) break;
    out[name] = parseMacroArms(cap[1]);
    re.lastIndex = cap[0];
  }
  return out;
}

function parseMacroArms(body) {
  const arms = [];
  let i = 0;
  while (i < body.length) {
    const lp = body.indexOf('(', i);
    if (lp < 0) break;
    const lhs = captureBalanced(body, lp, '(', ')');
    if (!lhs) break;
    const after = body.slice(lhs[0] + 1);
    const arrow = after.match(/^\s*=>\s*/);
    if (!arrow) {
      i = lhs[0] + 1;
      continue;
    }
    const rb = lhs[0] + 1 + arrow[0].length;
    if (body[rb] !== '{') {
      i = rb;
      continue;
    }
    const rhs = captureBalanced(body, rb, '{', '}');
    if (!rhs) break;
    const metas = [...lhs[1].matchAll(/\$([a-z_]\w*)\s*:\s*(\w+)/g)].map(x => x[1]);
    arms.push({ metas, rhs: rhs[1] });
    i = rhs[0] + 1;
  }
  return arms;
}

function expandMacroArm(arms, argTexts) {
  for (const arm of arms) {
    if (arm.metas.length !== argTexts.length) continue;
    let out = arm.rhs;
    arm.metas.forEach((mn, idx) => {
      out = out.replace(new RegExp('\\$' + mn + '(?![\\w])', 'g'), argTexts[idx]);
    });
    return out;
  }
  return null;
}

function findModRanges(src) {
  const out = [];
  const re = /\b(?:pub\s+)?mod\s+([a-z_]\w*)\s*\{/g;
  let m;
  while ((m = re.exec(src))) {
    const open = src.indexOf('{', m.index + m[0].length - 1);
    const cap = captureBalanced(src, open, '{', '}');
    if (cap) out.push({ name: m[1], start: open, end: cap[0] });
    re.lastIndex = cap ? cap[0] : re.lastIndex;
  }
  return out;
}

function buildHandlerModules(templates) {
  const mods = new Map();
  for (const f of collectRsFiles(join(BACKEND, 'src', 'handlers'))) {
    const src = readFileSync(f, 'utf-8');
    const base = f.split(/[\\/]/).pop().replace('.rs', '');
    const entry = {
      file: base + '.rs',
      rets: extractReturnTypes(src),
      bodies: extractFnBodies(src),
      macroFns: {}, // 顶层宏展开生成的 fn
      modMacroFns: {}, // 文件内 mod 里宏展开生成的 fn
      reexports: [],
    };
    for (const m of src.matchAll(
      /pub\s+use\s+((?:crate::)?[a-z_][\w]*(?:::[a-z_][\w]*)*)\s*::\s*(\*|\{[^}]*\})/g
    )) {
      const path = m[1].replace(/^crate::/, '');
      const what = m[2];
      const names =
        what === '*'
          ? ['*']
          : what
              .slice(1, -1)
              .split(',')
              .map(s =>
                s
                  .trim()
                  .split(/\s+as\s+/)
                  .pop()
              )
              .filter(Boolean);
      entry.reexports.push({ path, names });
    }
    const modRanges = findModRanges(src);
    const invRe = /\bdefine_([a-z_]*?)handlers!\s*\(/g;
    let im;
    while ((im = invRe.exec(src))) {
      const macroName = 'define_' + im[1] + 'handlers';
      const arms = templates[macroName];
      if (!arms) continue;
      const open = src.indexOf('(', im.index + im[0].length - 1);
      const cap = captureBalanced(src, open, '(', ')');
      if (!cap) continue;
      const args = splitTopLevelRust(cap[1]).map(s => s.trim());
      const expanded = expandMacroArm(arms, args);
      if (!expanded) continue;
      const genBodies = extractFnBodies(expanded);
      const genRets = extractReturnTypes(expanded);
      const host = modRanges.find(r => cap[0] > r.start && cap[0] < r.end);
      const target = host ? (entry.modMacroFns[host.name] ||= {}) : entry.macroFns;
      for (const fn of Object.keys(genBodies))
        target[fn] = { ret: genRets[fn] || '', body: genBodies[fn], via: macroName };
      invRe.lastIndex = cap[0];
    }
    mods.set(base, entry);
  }
  return mods;
}

// 解析 `module::fn`：本文件 fn → 本文件宏生成 → 文件内 mod 宏生成(经 pub use) → pub use 转出模块
function resolveHandlerSymbol(mods, mod, fn, depth = 0) {
  if (depth > 3) return null;
  const M = mods.get(mod);
  if (!M) return null;
  if (M.bodies[fn]) return { ret: M.rets[fn] || '', body: M.bodies[fn], via: `${mod}::${fn}` };
  if (M.macroFns[fn]) return { ...M.macroFns[fn], via: `${mod}::${fn}<${M.macroFns[fn].via}>` };
  for (const modName of Object.keys(M.modMacroFns)) {
    const tbl = M.modMacroFns[modName];
    if (tbl[fn]) {
      const reX = M.reexports.find(
        r => r.path.split('::').pop() === modName && (r.names.includes(fn) || r.names.includes('*'))
      );
      if (reX) return { ...tbl[fn], via: `${mod}::${modName}::${fn}<${tbl[fn].via}>` };
    }
  }
  for (const r of M.reexports) {
    if (!r.names.includes(fn) && !r.names.includes('*')) continue;
    const target = r.path.split('::').pop();
    const hit = resolveHandlerSymbol(mods, target, fn, depth + 1);
    if (hit) return { ...hit, via: `${mod}==${r.path}==>${hit.via}` };
  }
  return null;
}

// ---------- 动态载荷回溯解析（Value / JsonValue -> 真实构造形态）----------
// shape: {kind:'array'|'wrapper'|'single'|'empty', carrier?, raw, note?}
const MAX_RESOLVE_DEPTH = 12; // 环由 ctx.visited(DFS 栈)挡，深度只兜底；4 会被「解壳→进 service→再解壳」的正常链长吃光

function shapeFromJsonMacroObject(objText, body, ctx, depth) {
  const keys = parseJsonMacroKeys(objText, body, ctx, depth);
  tr(
    'json!',
    '键=' +
      keys
        .map(k => `${k.key}:${k.arr === true ? '数组' : k.arr === false ? '非数组' : '?'}`)
        .join(' ')
  );
  if (!keys.length) return null;
  const confirmed = keys.filter(k => k.arr === true).map(k => k.key);
  const unknown = keys.filter(k => k.arr == null).map(k => k.key);
  if (!confirmed.length) {
    // 有键的值判不出来（如 `"data": to_value(rows)?`）-> 整体不判定，交给未分类，避免假「单对象」失配
    if (unknown.length) return null;
    return { kind: 'single', raw: 'json!{' + keys.map(k => k.key).join(',') + '}' };
  }
  // 手写顶层信封：handler 返回类型里没有 ApiResponse<，而函数体构造了 {code, data} ——
  // 这与 ApiResponse 的线上形状等价（code=200 + data），因此按 data 的载荷参与比对，
  // 并把「绕过 ApiResponse」记在 note 里（属一致性缺陷，登记而非在此判负）。
  if (ctx.noEnvelope) {
    const codeK = keys.find(k => k.key === 'code');
    const dataK = keys.find(k => k.key === 'data');
    if (codeK && dataK) {
      const inner = resolveExpr(dataK.val, body, ctx, depth + 1);
      if (inner) return { ...inner, note: '手写顶层信封 {code,data}（未经 ApiResponse::success）' };
    }
  }
  return shapeOfArrayKeys({
    vecKey: confirmed,
    hasTotal: keys.some(k => TOTAL_KEYS.includes(k.key)),
    hasId: keys.some(k => k.key === 'id' && k.arr === false),
    label: 'json!{' + keys.map(k => k.key).join(',') + '}',
  });
}

// 解析一个 Rust 表达式文本，得到其序列化后的载荷形态
function resolveExpr(expr, body, ctx, depth = 0) {
  if (!expr || depth > MAX_RESOLVE_DEPTH) return null;
  const e = normExpr(expr)
    .replace(/\.map_err\s*\([\s\S]*\)\s*\??$/, '')
    .trim();
  if (!e) return null;

  // to_value(E)：只认「表达式本身就是 to_value(...)」，否则 `json!({ "items": to_value(list)? })`
  // 会被内层 to_value 抢先命中，把整个信封误判成裸数组。
  if (/^(?:[A-Za-z_][\w:]*::)*to_value\s*\(/.test(e)) {
    const open = e.indexOf('(');
    const inner = captureBalanced(e, open, '(', ')');
    return inner ? resolveExpr(inner[1], body, ctx, depth + 1) : null;
  }

  // json!(...)
  const jm = e.match(/^(?:serde_json::|serde_json::[A-Za-z_]+::)?json!\s*\(/);
  if (jm) {
    const open = e.indexOf('(');
    const inner = captureBalanced(e, open, '(', ')');
    if (!inner) return null;
    const arg = inner[1].trim();
    if (arg.startsWith('{')) {
      const obj = captureBalanced(arg, arg.indexOf('{'), '{', '}');
      return obj ? shapeFromJsonMacroObject(obj[1], body, ctx, depth) : null;
    }
    if (arg.startsWith('[')) return { kind: 'array', raw: 'json![..]' };
    return resolveExpr(arg, body, ctx, depth + 1);
  }

  // 已知分页封装
  if (
    /^(?:crate::|common::|utils::)?(?:[A-Za-z_]\w*::)*PaginatedResponse\s*(?:::new\s*\(|\{)/.test(e)
  )
    return { kind: 'wrapper', carrier: 'items', raw: 'PaginatedResponse' };
  if (/^(?:crate::|common::|utils::)?(?:[A-Za-z_]\w*::)*PageResponse\s*(?:::new\s*\(|\{)/.test(e))
    return { kind: 'wrapper', carrier: 'data', raw: 'PageResponse' };

  // 调用表达式：Type::method(..) / method(..) / recv.method(..)
  const callStatic = e.match(/^(?:[A-Za-z_][\w:]*::)?([A-Z][\w]*)::([A-Za-z_]\w*)\s*\(/);
  const callMethod = e.match(/^([A-Za-z_]\w*)(?:::<[^>]*>)?\.([A-Za-z_]\w*)\s*\(/);
  const callBare = e.match(/^([A-Za-z_]\w*)\s*\(/);
  let callee = null;
  if (callStatic) callee = { name: callStatic[2], recvType: callStatic[1] };
  else if (callMethod)
    callee = {
      name: callMethod[2],
      recvType: varType(body, callMethod[1]),
      methodLike: true,
    };
  else if (callBare && !ctx.structIndex[callBare[1]])
    callee = { name: callBare[1], recvType: null };
  if (callee) {
    const viaCall = resolveFnShape(callee, ctx, depth + 1);
    if (viaCall) return viaCall;
  }

  // `.map(..).collect()` / `.into_iter()...collect()`：Rust 里这就是 Vec（HashMap 需 tuple 迭代器）
  // turbofish 里可能嵌套 <>（`collect::<Vec<_>>()`），故只匹配「collect 起头 + 收尾()」
  if (
    /\.collect\b/.test(e) &&
    /\.(?:map|filter|into_iter|iter|values|keys|cloned|copied)[.(]/.test(e)
  )
    return { kind: 'array', raw: e.slice(0, 48) };

  // 单参包装构造器剥壳：Json(X) / Some(X) / Ok(X)
  const wrap = e.match(/^[A-Z][\w]*\s*\(/);
  if (wrap && !/^Vec\s*</.test(e)) {
    const open = e.indexOf('(');
    const inner = captureBalanced(e, open, '(', ')');
    if (inner) {
      const first = splitTopLevelRust(inner[1])[0];
      if (first && first.trim()) {
        const s = resolveExpr(first, body, ctx, depth + 1);
        if (s) return s;
      }
    }
  }

  // 具名类型构造：Type::new(..) / Type { .. } / Type::default()
  const ctor = e.match(/^(?:[A-Za-z_]\w*::)*([A-Z][\w]*)\s*(?:::new\s*\(|\{|::default\s*\()/);
  if (ctor && ctx.structIndex[ctor[1]]) {
    const c = classifyPayload(ctor[1], ctx.structIndex);
    if (c && c.kind !== 'unknown' && c.kind !== 'opaque')
      return { kind: c.kind, carrier: c.carrier, raw: c.raw };
  }

  // 裸标识符：回溯 let 绑定（含元组解构）
  const id = e.match(/^[A-Za-z_]\w*$/);
  if (id) {
    const b = findBinding(body, id[0]);
    if (!b) return null;
    if (b.tupleIdx != null) {
      const ty = callReturnType(b.expr, body, ctx, depth + 1);
      const el = ty ? tupleElemType(stripWrappers(ty), b.tupleIdx) : null;
      if (el == null) return null;
      if (typeIsArray(el)) return { kind: 'array', raw: `${id[0]}: ${el}` };
      const c = classifyPayload(el.replace(/\s+/g, ' ').trim(), ctx.structIndex);
      return c && c.kind !== 'unknown' && c.kind !== 'opaque'
        ? { kind: c.kind, carrier: c.carrier, raw: c.raw }
        : null;
    }
    return resolveExpr(b.expr, body, ctx, depth + 1);
  }
  // 数组字面量 / vec![]
  if (/^\[/.test(e) || /^vec!\s*\(/.test(e)) return { kind: 'array', raw: e.slice(0, 40) };
  if (ctor) return { kind: 'single', raw: ctor[1] };
  return null;
}

// 解析过程追溯：`ENVELOPE_DEBUG=<前端 api 函数名> node scripts/check-api-envelope.mjs`
// 排查「为什么这个端点判不出」时必须能逐步看到决策点，否则只能靠猜。
let TRACE_FN = process.env.ENVELOPE_DEBUG || '';
const tr = (stage, msg) => {
  if (TRACE_FN && ctxTraceOn) console.log(`    · [${stage}] ${msg}`);
};
let ctxTraceOn = false;

// 按函数名/接收者类型解析其「返回载荷形态」：优先返回类型标注，Value 则进函数体再解析
function resolveFnShape(callee, ctx, depth) {
  tr(
    'callee',
    `${callee.recvType || '?'}::${callee.name}${callee.methodLike ? '' : ' (自由调用)'}`
  );
  if (depth > MAX_RESOLVE_DEPTH) return null;
  const { name, recvType } = callee;
  // 方法调用但接收者类型未知 -> 放弃：按名字全局回退会把 `params.get(..)` 认成某 service::get，
  // 从而给端点安上别人的返回形状（本门禁一度因此把 `"page": page` 判成数组键）。
  if (callee.methodLike && !recvType) return null;
  const scoped = recvType ? ctx.fnIndex.byStruct.get(recvType + '#' + name) : null;
  // 接收者类型已知但该 impl 里查不到此方法 -> 判不出。退回「按名字全局找」会把别的服务的
  // 返回类型安上来（曾把 fixed_asset::Model 判成别的实体的 {items,total} 信封）。
  if (recvType && (!scoped || !scoped.length)) return null;
  const cands = scoped || ctx.fnIndex.byName.get(name);
  if (!cands || !cands.length) return null;
  const shapes = [];
  for (const c of cands) {
    const tag = `${c.file}#${c.struct || '-'}#${name}`;
    if (ctx.visited.has(tag)) continue;
    ctx.visited.add(tag);
    let s = null;
    if (c.ret) {
      const payload = stripWrappers(c.ret);
      const byType = classifyPayload(payload, ctx.structIndex);
      if (byType && byType.kind !== 'unknown' && byType.kind !== 'opaque') {
        s = { kind: byType.kind, carrier: byType.carrier, raw: byType.raw };
      } else {
        const arr = typeIsArray(payload);
        if (arr) s = { kind: 'array', raw: payload };
      }
    }
    if (!s) s = resolveBodyPayload(c.body, ctx, depth + 1);
    ctx.visited.delete(tag);
    if (s) shapes.push(s);
  }
  if (!shapes.length) return null;
  const sig = s => `${s.kind}:${s.carrier || ''}`;
  const distinct = new Set(shapes.map(sig));
  if (distinct.size > 1)
    return {
      kind: 'conflict',
      raw: [...distinct].join(' | '),
      note: `同名函数多处返回形态不一致(${name})`,
    };
  return {
    ...shapes[0],
    note:
      (shapes[0].note ? shapes[0].note + ' ' : '') +
      `经 ${recvType ? recvType + '::' : ''}${name}() 返回体还原`,
  };
}

// 元组返回里的第 i 个元素是否为数组：`(Vec<X>, u64)` -> true
function typeIsArray(t) {
  const s = (t || '').replace(/\s+/g, ' ').trim();
  if (/^(?:std::vec::)?Vec\s*</.test(s) || /^\[\s*/.test(s)) return true;
  return false;
}

function tupleElemType(t, idx) {
  const s = (t || '').replace(/\s+/g, ' ').trim();
  if (!s.startsWith('(') || !s.endsWith(')')) return null;
  const parts = splitTopLevelRust(s.slice(1, -1));
  return parts[idx] ? parts[idx].trim() : null;
}

// 从一个函数体里收集「被返回的载荷」表达式（ApiResponse::success(X) 或 Ok(X)）
function resolveBodyPayload(body, ctx, depth) {
  if (!body || depth > MAX_RESOLVE_DEPTH) return null;
  const exprs = collectArgs(body, /ApiResponse::(?:success|ok)\s*\(/g).concat(
    collectArgs(body, /\bOk\s*\(/g)
  );
  tr(
    'body',
    `返回实参 ${exprs.length} 个: ` +
      exprs.map(x => x.replace(/\s+/g, ' ').slice(0, 110)).join(' | ')
  );
  const shapes = [];
  for (const ex of exprs) {
    const s = resolveExpr(ex, body, ctx, depth + 1);
    if (s) shapes.push(s);
  }
  if (!shapes.length) return null;
  const distinct = new Set(shapes.map(s => `${s.kind}:${s.carrier || ''}`));
  if (distinct.size > 1)
    return { kind: 'conflict', raw: [...distinct].join(' | '), note: '函数体内多处返回形态不一致' };
  return shapes[0];
}

// 收集某调用模式的全部首个实参（配平括号）
function collectArgs(src, re) {
  const out = [];
  let m;
  while ((m = re.exec(src))) {
    const open = src.indexOf('(', m.index + m[0].length - 1);
    if (open < 0) continue;
    const cap = captureBalanced(src, open, '(', ')');
    if (!cap) continue;
    const first = splitTopLevelRust(cap[1])[0];
    if (first && first.trim()) out.push(first.trim());
    re.lastIndex = cap[0];
  }
  return out;
}

function splitTopLevelRust(s) {
  const out = [];
  let depth = 0;
  let inStr = null;
  let cur = '';
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (inStr) {
      cur += c;
      if (c === '\\') {
        cur += s[++i];
      } else if (c === inStr) inStr = null;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      inStr = c;
      cur += c;
      continue;
    }
    if (c === '<' || c === '(' || c === '[' || c === '{') depth++;
    else if (c === '>' || c === ')' || c === ']' || c === '}') depth--;
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

// ---------- 前端具名类型索引（把 `ApiResponse<Foo>` 里的 Foo 展开成可比对的形状）----------
// 此前 269 条以「具名 TS 类型」记为盲区，等价于把最大的一类信封漂移留在看不见的位置。
// 展开规则保守：定义唯一且能确定承载数组的键才参与比对；同名多定义、泛型实参、
// 继承链上有解析不到的父类型 —— 一律退回盲区（判不出就比，只会产出假失配）。
function buildTsTypeIndex() {
  const index = new Map();
  const files = [];
  (function walk(dir) {
    for (const e of readdirSyncTs(dir)) files.push(e);
  })(join(FRONTEND, 'src'));
  for (const f of files) {
    const src = readFileSync(f, 'utf-8');
    const rel = f.replace(FRONTEND, '').replace(/\\/g, '/');
    for (const m of src.matchAll(
      /\b(?:export\s+)?interface\s+([A-Z]\w*)\s*(<[^{>]*>)?\s*(?:extends\s+([A-Za-z_][\w<>,.\s]*?))?\s*\{/g
    )) {
      if (m[2]) continue; // 泛型 interface：元素类型是形参，判不出承载键 -> 不展开
      const open = src.indexOf('{', m.index + m[0].length - 1);
      const cap = captureBalanced(src, open, '{', '}');
      if (!cap) continue;
      addTsEntry(index, m[1], {
        kind: 'object',
        body: cap[1],
        extends: m[3] ? splitObjFields(m[3]).map(s => s.trim()) : [],
        file: rel,
      });
    }
    for (const m of src.matchAll(
      /\b(?:export\s+)?type\s+([A-Z]\w*)\s*(<[^=]*>)?\s*=\s*/g
    )) {
      if (m[2]) continue;
      const text = readUntilStatementEnd(src, m.index + m[0].length) || '';
      const t = text.trim().replace(/;\s*$/, '');
      if (!t) continue;
      if (t.startsWith('{')) addTsEntry(index, m[1], { kind: 'object', body: t.slice(1, -1), extends: [], file: rel });
      else addTsEntry(index, m[1], { kind: 'alias', text: t, file: rel });
    }
  }
  return index;
}

function addTsEntry(index, name, entry) {
  const prev = index.get(name);
  if (!prev) {
    index.set(name, entry);
    return;
  }
  if (prev.kind === 'ambiguous') return;
  const same =
    prev.kind === entry.kind &&
    (prev.body || '') === (entry.body || '') &&
    (prev.text || '') === (entry.text || '');
  if (!same) index.set(name, { kind: 'ambiguous', file: prev.file });
}

function readdirSyncTs(dir) {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) out.push(...readdirSyncTs(p));
    else if (e.name.endsWith('.ts')) out.push(p);
  }
  return out;
}

// 展开具名类型 -> 形状（object 含 ≥1 个数组键才判 wrapper；判不出退回 named 盲区）
function expandTsType(name, index, depth = 0, seen = new Set()) {
  if (depth > 3 || seen.has(name)) return { kind: 'named', raw: name };
  const ent = index.get(name);
  if (!ent || ent.kind === 'ambiguous') return { kind: 'named', raw: name };
  seen.add(name);
  if (ent.kind === 'alias') return classifyFrontendPayload(ent.text, index, depth + 1, seen);
  let body = ent.body;
  for (const par of ent.extends || []) {
    const pname = par.replace(/<.*>/, '').trim();
    const pe = index.get(pname);
    if (!pe || pe.kind !== 'object') return { kind: 'named', raw: `${name} extends ${pname}(未解析)` };
    body = `${pe.body}\n;${body}`;
  }
  return classifyObjectBody(body, index, depth + 1, seen, name);
}

// 对象体 -> 形状（inline 与具名共用一套判定，避免两条口径漂移）
function classifyObjectBody(body, index, depth, seen, label) {
  const top = splitObjFields(body);
  const vecKey = [];
  for (const entry of top) {
    const kv = entry.match(/^\s*([A-Za-z_]\w*)\s*\??\s*:\s*([\s\S]*)$/);
    if (!kv) continue;
    const vt = kv[2].trim();
    if (/\[\]$/.test(vt) || /^Array</.test(vt) || /^\[\s*\]/.test(vt)) vecKey.push(kv[1]);
  }
  const keys = top
    .map(e => (e.match(/^\s*([A-Za-z_]\w*)\s*\??:/) || [])[1])
    .filter(Boolean);
  return shapeOfArrayKeys({
    vecKey,
    hasTotal: keys.some(k => TOTAL_KEYS.includes(k)),
    hasId: keys.includes('id'),
    label: label || '内联对象',
  });
}

// ---------- 前端：解析 api 函数的「声明信封形状」+ 调用的 (method,path) ----------
// 复用 check-api-paths 的 resolveFrontendUrl / normalizePath / constStringMap。
function parseFrontendApiFunctions(tsIndex) {
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
      const feShape = classifyFrontendReturn(retType, tsIndex);
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
function classifyFrontendReturn(retType, tsIndex) {
  // 取 Promise<ApiResponse< INNER >> 的 INNER
  const mm = retType.match(/ApiResponse<([\s\S]*)>\s*$/);
  let inner;
  if (mm) inner = mm[1].trim();
  else {
    const pm = retType.match(/^Promise<([\s\S]*)>\s*$/);
    inner = pm ? pm[1].trim() : retType;
  }
  return classifyFrontendPayload(inner, tsIndex);
}

function classifyFrontendPayload(inner, tsIndex, depth = 0, seen = new Set()) {
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
  if (/^\{/.test(p))
    return classifyObjectBody(p.replace(/^\{/, '').replace(/\}\s*$/, ''), tsIndex, depth, seen, p);
  // Record<...> / 动态映射 -> opaque
  if (/^Record\s*</.test(p)) return { kind: 'opaque', raw: p };
  // 具名类型：展开其定义后参与比对；展开不了（泛型/多定义/父类型未解析）才退回盲区
  const bare = p.match(/^([A-Z]\w*)$/);
  if (bare && tsIndex && depth <= 3) {
    const s = expandTsType(bare[1], tsIndex, depth, seen);
    if (s.kind !== 'named') return { ...s, raw: `${bare[1]} => ${s.raw}` };
    return { kind: 'named', raw: s.raw };
  }
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
  const structIndex = buildStructIndex();
  const fnIndex = buildGlobalFnIndex();
  const handlerMods = buildHandlerModules(loadHandlerMacroTemplates());
  if (process.env.ENVELOPE_INDEX) {
    console.log(`[index] byStruct keys=${fnIndex.byStruct.size} byName=${fnIndex.byName.size}`);
    for (const probe of (process.env.ENVELOPE_INDEX || '').split(',')) {
      const [st, nm] = probe.split('#');
      console.log(
        `[index] ${probe} -> ${JSON.stringify((fnIndex.byStruct.get(st + '#' + nm) || fnIndex.byName.get(nm) || []).map(c => ({ file: c.file, struct: c.struct, ret: (c.ret || '').slice(0, 80), bodyLen: (c.body || '').length })))}`
      );
    }
  }
  const { handlers } = walkBackendRoutes();
  const tsTypeIndex = buildTsTypeIndex();
  const feFunctions = parseFrontendApiFunctions(tsTypeIndex);

  // noEnvelope：handler 返回类型未出现 ApiResponse< —— 供 json! 分支识别「手写顶层信封」
  const mkCtx = noEnvelope => ({ structIndex, fnIndex, visited: new Set(), noEnvelope });

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
    ctxTraceOn = TRACE_FN === fn.name;
    if (ctxTraceOn) console.log(`  [trace] ${fn.name} 端点 ${key} handler=${h.handler}`);
    const parts = String(h.handler || '')
      .split('::')
      .filter(Boolean);
    const sym =
      parts.length >= 2
        ? resolveHandlerSymbol(handlerMods, parts[parts.length - 2], parts[parts.length - 1])
        : null;
    if (!sym) {
      // 模块内既无同名 fn、也非宏生成/转出 -> 编译期即失败级别的缺陷，必须显式暴露
      const listLike = isListLikeKind(fn.feShape.kind);
      results.push({
        fn,
        status: listLike ? 'handler-not-found' : 'skip',
        handler: h.handler,
        key,
        feShape: fn.feShape,
        reason: listLike
          ? `路由 handler 符号 ${h.handler} 在其模块内未定义（非本文件 fn / 非 define_*_handlers! 宏生成 / 非 pub use 转出）`
          : `未定位到返回类型(前端非列表,盲区): ${h.handler}`,
      });
      continue;
    }
    let be = classifyBackendReturn(sym.ret, structIndex);
    // 动态 JSON：进函数体还原真实构造（to_value(具体类型) / json!({...}) / 变量回溯 / service 递归）
    if (be.kind === 'opaque' || be.kind === 'unknown') {
      const resolved = resolveBodyPayload(
        sym.body,
        mkCtx(!/ApiResponse\s*</.test(sym.ret || '')),
        0
      );
      tr(
        'resolve',
        '还原: ' +
          (resolved ? resolved.kind + '/' + (resolved.carrier || '') + ' ' + resolved.raw : 'null')
      );
      if (resolved && resolved.kind !== 'conflict') {
        be = {
          kind: resolved.kind,
          carrier: resolved.carrier,
          raw: `${resolved.raw}${resolved.note ? ' [' + resolved.note + ']' : ''}`,
          resolvedFromBody: true,
        };
      } else if (resolved && resolved.kind === 'conflict') {
        be = { kind: 'unknown', raw: `体内多形态: ${resolved.raw} (${resolved.note})` };
      }
    }
    const cmp = compare(fn.feShape, be);
    results.push({
      fn,
      status: cmp.status,
      handler:
        h.handler +
        (sym.via && !sym.via.endsWith(h.handler.split('::').slice(-2).join('::'))
          ? `  [${sym.via}]`
          : ''),
      key,
      be,
      ret: sym.ret,
      feShape: fn.feShape,
    });
  }

  // ---------- 统计与输出 ----------
  const buckets = {
    ok: [],
    mismatch: [],
    unclassified: [],
    'handler-not-found': [],
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

  if (buckets['handler-not-found'].length) {
    console.log(
      '\n[handler 符号未定义 · 路由引用的 handler 在模块内不存在 —— 编译期即失败，必须先修]'
    );
    for (const r of buckets['handler-not-found']) {
      console.log(`  - ${fmtFE(r)}  端点: ${r.key}`);
      console.log(`      handler  : ${r.handler}`);
      console.log(`      原因     : ${r.reason}`);
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
    `失配: ${buckets.mismatch.length}  |  未分类(未豁免): ${failingUnclassified.length}  |  已豁免: ${buckets.unclassified.length - failingUnclassified.length}  |  handler 符号未定义: ${buckets['handler-not-found'].length}`
  );
  if (
    buckets.mismatch.length ||
    failingUnclassified.length ||
    buckets['handler-not-found'].length
  ) {
    console.error(
      `\nFAIL: 失配 ${buckets.mismatch.length} 条 + 未分类(未豁免) ${failingUnclassified.length} 条 + handler 符号未定义 ${buckets['handler-not-found'].length} 条`
    );
    process.exit(1);
  }
  console.log(
    '\nOK: 无可比对失配、无未豁免未分类（仅统计，不代表覆盖全部消费点，见脚本头盲区说明）。'
  );
}

main();
