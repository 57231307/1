#!/usr/bin/env node
// E2E 分片覆盖自检：由 CI job `ci-build-fe` 的「E2E 分片覆盖自检」步骤执行
// （.github/workflows/ci-cd.yml），入参仅可选 --workflow/--min-shard-list 覆盖默认路径。
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
//      分片登记集合（e2e-shard-registry.mjs 的 E2E_SHARD_MATRIX）与 workflow 分片编排
//      的 case 登记集合必须双向相等（登记了但编排没跑 = 不执行；编排登记了但矩阵
//      没有 = workflow 自身的 ::error 守卫，这里同样判红）。
//   C. 全集并集校验：对每个分组用真实 `playwright test --list`（只收集、不执行任何用例）
//      取"该片会跑什么"，各组并集与 chromium 项目全集做差集；missing 非空即丢覆盖，
//      extra（分组里写了不存在的目标/空匹配）同样判红。
//
// 用法（在任意目录均可，路径以脚本自身位置解析）：
//   node scripts/check-e2e-shard-coverage.mjs [--workflow <path>] [--min-shard-list <n>]
// 附带产出：三层校验全过时的分组期望数落盘 frontend/reports/e2e-shard-group-counts.json，
//   随 frontend-build-log 产物上传，供 e2e-authenticity-guard 的判据一做"Σ各片 executed==
//   分组 --list 期望数"对账；校验失败不落盘，旧文件不冒充本轮事实。
// 退出码：0=全覆盖通过；1=任一层校验失败。
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import {
  DEFAULT_WORKFLOW_PATH,
  E2E_SHARD_MATRIX,
  FRONTEND_DIR,
  groupAssignments,
  locateCiE2eBlock,
  parseShardAssignments,
  readWorkflowText,
} from './e2e-shard-registry.mjs';

function arg(name, dflt) {
  const i = process.argv.indexOf(`--${name}`);
  if (i !== -1 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--'))
    return process.argv[i + 1];
  return dflt;
}
const WORKFLOW_PATH = path.resolve(arg('workflow', DEFAULT_WORKFLOW_PATH));

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
// 行尾归一与 job 块定位、分片编排解析全部走 e2e-shard-registry.mjs：本脚本与
// check-e2e-main-push-shards.mjs（main push 选片锁 + 矩阵发射）共用同一份解析器，
// 避免"两份副本对同一份 workflow 得出不同结论"（本仓库已发生过内嵌脚本副本过期）。
const wf = readWorkflowText(WORKFLOW_PATH);
const ciE2eBlock = locateCiE2eBlock(wf);
if (!ciE2eBlock) {
  annotate('无法在 workflow 中定位 ci-e2e job（解析锚点 "  ci-e2e:" 失配）');
  process.exit(1);
}

// 矩阵分片集合的当前事实来源：e2e-shard-registry.mjs 的 E2E_SHARD_MATRIX。
// ci-e2e 的 strategy.matrix.include 已改为按事件 fromJSON 发射（见 ci-cd.yml），
// 静态 include 块不再是可比对的对象；登记集合与 workflow 分片编排的双向相等
// 校验语义原样保留（改一边忘了改另一边仍然在这里判红）。
const matrixShards = E2E_SHARD_MATRIX.map(m => m.shard);
if (matrixShards.length === 0) {
  annotate('E2E_SHARD_MATRIX 为空：矩阵没有任何事实来源，自检失去对象');
  process.exit(1);
}

const parsed = parseShardAssignments(ciE2eBlock);
if (parsed.fatal) {
  annotate('分片编排解析失败：' + parsed.fatalReason);
  process.exit(1);
}
for (const e of parsed.errors) fail(e);
const assignments = parsed.assignments;

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
      `B. 分片编排登记了 shard ${s}，但 E2E_SHARD_MATRIX 登记缺失，该切分永远不会被任何 job 执行（静默丢覆盖）`
    );

// 同一 (targets,total) 分组内的 hash 序号必须恰好 1..total
const groups = groupAssignments(assignments); // key -> {targets, total, members: [{shard,no}]}
for (const [key, g] of groups) {
  const seen = new Set();
  for (const { shard, no } of g.members) {
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
  const groupCounts = {};
  for (const [key, g] of groups) {
    const ids = playwrightList(g.targets);
    if (!ids) continue;
    groupTotal += ids.length;
    for (const id of ids) seenInGroups.add(id);
    groupCounts[key] = { targets: g.targets, total: g.total, expected: ids.length };
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
  // 三层全过才落盘：判据一（check-e2e-authenticity.mjs）拿本文件做"Σ各片 executed ==
  // 分组 --list 期望数"的对账基准；errors 非空时不落盘，避免旧计数冒充本轮事实。
  if (errors.length === 0 && universe) {
    const countsDir = path.join(FRONTEND_DIR, 'reports');
    mkdirSync(countsDir, { recursive: true });
    writeFileSync(
      path.join(countsDir, 'e2e-shard-group-counts.json'),
      JSON.stringify({
        schema: 'e2e-shard-group-counts/1',
        universeTotal: universe.length,
        groups: groupCounts,
      })
    );
    note(
      `分组期望数已落盘 frontend/reports/e2e-shard-group-counts.json（${Object.keys(groupCounts).length} 组）`
    );
  }
}

if (errors.length > 0) {
  console.error(`\nE2E 分片覆盖自检失败（${errors.length} 项）：`);
  for (const e of errors) console.error(' - ' + e);
  process.exit(1);
}
note('✅ E2E 分片覆盖自检通过：分配并集 == --list 全集，且每个分组的 hash 分片序号 1..N 无缺口');
