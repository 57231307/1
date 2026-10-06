#!/usr/bin/env node
/**
 * check-api-envelope.mjs —— 「前端响应信封形状 ↔ 后端 handler 实际载荷」一致性门禁
 *
 * ⚠️ 状态更新：本脚本已接入 CI（.github/workflows/ci-cd.yml · ci-static-checks · 阻断）。
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
import { join, resolve } from 'path';
import { fileURLToPath } from 'url';
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

// 分页/计数键：这些与数组键并列时是「列表信封」的特征，不算兄弟业务字段
const PAGE_KEYS = [
  'total',
  'count',
  'page',
  'page_size',
  'current',
  'size',
  'pages',
  'limit',
  'offset',
  'has_more',
];

function shapeOfArrayKeys({ vecKey, hasTotal, hasId, label, allKeys }) {
  if (!vecKey.length) return { kind: 'single', raw: label };
  if (hasId) return { kind: 'single', raw: `${label} (含 id，按实体处理)` };
  const canon = vecKey.find(k => LIST_KEYS.includes(k));
  // 详情聚合体：{bom, items} / {user, roles} 这类「单对象 + 内嵌行数组」不是列表信封。
  // 判据是「有规范数组键、却无分页标记、还带着非分页的兄弟字段」，
  // 否则会把 get_xxx_by_id 的详情响应误判成信封，制造假失配。
  if (canon && !hasTotal && allKeys) {
    const extra = allKeys.filter(
      k => !vecKey.includes(k) && !PAGE_KEYS.includes(k) && k !== 'code' && k !== 'message'
    );
    if (extra.length)
      return {
        kind: 'single',
        raw: `${label} (兄弟字段 ${extra.join('/')} + 无分页标记，按详情聚合体处理)`,
      };
  }
  if (!canon && !hasTotal)
    return {
      kind: 'single',
      raw: `${label} (数组键 ${vecKey.join('/')} 非规范列表名且无分页标记)`,
    };
  const carrier = canon || vecKey[0];
  // 声明侧同时给出多个规范列表键（如 PageResult 的 data/list/items/users）时，
  // 该类型"怎么返回都算对"，比对它没有意义 —— 记为 ambiguous 交由 compare() 判未分类，
  // 不能在这里替它挑一个承载键（那就是本门禁要消灭的猜测）。
  const canonKeys = vecKey.filter(k => LIST_KEYS.includes(k));
  return {
    kind: 'wrapper',
    carrier,
    raw: label,
    ambiguous: canonKeys.length > 1 ? canonKeys : undefined,
    note: vecKey.length > 1 ? `多数组键 ${vecKey.join('/')}` : '',
  };
}

// ---------- Rust 词法扫描（Rust 词法切分不再"猜深度"） ----------
// 旧实现的 Rust 侧切分/配平一律按字符猜深度，三条系统性错源（本轮逐条实测，样本见
// 下列 ①②③ 逐条）：
//   ① `'` 一律当字符串开引号 —— 生命周期 `&'a str` / `&'static str` / `<'_>` 会把后面
//      最近的真引号之间整段当字符串，参数右括号 `)` 被吞 ⇒ `captureBalanced` 返回 null，
//      签名/返回类型整条丢，落到「后端无体提取器」「未定位到 handler 签名」的假红；
//   ② 完全不认注释 —— `//` 里的逗号被当顶层分隔、里的 `>` 把深度打成负数，
//      宏实参数组数量与 `macro_rules` 元变量数量不符 ⇒ 宏展开整体丢失；
//      注释里的一个 `"` 还能把后半段全部"吃掉"（块注释跨行，`inStr` 永不复位）；
//   ③ `->` / `=>` / 比较运算符的 `>` 全被当成尖括号闭合 —— 深度错乱后顶层逗号切不出来。
// 全仓只留这一份 Rust 词法扫描，Rust 侧所有切分/配平都必须走它（两处各写一份必然漂移）。
const RUST_XID = /[A-Za-z0-9_]/;

/// 从 i 起若是一个「不可分割词法单元」的起点（行注释 / 块注释(可嵌套) / 字符串 /
/// 原始字符串 r#""# / 字节串 / 字符字面量 / 生命周期），返回其结束下标（不含）；否则 -1。
/// 生命周期与字符字面量的区分按 Rust 词法规则：`'` 后恰好一个字符（或一个转义序列）再接
/// `'` 才是字符字面量，否则是生命周期（`'a` / `'static` / `'_`）。
function rustLexemeEnd(s, i) {
  const c = s[i];
  const prevIsXid = i > 0 && RUST_XID.test(s[i - 1]);
  if (c === '/' && s[i + 1] === '/') {
    let j = i;
    while (j < s.length && s[j] !== '\n') j++;
    return j;
  }
  if (c === '/' && s[i + 1] === '*') {
    let j = i + 2;
    let level = 1;
    while (j < s.length && level > 0) {
      if (s[j] === '/' && s[j + 1] === '*') {
        level++;
        j += 2;
      } else if (s[j] === '*' && s[j + 1] === '/') {
        level--;
        j += 2;
      } else j++;
    }
    return j;
  }
  // 原始字符串（可带 b 前缀）：r"…" / r#"…"# / br#"…"#
  if (!prevIsXid && (c === 'r' || c === 'b')) {
    const raw = /^(?:b)?r(#+)?"/.exec(s.slice(i));
    if (raw) {
      const hashes = raw[1] || '';
      const start = i + raw[0].length;
      const end = s.indexOf('"' + hashes, start);
      return end < 0 ? s.length : end + 1 + hashes.length;
    }
    // 字节串 b"…"
    if (c === 'b' && s[i + 1] === '"') {
      let j = i + 2;
      while (j < s.length) {
        if (s[j] === '\\') {
          j += 2;
          continue;
        }
        if (s[j] === '"') return j + 1;
        j++;
      }
      return j;
    }
  }
  if (c === '"') {
    let j = i + 1;
    while (j < s.length) {
      if (s[j] === '\\') {
        j += 2;
        continue;
      }
      if (s[j] === '"') return j + 1;
      j++;
    }
    // 未闭合字符串（被截断的片段）：吃到结尾，宁可多吞也不许把串内逗号当分隔
    return j;
  }
  if (c === "'") {
    if (s[i + 1] === '\\') {
      let j = i + 2;
      if (s[j] === 'u') {
        const close = s.indexOf('}', j);
        if (close < 0) return s.length;
        j = close + 1;
      } else j += 1;
      if (s[j] === "'") return j + 1;
      return i + 1; // 不是合法字符字面量 -> 只吃引号本身，按普通字符继续扫
    }
    if (s[i + 2] === "'") return i + 3; // 'x'
    let j = i + 1;
    while (j < s.length && RUST_XID.test(s[j])) j++;
    return j > i + 1 ? j : i + 1; // 生命周期 'a / 'static / '_；孤立引号只吃自己
  }
  return -1;
}

/// 宏调用 / 泛型实参表这类「定界组」的结束下标（含闭合符）；未配平返回 -1。
/// 组内走同一套词法扫描：字符串/注释/生命周期里的定界符不参与计数。
function macroGroupEnd(s, k, open) {
  const close = open === '(' ? ')' : open === '[' ? ']' : '}';
  let depth = 0;
  let i = k;
  while (i < s.length) {
    const e = rustLexemeEnd(s, i);
    if (e > i) {
      i = e;
      continue;
    }
    const c = s[i];
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return i + 1;
    }
    i++;
  }
  return -1;
}

/// 从 i 处的 `<` 起，找它作为泛型实参表闭合 `>` 的结束下标（含 `>`）；配不出来返回 -1。
/// 只有「配得平」的 `<…>` 才被当作一组原子消费 —— 比较运算符 / `->` / `=>` 里的 `<`、`>`
/// 不再污染深度（旧实现把 `->` 的 `>` 计入，深度被打成负数后同签名内其后的顶层逗号
/// 永远切不出来，实测把 3 个形参读成 1 个、把 `Json<T>` 判成「后端无体提取器」）。
/// 嵌套 `<`（`HashMap<String, Vec<i32>>`）按同层计数；生命周期 `'_` / `'a` 由词法扫描跳过。
function angleGroupEnd(s, i) {
  let angle = 0;
  let j = i;
  while (j < s.length) {
    const e = rustLexemeEnd(s, j);
    if (e > j) {
      j = e;
      continue;
    }
    const c = s[j];
    if (c === '<') angle++;
    else if (c === '>') {
      // `->` / `=>` 里的 `>` 不是泛型表闭合符（trait 返回值 `Box<dyn Fn(A) -> B, C>` 是真实写法）
      const p = j > 0 ? s[j - 1] : '';
      if (p === '-' || p === '=') {
        j++;
        continue;
      }
      angle--;
      if (angle === 0) return j + 1;
    }
    j++;
  }
  return -1;
}

/// 通用配平：skip(i) 返回词法单元结束下标（不是单元起点时返回 <= i 的值）。
/// 返回 [闭合下标, 内部文本]；起点不是 open 或未配平 -> null（调用方必须按判不出处理）。
function balancedScanWith(src, idx, open, close, skip) {
  if (src[idx] !== open) return null;
  let depth = 0;
  for (let i = idx; i < src.length; i++) {
    const e = skip(src, i);
    if (e > i) {
      i = e - 1;
      continue;
    }
    const c = src[i];
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return [i, src.slice(idx + 1, i)];
    }
  }
  return null;
}

/// Rust 源码/类型文本用这一份：注释、字符串、原始串、字节串、字符字面量、生命周期全部跳过。
function captureBalancedRust(src, idx, open, close) {
  return balancedScanWith(src, idx, open, close, rustLexemeEnd);
}

/// 通用（TS/JS 文本用）：`'` `"` `\\`` 字符串与 `//` `/*` 注释跳过。
/// 旧实现把反引号当字符串起点、把 Rust 生命周期当字符串起点 —— 后者是 #251 的根因，
/// 前者是 JS 模板串，两者语义不同，必须按语言分开。
function jsLexemeEnd(s, i) {
  const c = s[i];
  if (c === '/' && s[i + 1] === '/') {
    let j = i;
    while (j < s.length && s[j] !== '\n') j++;
    return j;
  }
  if (c === '/' && s[i + 1] === '*') {
    let j = i + 2;
    while (j < s.length && !(s[j] === '*' && s[j + 1] === '/')) j++;
    return Math.min(j + 2, s.length);
  }
  if (c === '"' || c === "'" || c === '`') {
    let j = i + 1;
    while (j < s.length) {
      if (s[j] === '\\') {
        j += 2;
        continue;
      }
      if (s[j] === c) return j + 1;
      if (c === '`' && s[j] === '$' && s[j + 1] === '{') {
        // 模板串插值：整段插值按配平吃掉，内部的引号/括号都不影响外层判定
        let d = 1;
        j += 2;
        while (j < s.length && d > 0) {
          if (s[j] === '{') d++;
          else if (s[j] === '}') d--;
          else if (s[j] === '"' || s[j] === "'" || s[j] === '`') {
            const q = s[j];
            j++;
            while (j < s.length && s[j] !== q) {
              if (s[j] === '\\') j++;
              j++;
            }
          }
          if (d > 0) j++;
        }
      }
      j++;
    }
    return j;
  }
  return -1;
}

// TS/JS 文本专用配平（Rust 侧一律用 captureBalanced = captureBalancedRust，看板 #251）。
function captureBalancedJs(src, idx, open, close) {
  return balancedScanWith(src, idx, open, close, jsLexemeEnd);
}

// 默认配平器面向 Rust 源码/类型文本（本文件绝大多数调用点在扫 backend）。
// 看板 #251：旧实现把 `'` 当字符串开引号、完全不认注释，生命周期与 `//` 注释会把配平切歪。
function captureBalanced(src, idx, open, close) {
  return balancedScanWith(src, idx, open, close, rustLexemeEnd);
}

// Rust 侧专用：在「圆/方括号尚未配平」的区间里找到第一个顶层 `{` 或 `;` 的下标。
// 尖括号一律不参与计数（Rust 里 `<` 既可能是泛型也可能是比较/移位，靠猜必然错），
// 只按 () 与 [] 配平；本仓 handler 签名实测零例外（区间内无 lambda/数组下标）。
// 返回 -1 表示没找到（调用方按判不出处理，不许继续猜）。
function scanToFirstTopLevel(src, j, pred) {
  let par = 0;
  let bkt = 0;
  while (j < src.length) {
    const e = rustLexemeEnd(src, j);
    if (e > j) {
      j = e;
      continue;
    }
    const c = src[j];
    if (c === '(') par++;
    else if (c === ')') par--;
    else if (c === '[') bkt++;
    else if (c === ']') bkt--;
    else if (par <= 0 && bkt <= 0 && pred(c)) return j;
    j++;
  }
  return -1;
}

// ---------- 后端：handler 函数返回类型索引 ----------
// 从 handler 源码里按 `fn NAME(...)` 提取其返回类型文本（`->` 与函数体 `{` 之间）。
function extractReturnTypes(src) {
  const out = {};
  const re = /\bfn\s+([A-Za-z_]\w*)\s*[(<]/g;
  let m;
  while ((m = re.exec(src))) {
    const name = m[1];
    // 先跳到参数列表的匹配右括号（走 Rust 词法扫描：生命周期/注释/原始串不得吞掉右括号）
    const i = m.index + m[0].length - 1; // 指向 '(' 或 '<'
    if (src[i] !== '(') continue; // 带泛型的 fn 少见，这里只处理普通签名
    const cp = captureBalancedRust(src, i, '(', ')');
    if (!cp) continue;
    // i 指向参数右括号；找紧随其后的 `->`
    const after = src.slice(cp[0] + 1);
    const arrow = after.match(/^\s*(?:async\s+)?->/);
    if (!arrow) continue;
    const j = cp[0] + 1 + arrow[0].length; // 指向 `->` 之后
    // 读到第一个「非嵌套」的 `{`（函数体起点）或 `;`（trait 声明式 fn）
    const stop = scanToFirstTopLevel(src, j, c => c === '{' || c === ';');
    if (stop < 0) continue;
    const ret = src.slice(j, stop).trim();
    if (ret) out[name] = ret;
  }
  return out;
}

// ---------- 动态 JSON（Value / JsonValue）载荷还原 ----------
// 详见文件头「动态载荷还原」：to_value(具体类型) / json! 顶层键 / 变量与元组回溯 / service 递归。
// 解析不出来就保持「未分类 → 判负」，禁止把「读不懂」当成「没问题」。

// 按 fn 名提取「函数体文本」（配平大括号），供动态载荷回溯使用
function extractFnBodies(src) {
  const out = {};
  const re = /\bfn\s+([A-Za-z_]\w*)\s*[(<]/g;
  let m;
  while ((m = re.exec(src))) {
    const name = m[1];
    let i = m.index + m[0].length - 1;
    if (src[i] !== '(') continue;
    const cp = captureBalancedRust(src, i, '(', ')');
    if (!cp) continue;
    // 签名里可能带 `-> Foo { ... }`：函数体起点 = 参数右括号之后的第一个「顶层」'{'
    const start = scanToFirstTopLevel(src, cp[0] + 1, c => c === '{' || c === ';');
    if (start < 0 || src[start] !== '{') continue; // ';' = trait 声明，无函数体
    const body = captureBalancedRust(src, start, '{', '}');
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

// json! / vec! 等宏实参的顶层条目切分：与 Rust 切分器同源（看板 #251）。
// 旧实现自带一套扫描：不认注释（`json!({ // don't use\n "a":1})` 里注释中的 `"` 会把后半段
// 当成字符串），`'` 一律当字符串开引号，反引号也不是 Rust 语法。
function splitJsonMacroEntries(objText) {
  return splitRustTopLevelArgs(objText);
}

/// 去 Rust 注释（`//` 到行尾、`/* */` 可嵌套），字符串/字符字面量/生命周期原样保留。
/// 必须存在的原因（看板 #251 的连带坑）：表达式文本进 normExpr 前会先把换行压成空格，
/// 届时 `//` 注释就变成"一直到结尾"的注释，任何认注释的扫描器都会把后半段连同配平括号
/// 一起吞掉 —— 结果是 `to_value(PaginatedResponse::new(.., // 说明\n ..))` 判不出，
/// 已能归类的端点反而退回「未分类」。所以剥注释必须在压缩空白之前，且只剥注释。
function stripRustComments(s) {
  const src = String(s == null ? '' : s);
  let out = '';
  let i = 0;
  while (i < src.length) {
    const e = rustLexemeEnd(src, i);
    if (e > i) {
      const isComment = src[i] === '/' && (src[i + 1] === '/' || src[i + 1] === '*');
      out += isComment ? ' ' : src.slice(i, e);
      i = e;
      continue;
    }
    out += src[i];
    i++;
  }
  return out;
}

// 表达式文本归一：先剥注释（见 stripRustComments），再压缩空白 + 去掉点号两侧空白
// （Rust 链式调用常跨行书写，`service\n .list()` 必须先归一）
function normExpr(s) {
  return stripRustComments(s || '')
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
      const expr = readUntilStatementEndRust(body, start);
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
    const expr = readUntilStatementEndRust(body, start);
    if (expr) hit = { expr, annotation: nm[1] ? nm[1].trim() : null };
  }
  return hit || tupleHit;
}

// Rust 版「读到语句结束」：走 Rust 词法扫描，且 <> 不参与深度（比较运算符 / 生命周期
// 里的 `>` 会把深度打负，导致 `;` 永远等不到，语句文本一路吃到函数体结尾）。
// 看板 #251：findBinding 拿到的"绑定表达式"曾被注释和生命周期切歪，回溯返回类型时
// 读出的是整段函数体 —— 于是「载荷形状判不出」被误报成形状不符。
function readUntilStatementEndRust(body, start) {
  let depth = 0;
  let expr = '';
  let i = start;
  while (i < body.length) {
    const e = rustLexemeEnd(body, i);
    if (e > i) {
      expr += body.slice(i, e);
      i = e;
      continue;
    }
    const c = body[i];
    if (c === '(' || c === '[' || c === '{') depth++;
    else if (c === ')' || c === ']' || c === '}') depth = Math.max(0, depth - 1);
    else if (c === ';' && depth === 0) break;
    expr += c;
    i++;
  }
  return expr.trim() || null;
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
      // struct 声明前的属性（`#[serde(rename_all = "camelCase")]` 等）决定线上键名：
      // 请求体侧门禁要靠它把 Rust 字段名换算成 JSON 键名。取声明前 600 字符内的属性行即可。
      const pre = src.slice(Math.max(0, m.index - 600), m.index);
      const attrs = (pre.match(/#\[[^\]]*\]/g) || []).join(' ');
      const fields = parseStructFields(body.slice(1, -1));
      const vecFields = fields
        .filter(fld => /^\s*(?:Vec|std::vec::Vec)\s*</.test(fld.type))
        .map(fld => ({ name: fld.name, elem: genericArg(fld.type) }));
      // SeaORM 实体每个文件都有一个 `pub struct Model`：只按裸名查会把别的实体字段安上来
      const base = f.split(/[\/]/).pop().replace('.rs', '');
      const structEntry = { vecFields, allFields: fields, attrs };
      addStructEntry(index, `${base}::${name}`, structEntry);
      addStructEntry(index, name, structEntry);
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
  // 看板 #251：这里过去只跟踪 <>，`Result<(Vec<X>, u64), E>` 会在元组内部被切开，
  // 而且同样不认注释/生命周期。统一走 Rust 切分器，两处口径不可能再漂移。
  return splitRustTopLevelArgs(s);
}

// 去掉 // 行注释与 /* */ 块注释（字符串/模板字面量内的不算）。
// 不做这一步的话，带 JSDoc 的字段整块被当成"非字段"丢掉，前端键集会少，
// 门禁因此报出假的"后端必填前端没给"（本仓库第一次踩到是由并行修复任务报回的）。
function stripTsComments(src) {
  let out = '';
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    if (c === '"' || c === "'" || c === '`') {
      const q = c;
      out += c;
      i++;
      while (i < src.length) {
        out += src[i];
        if (src[i] === '\\') out += src[++i];
        else if (src[i] === q) {
          i++;
          break;
        }
        i++;
      }
      continue;
    }
    if (c === '/' && src[i + 1] === '/') {
      while (i < src.length && src[i] !== String.fromCharCode(10)) i++;
      continue;
    }
    if (c === '/' && src[i + 1] === '*') {
      const end = src.indexOf('*/', i + 2);
      i = end < 0 ? src.length : end + 2;
      continue;
    }
    out += c;
    i++;
  }
  return out;
}

// TS 对象类型字段分隔符可为 `,` 或 `;`；按顶层（<> {} [] () 深度为 0）切分。
function splitObjFields(raw) {
  const s = stripTsComments(raw);
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
  const info =
    structIndex[tail] || structIndex[qualified] || structIndex[qualified.split('::').pop()];
  if (info && info.ambiguous)
    return { kind: 'unknown', raw: `${p} (同名 struct 多处定义，判不出)` };
  if (info) {
    return shapeOfArrayKeys({
      vecKey: info.vecFields.map(v => v.name),
      hasTotal: info.allFields.some(f => TOTAL_KEYS.includes(f.name)),
      hasId: info.allFields.some(f => f.name === 'id'),
      allKeys: info.allFields.map(f => f.name),
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

// `fn name(a: A, b: B)` 的参数列表文本：请求体侧门禁从中解析 `Json<T>` / `Query<T>` 的 T，
// 从而把「前端提交的键集合」与「后端反序列化结构体字段」对齐（响应侧只看返回类型）。
function extractFnSignatures(src) {
  const out = {};
  const re = /\bfn\s+([A-Za-z_]\w*)\s*[(<]/g;
  let m;
  while ((m = re.exec(src))) {
    const open = m.index + m[0].length - 1;
    if (src[open] !== '(') continue;
    const cap = captureBalanced(src, open, '(', ')');
    if (cap) out[m[1]] = cap[1];
    re.lastIndex = open;
  }
  return out;
}

function buildHandlerModules(templates) {
  const mods = new Map();
  const dirMembers = {}; // 目录型模块名 -> 该目录下已索引的文件条目
  // 路由里的 handler 也可能指向 handlers/ 之外的 Axum 模块（如 `websocket::notifications::*`、
  // `search_api::search_sales_orders`）：不一起索引就会把它们当成"未定位"，
  // 从而在门禁的覆盖统计里留下说不清的空洞 —— routes/ 下就有一批就地定义的 handler。
  const dirs = [
    join(BACKEND, 'src', 'handlers'),
    join(BACKEND, 'src', 'websocket'),
    join(BACKEND, 'src', 'routes'),
  ].filter(d => {
    try {
      return readdirSync(d).length >= 0;
    } catch {
      return false;
    }
  });
  for (const f of dirs.flatMap(d => collectRsFiles(d))) {
    const src = readFileSync(f, 'utf-8');
    const base = f.split(/[\\/]/).pop().replace('.rs', '');
    // 聚合文件不按裸名 'mod' 索引：handlers/mod.rs 与 routes/mod.rs 会互相覆盖，
    // 而后路由径里永远不会出现 `mod::fn` 这种写法，索引它们只有副作用没有收益。
    if (base === 'mod') continue;
    const entry = {
      file: base + '.rs',
      rets: extractReturnTypes(src),
      sigs: extractFnSignatures(src),
      bodies: extractFnBodies(src),
      macroFns: {}, // 顶层宏展开生成的 fn
      modMacroFns: {}, // 文件内 mod 里宏展开生成的 fn
      nested: {}, // 文件内 `pub mod xxx { ... }` 里手写的 fn（如 report_enhanced_handler::subscriptions::list）
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
    for (const r of modRanges) {
      const text = src.slice(r.start + 1, r.end);
      entry.nested[r.name] = {
        rets: extractReturnTypes(text),
        bodies: extractFnBodies(text),
        sigs: extractFnSignatures(text),
      };
    }
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
      const genSigs = extractFnSignatures(expanded);
      const host = modRanges.find(r => cap[0] > r.start && cap[0] < r.end);
      const target = host ? (entry.modMacroFns[host.name] ||= {}) : entry.macroFns;
      for (const fn of Object.keys(genBodies))
        target[fn] = {
          ret: genRets[fn] || '',
          sig: genSigs[fn] || '',
          body: genBodies[fn],
          via: macroName,
        };
      invRe.lastIndex = cap[0];
    }
    if (mods.has(base)) {
      // 同名模块跨目录（handlers/x.rs 与 routes/x.rs 都存在）时静默覆盖，
      // 会让解析结果指向另一个模块 —— 那是伪造证明。两边都不采信，判不出。
      mods.set(base, {
        file: `${base}.rs`,
        rets: {},
        sigs: {},
        bodies: {},
        macroFns: {},
        modMacroFns: {},
        nested: {},
        reexports: [],
        ambiguousModule: true,
      });
    } else {
      mods.set(base, entry);
    }
    // 目录型模块：`pub mod advanced;` 指向 handlers/advanced/ 这个文件夹时，
    // 路由写的是 `advanced::list_purchase_contracts`，但索引里没有名为 advanced 的条目
    // （只有 analytics / decide / … 这些文件名），会被误判为「handler 符号未定义」。
    const root = dirs.find(d => f.startsWith(d));
    if (root) {
      const rest = f
        .slice(root.length)
        .replace(/^[\\/]/, '')
        .replace(/\\/g, '/')
        .split('/')
        .slice(0, -1);
      if (rest.length) (dirMembers[rest[rest.length - 1]] ||= []).push(entry);
    }
  }
  for (const [dirName, members] of Object.entries(dirMembers)) {
    // 目录名也可能与某个同名文件模块撞车（routes/color_card.rs 注册路由、
    // handlers/color_card/ 目录放 handler）。此时不能跳过合并，
    // 否则 `color_card::list_issues` 只会在那个小写的路由文件里找，判成"符号未定义"。
    const merged = mods.get(dirName) || {
      file: dirName + '/',
      rets: {},
      sigs: {},
      bodies: {},
      macroFns: {},
      modMacroFns: {},
      nested: {},
      reexports: [],
    };
    const cand = {};
    for (const M of [...members, merged]) {
      for (const fn of Object.keys(M.bodies)) (cand['b' + fn] ||= []).push(['bodies', M]);
      for (const fn of Object.keys(M.macroFns)) (cand['m' + fn] ||= []).push(['macroFns', M]);
    }
    for (const key of Object.keys(cand)) {
      if (cand[key].length !== 1) continue; // 同名落点不唯一 -> 判不出（不猜）
      const [kind, M] = cand[key][0];
      const fn = key.slice(1);
      if (kind === 'bodies') {
        merged.bodies[fn] = M.bodies[fn];
        merged.rets[fn] = M.rets[fn] || '';
        merged.sigs[fn] = M.sigs[fn] || '';
      } else merged.macroFns[fn] = M.macroFns[fn];
    }
    mods.set(dirName, merged);
  }
  return mods;
}

// 解析 `module::fn`：本文件 fn → 本文件宏生成 → 文件内 mod 宏生成(经 pub use) → pub use 转出模块
function resolveHandlerSymbol(mods, mod, fn, depth = 0) {
  if (depth > 3) return null;
  const M = mods.get(mod);
  if (!M) return null;
  if (M.bodies[fn])
    return {
      ret: M.rets[fn] || '',
      sig: M.sigs[fn] || '',
      body: M.bodies[fn],
      via: `${mod}::${fn}`,
    };
  if (M.macroFns[fn]) return { ...M.macroFns[fn], via: `${mod}::${fn}<${M.macroFns[fn].via}>` };
  for (const modName of Object.keys(M.modMacroFns)) {
    const tbl = M.modMacroFns[modName];
    if (tbl[fn]) {
      return { ...tbl[fn], via: `${mod}::${modName}::${fn}<${tbl[fn].via}>` };
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

// 按「路由里写下的完整 handler 路径」解析符号：路由可能写成 `mod_a::fn`、`mod_a::sub_mod::fn`
// 或 `websocket::notifications::fn`。规则仍是"必须有唯一确定的落点"，
// 绝不为凑答案按裸函数名全局回退（那条假判定路径本文件已写明教训）。
// 路由文件常以 `use crate::handlers::xxx_handler::{a, b};` 引入裸名 handler，
// 于是路由里写的是 `.route("/x", get(list_sales_returns))` 这种单段符号。
// 不跟随普通 use（非 pub use）就会把它误报成「handler 符号未定义」。
// 候选不唯一时判不出（返回 null），绝不按裸函数名全局回退找同名 fn。
const useImportCache = new Map();
function resolveHandlerByUseImport(mods, routesFile, fnName) {
  if (!routesFile || !fnName) return null;
  // 先按「路由注册所在文件自己就是 handler 模块」解：本仓有 `sales_return_handler::router()`
  // 这种 router 定义在 handler 文件内部、路由与 fn 同源的写法，此时裸名就在本文件。
  const selfBase = String(routesFile).split(/[\\/]/).pop().replace(/\.rs$/, '');
  const direct = resolveHandlerSymbol(mods, selfBase, fnName);
  if (direct) return direct;
  let byName = useImportCache.get(routesFile);
  if (!byName) {
    byName = new Map();
    let src = '';
    try {
      src = readFileSync(routesFile, 'utf-8');
    } catch {
      src = '';
    }
    const add = (name, modBase) => {
      if (!name || !modBase) return;
      byName.set(name, [...(byName.get(name) || []), modBase]);
    };
    for (const m of src.matchAll(/use\s+([\w:]+)::\{([^}]*)\}\s*;/g)) {
      const modBase = m[1].split('::').pop();
      for (const part of m[2].split(',')) {
        const name = part
          .trim()
          .split(/\s+as\s+/)
          .pop()
          .trim();
        add(name, modBase);
      }
    }
    for (const m of src.matchAll(/use\s+([\w:]+)::([A-Za-z_]\w*)\s*;/g))
      add(m[2], m[1].split('::').pop());
    useImportCache.set(routesFile, byName);
  }
  const cands = byName.get(fnName) || [];
  if (cands.length !== 1) return null;
  return resolveHandlerSymbol(mods, cands[0], fnName);
}

function resolveHandlerSymbolPath(mods, parts) {
  for (let i = 0; i < parts.length - 1; i++) {
    const base = parts[i];
    const M = mods.get(base);
    if (!M) continue;
    const rest = parts.slice(i + 1);
    if (rest.length === 1) return resolveHandlerSymbol(mods, base, rest[0]);
    if (rest.length === 2) {
      const nested = M.nested[rest[0]];
      const fn = rest[1];
      if (nested && nested.bodies[fn])
        return {
          ret: nested.rets[fn] || '',
          sig: nested.sigs[fn] || '',
          body: nested.bodies[fn],
          via: `${base}::${rest[0]}::${fn}`,
        };
      const macroInMod = M.modMacroFns[rest[0]] && M.modMacroFns[rest[0]][fn];
      if (macroInMod)
        return { ...macroInMod, via: `${base}::${rest[0]}::${fn}<${macroInMod.via}>` };
    }
    return null; // 该模块存在但路径形态不认识 -> 判不出，不去猜
  }
  // 全仓唯一匹配的 `mod名#fn名`（跨文件的 mod 同名会视为歧义）
  const fn = parts[parts.length - 1];
  const modName = parts.length >= 3 ? parts[parts.length - 2] : null;
  if (!modName) return null;
  const hits = [];
  for (const [base, M] of mods) {
    const nested = M.nested[modName];
    if (nested && nested.bodies[fn])
      hits.push({
        ret: nested.rets[fn] || '',
        sig: nested.sigs[fn] || '',
        body: nested.bodies[fn],
        via: `${base}::${modName}::${fn}`,
      });
    const macroInMod = M.modMacroFns[modName] && M.modMacroFns[modName][fn];
    if (macroInMod)
      hits.push({ ...macroInMod, via: `${base}::${modName}::${fn}<${macroInMod.via}>` });
  }
  return hits.length === 1 ? hits[0] : null;
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
    allKeys: keys.map(k => k.key),
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

// Rust 顶层逗号切分（看板 #251 的正解，替代旧的"猜深度"扫描）。三条与旧实现不同的硬规则：
//  ① 注释/字符串/原始串/字节串/字符字面量/生命周期整段跳过 —— 里面的 `,` `<` `>` `(` `)`
//     一律不参与判定（旧实现把 `'` 当字符串开引号，一个生命周期就吞掉后半段）；
//  ② 尖括号按语言环境识别，不再"见 < 就加、见 > 就减"：只有紧跟在标识符/`)`/`::` 之后
//     且后面不是 `=` 的 `<` 才算泛型实参表开始；`>` 只在尖括号深度 > 0、且前一个字符不是
//     `-`/`=`（即 `->`、`=>`）时才闭合 —— 旧实现把 `->` 的 `>` 计入，深度被打成负数后，
//     同一签名里其后的顶层逗号永远切不出来（假"单实参"+ 漏判）；
//  ③ 宏调用（`name!` / `path::name!` 后跟 `()`/`[]`/`{}`）整组原子消费，绝不递归进去找逗号
//     —— 宏实参里的 `<`、`>`、注释、`json!` 里的嵌套逗号都不影响外层切点。
// 与旧版必须保持一致的两点（改了就漂移，别动）：片段不 trim、末尾空片段丢弃
// （expandMacroArm 按实参数与元变量个数严格配对，多一个空实参会让整条宏解析失效）。
function splitRustTopLevelArgs(s) {
  const src = s == null ? '' : String(s);
  const out = [];
  let cur = '';
  let paren = 0;
  let bracket = 0;
  let brace = 0;
  let i = 0;
  const top = () => paren === 0 && bracket === 0 && brace === 0;
  while (i < src.length) {
    const e = rustLexemeEnd(src, i);
    if (e > i) {
      cur += src.slice(i, e);
      i = e;
      continue;
    }
    // ③ 宏调用：整组原子消费（未配平则退回普通字符扫描，由 --self-test 的守恒断言暴露）
    if (/[A-Za-z_]/.test(src[i]) && !(i > 0 && RUST_XID.test(src[i - 1]))) {
      const mm = /^[A-Za-z_]\w*!\s*([({\[])/.exec(src.slice(i));
      if (mm) {
        const open = i + mm[0].length - 1;
        const end = macroGroupEnd(src, open, src[open]);
        if (end > 0) {
          cur += src.slice(i, end);
          i = end;
          continue;
        }
      }
    }
    const c = src[i];
    // 泛型实参表：整组原子消费（组内的 `,` 属于类型参数，不是实参分隔符）。
    // 判定不靠"猜深度"：只有「前一个非空白字符是标识符/`)`/`::`」且「同层能配平出 `>`」
    // 的 `<` 才是泛型表；比较运算符/移位里的 `<` 配不出平衡的 `>`，按普通字符处理。
    if (c === '<' && src[i + 1] !== '=') {
      let k = i - 1;
      while (k >= 0 && /\s/.test(src[k])) k--;
      const prev = k >= 0 ? src[k] : '';
      const candidate = /[A-Za-z0-9_)]/.test(prev) || (prev === ':' && src[k - 1] === ':');
      if (candidate) {
        const end = angleGroupEnd(src, i);
        if (end > 0) {
          cur += src.slice(i, end);
          i = end;
          continue;
        }
      }
      cur += c;
      i++;
      continue;
    }
    if (c === '(') paren++;
    else if (c === ')') paren = Math.max(0, paren - 1);
    else if (c === '[') bracket++;
    else if (c === ']') bracket = Math.max(0, bracket - 1);
    else if (c === '{') brace++;
    else if (c === '}') brace = Math.max(0, brace - 1);
    if (c === ',' && top()) {
      out.push(cur);
      cur = '';
      i++;
      continue;
    }
    cur += c;
    i++;
  }
  if (cur.trim()) out.push(cur);
  return out;
}

// 兼容旧调用名（本文件与 check-api-request 都按此名 import）：语义 = Rust 切分。
// TS/JS 文本的切分不走这里，见 check-api-request.mjs 的 splitTopLevelCommas/Bar/Amp。
function splitTopLevelRust(s) {
  return splitRustTopLevelArgs(s);
}

// TS/JS 顶层切分（看板 #251 同族修正）：与 Rust 版的差异必须显式存在——
//  ① `'` 在 TS 里是字符串引号（Rust 里多半是生命周期），不能按生命周期规则只吃一个词；
//  ② `=>`（箭头函数类型/箭头函数实参）里的 `>` 绝不能当闭合符：旧实现把它计入深度，
//     `() => void` 之后深度变成 -1，同一参数表/实参表里其后的顶层逗号再也切不出来 ——
//     实测 fund.ts:50 一类「形参注解带回调」的载荷恒判不出，且 `request.post(url,
//     items.map(i => i.id), { params })` 的第 3 个实参会被并进第 2 个（取错载荷位）；
//  ③ 深度一律夹在 0 以上（打负 = 后面的逗号全切不出来，属静默漏判）。
// 片段不 trim；尾部空片段丢弃（与 splitRustTopLevelArgs/splitTopLevelCommas 口径一致）。
function splitTsTopLevel(s, sep, opts = {}) {
  const src = s == null ? '' : String(s);
  const out = [];
  let cur = '';
  let depth = 0;
  let i = 0;
  while (i < src.length) {
    const e = jsLexemeEnd(src, i);
    if (e > i) {
      cur += src.slice(i, e);
      i = e;
      continue;
    }
    const c = src[i];
    if (c === '<' || c === '(' || c === '[' || c === '{') depth++;
    else if (c === '>' || c === ')' || c === ']' || c === '}') {
      if (c === '>' && src[i - 1] === '=') {
        cur += c;
        i++;
        continue;
      }
      depth = Math.max(0, depth - 1);
    }
    // 联合类型里 `||` 不是分隔符（仅当调用方要求时保护，保持旧口径）
    const dupGuard = opts.guardDouble && (src[i + 1] === sep || src[i - 1] === sep);
    if (c === sep && depth === 0 && !dupGuard) {
      out.push(cur);
      cur = '';
      i++;
      continue;
    }
    cur += c;
    i++;
  }
  if (cur.trim()) out.push(cur);
  return out;
}

// TS 实参表/参数表（逗号切分）。
function splitTsTopLevelArgs(s) {
  return splitTsTopLevel(s, ',');
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
      // 泛型 interface 同样索引：判「载荷是哪个键」只需要字段名与「值是不是数组」，
      // 不需要知道元素类型（`items: T[]` 依然是数组键）。
      // 此前此处 `if (m[2]) continue` 把所有带类型参数的 interface 直接跳过，
      // 使 ApiResponse 之外的具名泛型包装（PagedResponse<T> 等）整批落在盲区里没人核对。
      const open = src.indexOf('{', m.index + m[0].length - 1);
      const cap = captureBalancedJs(src, open, '{', '}');
      if (!cap) continue;
      addTsEntry(index, m[1], {
        kind: 'object',
        body: cap[1],
        extends: m[3] ? splitObjFields(m[3]).map(s => s.trim()) : [],
        file: rel,
      });
    }
    for (const m of src.matchAll(/\b(?:export\s+)?type\s+([A-Z]\w*)\s*(<[^=]*>)?\s*=\s*/g)) {
      if (m[2]) continue;
      const text = readUntilStatementEnd(src, m.index + m[0].length) || '';
      const t = text.trim().replace(/;\s*$/, '');
      if (!t) continue;
      if (t.startsWith('{'))
        addTsEntry(index, m[1], { kind: 'object', body: t.slice(1, -1), extends: [], file: rel });
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
    if (!pe || pe.kind !== 'object')
      return { kind: 'named', raw: `${name} extends ${pname}(未解析)` };
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
  const keys = top.map(e => (e.match(/^\s*([A-Za-z_]\w*)\s*\??:/) || [])[1]).filter(Boolean);
  return shapeOfArrayKeys({
    vecKey,
    hasTotal: keys.some(k => TOTAL_KEYS.includes(k)),
    hasId: keys.includes('id'),
    allKeys: keys,
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
    // 逐个 `export function NAME(...) (: Promise<...>)? { body }`
    // 返回类型注解必须是可选匹配：本仓大量 `export function createArReconciliation(data:
    // Partial<X>) { return request.post(...) }` 不写 `: Promise<...>`，旧正则要求它，
    // 整条函数对两套契约门禁都不可见 —— 真·静默跳过（挂账盲区 (a)）。
    // 无注解时只登记确实含 request 调用的函数，避免把纯工具导出拉进统计。
    const fnRe =
      /export\s+(?:async\s+)?function\s+([A-Za-z_]\w*)\s*\(([^)]*)\)\s*(?::\s*Promise<([\s\S]*?)>)?\s*\{/g;
    let m;
    while ((m = fnRe.exec(src))) {
      const name = m[1];
      const retType = (m[3] || '').trim();
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
      // 无 `: Promise<...>` 注解的导出函数：响应形状无从判定 -> 显式 opaque
      // （envelope 侧按盲区 skip），但没有 request 调用的纯工具导出不登记，避免刷统计。
      if (!retType && !call) continue;
      const feShape = retType
        ? classifyFrontendReturn(retType, tsIndex)
        : { kind: 'opaque', raw: '(无显式返回类型注解)' };
      list.push({ name, file: rel, line, feShape, retType, call, sig: m[2] || '' });
    }
    // 逐个 `export const NAME = (...) => request.get<ApiResponse<X>>(url, ...)`
    // —— 返回类型挂在调用表达式上而非函数签名上，上一趟正则（要求 `: Promise<...>`）看不见它们。
    // 全仓 49 个 api 文件共 500 处这种写法，规模与已比对量同级，不解析等于一半接口没有门禁。
    const genRe = /request\.(get|post|put|delete|patch)\s*</g;
    let gm;
    while ((gm = genRe.exec(src))) {
      const lt = src.indexOf('<', gm.index + 'request.'.length);
      const typeCap = lt >= 0 ? captureBalancedJs(src, lt, '<', '>') : null;
      if (!typeCap) continue;
      const retType = typeCap[1].trim();
      const paren = src.indexOf('(', typeCap[0]);
      const argCap = paren >= 0 ? captureBalancedJs(src, paren, '(', ')') : null;
      if (!argCap) continue;
      const argTexts = splitTsTopLevelArgs(argCap[1]).map(a => a.trim());
      const url = resolveFrontendUrl(argTexts[0] || '', consts);
      if (!url) continue;
      let path = url;
      if (!path.startsWith(BASE_URL)) path = BASE_URL + (path.startsWith('/') ? path : '/' + path);
      // 方法名必须大写：共享路由表（check-api-paths 的 walkBackendRoutes）以
      // `"<path> GET"` 形态建键，小写会让整批条目假报「路由未注册」。
      const call = { method: gm[1].toUpperCase(), path: normalizePath(path), args: argTexts };
      // 归属到最近一个 export 的符号名（从调用点往前扫）
      const head = src.slice(0, gm.index);
      const nm = [
        ...head.matchAll(/export\s+(?:async\s+)?(?:function|const|let)\s+([A-Za-z_]\w*)/g),
      ].pop();
      if (!nm) continue;
      const name = nm[1];
      const line = head.split('\n').length;
      if (list.some(r => r.name === name && r.call && r.call.path === call.path)) continue;
      list.push({
        name,
        file: rel,
        line,
        feShape: classifyFrontendReturn(retType, tsIndex),
        retType,
        call,
        // 挂账盲区 (a)：箭头函数的形参签名此前写死 ''，check-api-request 拿不到
        // `data: Partial<X>` 之类的类型注解，229 条落进「形参在签名里找不到类型」。
        // 现在从「导出符号 -> 调用点」的片段里还原参数表。
        sig: sigForSymbol(src, nm.index, gm.index),
      });
    }
    // 第三趟：既无 `: Promise<...>` 注解、调用又无泛型返回标注（`request.post('/x', data)`）
    // 的箭头函数/声明函数 —— 前两趟都看不见它们，整个接口对门禁不存在（真·静默跳过）。
    // 载荷侧（check-api-request）必须看到它们：sig 回填形参类型，请求体/查询键集照常比对。
    const plainRe = /request\.(get|post|put|delete|patch)\s*\(/g;
    let pm;
    while ((pm = plainRe.exec(src))) {
      const paren = pm.index + pm[0].length - 1;
      const argCap = captureBalancedJs(src, paren, '(', ')');
      if (!argCap) continue;
      const argTexts = splitTsTopLevelArgs(argCap[1]).map(a => a.trim());
      const url = resolveFrontendUrl(argTexts[0] || '', consts);
      if (!url) continue; // URL 不可静态还原：路由存在性由 check-api-paths 专门判负
      let path = url;
      if (!path.startsWith(BASE_URL)) path = BASE_URL + (path.startsWith('/') ? path : '/' + path);
      const call = { method: pm[1].toUpperCase(), path: normalizePath(path), args: argTexts };
      const head = src.slice(0, pm.index);
      const nm = [
        ...head.matchAll(/export\s+(?:async\s+)?(?:function|const|let)\s+([A-Za-z_]\w*)/g),
      ].pop();
      if (!nm) continue;
      const name = nm[1];
      if (list.some(r => r.name === name && r.call && r.call.path === call.path)) continue;
      const line = head.split('\n').length;
      const seg = src.slice(nm.index, pm.index);
      const rm = /:\s*Promise<([\s\S]*?)>\s*(?:=>|\{)/.exec(seg);
      const retType = rm ? rm[1].trim() : '';
      list.push({
        name,
        file: rel,
        line,
        feShape: retType
          ? classifyFrontendReturn(retType, tsIndex)
          : { kind: 'opaque', raw: '(无显式返回类型注解)' },
        retType,
        call,
        sig: sigForSymbol(src, nm.index, pm.index),
      });
    }
  }
  return list;
}

// 从「导出符号起点 -> 调用点」片段里还原被调用函数的参数表文本。
// 支持三种真实写法：`export function NAME(a: X, b: Y)`、
// `export const NAME = (a: X, b: Y) =>`、`export const NAME: Fn = (a) =>`（带注解的退化为 ''，
// 与旧行为一致，不猜）。返回 '' 表示取不到 —— 调用方必须按盲区处理，禁止当无参数。
function sigForSymbol(src, symIdx, callIdx) {
  const seg = src.slice(symIdx, Math.min(callIdx, symIdx + 20000));
  let m = /^export\s+(?:async\s+)?function\s+[A-Za-z_]\w*\s*\(([^)]*)\)/.exec(seg);
  if (m) return m[1];
  const ai = seg.indexOf('=>');
  const head = (ai >= 0 ? seg.slice(0, ai) : seg).trimEnd();
  m = /=\s*(?:async\s*)?\(([^)]*)\)\s*(?::[\s\S]*)?$/.exec(head);
  if (m) return m[1];
  m = /=\s*(?:async\s+)?([A-Za-z_]\w*)\s*$/.exec(head);
  return m ? m[1] : '';
}

function extractFirstCall(body, consts) {
  const re = /request\.(get|post|put|delete|patch)(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\s*\(\s*/g;
  const m = re.exec(body);
  if (!m) return null;
  const method = m[1].toLowerCase();
  // 用配平括号取「全部实参」而非只取第一个：请求体侧门禁要看第二个实参（payload / { params }）。
  // 原手写扫描在第一个顶层逗号处即停，拿不到 payload，也无法处理实参里带逗号的嵌套调用。
  const open = body.indexOf('(', m.index);
  const cap = open >= 0 ? captureBalancedJs(body, open, '(', ')') : null;
  if (!cap) return null;
  const argTexts = splitTsTopLevelArgs(cap[1]).map(a => a.trim());
  const arg = argTexts[0] || '';
  const url = resolveFrontendUrl(arg.trim(), consts);
  if (!url) return null;
  let path = url;
  if (!path.startsWith(BASE_URL)) path = BASE_URL + (path.startsWith('/') ? path : '/' + path);
  return { method: method.toUpperCase(), path: normalizePath(path), args: argTexts };
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
  // request.get<T>() 的 T 是「整个响应体」。写成 T = { data: X } 时（本仓 ApiResponse 的简写形式），
  // 真正的载荷是 X；不先剥这一层就会把 ApiResponse{data:PagedResponse} 误判成"单对象 vs 信封"。
  if (!mm && /^\{/.test(inner)) {
    // splitObjFields 返回字段文本（非 {key,val}），这里只认顶层 `data:` 一个字段的情形
    const fields = splitObjFields(inner.replace(/^\{/, '').replace(/\}\s*$/, ''));
    const dataField = fields
      .map(f => /^data\s*\??\s*:\s*([\s\S]+)$/.exec(String(f).trim()))
      .find(Boolean);
    if (dataField) return classifyFrontendPayload(dataField[1].trim(), tsIndex);
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
  // 具名类型：展开其定义后参与比对；展开不了（泛型/多定义/父类型未解析）才退回盲区。
  // 带类型实参的具名类型（PageResult<T> 等）此前只匹配裸标识符，会整体掉进盲区 = 等于没检查，
  // 故先去掉实参列表再查索引（列表元素类型不影响「载荷是哪个键」这一判定）。
  const bare = p.match(/^([A-Z]\w*)(?:<[\s\S]*>)?$/);
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
  [
    `${BASE_URL}/crm/five-dimension/stats GET`,
    'five_dimension_handler.rs:78-79 Ok(ApiResponse::success(json!({"items": stats.0, …})))：顶层 items 由 json! 手拼，前端 {items} 与之相符',
  ],
  // 以下各条为「后端 data 是手工 json! / 未被 struct 索引覆盖的 struct」，门禁无法静态判形。
  // 每条都已逐次阅读 handler 函数体核实前端读的键确实存在（证据见各条 file:line）；
  // 属"已人工核对"而非"已静态验证"。根治办法是把这 8 个 handler 的返回改成强类型响应
  // （PaginatedResponse<T> / 专用 Response struct），已登记为解冻后的后端批次，届时删除本条目。
  [
    `${BASE_URL}/ai/process-optimizations/batch POST`,
    'ai_extend_handler.rs:414/427/436 results 数组由 json! 逐条 push，顶层 {total,succeeded,failed,results}；前端 {results} 与之相符',
  ],
  [
    `${BASE_URL}/ai/quality-predictions/batch POST`,
    'ai_extend_handler.rs:492-496 显式 json!({"total","succeeded",…,"results"})；前端 {results} 与之相符',
  ],
  [
    `${BASE_URL}/color-prices/batch-adjust POST`,
    'color_price_handler.rs:222-225 json!({"auto_approved",…,"total"})；前端 {auto_approved} 与之相符',
  ],
  [
    `${BASE_URL}/color-prices/tiers/* GET`,
    'color_price_handler.rs:344-345 json!({"items","total"})；前端 {items} 与之相符',
  ],
  [
    `${BASE_URL}/color-prices/seasonal-rules GET`,
    'color_price_handler.rs:481-483 json!({"items","total"})；前端 {items} 与之相符',
  ],
  [
    `${BASE_URL}/export-approvals GET`,
    'export_approval_handler.rs:69-71 json!({"items": vo.items,"total": vo.total})；前端 {items} 与之相符',
  ],
  [
    `${BASE_URL}/export-approvals/pending-for-me GET`,
    'export_approval_handler.rs:92-94 json!({"items","total"})；前端 {items} 与之相符',
  ],
  [
    `${BASE_URL}/bpm/definitions GET`,
    'bpm_definition_handler.rs:48-63 page_to_frontend_json 手拼 json!({"list",...})，返回类型是未定型的 serde_json::Value 故静态判不出；已读码确认承载键为 list，前端 ProcessDefinitionPage{list} 与之一致。根治：helper 改返回强类型分页结构',
  ],
  [
    `${BASE_URL}/bpm/templates GET`,
    'bpm_definition_handler.rs:190-197 复用同一 page_to_frontend_json（{list,...}）；同上，前端已钉 list',
  ],
  [
    `${BASE_URL}/ai/process-optimizations GET`,
    'ai_extend_handler.rs:146-151 手拼 json!({"items","total","page","page_size"})；items 来自 service 的 vo.items（Vec），门禁无法静态证明其元素类型，但顶层承载键已读码确认为 items，前端 PaginatedResponse<T> 与之后端真相一致',
  ],
  [
    `${BASE_URL}/ai/quality-predictions GET`,
    'ai_extend_handler.rs:237-242 同一形状的 json!({"items","total","page","page_size"})；同上，前端已钉 items',
  ],
  [
    `${BASE_URL}/products/import POST`,
    '返回 utils/import_export.rs:37 ImportResult{total_count,success_count,error_count,errors}：errors 是详情对象内嵌数组而非列表信封，前端按 {errors} 读正确；struct 未被索引故判未分类',
  ],
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
  if (fe.kind === 'wrapper' && fe.ambiguous)
    return `前端声明同时给出多个规范列表键 ${fe.ambiguous.join('/')}，后端怎么返回都"对得上"；应按后端真实形状钉死为其中一个`;
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
  if (fe.ambiguous) return { status: 'unclassified' }; // 万能类型：怎么返回都不算错，比对无意义 -> 逼出显式承载键
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
        ? resolveHandlerSymbolPath(handlerMods, parts)
        : resolveHandlerByUseImport(handlerMods, h.routesFile, parts[0]);
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
          ? `路由 handler 符号 ${h.handler} 在其模块内未定义（非本文件 fn / 非宏生成 / 非 pub use 转出 / 非路由文件 use 引入）`
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

// 只有被直接执行时才跑：请求体侧门禁 check-api-request.mjs 会 import 本文件的解析器，
// 两套各自实现必然漂移（本仓库反复栽过的根因之一）。
const invokedDirectly =
  !!process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) main();

export {
  BACKEND,
  buildGlobalFnIndex,
  buildHandlerModules,
  buildStructIndex,
  buildTsTypeIndex,
  captureBalanced,
  captureBalancedJs,
  captureBalancedRust,
  extractReturnTypes,
  loadHandlerMacroTemplates,
  parseFrontendApiFunctions,
  readFileSync,
  readUntilStatementEnd,
  readUntilStatementEndRust,
  resolveHandlerSymbol,
  resolveHandlerSymbolPath,
  rustLexemeEnd,
  sigForSymbol,
  splitObjFields,
  splitRustTopLevelArgs,
  splitTopLevelRust,
  splitTsTopLevel,
  splitTsTopLevelArgs,
  stripRustComments,
};
