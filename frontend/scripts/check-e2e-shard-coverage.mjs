#!/usr/bin/env node
// E2E 分片覆盖自检（任务 #306）
//
// 目标：把"分片矩阵分配集合的并集 == playwright --list 全集"做成 CI 内可执行的硬门禁。
// 背景缺陷：ci-cd.yml 的 ci-e2e 分片按"目录分组 + --shard hash 分片"手工登记，
// e2e/fullflow/（17 spec / 96 用例）从未被任何分片命令传入，主 CI 全绿但 96 条用例
// 一条都没执行（静默丢覆盖）。本脚本对 workflow 的分片编排做三层校验，任何一层
// 失配都以退出码 1 判红，截断/漏登记/空洞分片不可能静默通过：
//   A. 解析锚点校验：workflow 的 if/elif/else 分片结构与 case 登记行必须能被逐字解析，
//      结构漂移（改写法、删分支、改了目录名格式）→ 报错，绝不按"解析不出=没这问题"处理。
//   B. hash 分片完整性：同一目录分组内，矩阵分配到的 --shard 序号必须恰好是 1..TOTAL
//      无缺口（缺 1/2 意味着该组一半用例被 Playwright 哈希分到无人执行的片上 → 静默丢）。
//      矩阵 shard 编号集合与 case 登记集合必须双向相等（登记了但矩阵没有 = 不执行；
//      矩阵有但没登记 = workflow 自身的 ::error 守卫，这里同样判红）。
//   C. 全集并集校验：对每个分组用真实 `playwright test --list`（只收集、不执行任何用例）
//      取"该片会跑什么"，各组并集与 chromium 项目全集做差集；missing 非空即丢覆盖，
//      extra（分组里写了不存在的目标/空匹配）同样判红。
//
// 用法（在任意目录均可，路径以脚本自身位置解析）：
//   node scripts/check-e2e-shard-coverage.mjs [--workflow <path>] [--min-shard-list <n>]
// 退出码：0=全覆盖通过；1=任一层校验失败。
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..', '..');
const FRONTEND_DIR = path.join(REPO_ROOT, 'frontend');

function arg(name, dflt) {
  const i = process.argv.indexOf(`--${name}`);
  if (i !== -1 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--'))
    return process.argv[i + 1];
  return dflt;
}
const WORKFLOW_PATH = path.resolve(
  arg('workflow', path.join(REPO_ROOT, '.github', 'workflows', 'ci-cd.yml'))
);

const errors = [];
function fail(msg) {
  errors.push(msg);
}
function note(msg) {
  console.log(msg);
}
function annotate(msg) {
  console.log(`::error::${msg}`);
}

// ---------------------------------------------------------------- 1. 解析 workflow
const wf = readFileSync(WORKFLOW_PATH, 'utf8');

// ci-e2e job 文本块：从 "  ci-e2e:" 到下一个顶层 job key（两空格缩进的 "  <id>:"）
const ciE2eStart = wf.indexOf('\n  ci-e2e:');
if (ciE2eStart === -1) {
  annotate('无法在 workflow 中定位 ci-e2e job（解析锚点 "  ci-e2e:" 失配）');
  process.exit(1);
}
const rest = wf.slice(ciE2eStart + 1);
const nextJob = rest.slice(1).match(/^  [a-zA-Z0-9_-]+:/m);
const ciE2eBlock = nextJob ? rest.slice(0, rest.indexOf(nextJob[0], 1)) : rest;

// 矩阵 shard 编号（strategy.matrix.include 下的 "- shard: N" 行）
const matrixShards = [...ciE2eBlock.matchAll(/^\s+- shard: (\d+)$/gm)].map(m => Number(m[1]));
if (matrixShards.length === 0) {
  annotate('ci-e2e 矩阵未解析出任何 shard（解析锚点 "- shard: N" 失配）');
  process.exit(1);
}

// 跑测 step 的 bash 体：从 "name: 运行 E2E 测试" 的 run: | 到下一个 "- name:"
const stepM = ciE2eBlock.match(
  /- name: 运行 E2E 测试[^\n]*\n(?:[^\n]*\n)*?\s+run: \|\n([\s\S]*?)\n\s*- name: /
);
if (!stepM) {
  annotate('无法提取「运行 E2E 测试」step 的 bash 体（step 名或 run: | 结构漂移）');
  process.exit(1);
}
const bash = stepM[1];

// 三类目录分支：if/elif 的 "$SHARD" -le 阈值按出现顺序 = flow / smoke / traversal
const conds = [...bash.matchAll(/\[ "\$SHARD" -le (\d+) \]; then/g)].map(m => ({
  threshold: Number(m[1]),
  at: m.index,
}));
if (conds.length !== 3) {
  annotate(
    `分片分支结构漂移：期望 3 个 "[ "$SHARD" -le N ]" 条件（flow/smoke/traversal），实得 ${conds.length} 个`
  );
}
const branchBodies = [];
for (let i = 0; i < conds.length; i++) {
  const end = i + 1 < conds.length ? conds[i + 1].at : bash.indexOf('else', conds[i].at);
  branchBodies.push(bash.slice(conds[i].at, end));
}

// 分支内派生量：SPEC_TARGETS=<dir>/、SHARD_TOTAL=N、SHARD_NO=$((SHARD ± K))
function parseDerivedBranch(body, kind) {
  const t = body.match(/^\s*SPEC_TARGETS=(e2e\/\S+?)\s*$/m);
  const total = body.match(/^\s*SHARD_TOTAL=(\d+)\s*$/m);
  const no = body.match(/^\s*SHARD_NO=\$\(\(SHARD (\+|-) (\d+)\)\)\s*$/m);
  if (!t || !total || !no) {
    fail(`A. ${kind} 分支解析失败（SPEC_TARGETS/SHARD_TOTAL/SHARD_NO 锚点失配），拒绝猜测分片语义`);
    return null;
  }
  // 捕获组：no[1]=±、no[2]=偏移量
  return {
    targets: [t[1]],
    total: Number(total[1]),
    noFor: s => s + (no[1] === '+' ? 1 : -1) * Number(no[2]),
  };
}

let assignments = []; // { shard, targets: string[], total, no }
if (conds.length === 3) {
  const flow = parseDerivedBranch(branchBodies[0], 'flow');
  // smoke 分支不套 run_shard，是内联命令：--shard=$((SHARD - K))/N <dir>
  const smokeM = branchBodies[1].match(/--shard=\$\(\(SHARD (\+|-) (\d+)\)\)\/(\d+) (e2e\/\S+\/)/);
  const traversal = parseDerivedBranch(branchBodies[2], 'traversal');
  if (flow) {
    for (let s = 0; s <= conds[0].threshold; s++) {
      assignments.push({ shard: s, targets: flow.targets, total: flow.total, no: flow.noFor(s) });
    }
  }
  if (!smokeM) fail('A. smoke 分支解析失败（内联 --shard=$((SHARD - N))/5 e2e/smoke/ 锚点失配）');
  else {
    const off = Number(smokeM[2]),
      total = Number(smokeM[3]);
    for (let s = conds[0].threshold + 1; s <= conds[1].threshold; s++) {
      assignments.push({
        shard: s,
        targets: [smokeM[4]],
        total,
        no: s + (smokeM[1] === '+' ? off : -off),
      });
    }
  }
  if (traversal) {
    for (let s = conds[1].threshold + 1; s <= conds[2].threshold; s++) {
      assignments.push({
        shard: s,
        targets: traversal.targets,
        total: traversal.total,
        no: traversal.noFor(s),
      });
    }
  }
}

// extras：else 分支里的 case 登记行（与 workflow :2012 的 ::error 守卫同一事实来源）
const extrasRe =
  /^\s+(\d+)\)\s*SPEC_TARGETS="([^"]+)"\s*;\s*SHARD_NO=(\d+)\s*;\s*SHARD_TOTAL=(\d+)\s*;;/gm;
const extrasCases = new Map();
for (const m of bash.matchAll(extrasRe)) {
  extrasCases.set(Number(m[1]), {
    targets: m[2].trim().split(/\s+/),
    no: Number(m[3]),
    total: Number(m[4]),
  });
}
if (extrasCases.size === 0)
  fail(
    'A. extras case 登记行解析为 0 条（锚点 `N) SPEC_TARGETS="..."; SHARD_NO=..; SHARD_TOTAL=.. ;;` 失配）'
  );
for (const [s, c] of extrasCases) {
  if (s <= conds.at(-1)?.threshold) fail(`A. extras case ${s} 落在目录分支阈值范围内，会永不执行`);
  else assignments.push({ shard: s, targets: c.targets, total: c.total, no: c.no });
}

// ---------------------------------------------------------------- 2. 校验 A/B 结构层
const matrixSet = new Set(matrixShards);
const assignByShard = new Map(assignments.map(a => [a.shard, a]));
for (const s of matrixSet)
  if (!assignByShard.has(s))
    fail(
      `B. 矩阵 shard ${s} 在分片编排（分支阈值 + extras case）中无登记 → 该 job 拒绝空目标集判红，等同丢覆盖`
    );
for (const s of assignByShard.keys())
  if (!matrixSet.has(s))
    fail(
      `B. 分片编排登记了 shard ${s}，但 ci-e2e 矩阵 include 缺失 → 该切分永远不会被任何 job 执行（静默丢覆盖）`
    );

// 同一 (targets,total) 分组内的 hash 序号必须恰好 1..total
const groups = new Map(); // key -> {targets, total, nos: [{shard,no}]}
for (const a of assignByShard.values()) {
  const key = [...a.targets].sort().join(' ') + `|total=${a.total}`;
  if (!groups.has(key)) groups.set(key, { targets: a.targets, total: a.total, nos: [] });
  groups.get(key).nos.push({ shard: a.shard, no: a.no });
}
for (const [key, g] of groups) {
  const seen = new Set();
  for (const { shard, no } of g.nos) {
    if (no < 1 || no > g.total)
      fail(
        `B. 分组 ${key}：shard ${shard} 的 --shard=${no}/${g.total} 越界（Playwright 会直接报错或空转）`
      );
    if (seen.has(no))
      fail(
        `B. 分组 ${key}：--shard=${no}/${g.total} 被多个 job 重复分配（同一批用例重复执行/掩盖空位）`
      );
    seen.add(no);
  }
  for (let n = 1; n <= g.total; n++)
    if (!seen.has(n))
      fail(
        `B. 分组 ${key}：hash 分片序号 ${n}/${g.total} 无人认领 → 该组约 1/${g.total} 用例被分到不存在于矩阵的片上，静默丢失`
      );
}

// ---------------------------------------------------------------- 3. 校验 C：真实 --list 并集
function playwrightList(targets) {
  const npx = process.platform === 'win32' ? 'npx.cmd' : 'npx';
  const args = [
    'playwright',
    'test',
    '--project=chromium',
    '--list',
    '--reporter=json',
    ...targets,
  ];
  const r = spawnSync(npx, args, {
    cwd: FRONTEND_DIR,
    encoding: 'utf8',
    shell: process.platform === 'win32',
    maxBuffer: 512 * 1024 * 1024,
  });
  if (r.status !== 0) {
    fail(
      `C. \`npx playwright test --list ${targets.join(' ')}\` 退出码 ${r.status}（目标目录不存在/无匹配时会如此报错）：\n${(r.stderr || r.stdout || '').split('\n').slice(0, 8).join('\n')}`
    );
    return null;
  }
  const jsonStart = (r.stdout || '').indexOf('{');
  if (jsonStart === -1) {
    fail(`C. --list ${targets.join(' ')} 无 JSON 输出`);
    return null;
  }
  let doc;
  try {
    doc = JSON.parse(r.stdout.slice(jsonStart));
  } catch (e) {
    fail(`C. --list ${targets.join(' ')} JSON 解析失败: ${e.message}`);
    return null;
  }
  const ids = [];
  const walk = (suite, prefix) => {
    const here = suite.title ? [...prefix, suite.title] : prefix;
    for (const sp of suite.specs || []) {
      for (const t of sp.tests || []) {
        if (t.projectId !== 'chromium') continue;
        ids.push(`${sp.title} :: ${[...here, t.title].join(' › ')}`);
      }
    }
    for (const sub of suite.suites || []) walk(sub, here);
  };
  for (const root of doc.suites || []) walk(root, []);
  if (ids.length === 0)
    fail(
      `C. 分组 ${targets.join(' ')} 的 --list 结果为 0 条用例（目标写错/目录空/全被 testIgnore 排除，均属静默丢覆盖）`
    );
  return ids;
}

note(`workflow: ${WORKFLOW_PATH}`);
note(`矩阵 shard：${[...matrixSet].sort((x, y) => x - y).join(', ')}（共 ${matrixSet.size} 个）`);
note(`目录分组：${groups.size} 个（同 targets+total 计一组）`);

if (errors.length === 0) {
  const universe = playwrightList([]);
  const seenInGroups = new Set();
  let groupTotal = 0;
  for (const [key, g] of groups) {
    const ids = playwrightList(g.targets);
    if (!ids) continue;
    groupTotal += ids.length;
    for (const id of ids) seenInGroups.add(id);
    note(`  组 ${key} → ${ids.length} 条`);
  }
  if (universe) {
    const missing = universe.filter(id => !seenInGroups.has(id));
    const extra = [...seenInGroups].filter(id => !universe.includes(id));
    for (const id of missing.slice(0, 40)) annotate(`C. 全集内但无任何分片分配的用例：${id}`);
    if (missing.length > 40) annotate(`C. …另有 ${missing.length - 40} 条同类缺失用例`);
    for (const id of extra.slice(0, 10))
      annotate(`C. 分片分配到但不在 chromium 全集内的用例（testMatch/分组漂移）：${id}`);
    note(
      `全集 ${universe.length} 条 / 分组并集 ${seenInGroups.size} 条（分组列表合计 ${groupTotal} 条，重复分配已在上游判红）/ 差集 missing=${missing.length} extra=${extra.length}`
    );
    if (missing.length)
      fail(`C. ${missing.length} 条用例存在于 testMatch 全集但从未被任何分片分配（静默丢覆盖）`);
    if (extra.length) fail(`C. ${extra.length} 条被分片分配的用例不在全集内`);
  }
}

if (errors.length > 0) {
  console.error(`\nE2E 分片覆盖自检失败（${errors.length} 项）：`);
  for (const e of errors) console.error(' - ' + e);
  process.exit(1);
}
note('✅ E2E 分片覆盖自检通过：分配并集 == --list 全集，且每个分组的 hash 分片序号 1..N 无缺口');
