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
 *       node scripts/check-api-keys.mjs --self-test
 *       （--self-test 用内联夹具双向自证判定器本身：正例必抓、反例必不报，不扫仓库）
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
export function buildUniverse() {
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
  return universe;
}

// ---------- 前端接口声明键 ----------
/**
 * 扫描单个 TS 源文件，返回"块内键 ∉ 后端字段全集"的孤儿列表。
 *
 * iface 状态跟踪的硬约束（本判定器根修，缺陷形态见提交 79f00769 的说明）：
 * 旧实现是「命中任意 interface/type 定义行就置 iface、只有遇到 } 才归零」，
 * 于是单行类型别名 `export type X = PaginatedResponse<Y>;` 置上 iface 后永不归零，
 * 紧随其后的任意对象字面量（展示层常量、函数体内对象等）的键都被当成该接口的键
 * 送去比对后端字段全集 —— export-inspection 的 pass/fail/qualified/unqualified
 * 被误报成 ExportInspectionPage 的"编造键"即此族。
 *
 * 现口径：只有 `interface X` / `type X =` 之后**同一行或下一条非空行真的开了 `{`**
 * 才进入 iface 状态，块闭合（花括号配平归零）即退出；单行别名、含分号的别名体、
 * `export const x = {...}`、函数体内对象字面量都不得继承上一个 iface。
 * 真接口/块式别名里的键仍逐键比对，正向检测力不降（由 --self-test 夹具双向钉死）。
 */
export function scanTsSource(rel, source, universe) {
  const orphans = [];
  const lines = source.split(/\r?\n/);
  let iface = null;
  let pending = null; // 定义行已出现但同行未见 '{'：等下一条非空行行首开块
  let depth = 0;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    if (iface === null && pending !== null) {
      if (trimmed === '') continue;
      if (trimmed.startsWith('{')) {
        iface = pending;
        pending = null;
      } else {
        pending = null; // 后续行没开块（联合类型/多行表达式等）：丢弃待定状态
      }
    }
    if (iface === null) {
      const def = line.match(/^(?:export\s+)?(?:interface|type)\s+([A-Za-z0-9_]+)(.*)$/);
      if (!def) continue;
      const rest = def[2];
      if (rest.includes('{')) {
        iface = def[1]; // 同行真开了 '{'：进入块状态，本行继续走深度/键扫描
      } else if (!rest.includes(';')) {
        pending = def[1]; // 声明未完：等下一非空行行首的 '{'
        continue;
      } else {
        continue; // 单行别名（type X = Foo<Y>;）等：没开块，不进 iface 状态
      }
    }
    depth += (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;
    for (const m of line.matchAll(/^\s*(?:readonly\s+)?([a-z][a-zA-Z0-9_]*)\??\s*:/g)) {
      const key = m[1];
      if (!universe.has(key)) orphans.push({ file: rel, line: i + 1, iface, key });
    }
    if (depth <= 0 && /\}/.test(line)) {
      iface = null;
      depth = 0;
    }
  }
  return orphans;
}

// ---------- 内联夹具自证（--self-test）----------
/**
 * 判定器自身的双向自证：夹具不落仓库文件、不扫真实前后端，只走 scanTsSource 真实代码路径。
 * 正例（真接口/块式别名里的编造键）必须被抓到；反例（单行别名之后的展示层常量、
 * interface 之前的展示层常量、函数体内对象字面量）必须一条不报。
 * 任一断言不符即退出码 1 —— 防止"检测力丢失后输出 0 错"的假绿。
 */
function runSelfTest() {
  const universe = new Set([
    'id',
    'name',
    'result',
    'status',
    'page',
    'page_size',
    'items',
    'total',
    'params',
  ]);
  const cases = [
    {
      name: '正例：interface 块内后端不存在的键必须报红',
      src: `export interface Widget {
  id: number;
  result: string;
  ghost_key: string;
}
`,
      expect: ['Widget.ghost_key'],
    },
    {
      name: '正例：{ 在定义行下一行才开的 interface 同样必须跟踪',
      src: `export interface Multi
{
  id: number;
  ghost_two: string;
}
`,
      expect: ['Multi.ghost_two'],
    },
    {
      name: '正例：块式 type 别名（type X = { ... }）里的编造键必须报红',
      src: `export type WidgetShape = {
  id: number;
  ghost_three: string;
};
`,
      expect: ['WidgetShape.ghost_three'],
    },
    {
      name: '反例（本次真实形态）：单行别名之后紧跟展示层常量必须一条不报',
      src: `export interface ExportInspection {
  id: number;
  result: string;
}
export type ExportInspectionPage = PaginatedResponse<ExportInspection>;
export const inspectionResultTagMap: Record<string, 'info' | 'danger'> = {
  pending: 'info',
  pass: 'info',
  fail: 'danger',
  qualified: 'danger',
  unqualified: 'danger',
};
`,
      expect: [],
    },
    {
      name: '回归反例：展示层常量放在 interface 之前必须不报，且其后的 interface 键照常比对',
      src: `export const inspectionResultTagMap: Record<string, 'info' | 'danger'> = {
  pass: 'info',
  fail: 'danger',
};
export interface Widget {
  id: number;
  ghost_key: string;
}
`,
      expect: ['Widget.ghost_key'],
    },
    {
      name: '反例：interface 闭合后函数体内的对象字面量不得继承上一个 iface',
      src: `export interface Widget {
  id: number;
}
export function makeWidget(): Widget {
  return {
    id: 1,
    ghost_in_body: 2,
  };
}
`,
      expect: [],
    },
  ];
  let pass = 0;
  let fail = 0;
  console.log('=== self-test：check-api-keys iface 跟踪双向自证 ===');
  for (const c of cases) {
    const got = scanTsSource('fixture.ts', c.src, universe)
      .map(o => `${o.iface}.${o.key}`)
      .sort();
    const want = [...c.expect].sort();
    const okFlag = got.length === want.length && got.every((v, k) => v === want[k]);
    if (okFlag) {
      pass++;
      console.log(`  PASS  ${c.name}  [检出 ${JSON.stringify(got)}]`);
    } else {
      fail++;
      console.log(
        `  FAIL  ${c.name}  -> 期望 ${JSON.stringify(want)}，实际 ${JSON.stringify(got)}`
      );
    }
  }
  console.log(`---- self-test 结论：${pass} 通过 / ${fail} 失败 ----`);
  if (fail) {
    console.error('[api-keys] ✗ 判定器自证失败：iface 跟踪存在漏报或误报，门禁不可信');
    process.exit(1);
  }
  console.log('[api-keys] ✅ iface 跟踪自证通过（正例必抓、反例必不报）');
  process.exit(0);
}

// ---------- 主流程 ----------
function main() {
  const universe = buildUniverse();
  const orphans = [];
  for (const f of walk(path.join(FRONTEND, 'src', 'api')).filter(x => /\.ts$/.test(x))) {
    const rel = path.relative(REPO, f).replace(/\\/g, '/');
    orphans.push(...scanTsSource(rel, fs.readFileSync(f, 'utf8'), universe));
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
    console.error(
      '[api-keys] ✗ 缺基线文件 scripts/api-keys-baseline.json（先跑 --write-baseline）'
    );
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
    console.log(`  已收敛 ${f}: ${base.count} → ${current[f] ?? 0}（请连同基线一起下调）`);
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
}

const invokedDirectly =
  process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  if (process.argv.includes('--self-test')) {
    runSelfTest();
  } else {
    main();
  }
}
