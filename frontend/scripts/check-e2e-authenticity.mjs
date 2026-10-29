#!/usr/bin/env node
// check-e2e-authenticity.mjs — E2E 真实性门禁的产物侧判据（判据一/二/三/四）。
// 功能：从各分片上报产物与跑测日志里取证，把"看起来绿"与"真实跑绿"区分开：
//   判据一 空片：每片 executed>0；分组内全部片都在本轮时，Σexecuted 必须等于
//          check-e2e-shard-coverage.mjs 落盘的本组 --list 期望数（对不上即静默丢量）。
//   判据二 零断言：passed 用例的每一枚尝试都无 expect 步骤 → 点名 file:line 判红；
//          豁免只走基线清单（zeroAssertionExempt），不许在代码里写死放行。
//   判据三 skip/only 泄漏与 flaky：源码里 test.only(/test.describe.only( 超过基线即红；
//          test.skip( 计数超过基线即红（只降不升棘轮）；重试后才过的用例数超过阈值
//          发 ::warning::（flaky 掩盖面，暂不判红，阈值随基线文件收紧）。
//   判据四 真请求：每片跑测日志（DEBUG=pw:api）里 `← 2xx` 响应行计数必须达到下限，
//          后端没起来却被 catch 吞成通过的片在这里现形（与"报告 0 条用例仍绿"同族）。
// 调用方：.github/workflows/ci-cd.yml job e2e-authenticity-guard 的判据步骤与自证步骤。
// 入参：--artifacts <下载产物根目录>（子目录名 e2e-report-shard-<N>-<runid>）、
//       --counts <e2e-shard-group-counts.json>、--repo <仓库根>、--baseline <基线 json>；
//       --self-test 不读外部输入，用合成记录做双向探针（好样本必绿、每类坏样本必红）。
// 传给谁：结果只进 stdout 与 ::error::/::warning:: annotation，不外发。
// 存什么：不持久化任何状态；同入参必同出参。
// 存哪里：本文件；数据源为 e2e-assert-reporter.mjs 的 reports/e2e-assert-stats.json、
//       playwright-output*.txt、check-e2e-shard-coverage.mjs 的分组计数与
//       e2e-authenticity-baseline.json 基线。
import fs from 'node:fs';
import path from 'node:path';
import {
  DEFAULT_WORKFLOW_PATH,
  groupAssignments,
  locateCiE2eBlock,
  parseShardAssignments,
  readWorkflowText,
} from './e2e-shard-registry.mjs';

const SCHEMA = 'e2e-assert-stats/1';
const HTTP2XX = /← 2[0-9][0-9]\b/g;

function arg(name, dflt) {
  const i = process.argv.indexOf(`--${name}`);
  if (i !== -1 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--'))
    return process.argv[i + 1];
  return dflt;
}

function fatal(msg) {
  console.error(`::error::E2E 真实性门禁（产物判据）：${msg}`);
  process.exit(1);
}

// ---------------------------------------------------------------- 装载（I/O 层）

function loadShardRecords(artifactsDir) {
  const records = [];
  if (!fs.existsSync(artifactsDir)) fatal(`产物目录不存在：${artifactsDir}`);
  for (const d of fs.readdirSync(artifactsDir)) {
    const m = /^e2e-report-shard-(\d+)-/.exec(d);
    if (!m) continue;
    const dir = path.join(artifactsDir, d);
    const shard = Number(m[1]);
    const statsFile = path.join(dir, 'e2e-assert-stats.json');
    if (!fs.existsSync(statsFile)) {
      records.push({ shard, missingStats: true, dir });
      continue;
    }
    let doc;
    try {
      doc = JSON.parse(fs.readFileSync(statsFile, 'utf8'));
    } catch (e) {
      fatal(`分片 ${shard} 的 e2e-assert-stats.json 解析失败：${e.message}`);
    }
    if (doc.schema !== SCHEMA)
      fatal(
        `分片 ${shard} 的上报 schema 不是 ${SCHEMA}（got ${String(doc.schema)}），判据失去共同语言`
      );
    let http2xx = 0;
    for (const f of fs.readdirSync(dir)) {
      if (!/^playwright-output.*\.txt$/.test(f)) continue;
      const txt = fs.readFileSync(path.join(dir, f), 'utf8');
      http2xx += (txt.match(HTTP2XX) || []).length;
    }
    records.push({ shard, doc, http2xx, dir });
  }
  if (records.length === 0)
    fatal(`未在任何产物目录里找到 e2e-report-shard-* 分片产物：本轮 E2E 无取证材料，按未验证判红`);
  return records;
}

function loadCounts(countsFile) {
  if (!fs.existsSync(countsFile))
    fatal(
      `分组期望数文件不存在：${countsFile}（check-e2e-shard-coverage.mjs 未落盘，判据一失去对账对象）`
    );
  const j = JSON.parse(fs.readFileSync(countsFile, 'utf8'));
  if (j.schema !== 'e2e-shard-group-counts/1')
    fatal(`分组期望数 schema 非 e2e-shard-group-counts/1`);
  return j.groups;
}

function walkTs(dir, acc) {
  if (!fs.existsSync(dir)) return acc;
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) walkTs(p, acc);
    else if (/\.(ts|tsx)$/.test(ent.name)) acc.push(p);
  }
  return acc;
}

function scanSourceMarkers(repoRoot) {
  const files = walkTs(path.join(repoRoot, 'frontend', 'e2e'), []);
  const hits = { skip: [], only: [], flakyThresholdNote: null };
  const onlyRe = /\btest\.(only|describe\.only)\(|\btest\.describe\.only\(/g;
  const skipRe = /\btest\.skip\(/g;
  for (const f of files) {
    const lines = fs.readFileSync(f, 'utf8').split('\n');
    lines.forEach((line, idx) => {
      if (line.trim().startsWith('//')) return;
      for (const m of line.matchAll(onlyRe))
        hits.only.push({ file: path.relative(repoRoot, f), line: idx + 1, text: m[0] });
      for (const m of line.matchAll(skipRe))
        hits.skip.push({ file: path.relative(repoRoot, f), line: idx + 1, text: m[0] });
    });
  }
  return hits;
}

// ---------------------------------------------------------------- 判据核心（纯函数，自测直接喂记录）

export function runCoreChecks({ shardRecords, groups, counts, only, skip, baseline }) {
  const reds = [];
  const warns = [];
  const b = {
    skipMarkers: 0,
    onlyMax: 0,
    flakyWarnThreshold: 8,
    realHttp2xxMinPerShard: 1,
    zeroAssertionExempt: [],
    ...baseline,
  };
  const exemptKeys = new Set((b.zeroAssertionExempt || []).map(e => `${e.file}|${e.title}`));

  // 判据一：空片 + 分组对账
  let totalExecuted = 0;
  for (const r of shardRecords) {
    if (r.missingStats) {
      reds.push(
        `判据一 分片 ${r.shard} 无 e2e-assert-stats.json：跑测步没走完上报或产物缺文件，按未验证判红`
      );
      continue;
    }
    const s = r.doc.stats || {};
    const executed = (s.expected || 0) + (s.unexpected || 0) + (s.flaky || 0);
    totalExecuted += executed;
    if (executed <= 0)
      reds.push(
        `判据一 分片 ${r.shard} executed=0（skipped=${s.skipped || 0}）：0 个用例的"success"就是假绿形态`
      );
  }
  const shardOf = new Map();
  for (const [key, g] of groups) {
    for (const { shard } of g.members) shardOf.set(shard, key);
  }
  for (const [key, g] of groups) {
    const c = counts[key];
    if (!c) continue;
    const present = g.members.filter(m =>
      shardRecords.some(r => r.shard === m.shard && !r.missingStats)
    );
    if (present.length === 0) continue; // 本轮矩阵没选到这组（如 main push 子集），不在此处对账
    if (present.length !== g.members.length) {
      const execSum = present.reduce((acc, m) => {
        const r = shardRecords.find(x => x.shard === m.shard);
        const s = r.doc.stats || {};
        return acc + (s.expected || 0) + (s.unexpected || 0) + (s.flaky || 0);
      }, 0);
      if (execSum > c.expected)
        reds.push(
          `判据一 分组 ${key} 子集执行数 ${execSum} 超过登记矩阵 ${c.expected}：矩阵与产物矛盾`
        );
      continue;
    }
    const sum = g.members.reduce((acc, m) => {
      const r = shardRecords.find(x => x.shard === m.shard);
      const s = r.doc.stats || {};
      return acc + (s.expected || 0) + (s.unexpected || 0) + (s.flaky || 0);
    }, 0);
    if (sum !== c.expected)
      reds.push(
        `判据一 分组 ${key} 各片 executed 之和 ${sum} ≠ 登记矩阵 ${c.expected}（差 ${c.expected - sum} 条被静默吞掉或多跑）`
      );
  }

  // 判据二：passed 用例零断言点名
  for (const r of shardRecords) {
    if (r.missingStats) continue;
    for (const t of r.doc.tests || []) {
      if (t.status !== 'passed') continue;
      const hasAssert = (t.assertionsPerAttempt || []).some(a => typeof a === 'number' && a > 0);
      if (hasAssert) continue;
      const key = `${t.file}|${t.title}`;
      if (exemptKeys.has(key)) continue;
      reds.push(
        `判据二 零断言用例 ${t.file}:${t.line} 「${t.title}」passed 但没有任何 expect 步骤`
      );
    }
  }

  // 判据三：only 恒零、skip 棘轮、flaky 阈值告警
  if (only.length > (b.onlyMax || 0))
    for (const h of only.slice(0, 10))
      reds.push(`判据三 源码出现 test.only 类标记 ${h.file}:${h.line}（基线上限 ${b.onlyMax}）`);
  if (skip.length > b.skipMarkers)
    reds.push(
      `判据三 源码 test.skip( 计数 ${skip.length} 超基线 ${b.skipMarkers}，新增位点：` +
        skip
          .slice(b.skipMarkers)
          .slice(0, 10)
          .map(h => `${h.file}:${h.line}`)
          .join('、')
    );
  let flaky = 0;
  for (const r of shardRecords) {
    if (r.missingStats) continue;
    flaky += (r.doc.stats || {}).flaky || 0;
  }
  if (flaky > b.flakyWarnThreshold)
    warns.push(
      `判据三 本轮 flaky（重试后才过）用例 ${flaky} 枚，超过阈值 ${b.flakyWarnThreshold}：掩盖面扩大`
    );

  // 判据四：每片真实 2xx 证据
  for (const r of shardRecords) {
    if (r.missingStats) continue;
    const s = r.doc.stats || {};
    const executed = (s.expected || 0) + (s.unexpected || 0) + (s.flaky || 0);
    if (executed === 0) continue; // 空片已在判据一点名，避免重复计红
    if ((r.http2xx || 0) < b.realHttp2xxMinPerShard)
      reds.push(
        `判据四 分片 ${r.shard} 跑测日志里 pw:api 真实 2xx 响应行 ${r.http2xx || 0} 条，低于下限 ${b.realHttp2xxMinPerShard}：疑似后端未达/请求全被吞`
      );
  }

  return { reds, warns, totalExecuted, flaky };
}

// ---------------------------------------------------------------- 自证（双向探针）

export function selfTest() {
  const groups = new Map([
    [
      'fixture-group',
      {
        targets: ['e2e/fixture/'],
        total: 2,
        members: [
          { shard: 900, no: 1 },
          { shard: 901, no: 2 },
        ],
      },
    ],
  ]);
  const goodDoc = extra => ({
    schema: SCHEMA,
    stats: { expected: 2, unexpected: 0, flaky: 0, skipped: 0, ...extra },
    tests: [
      {
        file: 'e2e/fixture/a.spec.ts',
        line: 10,
        title: 'good one',
        status: 'passed',
        assertionsPerAttempt: [3],
      },
      {
        file: 'e2e/fixture/b.spec.ts',
        line: 20,
        title: 'good two',
        status: 'passed',
        assertionsPerAttempt: [1],
      },
    ],
  });
  const baseline = {
    skipMarkers: 2,
    onlyMax: 0,
    flakyWarnThreshold: 8,
    realHttp2xxMinPerShard: 1,
    zeroAssertionExempt: [],
  };
  const cases = [];
  const push = (name, fn) => cases.push({ name, fn });

  push('好样本必须全绿', () => {
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc: goodDoc(), http2xx: 5 },
        { shard: 901, doc: goodDoc(), http2xx: 2 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 4 } },
      only: [],
      skip: [],
      baseline,
    });
    return r.reds.length === 0 ? null : `好样本被误判红：${r.reds.join(';')}`;
  });
  push('空片必红（判据一）', () => {
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc: goodDoc({ expected: 0, skipped: 4 }), http2xx: 0 },
        { shard: 901, doc: goodDoc(), http2xx: 2 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 2 } },
      only: [],
      skip: [],
      baseline,
    });
    return r.reds.some(x => x.startsWith('判据一 分片 900 executed=0')) ? null : '空片未被点名';
  });
  push('分组对账缺口必红（判据一）', () => {
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc: goodDoc({ expected: 1 }), http2xx: 1 },
        { shard: 901, doc: goodDoc({ expected: 1 }), http2xx: 1 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 9 } },
      only: [],
      skip: [],
      baseline,
    });
    return r.reds.some(x => x.includes('登记矩阵 9')) ? null : 'Σexecuted≠expected 未被点名';
  });
  push('缺 stats 文件必红（判据一 fail-closed）', () => {
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, missingStats: true },
        { shard: 901, doc: goodDoc(), http2xx: 3 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 2 } },
      only: [],
      skip: [],
      baseline,
    });
    return r.reds.some(x => x.includes('无 e2e-assert-stats.json')) ? null : '缺上报未判红';
  });
  push('零断言 passed 必红（判据二）', () => {
    const doc = goodDoc();
    doc.tests[1].assertionsPerAttempt = [0];
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc, http2xx: 1 },
        { shard: 901, doc: goodDoc(), http2xx: 1 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 4 } },
      only: [],
      skip: [],
      baseline,
    });
    return r.reds.some(x => x.startsWith('判据二 零断言用例')) ? null : '零断言未点名';
  });
  push('豁免清单必须生效（判据二不越权）', () => {
    const doc = goodDoc();
    doc.tests[1].assertionsPerAttempt = [0];
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc, http2xx: 1 },
        { shard: 901, doc: goodDoc(), http2xx: 1 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 4 } },
      only: [],
      skip: [],
      baseline: {
        ...baseline,
        zeroAssertionExempt: [{ file: 'e2e/fixture/b.spec.ts', title: 'good two' }],
      },
    });
    return r.reds.length === 0 ? null : '豁免未生效：' + r.reds.join(';');
  });
  push('test.only 泄漏必红（判据三）', () => {
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc: goodDoc(), http2xx: 1 },
        { shard: 901, doc: goodDoc(), http2xx: 1 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 4 } },
      only: [{ file: 'e2e/fixture/a.spec.ts', line: 3, text: 'test.only(' }],
      skip: [],
      baseline,
    });
    return r.reds.some(x => x.startsWith('判据三 源码出现 test.only')) ? null : 'only 泄漏未判红';
  });
  push('skip 计数升高必红（判据三棘轮）', () => {
    const skips = [1, 2, 3].map(l => ({
      file: 'e2e/fixture/a.spec.ts',
      line: l,
      text: 'test.skip(',
    }));
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc: goodDoc(), http2xx: 1 },
        { shard: 901, doc: goodDoc(), http2xx: 1 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 4 } },
      only: [],
      skip: skips,
      baseline,
    });
    return r.reds.some(x => x.startsWith('判据三 源码 test.skip( 计数 3 超基线 2'))
      ? null
      : 'skip 棘轮失效';
  });
  push('无真实 2xx 证据必红（判据四）', () => {
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc: goodDoc(), http2xx: 0 },
        { shard: 901, doc: goodDoc(), http2xx: 1 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 4 } },
      only: [],
      skip: [],
      baseline,
    });
    return r.reds.some(x => x.startsWith('判据四 分片 900')) ? null : '零真实响应未判红';
  });
  push('flaky 越阈值只告警不判红（判据三阈值面）', () => {
    const docA = goodDoc({ flaky: 9, expected: 0 });
    const r = runCoreChecks({
      shardRecords: [
        { shard: 900, doc: docA, http2xx: 1 },
        { shard: 901, doc: goodDoc(), http2xx: 1 },
      ],
      groups,
      counts: { 'fixture-group': { expected: 11 } },
      only: [],
      skip: [],
      baseline,
    });
    return r.reds.length === 0 && r.warns.some(x => x.startsWith('判据三 本轮 flaky'))
      ? null
      : 'flaky 告警形态不符';
  });

  let fail = 0;
  for (const c of cases) {
    const err = c.fn();
    if (err) {
      fail += 1;
      console.error(`SELFTEST_FAIL ${c.name}：${err}`);
    } else {
      console.log(`SELFTEST_OK_CASE ${c.name}`);
    }
  }
  if (fail > 0) {
    console.error(
      `::error::E2E 真实性门禁自证失败 ${fail}/${cases.length}：判据失去检测能力，禁止上线`
    );
    process.exit(1);
  }
  console.log(`AUTHGUARD_SELFTEST_OK ${cases.length} 项双向探针（好样本绿、坏样本红）全部符合预期`);
}

// ---------------------------------------------------------------- 正式路径

function mainRun() {
  const artifacts = path.resolve(arg('artifacts', ''));
  const countsFile = path.resolve(arg('counts', ''));
  const repoRoot = path.resolve(arg('repo', '.'));
  const baselineFile = path.resolve(arg('baseline', ''));
  if (!artifacts || !countsFile || !fs.existsSync(baselineFile))
    fatal('缺 --artifacts / --counts / --baseline 入参（fail-closed，不做无对账对象的"通过"）');
  const shardRecords = loadShardRecords(artifacts);
  const counts = loadCounts(countsFile);
  const baseline = JSON.parse(fs.readFileSync(baselineFile, 'utf8'));

  const wf = readWorkflowText(DEFAULT_WORKFLOW_PATH);
  const block = locateCiE2eBlock(wf);
  if (!block) fatal('无法在 workflow 定位 ci-e2e job（分片编排解析锚点失配）');
  const parsed = parseShardAssignments(block);
  if (parsed.fatal) fatal('分片编排解析失败：' + parsed.fatalReason);
  const groups = groupAssignments(parsed.assignments);

  const hits = scanSourceMarkers(repoRoot);
  const out = runCoreChecks({
    shardRecords,
    groups,
    counts,
    only: hits.only,
    skip: hits.skip,
    baseline,
  });
  for (const w of out.warns) console.log(`::warning::${w}`);
  for (const r of out.reds) console.error(`::error::${r}`);
  if (out.reds.length > 0) {
    console.error(
      `AUTHGUARD_VERDICT RED ${out.reds.length} 项（总执行 ${out.totalExecuted} 条，flaky ${out.flaky} 枚）`
    );
    process.exit(1);
  }
  console.log(
    `AUTHGUARD_VERDICT OK：${shardRecords.length} 片、总执行 ${out.totalExecuted} 条、flaky ${out.flaky} 枚、源码 skip=${hits.skip.length}(基线 ${baseline.skipMarkers}) only=${hits.only.length}`
  );
}

if (process.argv.includes('--self-test')) selfTest();
else mainRun();
