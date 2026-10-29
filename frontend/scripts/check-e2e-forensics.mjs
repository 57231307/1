#!/usr/bin/env node
// check-e2e-forensics.mjs — 判据六：失败分片取证真实性（从"看结论"到"看证据"）。
// 功能：对 run 内每一个 🎭 E2E 分片 job 核验三件事：
//   1. 每片必须留下轻量报告制品 e2e-report-shard-<N>-<run>（跑了却没取证残留＝盲区）；
//   2. 判红/cancel/timed_out 的片，job 日志必须含自述行 `E2E-FORENSICS-MAP shard0based=<N>`
//      且 N 与该 job 矩阵位一致（防按显示名反查取错片，把红读成全绿）；
//   3. 失败片必须有 ≥1 条 `E2E-ASSERT-BEGIN`（断言原文进日志正文）与诊断制品
//      e2e-diag-shard-<N>-<run>；缺任一即判红——"失败但取不出断言原文"的形态正是
//      本判据要拦的（全局 setup 崩、超时 kill 前无附件都在此点名）。
// 退出码语义：0 PASS；1 FAIL（点名缺哪一环）。本判据不设 2=SKIP——"失败片取不到证据"
// 正是本判据要拦的违规形态本身；无 E2E job 的合法轮（tag/dispatch）由步骤 if 拦截，
// 脚本一旦真跑就必须给出可判红的结论（三态语义在判据一~四与棘轮脚本保留，此处有意收口）。
// 调用方：.github/workflows/ci-cd.yml job e2e-authenticity-guard 判据六步骤；
//   仅在 needs.ci-e2e.result 为 success/failure（本轮真跑过 E2E）时执行。
// 入参：--run <run_id>（缺省 env GITHUB_RUN_ID）、--repo <owner/name>（缺省 env GITHUB_REPOSITORY）、
//   token 只从 env GITHUB_TOKEN 读（不落日志、不入仓库）；--jobs-file/--artifacts-file/--logs-dir
//   为离线注入模式（自测用），给了就完全不联网。
// 传给谁：结果只进 stdout 与 ::error:: annotation。
// 存什么：不持久化；label→shard 映射取 e2e-shard-registry.mjs 的 E2E_SHARD_MATRIX 单一来源。
// 存哪里：本文件；取数端点为 /actions/runs/{id}/jobs、/actions/runs/{id}/artifacts、
//   /actions/jobs/{job_id}/logs（302 到对象存储时 urllib/fetch 不透传 Authorization，凭据不出站）。
import fs from 'node:fs';
import { E2E_SHARD_MATRIX } from './e2e-shard-registry.mjs';

const JOB_NAME_PREFIX = '🎭 E2E: ';
const FAIL_CONCLUSIONS = new Set([
  'failure',
  'cancelled',
  'timed_out',
  'startup_failure',
  'action_run_failure',
]);

function arg(name, dflt) {
  const i = process.argv.indexOf(`--${name}`);
  if (i !== -1 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--'))
    return process.argv[i + 1];
  return dflt;
}

export function checkForensics({ jobs, artifacts, logText, runId, redPrefix = '判据六' }) {
  const reds = [];
  const labelToShard = new Map(E2E_SHARD_MATRIX.map(m => [m.label, m.shard]));
  const artifactNames = new Set(artifacts.map(a => a.name));
  let e2eJobCount = 0;
  for (const job of jobs) {
    if (!job.name || !job.name.startsWith(JOB_NAME_PREFIX)) continue;
    e2eJobCount += 1;
    const label = job.name.slice(JOB_NAME_PREFIX.length);
    const shard = labelToShard.get(label);
    if (shard === undefined) {
      reds.push(
        `${redPrefix} E2E job 显示名「${label}」不在 E2E_SHARD_MATRIX 登记内：矩阵与 job 名漂移`
      );
      continue;
    }
    const reportName = `e2e-report-shard-${shard}-${runId}`;
    if (!artifactNames.has(reportName))
      reds.push(
        `${redPrefix} 分片 ${shard}（${label}，结论=${job.conclusion}）无轻量报告制品 ${reportName}：跑了无残留即盲区`
      );
    if (FAIL_CONCLUSIONS.has(job.conclusion)) {
      const log = logText(job) || '';
      const m = /E2E-FORENSICS-MAP shard0based=(\d+) label=(.+?) /.exec(log);
      if (!m) {
        reds.push(
          `${redPrefix} 失败分片 ${shard} 日志缺 0 基映射行 E2E-FORENSICS-MAP：无法把日志与制品按片对上号`
        );
      } else if (Number(m[1]) !== shard) {
        reds.push(
          `${redPrefix} 失败分片 ${shard} 的映射行自称 shard0based=${m[1]}：日志与矩阵位串片`
        );
      }
      if (!log.includes('E2E-ASSERT-BEGIN'))
        reds.push(
          `${redPrefix} 失败分片 ${shard} 日志无 E2E-ASSERT-BEGIN 断言原文：失败未带证据即取证失效（附件缺失发生在落盘前则对照跑测步日志排查）`
        );
      const diagName = `e2e-diag-shard-${shard}-${runId}`;
      if (!artifactNames.has(diagName))
        reds.push(
          `${redPrefix} 失败分片 ${shard} 无诊断制品 ${diagName}（trace/video/error-context 原件缺位）`
        );
    }
  }
  if (e2eJobCount === 0)
    reds.push(
      `${redPrefix} 本轮 run 内一个 🎭 E2E job 都没有，但守卫被调起：矩阵为空即静默丢覆盖形态`
    );
  return { reds, e2eJobCount };
}

// ---------------------------------------------------------------- 联网装载

async function ghFetch(url, token) {
  const res = await fetch(url, {
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      'User-Agent': 'e2e-authenticity-guard',
    },
    redirect: 'follow',
  });
  if (!res.ok) throw new Error(`GET ${url} → HTTP ${res.status}`);
  return res;
}

async function fetchPaged(urlBase, token, key) {
  const out = [];
  for (let page = 1; page <= 20; page += 1) {
    const sep = urlBase.includes('?') ? '&' : '?';
    const res = await ghFetch(`${urlBase}${sep}per_page=100&page=${page}`, token);
    const doc = await res.json();
    const items = doc[key] || [];
    out.push(...items);
    if (items.length < 100) return out;
  }
  throw new Error(`分页超过 20 页仍未收口：${urlBase}`);
}

async function mainRun() {
  const token = process.env.GITHUB_TOKEN || '';
  const runId = arg('run', process.env.GITHUB_RUN_ID || '');
  const repo = arg('repo', process.env.GITHUB_REPOSITORY || '');
  if (!runId || !repo) {
    console.error('::error::判据六：缺 run/repo 入参（fail-closed）');
    process.exit(1);
  }
  const jobsFile = arg('jobs-file', '');
  const artifactsFile = arg('artifacts-file', '');
  const logsDir = arg('logs-dir', '');
  let jobs;
  let artifacts;
  let logText;
  if (jobsFile && artifactsFile) {
    jobs = JSON.parse(fs.readFileSync(jobsFile, 'utf8')).jobs;
    artifacts = JSON.parse(fs.readFileSync(artifactsFile, 'utf8')).artifacts;
    logText = job => {
      const f = path_join(logsDir, `${job.id}.log`);
      return fs.existsSync(f) ? fs.readFileSync(f, 'utf8') : '';
    };
  } else {
    if (!token) {
      console.error(
        '::error::判据六：无联网注入文件时必须有 GITHUB_TOKEN（缺失即 403 盲区，fail-closed 判红）'
      );
      process.exit(1);
    }
    const api = `https://api.github.com/repos/${repo}`;
    try {
      jobs = await fetchPaged(`${api}/actions/runs/${runId}/jobs`, token, 'jobs');
      artifacts = await fetchPaged(`${api}/actions/runs/${runId}/artifacts`, token, 'artifacts');
    } catch (e) {
      console.error(
        `::error::判据六：取数失败 ${e.message}（403＝token 无 actions:read，禁止按"取不到=没问题"放行）`
      );
      process.exit(1);
    }
    const logOf = {};
    for (const job of jobs) {
      if (!job.name.startsWith(JOB_NAME_PREFIX)) continue;
      if (!FAIL_CONCLUSIONS.has(job.conclusion)) continue;
      try {
        const res = await ghFetch(`${api}/actions/jobs/${job.id}/logs`, token);
        logOf[job.id] = await res.text();
      } catch (e) {
        logOf[job.id] = '';
        console.error(`::error::判据六：job ${job.id} 日志拉取失败 ${e.message}`);
      }
    }
    logText = job => logOf[job.id] || '';
  }
  const out = checkForensics({ jobs, artifacts, logText, runId: Number(runId) });
  report(out);
}

function path_join(dir, f) {
  return dir ? `${dir}/${f}` : f;
}

function report(out) {
  for (const r of out.reds) console.error(`::error::${r}`);
  if (out.reds.length > 0) {
    console.error(`FORENSICS_VERDICT RED ${out.reds.length} 项（E2E job ${out.e2eJobCount} 个）`);
    process.exit(1);
  }
  console.log(
    `FORENSICS_VERDICT OK：E2E job ${out.e2eJobCount} 个全部有报告残留，失败片断言原文与诊断制品齐备`
  );
}

// ---------------------------------------------------------------- 自证双向探针

function selfTest() {
  const run = 999;
  const s0 = E2E_SHARD_MATRIX[0];
  const s1 = E2E_SHARD_MATRIX[1];
  const jobs = [
    { id: 1, name: `${JOB_NAME_PREFIX}${s0.label}`, conclusion: 'success' },
    { id: 2, name: `${JOB_NAME_PREFIX}${s1.label}`, conclusion: 'failure' },
    { id: 3, name: '🧹 别的 job', conclusion: 'failure' },
  ];
  const artifacts = [
    { name: `e2e-report-shard-${s0.shard}-${run}` },
    { name: `e2e-report-shard-${s1.shard}-${run}` },
    { name: `e2e-diag-shard-${s1.shard}-${run}` },
  ];
  const goodLog = `x\nE2E-FORENSICS-MAP shard0based=${s1.shard} label=${s1.label} reportArtifact=r diagArtifact=d\nE2E-ASSERT-BEGIN /tmp/a/error-context.md\nboom\nE2E-ASSERT-END /tmp/a/error-context.md\n`;
  const logs = { 2: goodLog };
  const logText = j => logs[j.id] || '';
  let fail = 0;
  const expect = (name, cond) => {
    if (cond) console.log(`SELFTEST_OK_CASE ${name}`);
    else {
      fail += 1;
      console.error(`SELFTEST_FAIL ${name}`);
    }
  };
  expect('好样本全绿', checkForensics({ jobs, artifacts, logText, runId: run }).reds.length === 0);
  expect(
    '失败片缺报告制品必红',
    checkForensics({ jobs, artifacts: artifacts.slice(1), logText, runId: run }).reds.some(r =>
      r.includes('无轻量报告制品')
    )
  );
  expect(
    '失败片缺映射行必红',
    checkForensics({ jobs, artifacts, logText: () => 'no map here', runId: run }).reds.some(r =>
      r.includes('缺 0 基映射行')
    )
  );
  expect(
    '失败片缺断言原文必红',
    checkForensics({
      jobs,
      artifacts,
      logText: () => `E2E-FORENSICS-MAP shard0based=${s1.shard} label=${s1.label} x`,
      runId: run,
    }).reds.some(r => r.includes('E2E-ASSERT-BEGIN'))
  );
  expect(
    '映射行串片必红',
    checkForensics({
      jobs,
      artifacts,
      logText: () => goodLog.replace(`shard0based=${s1.shard}`, `shard0based=${s1.shard + 3}`),
      runId: run,
    }).reds.some(r => r.includes('串片'))
  );
  expect(
    '失败片缺诊断制品必红',
    checkForensics({
      jobs,
      artifacts: artifacts.filter(a => !a.name.includes('diag')),
      logText,
      runId: run,
    }).reds.some(r => r.includes('无诊断制品'))
  );
  expect(
    '矩阵外显示名必红',
    checkForensics({
      jobs: [{ id: 4, name: `${JOB_NAME_PREFIX}不存在的分组`, conclusion: 'success' }],
      artifacts,
      logText,
      runId: run,
    }).reds.some(r => r.includes('不在 E2E_SHARD_MATRIX'))
  );
  expect(
    '空矩阵 run 必红（无残留盲区形态）',
    checkForensics({ jobs: [], artifacts, logText, runId: run }).reds.some(r =>
      r.includes('一个 🎭 E2E job 都没有')
    )
  );
  if (fail > 0) {
    console.error(`::error::判据六自证失败 ${fail} 项：取证判据失去检测能力，禁止上线`);
    process.exit(1);
  }
  console.log(`FORENSICS_SELFTEST_OK 8 项双向探针（好绿、七类坏红）全部符合预期`);
}

if (process.argv.includes('--self-test')) selfTest();
else
  mainRun().catch(e => {
    console.error(`::error::判据六脚本自身异常：${e.message}`);
    process.exit(1);
  });
