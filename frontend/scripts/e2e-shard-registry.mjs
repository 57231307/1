#!/usr/bin/env node
// E2E 分片登记 + ci-cd.yml 分片编排解析（单一事实来源模块，被两个 CI 门禁脚本共用）
//
// 为什么需要这个模块：
//   ci-cd.yml 的 ci-e2e 原来把"有哪些分片"写两遍——strategy.matrix.include 的 42 条
//   "- shard: N / label: ..."（给 Actions 展开 job 用）与跑测 step 里的 `case "$SHARD"`
//   表（给 bash 决定 SPEC_TARGETS 用），文件自己在 :3041 注释里写着"两处表达必须同步演进"。
//   现在 main push 轮也要跑 e2e（只跑核心链子集），矩阵必须由程序按事件发射，静态
//   include 块不再是矩阵来源；把它继续留在 YAML 里就变成第三处会漂移的副本，因此
//   分片清单（shard + label）迁到本模块，执行语义（目录/序号/分母）仍以 ci-cd.yml 的
//   `case` 表为唯一真相，两者的一致性由脚本每次 CI 硬比对。
//
// 本模块只读文件、不执行 Playwright：
//   - parseShardAssignments(): 从 ci-cd.yml 解析每个 shard 的 {targets, no, total}
//   - 消费方：check-e2e-shard-coverage.mjs（并集==全集门禁）、
//             check-e2e-main-push-shards.mjs（main push 选片锁 + 矩阵发射）
//
// 解析锚点全部 fail-closed：锚点失配即 fatal，绝不按"解析不出来=没有问题"处理
// （本仓库已有多次"静默丢覆盖"由锚点漂移引起，见 check-e2e-shard-coverage.mjs 头注释）。
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(SCRIPT_DIR, '..', '..');
export const FRONTEND_DIR = path.join(REPO_ROOT, 'frontend');
export const DEFAULT_WORKFLOW_PATH = path.join(
  REPO_ROOT,
  '.github',
  'workflows',
  'ci-cd.yml'
);

// main push 核心链分片的片数上下界（用户裁定：方案甲，只加 2 到 4 片）
export const MAIN_PUSH_SHARD_MIN = 2;
export const MAIN_PUSH_SHARD_MAX = 4;

// 分片登记（shard + 展示名）。label 逐字沿用原 ci-cd.yml 静态 include 块，
// 迁移不改变 PR 轮的 job 显示名，历史 run 的检查名检索口径不断。
export const E2E_SHARD_MATRIX = [
  { shard: 0, label: 'flow: e2e/flow(1/20)' },
  { shard: 1, label: 'flow: e2e/flow(2/20)' },
  { shard: 2, label: 'flow: e2e/flow(3/20)' },
  { shard: 3, label: 'flow: e2e/flow(4/20)' },
  { shard: 4, label: 'flow: e2e/flow(5/20)' },
  { shard: 5, label: 'flow: e2e/flow(6/20)' },
  { shard: 6, label: 'flow: e2e/flow(7/20)' },
  { shard: 7, label: 'flow: e2e/flow(8/20)' },
  { shard: 8, label: 'flow: e2e/flow(9/20)' },
  { shard: 9, label: 'flow: e2e/flow(10/20)' },
  { shard: 10, label: 'flow: e2e/flow(11/20)' },
  { shard: 11, label: 'flow: e2e/flow(12/20)' },
  { shard: 12, label: 'flow: e2e/flow(13/20)' },
  { shard: 13, label: 'flow: e2e/flow(14/20)' },
  { shard: 14, label: 'flow: e2e/flow(15/20)' },
  { shard: 15, label: 'flow: e2e/flow(16/20)' },
  { shard: 16, label: 'flow: e2e/flow(17/20)' },
  { shard: 17, label: 'flow: e2e/flow(18/20)' },
  { shard: 18, label: 'flow: e2e/flow(19/20)' },
  { shard: 19, label: 'flow: e2e/flow(20/20)' },
  { shard: 20, label: 'smoke: e2e/smoke(1/5)' },
  { shard: 21, label: 'smoke: e2e/smoke(2/5)' },
  { shard: 22, label: 'smoke: e2e/smoke(3/5)' },
  { shard: 23, label: 'smoke: e2e/smoke(4/5)' },
  { shard: 24, label: 'smoke: e2e/smoke(5/5)' },
  { shard: 25, label: 'traversal: e2e/traversal(1/5)' },
  { shard: 26, label: 'traversal: e2e/traversal(2/5)' },
  { shard: 27, label: 'traversal: e2e/traversal(3/5)' },
  { shard: 28, label: 'traversal: e2e/traversal(4/5)' },
  { shard: 29, label: 'traversal: e2e/traversal(5/5)' },
  { shard: 30, label: 'extras: purchase 1/2（--shard hash 分片）' },
  { shard: 31, label: 'extras: purchase 2/2（--shard hash 分片）' },
  { shard: 32, label: 'extras: finance 1/2（--shard hash 分片）' },
  { shard: 33, label: 'extras: finance 2/2（--shard hash 分片）' },
  { shard: 34, label: 'extras: sales+quality' },
  { shard: 35, label: 'extras: crm+inventory' },
  { shard: 36, label: 'extras: enhanced+production' },
  { shard: 37, label: 'extras: bpm+fabric+mrp' },
  { shard: 38, label: 'extras: ai+quotations+system' },
  { shard: 39, label: 'extras: dashboard+e2e 根级 3 spec' },
  { shard: 40, label: 'extras: fullflow 1/2（--shard hash 分片）' },
  { shard: 41, label: 'extras: fullflow 2/2（--shard hash 分片）' },
];

// 读取 workflow 文本并归一行尾：本地工作区（core.autocrlf）是 CRLF、CI 检出是 LF，
// 解析锚点对 \r 敏感，两侧必须看到同一份文本，否则本地复演的假红会淹没真实漂移。
export function readWorkflowText(workflowPath) {
  return readFileSync(workflowPath, 'utf8').replace(/\r\n/g, '\n');
}

// 定位任意顶层 job 文本块：从 "  <id>:" 到下一个顶层 job key（两空格缩进）
export function locateJobBlock(wf, jobId) {
  const start = wf.indexOf(`\n  ${jobId}:`);
  if (start === -1) return null;
  const rest = wf.slice(start + 1);
  const nextJob = rest.slice(1).match(/^ {2}[a-zA-Z0-9_-]+:/m);
  return nextJob ? rest.slice(0, rest.indexOf(nextJob[0], 1)) : rest;
}

// 定位 ci-e2e job 文本块：从 "  ci-e2e:" 到下一个顶层 job key
export function locateCiE2eBlock(wf) {
  return locateJobBlock(wf, 'ci-e2e');
}

/**
 * 从 ci-e2e 的跑测 step bash 体解析每个 shard 的执行语义。
 * 返回 { fatal, fatalReason, errors, assignments }：
 *   fatal=true 表示解析锚点失配（结构性漂移），消费方必须直接判红并停止后续校验；
 *   errors 为 A 层语义错误（锚点在但内容不合规），与消费方自身错误合并上报。
 */
export function parseShardAssignments(ciE2eBlock) {
  const errors = [];
  const fail = msg => errors.push(msg);

  // 跑测 step 的 bash 体定位：不依赖 step 显示名（显示名带矩阵后缀、随编排改写漂移），
  // 按 run 内容特征（含 SHARD=<矩阵分片> 赋值行）在 ci-e2e 块内逐个 step 段落查找。
  // fail-closed：命中数必须恰好为 1。0 = 分片编排里找不到执行事实；
  // >1 = 出现第二处事实来源，无法保证校验的就是真实执行的那份。
  const FEATURE = 'SHARD=${{ matrix.shard }}';
  const stepStarts = [...ciE2eBlock.matchAll(/^ {6}- name: ([^\n]*)$/gm)];
  const shardRunBodies = [];
  for (let i = 0; i < stepStarts.length; i++) {
    const seg = ciE2eBlock.slice(
      stepStarts[i].index,
      i + 1 < stepStarts.length ? stepStarts[i + 1].index : ciE2eBlock.length
    );
    const runM = seg.match(/^ {8}run: \|\n/m);
    if (!runM) continue;
    let body = seg.slice(runM.index + runM[0].length);
    // block scalar 内容缩进深于 step 键（8 空格）；首个 ≤8 空格缩进的非空行即键区结束
    const dedent = body.match(/^ {0,8}\S/m);
    if (dedent) body = body.slice(0, dedent.index);
    if (body.includes(FEATURE)) {
      if (body.trim().length === 0) {
        return {
          fatal: true,
          fatalReason: `step 名 ${stepStarts[i][1]} 的 run 体命中分片特征但内容为空，判为解析异常`,
          errors,
          assignments: [],
        };
      }
      shardRunBodies.push({ name: stepStarts[i][1], body });
    }
  }
  if (shardRunBodies.length !== 1) {
    const reason =
      shardRunBodies.length === 0
        ? `按内容特征（${FEATURE}）未定位到任何跑测 step 的 bash 体：分片编排解析锚点漂移，拒绝把"看不见"当作无覆盖问题`
        : `分片调度特征命中 ${shardRunBodies.length} 个 run 体（${shardRunBodies
            .map(b => b.name)
            .join(' / ')}）：分片编排出现多处事实来源，必须先收敛为一处再自检，拒绝择一核算`;
    return { fatal: true, fatalReason: reason, errors, assignments: [] };
  }
  const bash = shardRunBodies[0].body.replace(/\n+$/, '');

  // 三类目录分支：if/elif 的 "$SHARD" -le 阈值按出现顺序 = flow / smoke / traversal
  const conds = [...bash.matchAll(/\[ "\$SHARD" -le (\d+) \]; then/g)].map(m => ({
    threshold: Number(m[1]),
    at: m.index,
  }));
  if (conds.length !== 3) {
    fail(
      `A. 分片分支结构漂移：期望 3 个 "[ "$SHARD" -le N ]" 条件（flow/smoke/traversal），实得 ${conds.length} 个`
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
    return {
      targets: [t[1]],
      total: Number(total[1]),
      noFor: s => s + (no[1] === '+' ? 1 : -1) * Number(no[2]),
    };
  }

  /** @type {{shard:number,targets:string[],no:number,total:number}[]} */
  const assignments = [];
  if (conds.length === 3) {
    const flow = parseDerivedBranch(branchBodies[0], 'flow');
    // smoke 分支不套 run_shard，是内联命令：--shard=$((SHARD - K))/N <dir>
    const smokeM = branchBodies[1].match(
      /--shard=\$\(\(SHARD (\+|-) (\d+)\)\)\/(\d+) (e2e\/\S+\/)/
    );
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

  // extras：else 分支里的 case 登记行（与 workflow 内 `::error::extras 分片 ... 未登记` 守卫同一事实来源）
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
  if (extrasCases.size === 0) {
    fail(
      'A. extras case 登记行解析为 0 条（锚点 `N) SPEC_TARGETS="..."; SHARD_NO=..; SHARD_TOTAL=.. ;;` 失配）'
    );
  }
  for (const [s, c] of extrasCases) {
    if (conds.length === 3 && s <= conds[conds.length - 1].threshold) {
      fail(`A. extras case ${s} 落在目录分支阈值范围内，会永不执行`);
    } else {
      assignments.push({ shard: s, targets: c.targets, total: c.total, no: c.no });
    }
  }

  return { fatal: false, fatalReason: '', errors, assignments };
}

/**
 * 按目录分组（同 targets + 同 total 为一组）：用于校验 hash 分片序号 1..N 无缺口，
 * 以及"某目录的整组是否只有一片"（main push 只允许选这种片，详见
 * check-e2e-main-push-shards.mjs 的选片判据）。
 */
export function groupAssignments(assignments) {
  const groups = new Map();
  for (const a of assignments) {
    const key = [...a.targets].sort().join(' ') + `|total=${a.total}`;
    if (!groups.has(key)) groups.set(key, { targets: a.targets, total: a.total, members: [] });
    groups.get(key).members.push({ shard: a.shard, no: a.no });
  }
  return groups;
}
