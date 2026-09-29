/**
 * route-snapshot.mjs — 后端路由「解析后完整集合」的基线快照与漂移检查
 *
 * 为什么需要：backend 的挂载风格不统一（域文件写相对路径 + mod.rs 组合前缀，
 * 但 mod.rs 自己也直接注册了绝对路径，failover.rs 整文件绝对路径，
 * 个别 handler 文件里自带 router()）。这让端点统计必须靠特例解析；
 * 而任何"统一挂载风格"的重构都需要一个不依赖编译的等价性证明。
 *
 * 本工具复用 walkBackendRoutes()（与 check-api-paths 同一套 nest/merge 语义，不另起口径），
 * 把 (METHOD, 完整路径, handler 符号, 定义文件) 集合固化成文本快照。
 * 重构前后快照必须逐字相同 —— 相同即证明"改的是挂载结构，不是 URL"。
 *
 * 快照头部还记录 routes/ 下 .layer() 的总数：路径集合相同但中间件作用域被挪动，
 * 也是运行期行为变化，头部数字一变就会显式判负，逼人去核对语义。
 *
 * 用法：
 *   node scripts/route-snapshot.mjs --write   # 生成/更新基线
 *   node scripts/route-snapshot.mjs --check   # 比对，漂移则 exit 1
 */
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { walkBackendRoutes, collectRsFiles } from './check-api-paths.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const SNAPSHOT = join(HERE, 'route-snapshot.txt');
const BACKEND_ROUTES = join(HERE, '..', '..', 'backend', 'src', 'routes');
const MODE = process.argv.includes('--write') ? 'write' : 'check';

function layerCount() {
  return collectRsFiles(BACKEND_ROUTES).reduce(
    (n, f) => n + (readFileSync(f, 'utf8').match(/\.layer\(/g) || []).length,
    0
  );
}

function currentSnapshot() {
  const { handlers } = walkBackendRoutes();
  const lines = [];
  for (const [key, info] of handlers) {
    const i = key.lastIndexOf(' ');
    lines.push(`${key.slice(i + 1)}\t${key.slice(0, i)}\t${info.handler}\t${info.routesFile}`);
  }
  lines.sort();
  return [`# layers=${layerCount()} endpoints=${lines.length}`, ...lines].join('\n') + '\n';
}

const body = currentSnapshot();

if (MODE === 'write') {
  writeFileSync(SNAPSHOT, body, 'utf8');
  console.log(`[route-snapshot] 基线已写入：${body.split('\n').length - 2} 条端点`);
  process.exit(0);
}

if (!existsSync(SNAPSHOT)) {
  console.error('[route-snapshot] ❌ 缺基线文件，请先执行 --write');
  process.exit(1);
}

const base = readFileSync(SNAPSHOT, 'utf8');
if (base === body) {
  console.log(`[route-snapshot] ✅ 与基线逐字一致（${body.split('\n').length - 2} 条端点）`);
  process.exit(0);
}

const bset = new Set(base.split('\n').filter(Boolean));
const nset = new Set(body.split('\n').filter(Boolean));
const added = [...nset].filter(l => !bset.has(l));
const removed = [...bset].filter(l => !nset.has(l));
console.error(
  `[route-snapshot] ❌ 路由集合漂移：新增 ${added.length} / 消失 ${removed.length}`
);
for (const l of added.slice(0, 25)) console.error(`  + ${l}`);
for (const l of removed.slice(0, 25)) console.error(`  - ${l}`);
process.exit(1);
