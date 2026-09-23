#!/usr/bin/env node
/**
 * 前端接口键 ↔ 后端字段全集 棘轮门禁。
 *
 * 比对的是"前端类型里声明的每一个键，在后端是否存在同名出参/入参字段"：
 * 后端多数列表接口直接序列化 SeaORM 实体，出参键即实体字段名，因此后端全局
 * 不存在的键必然取不到值 —— 这一族缺陷表现为列恒空、筛选恒 0 行、
 * v-if 门控永假，且因形状相同而不会被信封形状门禁发现。
 *
 * 判负口径：存量按 文件 → 未知键数 记入基线；只允许下降，不允许上升。
 * 单文件上升即失败并打印具体键；新增文件出现未知键同样失败。
 * 这样既不为历史债一次性阻塞 CI，也不允许再引入新的编造键。
 *
 * 用法：node scripts/check-api-keys.mjs [--write-baseline] [--json]
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND = path.resolve(HERE, '..');
const REPO = path.resolve(FRONTEND, '..');
const BASELINE = path.join(HERE, 'api-keys-baseline.json');

const walk = (dir, out = []) => {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === 'node_modules' || e.name === 'target' || e.name.startsWith('.')) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
};

// ---------- 后端字段全集 ----------
const universe = new Set();
for (const f of walk(path.join(REPO, 'backend', 'src')).filter(x => x.endsWith('.rs'))) {
  const lines = fs.readFileSync(f, 'utf8').split(/\r?\n/);
  for (const line of lines) {
    for (const m of line.matchAll(/pub(?:\([a-z]+\))?\s+([a-z_][a-z0-9_]*)\s*:/g))
      universe.add(m[1]);
    for (const m of line.matchAll(/"([a-z_][a-z0-9_]*)"\s*:/g)) universe.add(m[1]);
    for (const m of line.matchAll(/(?:rename|column_name)\s*=\s*"([a-z_][a-z0-9_]*)"/g))
      universe.add(m[1]);
  }
}

// ---------- 前端接口声明键 ----------
const orphans = [];
for (const f of walk(path.join(FRONTEND, 'src', 'api')).filter(x => /\.ts$/.test(x))) {
  const rel = path.relative(REPO, f).replace(/\\/g, '/');
  const lines = fs.readFileSync(f, 'utf8').split(/\r?\n/);
  let iface = null;
  let depth = 0;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const def = line.match(/^(?:export\s+)?(?:interface|type)\s+([A-Za-z0-9_]+)/);
    if (def && depth === 0) iface = def[1];
    if (!iface) continue;
    depth += (line.match(/[{(]/g) || []).length - (line.match(/[})]/g) || []).length;
    for (const m of line.matchAll(/^\s*(?:readonly\s+)?([a-z][a-zA-Z0-9_]*)\??\s*:/g)) {
      const key = m[1];
      if (!universe.has(key)) orphans.push({ file: rel, line: i + 1, iface, key });
    }
    if (depth <= 0 && /}/.test(line)) {
      iface = null;
      depth = 0;
    }
  }
}

const byFile = new Map();
for (const o of orphans) {
  if (!byFile.has(o.file)) byFile.set(o.file, []);
  byFile.get(o.file).push(o);
}
const current = {};
const currentKeys = {};
for (const [f, list] of byFile) {
  current[f] = list.length;
  currentKeys[f] = list.map(o => `${o.iface}.${o.key}`).sort();
}

if (process.argv.includes('--write-baseline')) {
  const files = {};
  for (const [f, keys] of Object.entries(currentKeys)) files[f] = { count: current[f], keys };
  fs.writeFileSync(BASELINE, JSON.stringify({ total: orphans.length, files }, null, 2) + '\n');
  console.log(
    `[api-keys] 基线已写入：${orphans.length} 个未知键，分布于 ${Object.keys(files).length} 个文件`
  );
  process.exit(0);
}

if (!fs.existsSync(BASELINE)) {
  console.error('[api-keys] ✗ 缺基线文件 scripts/api-keys-baseline.json（先跑 --write-baseline）');
  process.exit(1);
}
const rawBaseline = JSON.parse(fs.readFileSync(BASELINE, 'utf8'));
// 兼容仅记计数的旧格式
const baseline = {
  total: rawBaseline.total,
  files: Object.fromEntries(
    Object.entries(rawBaseline.files).map(([f, v]) => [
      f,
      typeof v === 'number' ? { count: v, keys: [] } : v,
    ])
  ),
};

const failures = [];
for (const [f, n] of Object.entries(current)) {
  const base = baseline.files[f];
  if (base === undefined) {
    failures.push(`新文件出现 ${n} 个后端不存在的接口键：${f} → ${currentKeys[f].join(' ')}`);
  } else if (n > base.count) {
    // 用基线键名集合做差，精确定位新增项（按偏移量取切片会把已有键误报成新增）
    const known = new Set(base.keys);
    const added = currentKeys[f].filter(k => !known.has(k));
    const shown = added.length ? added.join(' ') : `(基线未记键名，共 ${n - base.count} 个增量)`;
    failures.push(`${f}: 未知接口键由 ${base.count} 增至 ${n}（+${n - base.count}）→ ${shown}`);
  }
}

const reduced = Object.entries(baseline.files).filter(
  ([f, base]) => (current[f] ?? 0) < base.count
);
console.log(
  `[api-keys] 后端字段全集 ${universe.size}；前端未知接口键 ${orphans.length}（基线 ${baseline.total}），文件 ${Object.keys(current).length}`
);
for (const [f, base] of reduced) {
  console.log(`  已收敛 ${f}: ${base} → ${current[f] ?? 0}（请连同基线一起下调）`);
}
if (failures.length) {
  console.error(`\n[api-keys] ✗ ${failures.length} 个文件出现新的编造键：`);
  for (const m of failures) console.error(`  ${m}`);
  console.error('\n口径：前端类型里的每个键都必须在后端有同名字段（出参或入参）。');
  console.error('要么改读后端真实键，要么由后端补出该字段；不允许保留取不到值的键名。');
  process.exit(1);
}
if (reduced.length) {
  console.error(
    '\n[api-keys] ✗ 有文件已收敛但基线未下调，请跑 node scripts/check-api-keys.mjs --write-baseline'
  );
  process.exit(1);
}
console.log('[api-keys] ✅ 无新增编造键');
process.exit(0);
