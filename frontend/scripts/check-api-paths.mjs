#!/usr/bin/env node
/**
 * 前缀感知的「前端出站调用 ↔ 后端展开路由」对照检查（A 类防线）
 *
 * 背景：本仓库反复出现「UI↔API 契约漂移」。后端用 axum 的 nest()/route() 组装，
 * 前端调用写的是相对 baseURL(/api/v1/erp) 的路径；直接 grep 整路径必然因 nest 前缀
 * 与 `${}` 模板段而落空。本脚本把两侧都「展开到完整路径 + HTTP 方法」后精确对照：
 *
 *   - 前端：扫描 frontend/src/api 目录下的 .ts，取 request.{get,post,put,delete}(...)，
 *     首个实参可为字符串字面量 / 模板串 / 同文件 const 字符串常量；把 `${}` 段展开为
 *     通配 `*`、去 query、拼接 baseURL 前缀。
 *   - 后端：从 routes/mod.rs 的 create_router 出发，递归展开 nest()/merge()/route()，
 *     累积出完整 `/api/v1/erp/...` 路径；把 `{param}` / `:param` 段归一为 `*`，
 *     从 route 的第二实参里提取 get/post/put/delete/patch 方法集。
 *
 * 判定：
 *   A 类 = 前端调用了「后端不存在该 (路径, 方法) 组合」→ 非零退出并逐条打印。
 *   其余类别（后端有、前端零调用）只打印统计，不失败。
 *
 * 例外：确有「后端尚未实现该操作」的功能缺口（前端调用的语义后端根本没有对应实现），
 *   不能伪造端点、也不能静默。此类以「显式注释 + 具体原因」登记于 KNOWN_GAPS，
 *   在 A 类中命中时降级为「功能缺口(待产品确认)」提示而非失败——每条必须写明原因，
 *   禁止把整片前缀/方法批量塞进例外来掩盖真实漂移。
 */
import { readFileSync, readdirSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const FRONTEND = resolve(__dirname, '..');
export const BACKEND = resolve(FRONTEND, '..', 'backend');
export const BASE_URL = '/api/v1/erp';

// ---------- 路径归一：{param} / :param / ${expr} -> * ----------
export function normalizePath(p) {
  let s = String(p);
  const q = s.indexOf('?');
  if (q >= 0) s = s.slice(0, q); // 去 query string
  s = s.replace(/\$\{[^}]*\}/g, '*'); // 模板段 -> 通配
  s = s.replace(/\{[^}]*\}/g, '*'); // axum {param} -> 通配
  s = s.replace(/:(\/|[\w])/g, (_m, c) => (c === '/' ? '*' : '*' + c.slice(1)));
  s = s.replace(/:(\w+)/g, '*'); // :param -> 通配
  s = s.replace(/\/{2,}/g, '/'); // 折叠重复斜杠
  if (!s.startsWith('/')) s = '/' + s;
  return s.replace(/\/+$/g, '') || '/';
}

// ---------- 前端解析 ----------
export function collectTsFiles(dir) {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) out.push(...collectTsFiles(p));
    else if (e.name.endsWith('.ts') && !e.name.endsWith('.d.ts')) out.push(p);
  }
  return out;
}

// 解析同文件里的字符串常量 `const NAME = '...'` / `const NAME = \`...\``
export function constStringMap(src) {
  const map = {};
  for (const m of src.matchAll(/const\s+([A-Za-z_]\w*)\s*=\s*(['"`])([^'"`]*)\2/g)) {
    map[m[1]] = m[3];
  }
  return map;
}

// 取出 request.<method>(<firstArg> 里的 url 表达式（平衡括号，忽略逗号后的 config/data）
export function scanFrontendCalls(src, consts) {
  const calls = [];
  const re = /request\.(get|post|put|delete)(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\s*\(\s*/g;
  let m;
  while ((m = re.exec(src))) {
    const method = m[1].toLowerCase();
    const start = m.index + m[0].length;
    // 从 start 起取第一个实参：字符串/模板/标识符，直到第一个不在引号/花括号内的逗号或右括号
    let i = start;
    let arg = '';
    let quote = null;
    let braceDepth = 0;
    while (i < src.length) {
      const c = src[i];
      if (quote) {
        arg += c;
        if (c === quote) quote = null;
        else if (c === '$' && src[i + 1] === '{' && (quote === '`' || quote === '"')) {
          // 模板插值：吞掉整段 ${...}
          arg += '{';
          i += 2;
          let d = 1;
          while (i < src.length && d > 0) {
            if (src[i] === '{') d++;
            else if (src[i] === '}') d--;
            arg += src[i];
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
    if (url !== null) calls.push({ method, url });
  }
  return calls;
}

export function resolveFrontendUrl(arg, consts) {
  if (!arg) return null;
  // 纯字符串/模板字面量
  const strLit = arg.match(/^(['"`])([\s\S]*)\1$/);
  let raw = null;
  if (strLit) raw = strLit[2];
  else if (/^[A-Za-z_]\w*$/.test(arg) && arg in consts) raw = consts[arg];
  if (raw === null) return null;
  // 模板里的 `${IDENT}` 若指向同文件字符串常量，先代入常量值，
  // 剩余真正的路径参数 `${id}` 等留给 normalizePath 归一为通配。
  raw = raw.replace(/\$\{([A-Za-z_]\w*)\}/g, (m, id) => (id in consts ? consts[id] : m));
  return raw;
}

export function loadFrontendEndpoints() {
  const set = new Map(); // key path|method -> [{file,line}]
  const files = collectTsFiles(join(FRONTEND, 'src', 'api'));
  for (const f of files) {
    const src = readFileSync(f, 'utf-8');
    const consts = constStringMap(src);
    const rel = f.replace(FRONTEND, '').replace(/\\/g, '/');
    // 记录行号：按 request.<method> 出现顺序回扫
    for (const { method, url } of scanFrontendCalls(src, consts)) {
      let path = url;
      if (!path.startsWith(BASE_URL)) path = BASE_URL + (path.startsWith('/') ? path : '/' + path);
      const np = normalizePath(path);
      const key = `${np} ${method.toUpperCase()}`;
      if (!set.has(key)) set.set(key, []);
      set.get(key).push({ file: rel, url });
    }
  }
  return set;
}

// ---------- 后端解析 ----------
export function collectRsFiles(dir) {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) out.push(...collectRsFiles(p));
    else if (e.name.endsWith('.rs')) out.push(p);
  }
  return out;
}

// 从一段源码里按 `fn NAME(...)` 提取所有函数体（大括号配平）
export function extractFunctions(src) {
  const fns = {};
  const re = /\bfn\s+([A-Za-z_]\w*)\s*\(/g;
  let m;
  while ((m = re.exec(src))) {
    const name = m[1];
    const open = src.indexOf('{', m.index + m[0].length);
    if (open < 0) continue;
    let depth = 0;
    let i = open;
    let inStr = null;
    for (; i < src.length; i++) {
      const c = src[i];
      if (inStr) {
        if (c === '\\') i++;
        else if (c === inStr) inStr = null;
        continue;
      }
      if (c === '"' || c === "'") {
        inStr = c;
        continue;
      }
      if (c === '{') depth++;
      else if (c === '}') {
        depth--;
        if (depth === 0) break;
      }
    }
    fns[name] = src.slice(open + 1, i);
  }
  return fns;
}

// 在函数体内查找 `.route("PATH", <METHODCHAIN>)` / `.nest("P", TARGET)` / `.merge(TARGET)`
// 逐 token 扫描，遇到 .route/.nest/.merge 提取参数（支持跨行、字符串内含斜杠）
export function parseBuilder(body) {
  const routes = [];
  const nests = [];
  const merges = [];
  const re = /\.(route|nest|merge)\s*\(\s*/g;
  let m;
  while ((m = re.exec(body))) {
    const kind = m[1];
    const start = m.index + m[0].length;
    // 收集该调用的完整实参列表（平衡圆括号）
    let i = start;
    let depth = 1;
    let inStr = null;
    const argsRaw = [];
    let cur = '';
    for (; i < body.length; i++) {
      const c = body[i];
      if (inStr) {
        cur += c;
        if (c === '\\') {
          cur += body[++i];
        } else if (c === inStr) inStr = null;
        continue;
      }
      if (c === '"' || c === "'") {
        inStr = c;
        cur += c;
        continue;
      }
      if (c === '(') {
        depth++;
        cur += c;
        continue;
      }
      if (c === ')') {
        depth--;
        if (depth === 0) {
          argsRaw.push(cur);
          break;
        }
        cur += c;
        continue;
      }
      if (c === ',' && depth === 1) {
        argsRaw.push(cur);
        cur = '';
        continue;
      }
      cur += c;
    }
    if (kind === 'route') {
      const path = firstStringLiteral(argsRaw[0] || '');
      const methodArg = argsRaw.slice(1).join(',') || '';
      const methods = extractMethods(methodArg);
      const handlers = extractMethodHandlers(methodArg);
      if (path) routes.push({ path, methods, handlers });
    } else if (kind === 'nest') {
      const prefix = firstStringLiteral(argsRaw[0] || '');
      const target = parseTarget(argsRaw.slice(1).join(',') || '');
      if (prefix && target) nests.push({ prefix, target });
    } else {
      // merge
      const target = parseTarget(argsRaw.join(',') || '');
      if (target) merges.push({ target });
    }
    re.lastIndex = i;
  }
  return { routes, nests, merges };
}

function firstStringLiteral(s) {
  const m = s.match(/^\s*"((?:[^"\\]|\\.)*)"/);
  return m ? m[1] : null;
}

// 从 route 第二实参文本里提取 (方法 -> handler 引用) 对，支持链式
// `get(mod::h1).post(mod::h2)`；handler token 允许 `mod::fn` / `self::fn` / 裸 `fn`，
// 并容忍其后的泛型 turbofish（如 `get(service::list::<T>)`，罕见）。
export function extractMethodHandlers(s) {
  const out = [];
  const re =
    /(^|[^A-Za-z0-9_])(get|post|put|delete|patch|head|options)\s*\(\s*([A-Za-z_]\w*(?:::\w+)*)/g;
  let m;
  while ((m = re.exec(s))) out.push({ method: m[2].toUpperCase(), handler: m[3] });
  return out;
}

// 从 route 第二实参文本里提取顶层方法助词：get/post/put/delete/patch/head/options
// 允许方法词前为链式点号/括号/逗号/空白（如 `get(h).post(h2)`），
// 但不能是标识符的一部分（`_delete`/`to_post` 之类）。
export function extractMethods(s) {
  const out = new Set();
  const re = /(^|[^A-Za-z0-9_])(get|post|put|delete|patch|head|options)\s*\(/g;
  let m;
  while ((m = re.exec(s))) out.add(m[2].toUpperCase());
  return [...out];
}

// 解析调用目标：`mod::routes()` / `routes()` / `mod::sub(state.clone())`
// 锚定实参起始处的「第一个函数调用」，允许携带任意实参（如 state.clone()）。
export function parseTarget(s) {
  const t = (s || '').trim();
  const mQ = t.match(/^([A-Za-z_]\w*)::([A-Za-z_]\w*)\s*\(/);
  if (mQ) return { file: mQ[1], fn: mQ[2] };
  const mL = t.match(/^([A-Za-z_]\w*)\s*\(/);
  if (mL) return { file: null, fn: mL[1] };
  return null;
}

// 后端路由展开：从 routes/mod.rs::create_router 递归 nest()/merge()/route()，
// 累积出完整 `/api/v1/erp/...` 路径。返回两个结构（供其它门禁复用）：
//   endpoints: Set<"path METHOD">   —— 与历史 loadBackendEndpoints 完全一致的键集合
//   handlers : Map<"path METHOD", {handler, routesFile}>  —— 每条路由绑定的 handler 引用
//     handler 形如 `sales_contract_handler::list_contracts` 或裸 `health_check`；
//     routesFile 是该 .route() 语句所在的路由模块基名（用于同名 fn 消歧）。
export function walkBackendRoutes() {
  const dirs = [join(BACKEND, 'src', 'routes'), join(BACKEND, 'src', 'handlers')];
  const fileFns = {};
  for (const dir of dirs) {
    for (const f of collectRsFiles(dir)) {
      const base = f.split(/[\\/]/).pop().replace('.rs', '');
      if (fileFns[base]) continue; // routes 目录优先，避免同名覆盖
      fileFns[base] = extractFunctions(readFileSync(f, 'utf-8'));
    }
  }
  const endpoints = new Set(); // path|METHOD
  const handlers = new Map(); // path|METHOD -> {handler, routesFile}
  const getBuilder = file => fileFns[file] || {};
  const resolveTarget = (currentFile, target) => (target.file ? target.file : currentFile);

  const walk = (file, fnName, prefix) => {
    const fns = getBuilder(file);
    const body = fns[fnName];
    if (body === undefined) return; // 未索引到的外部模块，跳过
    const parsed = parseBuilder(body);
    // 该函数体若没有直接的 route/nest/merge，但整体是一次 Router 委托调用
    // （如 `sales_return_handler::router()`），递归进入被委托的构造器。
    if (
      !parsed.routes.length &&
      !parsed.nests.length &&
      !parsed.merges.length &&
      !/\.(route|nest|merge)\s*\(/.test(body)
    ) {
      const t = parseTarget(body);
      if (t) walk(resolveTarget(file, t), t.fn, prefix);
      return;
    }
    for (const r of parsed.routes) {
      const full = normalizePath(prefix + r.path);
      const handlerByMethod = {};
      for (const h of r.handlers || []) handlerByMethod[h.method] = h.handler;
      for (const meth of r.methods) {
        const key = `${full} ${meth}`;
        endpoints.add(key);
        if (!handlers.has(key) && handlerByMethod[meth])
          handlers.set(key, { handler: handlerByMethod[meth], routesFile: file });
      }
    }
    for (const n of parsed.nests) {
      const childPrefix = joinPrefix(prefix, n.prefix);
      walk(resolveTarget(file, n.target), n.target.fn, childPrefix);
    }
    for (const mg of parsed.merges) {
      walk(resolveTarget(file, mg.target), mg.target.fn, prefix);
    }
  };

  const joinPrefix = (a, b) => {
    const na = a === '/' ? '' : a;
    const nb = b.startsWith('/') ? b : '/' + b;
    return na + nb || '/';
  };

  // 入口：mod.rs::create_router
  walk('mod', 'create_router', '');
  return { endpoints, handlers };
}

function loadBackendEndpoints() {
  return walkBackendRoutes().endpoints;
}

// ---------- 已知例外（逐条显式登记，禁止通配/静默） ----------
// 分为两类，每条附「具体原因」。命中者不计入 A 类失败，但仍每次运行逐条打印，
// 以免被误当作稳态；本轮修完两类后仍无法在既定范围内消除者才登记于此。
//
// 【G = 功能缺口】前端调用的语义后端根本没有对应实现，不能伪造端点、也不能把
//     写操作降级成 GET 去迁就前端——登记为待产品/后端排期确认。
// 【D = 范围外前置漂移】属本任务两类（双重 nest）+ 列举 7 项方法之外的历史契约漂移：
//     后端在别的路径/方法上提供了同类操作，但「正确映射」需业务语义确认（如
//     receive vs confirm、confirm/send vs base send），不可盲改前端，单独排期处理。
const KNOWN_GAPS = new Map([
  // ---- 本轮范围内、经判读确认的 5 项功能缺口（对应任务列举 7 项中的 1/2/3/4/6） ----
  [
    `${BASE_URL}/ar-reconciliations-enhanced/auto-match GET`,
    'G: 触发式自动对账为写(后端 POST /auto-match)；前端另发 GET 想分页列出结果，后端无此只读列表端点（基础 /ar-reconciliations 入参/响应契约不同）',
  ],
  [
    `${BASE_URL}/ar-reconciliations/* POST`,
    'G: 前端 createReconciliationDetail 想为已有对账单新增明细行，后端无 /ar-reconciliations/{id} 的 POST 写端点（且该封装未被页面接线）',
  ],
  [
    `${BASE_URL}/currencies POST`,
    'G: 前端 createCurrency 新增币种，后端 currency_handler 无 create_currency，币种主数据目前只读+设本位币',
  ],
  [
    `${BASE_URL}/customer-shares GET`,
    'G: 前端 listCustomerShares 想全量列出共享记录，后端仅提供 by-customer/by-user 过滤查询，无全量 GET 列表',
  ],
  [
    `${BASE_URL}/data-import/templates POST`,
    'G: 前端 createImportTemplate 新建导入模板，后端 import_export_handler 无 create_import_template',
  ],
  // ---- 【D 类】范围外前置契约漂移（非本轮两类/7 项）：后端操作位于别处或语义待确认 ----
  [
    `${BASE_URL}/report-templates/* GET`,
    'D: 报表模板详情后端在 /reports/enhanced/templates/{id}（同 get/put/delete handler），report-templates.ts 走的是不存在的 /report-templates/{id}',
  ],
  [`${BASE_URL}/report-templates/* PUT`, 'D: 同上，更新应指向 /reports/enhanced/templates/{id}'],
  [`${BASE_URL}/report-templates/* DELETE`, 'D: 同上，删除应指向 /reports/enhanced/templates/{id}'],
  [
    `${BASE_URL}/report-templates/*/preview GET`,
    'D: 预览后端在 /reports/enhanced/templates/{id}/preview',
  ],
  [
    `${BASE_URL}/report-templates/*/generate POST`,
    'D: 生成后端在 /reports/enhanced/templates/{id}/execute（命名差异），且无 /generate，需确认映射',
  ],
  [
    `${BASE_URL}/system-update/backups/* GET`,
    'G: 后端 system_update 仅有 /system-update/backups 列表，无备份详情 /backups/{id}',
  ],
  [`${BASE_URL}/system-update/backups/* DELETE`, 'G: 后端无 /system-update/backups/{id} 删除端点'],
  [
    `${BASE_URL}/system-update/backups/*/restore POST`,
    'G: 后端无 /system-update/backups/{id}/restore 端点',
  ],
  [
    `${BASE_URL}/system-update/backups/*/download GET`,
    'G: 后端无 /system-update/backups/{id}/download 端点',
  ],
  [
    `${BASE_URL}/customers/select GET`,
    'D: 客户下拉后端在 /crm/customers/select，customer.ts 走的是不存在的 /customers/select',
  ],
  [
    `${BASE_URL}/crm/customers/*/tags/* POST`,
    'D: 给客户加标签后端为 /crm/customers/{id}/tags（add_tags，无 tagId 段），前端多带 /{tagId}',
  ],
  [
    `${BASE_URL}/crm/customers/*/tags/* DELETE`,
    'D: 删标签后端为 /crm/tags/{id}（delete_tag），非 /crm/customers/{id}/tags/{tagId}',
  ],
  [
    `${BASE_URL}/data-permissions/roles/*/* GET`,
    'G: 后端 data_permissions 仅有 /roles/{role_id}，无 /roles/{roleId}/{resourceType} 两段查询',
  ],
  [
    `${BASE_URL}/data-permissions/roles/*/* DELETE`,
    'G: 后端无 /roles/{roleId}/{resourceType} 两段删除端点',
  ],
  [
    `${BASE_URL}/purchase/purchase-contracts/export GET`,
    'G: 采购合同后端无 /purchase-contracts/export 导出端点（其它域导出惯例不涵盖此资源）',
  ],
  [
    `${BASE_URL}/purchase/receipts/*/receive POST`,
    'D: 收货确认后端在 /purchase/receipts/{id}/confirm（confirm_receipt），receive 与 confirm 是否同义需业务确认',
  ],
  [
    `${BASE_URL}/ar-reconciliations-enhanced/*/confirm/send POST`,
    'D: 发送客户对账确认后端为 /ar-reconciliations/{id}/send（base 域），增强域无 confirm/send',
  ],
]);

// ---------- 对照主流程 ----------
function main() {
  const fe = loadFrontendEndpoints();
  const be = loadBackendEndpoints();

  const aClass = [];
  const gaps = [];
  for (const [key, occ] of fe) {
    if (be.has(key)) continue;
    const reason = KNOWN_GAPS.get(key);
    if (reason) gaps.push({ key, reason, occ });
    else aClass.push({ key, occ });
  }

  // 其余类别：后端有、前端零调用（仅统计）
  let fePathSet = new Set();
  for (const k of fe.keys()) fePathSet.add(k.split(' ')[0]);
  let beNoFeCall = 0;
  for (const k of be) {
    const [p] = k.split(' ');
    if (!fePathSet.has(p)) beNoFeCall++;
  }

  console.log('=== check-api-paths: 前缀感知路由对照 ===');
  console.log(`前端 (path,method) 组合: ${fe.size}`);
  console.log(`后端 (path,method) 组合: ${be.size}`);
  console.log(`后端路径去重: ${new Set([...be].map(k => k.split(' ')[0])).size}`);
  console.log(`前端调用而后端不存在(全量): ${aClass.length + gaps.length}`);
  console.log(`  ├─ A 类(未登记/可修): ${aClass.length}`);
  console.log(`  └─ 已登记功能缺口:     ${gaps.length}`);
  console.log(`后端存在但前端该路径零调用: ${beNoFeCall}`);

  if (gaps.length) {
    console.log('\n[功能缺口 · 待产品/后端确认，非本轮静默豁免，逐条附原因]');
    for (const g of gaps) {
      console.log(`  - ${g.key}`);
      console.log(`      原因: ${g.reason}`);
      console.log(`      前端: ${[...new Set(g.occ.map(o => o.file))].join(', ')}`);
    }
  }

  if (aClass.length) {
    console.error('\n[A 类 · 前端调用命中后端不存在的 (路径,方法) 组合 —— 必须修复]');
    for (const a of aClass) {
      console.error(`  - ${a.key}`);
      for (const o of a.occ) console.error(`      ${o.file}  ("${o.url}")`);
    }
    console.error(`\nFAIL: A 类 ${aClass.length} 条`);
    process.exit(1);
  }

  console.log('\nOK: A 类为 0（功能缺口已逐条显式登记，非静默豁免）。');
}

// 仅在作为脚本直接执行时跑主流程；被其它门禁 import 复用解析能力时不触发（不改退出码）。
const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) main();
