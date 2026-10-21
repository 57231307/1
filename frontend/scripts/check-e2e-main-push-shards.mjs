#!/usr/bin/env node
/**
 * main push 轮 E2E 核心链分片选择锁 + ci-e2e 矩阵同源发射。
 *
 * 门禁用途（对应缺陷：ci-e2e 的 job 级 if 只认 pull_request，main push 轮全部
 * "🎭 E2E" job 结论为 skipped，而收尾门把 skipped 计为通过，于是主线轮次零 E2E 覆盖
 * 却全绿；实测证据 main run 4915 与 4916）：
 *   本脚本是"main push 跑哪几片"的唯一决定者，并把决定做成硬断言。它同时被
 *   ci-build-fe 调用两件事：判红（任一判据不成立就 exit 1）与发射（把本轮矩阵
 *   写成 JSON 交给 ci-e2e 的 strategy.matrix 消费）。
 *
 * 为什么选片必须落在"整组只有一片"的分片上（S 层，最关键的假绿防线）：
 *   ci-cd.yml 的分片语义是 Playwright 的 --shard=no/total 按用例 hash 分配，同一
 *   目录分组拆成 N 片时，只跑其中 1 片等于只跑约 1/N 的用例，而那片 job 的结论仍是
 *   success（它确实"跑完了自己的 1/N"）。所以凡 total 大于 1 的分组，其成员片一律
 *   不得单独选入 main push：要么整组全选（会突破片数上界），要么不选。
 *   本仓库三条重灾区链路的实际归属：
 *     报价审批  frontend/e2e/quotations/02-approval.spec.ts 落在分片 38，total 1
 *     验布四分制 frontend/e2e/quality/03-four-point.spec.ts 落在分片 34，total 1
 *     验布码单与染色 frontend/e2e/fabric/05-piece-label-print.spec.ts 落在分片 37，total 1
 *   报废审批链只存在于 frontend/e2e/flow/44e-state-machines-extended.spec.ts，
 *   属 flow 分组（total 20），按上面的判据不可单片选入，因此不在本清单内，
 *   该链在 main push 轮仍无覆盖，需另立改造工单（把该 spec 登记成独立 total 1 分组）。
 *
 * 校验层次（任一层失败即 exit 1，绝不"取不到就跳过"）：
 *   W. 接线锁：ci-e2e 的 job 级 if 必须同时放行 pull_request 与 main/master push，
 *      needs 必须含 ci-build-fe 与 ci-build-rust，strategy.matrix 必须是消费本脚本
 *      输出的 fromJSON 表达式（载荷为 {include:[{shard,label},...]}，与 GitHub 文档
 *      的 dynamic matrix 写法一致），且块内不得再残留静态 "- shard: N" 条目
 *      （残留即两处事实来源，且意味着 main push 仍会被静态矩阵支配）。
 *   R. 同源锁：E2E_SHARD_MATRIX 的 shard 集合与 ci-cd.yml 分片编排解析出的集合
 *      双向相等，label 唯一且非空。
 *   S. 选片锁：显式目录清单（MAIN_PUSH_CORE_TARGETS）逐条真实存在且含 spec 文件；
 *      每个目录在分片编排里恰好归属一个分片；所属分片所在分组必须 total 1 且仅一片；
 *      命中片数落在 MAIN_PUSH_SHARD_MIN 到 MAIN_PUSH_SHARD_MAX 之间。
 *      S 层在 pull_request 轮同样执行：清单腐坏要第一时间在 PR 上红，而不是等
 *      main push 轮才发现选不出片。
 *   E. 发射锁：本轮矩阵非空（空矩阵会让 job 被 GitHub 直接判 skipped，正是本门禁
 *      要消灭的形态），写入 GITHUB_OUTPUT 与可下载的 reports 产物。
 *
 * 用法（任意目录均可，workflow 路径以脚本自身位置解析）：
 *   node scripts/check-e2e-main-push-shards.mjs --event <github.event_name>
 *     --ref <github.ref> [--matrix-file <path>] [--github-output <path>]
 *     [--workflow <path>]
 * 退出码：0=全部判据成立并已发射；1=任一层失败（不发射）。
 */
import { existsSync, appendFileSync, mkdirSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import {
  DEFAULT_WORKFLOW_PATH,
  E2E_SHARD_MATRIX,
  MAIN_PUSH_SHARD_MAX,
  MAIN_PUSH_SHARD_MIN,
  FRONTEND_DIR,
  locateCiE2eBlock,
  locateJobBlock,
  groupAssignments,
  parseShardAssignments,
  readWorkflowText,
} from './e2e-shard-registry.mjs';

// ------------------------------------------------------------------ 选片策略（唯一清单）
// 显式目录清单：main push 轮要真实执行的核心链路所在目录。归属分片由本脚本从
// ci-cd.yml 的分片编排反查，不在这里写死分片号——分片号会随编排重排漂移，写死就会
// 出现"清单说跑 quotations、矩阵跑的是别的目录"这类静默错位。
const MAIN_PUSH_CORE_TARGETS = [
  // 报价审批链：frontend/e2e/quotations/02-approval.spec.ts（21 处报价审批断言）
  'e2e/quotations/',
  // 验布四分制链：frontend/e2e/quality/03-four-point.spec.ts 四分制评分数值断言
  'e2e/quality/',
  // 验布码单与染色链：frontend/e2e/fabric/05-piece-label-print.spec.ts（14 处验布）
  // 与 frontend/e2e/fabric/02-dye.spec.ts
  'e2e/fabric/',
];

// main/master 分支 ref（与 ci-cd.yml 的 on.push.branches 同源，改一处必须两处同步；
// W 层会校验 job 级 if 里确实出现了这两个 ref 字面量）
const MAIN_PUSH_REFS = ['refs/heads/main', 'refs/heads/master'];

const errors = [];
function fail(msg) {
  errors.push(msg);
}
function annotate(msg) {
  console.log(`::error::${msg}`);
}
function note(msg) {
  console.log(msg);
}

function arg(name, dflt) {
  const i = process.argv.indexOf(`--${name}`);
  if (i !== -1 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--')) return process.argv[i + 1];
  return dflt;
}

const EVENT = arg('event', process.env.GATE_EVENT_NAME || '');
const REF = arg('ref', process.env.GATE_REF || '');
const WORKFLOW_PATH = path.resolve(arg('workflow', DEFAULT_WORKFLOW_PATH));
const MATRIX_FILE = arg('matrix-file', '');
const GITHUB_OUTPUT_FILE = arg('github-output', process.env.GITHUB_OUTPUT || '');

if (!EVENT) {
  annotate('缺少 --event（应传 github.event_name），模式判定不允许猜测');
  process.exit(1);
}
if (!REF) {
  annotate('缺少 --ref（应传 github.ref），模式判定不允许猜测');
  process.exit(1);
}
if (!MATRIX_FILE) {
  annotate('缺少 --matrix-file：本轮矩阵没有可下载取证产物，拒绝"跑了但留不下证据"');
  process.exit(1);
}
if (!GITHUB_OUTPUT_FILE) {
  annotate('缺少 --github-output 且环境无 GITHUB_OUTPUT：矩阵没有消费者，拒绝静默发射');
  process.exit(1);
}

// ------------------------------------------------------------------ 模式判定（单一入口）
let mode;
if (EVENT === 'pull_request') mode = 'full';
else if (EVENT === 'push' && MAIN_PUSH_REFS.includes(REF)) mode = 'main-push-core';
else mode = 'full'; // 其余事件（tag push / workflow_dispatch）由 ci-e2e 的 job 级 if 排除，这里仍发射全量以保证矩阵非空

// ------------------------------------------------------------------ W 层：接线锁
const wf = readWorkflowText(WORKFLOW_PATH);
const block = locateCiE2eBlock(wf);
if (!block) {
  annotate('无法在 workflow 中定位 ci-e2e job（解析锚点两空格缩进的 ci-e2e 失配）');
  process.exit(1);
}

const jobIfs = [...block.matchAll(/^ {4}if: (.+)$/gm)];
if (jobIfs.length !== 1) {
  fail(`W. ci-e2e 的 job 级 if 应恰好 1 条，实得 ${jobIfs.length} 条（锚点 ^ 四空格 if: 失配）`);
} else {
  const cond = jobIfs[0][1];
  if (!cond.includes("github.event_name == 'pull_request'")) {
    fail(`W. ci-e2e 的 job 级 if 未放行 pull_request，PR 轮 42 片全量会被摘掉：${cond}`);
  }
  if (!MAIN_PUSH_REFS.some(r => cond.includes(`'${r}'`))) {
    fail(`W. ci-e2e 的 job 级 if 未放行 main/master push（本门禁的目的本身），main 轮仍会整族 skipped：${cond}`);
  }
}
const jobNeeds = block.match(/^ {4}needs: \[([^\]]*)\]/m);
if (!jobNeeds) fail('W. ci-e2e 的 needs 行解析失败（锚点 ^ 四空格 needs: 失配）');
else {
  for (const dep of ['ci-build-fe', 'ci-build-rust']) {
    if (!jobNeeds[1].split(',').map(s => s.trim()).includes(dep)) {
      fail(`W. ci-e2e 的 needs 缺 ${dep}：矩阵来源与 e2e 运行期产物来源必须同需，否则矩阵为空或产物缺失`);
    }
  }
}
const MATRIX_EXPR = '${{ fromJSON(needs.ci-build-fe.outputs.e2e_shard_matrix) }}';
const matrixLines = [...block.matchAll(/^ {6}matrix: (.+)$/gm)];
if (matrixLines.length !== 1) {
  fail(`W. ci-e2e 的 strategy.matrix 应恰好 1 条（六空格缩进），实得 ${matrixLines.length} 条`);
} else if (matrixLines[0][1].trim() !== MATRIX_EXPR) {
  fail(`W. ci-e2e 的矩阵不是本脚本发射的矩阵（应为 ${MATRIX_EXPR}，实为 ${matrixLines[0][1].trim()}）：静态矩阵与动态选片不可能同时成立，拒绝二选一核算`);
}
if (/^\s+- shard: \d+$/m.test(block)) {
  fail('W. ci-e2e 块内仍残留静态 "- shard: N" 条目：分片清单出现两处事实来源，必须迁到 e2e-shard-registry.mjs');
}

// 生产端接线：ci-build-fe 的 outputs 键、发射步骤的 id、以及调用本脚本的那行命令，
// 三者必须互相指认。任一处改名而另一处未同步时，needs.ci-build-fe.outputs 会是空串，
// fromJSON('') 让 ci-e2e 一个 job 都不生成（结论 skipped）——正是本门禁要消灭的形态，
// 所以这里静态判红，而不是等 e2e-authenticity-guard 事后追认。
const feBlock = locateJobBlock(wf, 'ci-build-fe');
if (!feBlock) {
  fail('W. 无法在 workflow 中定位 ci-build-fe job（矩阵的生产端），拒绝继续校验接线');
} else {
  const outM = feBlock.match(
    /e2e_shard_matrix: \$\{\{ steps\.([a-zA-Z0-9_-]+)\.outputs\.matrix \}\}/
  );
  if (!outM) {
    fail('W. ci-build-fe 的 outputs 里没有 e2e_shard_matrix: steps.<id>.outputs.matrix 映射');
  } else {
    const stepId = outM[1];
    if (!new RegExp(`^ {8}id: ${stepId}$`, 'm').test(feBlock)) {
      fail(`W. ci-build-fe 的 outputs 指向 steps.${stepId}，但块内没有 id 为 ${stepId} 的步骤（改 id 忘改 outputs，矩阵恒为空）`);
    }
    if (!feBlock.includes('node scripts/check-e2e-main-push-shards.mjs')) {
      fail('W. ci-build-fe 块内没有调用 check-e2e-main-push-shards.mjs：矩阵没有生产者');
    }
    if (!/--github-output "\$GITHUB_OUTPUT"/.test(feBlock)) {
      fail('W. ci-build-fe 调用本脚本时未传 --github-output "$GITHUB_OUTPUT"：矩阵只会打印、不会成为 job 输出');
    }
  }
}

// ------------------------------------------------------------------ R 层：同源锁
const parsed = parseShardAssignments(block);
if (parsed.fatal) {
  annotate(`W/R 层解析失败：${parsed.fatalReason}`);
  process.exit(1);
}
for (const e of parsed.errors) fail(e);
const assignments = parsed.assignments;
const assignIds = new Set(assignments.map(a => a.shard));
const registryIds = new Set(E2E_SHARD_MATRIX.map(m => m.shard));
for (const s of registryIds)
  if (!assignIds.has(s)) fail(`R. 登记 E2E_SHARD_MATRIX 有分片 ${s}，但 ci-cd.yml 分片编排无登记，该 job 会拒绝空目标集判红，等同丢覆盖`);
for (const s of assignIds)
  if (!registryIds.has(s)) fail(`R. ci-cd.yml 分片编排登记了 shard ${s}，但 E2E_SHARD_MATRIX 缺失，该切分永远不会被任何 job 执行（静默丢覆盖）`);
if (new Set(E2E_SHARD_MATRIX.map(m => m.label)).size !== E2E_SHARD_MATRIX.length)
  fail('R. E2E_SHARD_MATRIX 的 label 有重复：job 显示名是取证检索键，重名会让分片归因失效');
for (const m of E2E_SHARD_MATRIX) if (!m.label || !m.label.trim()) fail(`R. shard ${m.shard} 的 label 为空`);

const byShard = new Map(assignments.map(a => [a.shard, a]));
const groups = groupAssignments(assignments);

// ------------------------------------------------------------------ S 层：选片锁（两种模式都执行）
function countSpecs(target) {
  const abs = path.join(FRONTEND_DIR, target);
  if (!existsSync(abs)) return null;
  if (statSync(abs).isFile()) return target.endsWith('.spec.ts') ? 1 : 0;
  let n = 0;
  const walk = d => {
    for (const f of readdirSync(d)) {
      const p = path.join(d, f);
      if (statSync(p).isDirectory()) walk(p);
      else if (f.endsWith('.spec.ts')) n++;
    }
  };
  walk(abs);
  return n;
}

// 严格计数：目标不存在或目录内没有 spec 一律判红并返回 0（调用方仍会把 errors 收口成
// exit 1）。返回 0 而不是 null，是为了让"合计"求和不被兜底掩盖成看似正常的数字。
function countSpecsStrict(target, ctx) {
  const c = countSpecs(target);
  if (c === null) {
    fail(`S. ${ctx} frontend/${target} 在本仓不存在：清单与代码不同源，拒绝发射`);
    return 0;
  }
  if (c === 0) {
    fail(`S. ${ctx} frontend/${target} 存在但没有任何 .spec.ts 文件，选中它等于零执行却报 success`);
    return 0;
  }
  return c;
}

if (new Set(MAIN_PUSH_CORE_TARGETS).size !== MAIN_PUSH_CORE_TARGETS.length)
  fail('S. MAIN_PUSH_CORE_TARGETS 有重复目录条目');

const selected = new Map(); // 键为 shard，值为 {targets, groupKey, specsPolicyDir, specsShardTotal}
for (const target of MAIN_PUSH_CORE_TARGETS) {
  const specs = countSpecsStrict(target, '选片清单');
  const owners = assignments.filter(a => a.targets.includes(target));
  if (owners.length === 0) {
    fail(`S. 目录 ${target} 未被 ci-cd.yml 分片编排的任何分片登记：清单与 case 表同源断裂，main push 会以为在跑这条链`);
    continue;
  }
  if (owners.length > 1) {
    fail(`S. 目录 ${target} 同时归属多个分片（${owners.map(o => o.shard).join(', ')}）：选片语义不确定，必须先在编排里收敛成单一归属`);
    continue;
  }
  const owner = owners[0];
  const key = [...owner.targets].sort().join(' ') + `|total=${owner.total}`;
  const g = groups.get(key);
  if (!g || g.members.length !== 1 || g.total !== 1) {
    fail(
      `S. 目录 ${target} 所属分片 ${owner.shard} 的分组 ${key} 不是"整组仅一片且 total 1"（成员 ${g ? g.members.map(m => m.shard).join(',') : '未解析'}）：` +
        `--shard 按用例 hash 分配，单选该片只跑约 1/${g ? g.total : '?'} 用例而 job 仍报 success，属假绿形态，禁止选入 main push`
    );
    continue;
  }
  // 该分片注册的全部目标都要真实存在并含 spec：case 表按目录分组登记，选一个目录会把
  // 同组的兄弟目录一起带进同一个 job（选 e2e/quality/ 落到的分片 34 同组还有
  // e2e/sales/）。只校验清单里那一条，等于放行"兄弟目录已改名/删除，本片实际只跑半条"。
  let shardSpecs = 0;
  for (const t of owner.targets) shardSpecs += countSpecsStrict(t, `分片 ${owner.shard} 的注册目标`);
  selected.set(owner.shard, {
    targets: owner.targets,
    groupKey: key,
    specsPolicyDir: specs,
    specsShardTotal: shardSpecs,
  });
}
if (selected.size < MAIN_PUSH_SHARD_MIN || selected.size > MAIN_PUSH_SHARD_MAX) {
  fail(
    `S. 选片清单命中 ${selected.size} 片，超出允许区间 ${MAIN_PUSH_SHARD_MIN}..${MAIN_PUSH_SHARD_MAX}（用户裁定的方案甲上下界）：` +
      `片数过少说明清单或编排已漂移，过多说明 main push 轮在偷跑全量`
  );
}

if (errors.length > 0) {
  console.error(`\nmain push 核心链分片选择锁失败（${errors.length} 项）：`);
  for (const e of errors) console.error(' - ' + e);
  process.exit(1);
}

// ------------------------------------------------------------------ E 层：发射
const shards = E2E_SHARD_MATRIX.map(m => ({
  ...m,
  targets: byShard.get(m.shard).targets,
  no: byShard.get(m.shard).no,
  total: byShard.get(m.shard).total,
}));
const emitted =
  mode === 'main-push-core'
    ? shards.filter(s => selected.has(s.shard)).sort((a, b) => a.shard - b.shard)
    : shards.sort((a, b) => a.shard - b.shard);

if (emitted.length === 0) {
  annotate('E. 本轮矩阵为空：空矩阵会让 ci-e2e 的 job 被 GitHub 直接判 skipped，正是本门禁要消灭的形态，拒绝发射');
  process.exit(1);
}
for (const s of emitted) {
  if (!s.label || !s.label.trim()) {
    annotate(`E. 发射的分片 ${s.shard} 缺 label，job 显示名会变成 "🎭 E2E: " 空尾，取证无法归因`);
    process.exit(1);
  }
}

const matrixJson = JSON.stringify({ include: emitted.map(s => ({ shard: s.shard, label: s.label })) });
if (!matrixJson.includes('"shard"')) {
  annotate('E. 发射的矩阵 JSON 不含 shard 键，拒绝写入');
  process.exit(1);
}

note(`workflow: ${WORKFLOW_PATH}`);
note(`事件=${EVENT} ref=${REF} 模式=${mode}`);
note(`分片登记 ${E2E_SHARD_MATRIX.length} 片 / 分片编排 ${assignments.length} 片 / 目录分组 ${groups.size} 个（R 层双向相等已通过）`);
note(`main push 选片清单（显式目录，共 ${MAIN_PUSH_CORE_TARGETS.length} 条）：${MAIN_PUSH_CORE_TARGETS.join(' ')}`);
for (const [shard, info] of [...selected.entries()].sort((a, b) => a[0] - b[0])) {
  note(
    `  分片 ${shard} ← ${info.groupKey}：清单目录 ${info.specsPolicyDir} 个 spec，该片注册目录合计 ${info.specsShardTotal} 个 spec（同组兄弟目录会一并执行）`
  );
}
note(`本轮发射矩阵 ${emitted.length} 片：${emitted.map(s => `${s.shard}=${s.label}`).join(' | ')}`);
if (mode === 'main-push-core') {
  console.log(
    `::notice::main push 核心链 E2E 分片矩阵=${emitted.length}片(${emitted.map(s => s.shard).join(', ')}) 来源=check-e2e-main-push-shards.mjs 模式=${mode}`
  );
} else {
  console.log(`::notice::E2E 分片矩阵=${emitted.length}片 全量发射 事件=${EVENT} 模式=${mode}`);
}

if (MATRIX_FILE) {
  const out = path.resolve(MATRIX_FILE);
  mkdirSync(path.dirname(out), { recursive: true });
  writeFileSync(
    out,
    JSON.stringify(
      {
        generated_by: 'frontend/scripts/check-e2e-main-push-shards.mjs',
        event: EVENT,
        ref: REF,
        mode,
        workflow: path.relative(process.cwd(), WORKFLOW_PATH).split(path.sep).join('/'),
        registry_shards: E2E_SHARD_MATRIX.length,
        case_table_shards: assignments.length,
        main_push_core_targets: MAIN_PUSH_CORE_TARGETS,
        selected_shards: [...selected.keys()].sort((a, b) => a - b),
        // 选片的可核对底账：每条清单目录落到哪个分片、该片注册目录的 spec 文件数。
        // main push 轮的 e2e-report-shard-* 产物里的用例数应与此同源（spec 文件数
        // 不等于用例数，用例数还要看 playwright 收集结果，这里只锚定目录与文件）。
        selection_detail: [...selected.entries()]
          .sort((a, b) => a[0] - b[0])
          .map(([shard, info]) => ({
            shard,
            group: info.groupKey,
            targets: info.targets,
            specs_policy_dir: info.specsPolicyDir,
            specs_shard_total: info.specsShardTotal,
          })),
        // 本轮明确不覆盖的链路：不是遗漏，是判据不允许（详见文件头 S 层说明）
        known_uncovered_chains: [
          {
            chain: '报废审批两级审批链',
            location: 'frontend/e2e/flow/44e-state-machines-extended.spec.ts',
            reason:
              '属 flow 分组（--shard=?.20 按 hash 分配），单片选入只跑约 1/20 用例而 job 仍报 success，违反 S 层判据',
          },
        ],
        emitted: emitted.map(s => ({ shard: s.shard, label: s.label, targets: s.targets, no: s.no, total: s.total })),
      },
      null,
      2
    ) + '\n',
    'utf8'
  );
  note(`矩阵记录产物已写入 ${out}`);
}

if (GITHUB_OUTPUT_FILE) {
  appendFileSync(GITHUB_OUTPUT_FILE, `matrix=${matrixJson}\n`, 'utf8');
  note(`已写入 GITHUB_OUTPUT（matrix，${matrixJson.length} 字节）`);
} else {
  // 入口已判红，这里只是把"不可能到达"写死，不给未来改参数校验的人留静默口子
  annotate('E. 到达发射点却没有 --github-output，拒绝静默发射');
  process.exit(1);
}
try {
  const back = JSON.parse(matrixJson);
  if (!Array.isArray(back.include) || back.include.length !== emitted.length) throw new Error('round-trip 长度不一致');
} catch (e) {
  annotate(`E. 发射的矩阵 JSON 自校验失败：${e.message}`);
  process.exit(1);
}
note('main push 核心链分片选择锁通过：清单与 ci-cd.yml 分片编排同源、目录真实存在、矩阵非空');
