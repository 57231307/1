/**
 * check-route-mount.mjs — 后端路由「挂载结构」约定门禁（报告型，是否升级为阻断待定）
 *
 * 起因：同一套路由存在多种挂载写法，端点统计必须靠特例解析。实测口径（本脚本给出）：
 * 域文件基本都遵守"相对路径 + mod.rs 用 nest 组合前缀"，少数文件例外。
 *
 * 约定（唯一正确形态）：
 *   ① 域文件 `pub fn routes() -> Router<AppState>` 内只用**相对路径**字面量；
 *   ② 前缀只在 `routes/mod.rs` 里通过 `.nest("/api/v1/erp", ...)` 组合，每个域只 nest 一次；
 *   ③ 路由注册只写在 `backend/src/routes/` 下，handler 文件不得自带 `router()`；
 *   ④ 定义了 Router 的 pub fn 必须真的被挂上（孤儿路由 = 端点统计漏计 + 权限面失真）。
 *
 * 现存偏离用**按文件登记的条数 + 理由**显式豁免（禁止整片豁免；偏离减少后要删条目，
 * 反向偏离也判负，避免"豁免数当配额"用）。
 *
 * 用法：node scripts/check-route-mount.mjs [--json]
 */
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { collectRsFiles } from './check-api-paths.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = join(HERE, '..', '..', 'backend', 'src');
const ROUTES = join(SRC, 'routes');
const HANDLERS = join(SRC, 'handlers');
const JSON_OUT = process.argv.includes('--json');
const SRC_FWD = SRC.replace(/\\/g, '/');

/** R1 豁免：文件 -> { 允许的绝对路径条数, 理由 } */
const ABS_ALLOW = new Map([
  [
    'routes/failover.rs',
    {
      n: 5,
      why: '整文件写绝对路径。改法是相对化 + mod.rs nest("/api/v1/erp/admin/failover")；' + '属独立后端批次（本地不可编译验证），先显式登记。',
    },
  ],
]);

/** R2 豁免：handler 文件内的 Router 构造函数 */
const HANDLER_ROUTER_ALLOW = new Map([
  [
    'handlers/sales_return_handler.rs::router',
    '注册应移入 routes/；否则路由表解析与权限面统计都要为它开特例（本仓 check-api-paths 已为此打过补丁）',
  ],
]);

function toRel(f) {
  const p = f.replace(/\\/g, '/');
  return p.startsWith(SRC_FWD) ? p.slice(SRC_FWD.length + 1) : p;
}
function stripComments(src) {
  return src.replace(/\/\/[^\n]*/g, '').replace(/\/\*[\s\S]*?\*\//g, '');
}

const violations = [];
const stats = {
  routeFiles: 0,
  routeLiterals: 0,
  absoluteLiterals: 0,
  routerFns: 0,
  orphanRouters: 0,
};

// ---------- R1 相对路径 ----------
const routeFiles = collectRsFiles(ROUTES);
stats.routeFiles = routeFiles.length;
const allRouteSrc = [];
for (const f of routeFiles) {
  const rel = toRel(f);
  const src = stripComments(readFileSync(f, 'utf8'));
  allRouteSrc.push({ rel, src });
  const lits = [...src.matchAll(/\.route\(\s*"([^"]*)"/g)].map(m => m[1]);
  stats.routeLiterals += lits.length;
  const abs = lits.filter(p => p.startsWith('/api/v1'));
  stats.absoluteLiterals += abs.length;
  const allow = ABS_ALLOW.get(rel);
  const allowed = allow ? allow.n : 0;
  if (abs.length > allowed) {
    for (const p of abs.slice(allowed))
      violations.push({ rule: 'R1', where: rel, what: p, note: `该文件允许 ${allowed} 条，实际 ${abs.length} 条` });
  }
}

// ---------- R2 注册位置 ----------
for (const f of collectRsFiles(HANDLERS)) {
  const rel = toRel(f);
  const src = stripComments(readFileSync(f, 'utf8'));
  for (const m of src.matchAll(/pub\s+(?:async\s+)?fn\s+(\w*router\w*)\s*\(/g)) {
    const id = `${rel}::${m[1]}`;
    if (!HANDLER_ROUTER_ALLOW.has(id))
      violations.push({ rule: 'R2', where: rel, what: m[1], note: '路由注册只能在 backend/src/routes/ 下' });
  }
}

// ---------- R3 挂载可达（只揪孤儿，宽松口径避免误报） ----------
// 真实入口 `create_router` 由 bootstrap/middleware_bootstrap.rs 调用，
// 故"已挂载"的判据是：routes/ 之外（整个 backend/src）或别的路由文件里调用过它。
const called = new Set();
/**
 * 收集"调用点"。必须排除定义点本身：`pub fn foo() -> Router<..>` 里的 `foo()`
 * 不是调用，否则任何无参路由函数都会把自己判成已挂载（实测漏报过一次）。
 */
function addCallSites(src, set) {
  for (const m of src.matchAll(/\b([A-Za-z_]\w*)\s*\(/g)) {
    const before = src.slice(Math.max(0, m.index - 12), m.index);
    if (/\b(?:pub\s+)?(?:async\s+)?fn\s+[\w]*$/.test(before)) continue;
    set.add(m[1]);
  }
}
for (const { src } of allRouteSrc) addCallSites(src, called);
const outsideRefs = collectRsFiles(SRC)
  .map(f => toRel(f))
  .filter(rel => !rel.startsWith('routes/'));
for (const rel of outsideRefs) {
  let src = '';
  try {
    src = stripComments(readFileSync(join(SRC, rel), 'utf8'));
  } catch {
    continue;
  }
  addCallSites(src, called);
}
for (const { rel, src } of allRouteSrc) {
  for (const m of src.matchAll(/pub\s+fn\s+(\w+)\s*\([^)]*\)\s*->\s*(?:Router|impl\s+Mount)/g)) {
    stats.routerFns++;
    // 自己的定义行不算被引用
    const usedElsewhere = allRouteSrc.some(o => o.rel !== rel && o.src.includes(m[1] + '('));
    if (!called.has(m[1]) && !usedElsewhere) {
      stats.orphanRouters++;
      violations.push({ rule: 'R3', where: rel, what: m[1], note: '定义了 Router 但 routes/ 内无人调用（未挂载 = 死路由）' });
    }
  }
}

if (JSON_OUT) {
  console.log(JSON.stringify({ stats, violations }, null, 2));
  process.exit(violations.length ? 1 : 0);
}

console.log('=== check-route-mount: 后端挂载结构约定 ===');
console.log(
  `路由文件 ${stats.routeFiles} 个 / .route 字面量 ${stats.routeLiterals} 条 / 其中绝对路径 ${stats.absoluteLiterals} 条`
);
console.log(`Router 构造函数 ${stats.routerFns} 个，其中孤儿 ${stats.orphanRouters} 个`);
console.log(
  `已登记豁免：R1 ${[...ABS_ALLOW.entries()].map(([k, v]) => `${k}=${v.n}`).join(', ') || '无'}；` +
    `R2 ${HANDLER_ROUTER_ALLOW.size} 条`
);
if (!violations.length) {
  console.log('\nOK: 现存偏离均已显式登记且未超出登记量。');
  process.exit(0);
}
console.log(`\n❌ 超出登记的偏差 ${violations.length} 处：`);
for (const v of violations.slice(0, 40)) console.log(`  - ${v.rule} ${v.where} :: ${v.what}  [${v.note}]`);
process.exit(1);
