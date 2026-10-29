#!/usr/bin/env node
// check-e2e-coverage-ratchet.mjs — 判据五/七的棘轮判据（只降不升）。
// 功能：解析 frontend/scripts/e2e_route_coverage.py 输出中的两行真实计数
//   （"不可复现风险计数：解析不出路径的位点=X e2e 字面量对不上快照=Y 方法不符=Z
//   矩阵条目缺方法=W" 与 "三档：A 零覆盖=N ..."），与 e2e-coverage-ratchet.json 基线比对：
//   任一计数升高即点名判红；下降发 ::notice:: 提示收紧基线；哨兵行缺失按判据失效判红
//   （空操作绿比红更危险）。
// 调用方：.github/workflows/ci-cd.yml job e2e-authenticity-guard 的"清理位点与覆盖棘轮"步骤；
//   该步骤先用 e2e_route_coverage.py 自带 --self-test 做夹具双向探针，再跑本脚本比对。
// 入参：--report <py 输出全文文件> --baseline <基线 json>；--self-test 用内联好/坏样本
//   做双向探针（坏样本必红、好样本必绿），不读外部文件；--write-baseline 从 --report
//   解析五计数写回基线（基线唯一合法生成通道，禁手写数字）。
// 退出码三态：0 PASS；1 FAIL（计数升高/哨兵缺失等判据失效，点名）；2 数据缺失 SKIP
//   （report/baseline 文件不在，点名缺什么，由 workflow 转接出 ::warning:: 不当通过）。
// 传给谁：结果只进 stdout 与 ::error::/::notice:: annotation。
// 存什么：不持久化；基线数字的权威来源是 e2e-coverage-ratchet.json（本脚本
//   --write-baseline 生成），本脚本文档不重复记数。
// 存哪里：本文件；基线 frontend/scripts/e2e-coverage-ratchet.json；计数来源
//   e2e_route_coverage.py（其判据口径与该工具的 --self-test 夹具同源，单源不复制）。
import fs from 'node:fs';

const FIELDS = [
  ['unresolved', /解析不出路径的位点=(\d+)/],
  ['ghostLiteral', /e2e 字面量对不上快照=(\d+)/],
  ['methodMismatch', /方法不符=(\d+)/],
  ['registryNoMethod', /矩阵条目缺方法=(\d+)/],
  ['aZero', /A 零覆盖=(\d+)/],
];

function arg(name, dflt) {
  const i = process.argv.indexOf(`--${name}`);
  if (i !== -1 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--'))
    return process.argv[i + 1];
  return dflt;
}

export function parseReport(text) {
  const got = {};
  const miss = [];
  for (const [k, re] of FIELDS) {
    const m = re.exec(text);
    if (m) got[k] = Number(m[1]);
    else miss.push(k);
  }
  return { got, miss };
}

export function compareRatchet(got, baseline) {
  const reds = [];
  const tightens = [];
  for (const [k] of FIELDS) {
    if (got[k] > baseline[k])
      reds.push(
        `棘轮 ${k}：本轮 ${got[k]} > 基线 ${baseline[k]}（只降不升；点名明细见 e2e_route_coverage.py 报告原文）`
      );
    else if (got[k] < baseline[k]) tightens.push(`${k} ${baseline[k]}→${got[k]}`);
  }
  return { reds, tightens };
}

function selfTest() {
  const good =
    '三档：A 零覆盖=100  B 浅覆盖=50  C 深覆盖=70  （A+B+C=220）\n' +
    '不可复现风险计数：解析不出路径的位点=10  e2e 字面量对不上快照=5  方法不符=2  矩阵条目缺方法=0\n';
  const baseline = {
    unresolved: 10,
    ghostLiteral: 5,
    methodMismatch: 2,
    registryNoMethod: 0,
    aZero: 100,
  };
  const bad = good.replace('解析不出路径的位点=10', '解析不出路径的位点=11');
  const noSentinel = '随便什么输出，没有计数行';
  let fail = 0;
  const expect = (name, cond) => {
    if (cond) console.log(`SELFTEST_OK_CASE ${name}`);
    else {
      fail += 1;
      console.error(`SELFTEST_FAIL ${name}`);
    }
  };
  const p1 = parseReport(good);
  expect(
    '好样本五计数全解析',
    p1.miss.length === 0 && p1.got.unresolved === 10 && p1.got.aZero === 100
  );
  expect('好样本对基线持平为绿', compareRatchet(p1.got, baseline).reds.length === 0);
  const p2 = parseReport(bad);
  expect(
    '计数升高必红（反向夹具）',
    compareRatchet(p2.got, baseline).reds.some(r => r.includes('unresolved'))
  );
  const p3 = parseReport(noSentinel);
  expect('哨兵行缺失必红（空操作防线）', p3.miss.length === 5);
  const p4 = parseReport(good.replace('解析不出路径的位点=10', '解析不出路径的位点=8'));
  expect(
    '计数下降为绿并提示收紧',
    compareRatchet(p4.got, baseline).reds.length === 0 &&
      compareRatchet(p4.got, baseline).tightens.some(t => t.startsWith('unresolved'))
  );
  if (fail > 0) {
    console.error(`::error::覆盖棘轮自证失败 ${fail} 项：判据失去检测能力，禁止上线`);
    process.exit(1);
  }
  console.log('RATCHET_SELFTEST_OK 5 项双向探针（好绿、坏红、缺哨兵红、下降收紧）全部符合预期');
}

function mainRun() {
  const reportFile = arg('report', '');
  const baselineFile = arg('baseline', '');
  if (!reportFile || !baselineFile) {
    console.error('::error::覆盖棘轮：缺 --report / --baseline 入参（配置违规，判红）');
    process.exit(1);
  }
  if (!fs.existsSync(reportFile)) {
    console.log(
      `::warning::缺数据：判定器报告文件不存在 ${reportFile}，棘轮比对未执行（不判通过）`
    );
    console.log('RATCHET_VERDICT SKIP：数据缺失已点名');
    process.exit(2);
  }
  const text = fs.readFileSync(reportFile, 'utf8');
  const { got, miss } = parseReport(text);
  if (miss.length > 0) {
    console.error(
      `::error::覆盖棘轮：判定器输出缺少计数哨兵字段 ${miss.join(',')}，判据本身失效即判红（不接受空操作绿）`
    );
    process.exit(1);
  }
  if (process.argv.includes('--write-baseline')) {
    if (!fs.existsSync(baselineFile)) {
      console.log(
        `::warning::缺数据：基线文件不存在 ${baselineFile}，--write-baseline 需既有骨架可保留阈值字段`
      );
      process.exit(2);
    }
    const base = JSON.parse(fs.readFileSync(baselineFile, 'utf8'));
    const doc = {
      ...base,
      unresolved: got.unresolved,
      ghostLiteral: got.ghostLiteral,
      methodMismatch: got.methodMismatch,
      registryNoMethod: got.registryNoMethod,
      aZero: got.aZero,
    };
    fs.writeFileSync(baselineFile, JSON.stringify(doc, null, 2) + '\n');
    console.log(`BASELINE_WRITTEN ${JSON.stringify(got)} source=e2e_route_coverage.py 报告哨兵行`);
    process.exit(0);
  }
  if (!fs.existsSync(baselineFile)) {
    console.log(`::warning::缺数据：基线文件不存在 ${baselineFile}，棘轮比对未执行（不判通过）`);
    console.log('RATCHET_VERDICT SKIP：数据缺失已点名');
    process.exit(2);
  }
  const baseline = JSON.parse(fs.readFileSync(baselineFile, 'utf8'));
  const { reds, tightens } = compareRatchet(got, baseline);
  for (const r of reds) console.error(`::error::${r}`);
  if (reds.length > 0) {
    console.error(`RATCHET_VERDICT RED ${reds.length} 项升高：本轮计数 ${JSON.stringify(got)}`);
    process.exit(1);
  }
  if (tightens.length > 0) console.log(`::notice::覆盖棘轮可收紧：${tightens.join('，')}`);
  console.log(`RATCHET_VERDICT OK：本轮计数 ${JSON.stringify(got)} 全部 ≤ 基线`);
}

if (process.argv.includes('--self-test')) selfTest();
else mainRun();
