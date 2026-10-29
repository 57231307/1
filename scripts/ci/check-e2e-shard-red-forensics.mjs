#!/usr/bin/env node
/**
 * 脚本 scripts/ci/check-e2e-shard-red-forensics.mjs
 *
 * 功能，E2E 真实性门禁的三条扩展判据判定器，一个脚本三种模式，每种模式对应一条判据。
 * 模式 shard-stats，空片与用例总数对账，逐分片取实际执行用例数，要求每片大于 0，
 * 并要求各片执行数之和不低于分片数乘以每片最小用例数 1 的结构性下限，下限由本次
 * 报告现场推得，不写死用例总数常量。
 * 模式 zero-assertion，零断言用例点名，从 Playwright JSON 报告的 steps 树统计每个
 * 实际执行且通过的用例的断言节点数，零断言用例逐条点名文件与标题，非空即判红。
 * 模式 skip-only，skip 与 only 对账，源码侧逐文件统计 test.skip( test.fixme( 与
 * describe.skip( 的计数以及 .only( 的计数，出现 only 泄漏直接判红且不设基线，
 * skip 类计数与基线文件比对且只降不升，超基线逐文件点名超出数。
 *
 * 调用方，GitHub workflow 中 E2E 真实性门禁 job 的三个对账步骤，以及本地夹具
 * 反向自证命令。
 * 入参，--mode 取 shard-stats 或 zero-assertion 或 skip-only 三选一；--artifacts
 * 为分片轻量报告产物下载后的根目录，其下每个子目录是一片，子目录名为制品名
 * e2e-report-shard 加分片号加本轮标识；--e2e-src 为源码扫描目录，缺省 frontend/e2e；
 * --baseline 为 skip 基线文件路径，缺省 scripts/ci/e2e-skip-only-baseline.json；
 * --write-baseline 为基线生成开关，仅在 skip-only 模式生效。
 * 传给谁，判定只以进程退出码与标准输出回传给门禁步骤，不写任何业务数据。
 * 退出码三态，0 为 PASS，1 为 FAIL 且已逐条点名，2 为数据缺失的 SKIP 且点名缺失原因，
 * 门禁步骤只对 2 放告警，对 1 必须判红。
 * 存什么，--write-baseline 时存 skip 类标记的逐文件计数与总数。
 * 存哪里，默认写入 scripts/ci/e2e-skip-only-baseline.json，除此之外不落任何文件。
 *
 * 取数口径如实说明，判据一优先解析分片产物内 Playwright JSON 报告的 stats 字段，
 * expected unexpected flaky interrupted 之和为执行数，skipped 单列，该字段与文本
 * 汇总行同源于 Playwright 的同一份用例统计对象。当前仓库分片跑测的 reporter 配置
 * 只有 html 与 line，产物内没有 JSON 报告，故实际退回解析 playwright-output 文本
 * 输出末尾的汇总行，汇总行由 line reporter 继承的同一统计对象打印，passed failed
 * flaky interrupted 之和为执行数，skipped 单列，did not run 属未执行不计入。
 * 判据二只认 JSON 报告的 steps 树，断言数取 steps 内 category 为 assert 的节点数，
 * Playwright 把 expect 系列断言登记为 assert 类目节点且子步骤递归在 steps 字段内，
 * 所以该计数能代表断言数；个别版本步骤缺 category 字段时退化为按标题以 expect(
 * 或 expect 空格开头识别并在输出标注退化。JSON 报告缺失时本判据不向文本降级，
 * 因为文本输出没有逐用例步骤树，直接按数据缺失记 SKIP。
 * 判据三只扫仓库源码，注释内的同名字样经字符串感知的状态机剔除后不计，字符串
 * 内容保留计数，字面量里出现 skip 或 only 字样按可疑形态计入并要求人工澄清；
 * 状态机不处理正则字面量内部引号这类极端边界，遇到时按普通字符
 * 处理。宁可少计也绝不把注释计成代码，因为基线只降不升，基线偏小会让真实新增
 * 更早触红。only 泄漏不设基线直接判红。
 */
import fs from 'node:fs';
import path from 'node:path';

const EXIT_PASS = 0;
const EXIT_FAIL = 1;
const EXIT_SKIP = 2;

function parseArgs(argv) {
  const opts = {
    mode: '',
    artifacts: 'e2e-forensics-artifacts',
    e2eSrc: path.join('frontend', 'e2e'),
    baseline: path.join('scripts', 'ci', 'e2e-skip-only-baseline.json'),
    writeBaseline: false,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--mode') opts.mode = argv[++i];
    else if (a === '--artifacts') opts.artifacts = argv[++i];
    else if (a === '--e2e-src') opts.e2eSrc = argv[++i];
    else if (a === '--baseline') opts.baseline = argv[++i];
    else if (a === '--write-baseline') opts.writeBaseline = true;
    else {
      console.error(`::error::未知参数 ${a}，判定器拒绝继续，防止拼错的入参被当成通过`);
      process.exit(EXIT_FAIL);
    }
  }
  if (!['shard-stats', 'zero-assertion', 'skip-only'].includes(opts.mode)) {
    console.error('::error::--mode 必须为 shard-stats 或 zero-assertion 或 skip-only');
    process.exit(EXIT_FAIL);
  }
  return opts;
}

function log(s) {
  console.log(s);
}

function failNamed(msg) {
  console.error(`::error::${msg}`);
}

// 分片产物根目录下的每个子目录对应一片，子目录名即制品名。
// 返回 null 表示根目录不存在（下载步骤没跑或没产出），空数组表示目录在但一片都没下载下来。
function collectShardDirs(artifactsDir) {
  if (!fs.existsSync(artifactsDir) || !fs.statSync(artifactsDir).isDirectory()) {
    return null;
  }
  const dirs = [];
  for (const ent of fs.readdirSync(artifactsDir, { withFileTypes: true })) {
    if (!ent.isDirectory()) continue;
    const m = /^e2e-report-shard-(\d+)/.exec(ent.name);
    if (!m) continue;
    dirs.push({ name: ent.name, shard: Number(m[1]), dir: path.join(artifactsDir, ent.name) });
  }
  dirs.sort((a, b) => a.shard - b.shard);
  return dirs;
}

// 递归找目录内符合 Playwright JSON 报告结构的文件，判据是同时有 suites 数组与 stats 对象。
// 产物里还有 seed-failures 之类的其他 json，靠结构识别而不是文件名，避免把无关文件读成报告。
function findJsonReports(dir) {
  const found = [];
  const walk = (d) => {
    for (const ent of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, ent.name);
      if (ent.isDirectory()) walk(p);
      else if (ent.name.endsWith('.json')) {
        let obj = null;
        try {
          obj = JSON.parse(fs.readFileSync(p, 'utf8'));
        } catch {
          continue;
        }
        if (obj && Array.isArray(obj.suites) && obj.stats && typeof obj.stats === 'object') {
          found.push({ file: p, report: obj });
        }
      }
    }
  };
  walk(dir);
  return found;
}

// 从 JSON 报告 stats 推执行数与跳过数，缺字段按 0 计，字段名与 Playwright JSON 报告一致。
function statsFromJson(report) {
  const s = report.stats || {};
  const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? v : 0);
  return {
    executed: num(s.expected) + num(s.unexpected) + num(s.flaky) + num(s.interrupted),
    skipped: num(s.skipped),
    source: 'json',
  };
}

// line reporter 继承 list reporter 的汇总行打印，格式为行首缩进加数字加状态词。
// 每个状态词只取文件中最后一次命中，汇总块在输出末尾，先前的逐用例行与调试行都压不过它。
const SUMMARY_STATUSES = [
  'passed',
  'failed',
  'flaky',
  'interrupted',
  'skipped',
  'did not run',
  "didn't finish",
];

function parseTextSummary(dir) {
  const files = fs
    .readdirSync(dir)
    .filter((f) => /^playwright-output.*\.txt$/.test(f));
  let best = null;
  for (const f of files) {
    let text = '';
    try {
      text = fs.readFileSync(path.join(dir, f), 'utf8');
    } catch {
      continue;
    }
    const counts = {};
    let hit = false;
    for (const st of SUMMARY_STATUSES) {
      const re = new RegExp(`^\\s*(\\d+) ${st}\\b`, 'gm');
      let m;
      let last = null;
      while ((m = re.exec(text)) !== null) last = Number(m[1]);
      if (last !== null) {
        counts[st] = last;
        hit = true;
      }
    }
    if (!hit) continue;
    const n = (k) => counts[k] || 0;
    const executed = n('passed') + n('failed') + n('flaky') + n('interrupted') + n("didn't finish");
    const cand = { executed, skipped: n('skipped'), source: 'text', file: f, counts };
    // 多个候选文件时取合计最大的一片，重试轮留下的旧文件只会更小，不会盖掉最新汇总。
    if (!best || cand.executed + cand.skipped > best.executed + best.skipped) best = cand;
  }
  return best;
}

function shardStatsRecord(shardDir) {
  const jsons = findJsonReports(shardDir);
  if (jsons.length > 0) {
    let executed = 0;
    let skipped = 0;
    for (const { report } of jsons) {
      const s = statsFromJson(report);
      executed += s.executed;
      skipped += s.skipped;
    }
    return { executed, skipped, source: 'json' };
  }
  const t = parseTextSummary(shardDir);
  if (t) return { executed: t.executed, skipped: t.skipped, source: `text(${t.file})` };
  return null;
}

// 判据一，空片与用例总数对账。
function modeShardStats(opts) {
  const dirs = collectShardDirs(opts.artifacts);
  if (dirs === null) {
    failNamedSkip(`分片产物根目录 ${opts.artifacts} 不存在，下载步骤未执行或未产出，空片对账记 SKIP`);
  }
  if (dirs.length === 0) {
    failNamedSkip(
      `${opts.artifacts} 存在但没有 e2e-report-shard 前缀的分片子目录，报告制品没下载下来，空片对账记 SKIP`,
    );
  }
  let totalExecuted = 0;
  const broken = [];
  const empties = [];
  for (const d of dirs) {
    const rec = shardStatsRecord(d.dir);
    if (rec === null) {
      // 上游分片 job 结论为 success 时跑测进程必然打印过汇总行，取不到摘要说明制品残缺，
      // 这里按判红而不是 SKIP，因为缺失的正是上游声称产出过的数据。
      broken.push(d);
      log(`E2E-SHARD shard=${d.shard} artifact=${d.name} executed=未知 source=摘要缺失`);
      continue;
    }
    log(
      `E2E-SHARD shard=${d.shard} artifact=${d.name} executed=${rec.executed} skipped=${rec.skipped} source=${rec.source}`,
    );
    totalExecuted += rec.executed;
    if (rec.executed <= 0) empties.push(d);
  }
  const floor = dirs.length * 1;
  log(
    `E2E-SHARD-TOTAL shards=${dirs.length} totalExecuted=${totalExecuted} floor=${floor}（下限为分片数乘以每片最小用例数 1 的结构性下限，由本次报告现场推得）`,
  );
  let isFail = false;
  for (const d of broken) {
    failNamed(
      `分片 ${d.shard}（制品 ${d.name}）内既无 Playwright JSON 报告也无法从 playwright-output 文本末尾解析出汇总行，用例数无法核实，按空片嫌疑判红`,
    );
    isFail = true;
  }
  for (const d of empties) {
    failNamed(`分片 ${d.shard}（制品 ${d.name}）实际执行用例数为 0，命中空片形态，判红`);
    isFail = true;
  }
  if (totalExecuted < floor) {
    failNamed(
      `各片执行数之和 ${totalExecuted} 低于结构性下限 ${floor}，判红`,
    );
    isFail = true;
  }
  if (isFail) process.exit(EXIT_FAIL);
  log(`空片与用例总数对账 PASS，共 ${dirs.length} 片，执行数合计 ${totalExecuted}`);
  process.exit(EXIT_PASS);
}

function failNamedSkip(msg) {
  log(`E2E-FORENSICS-SKIP ${msg}`);
  process.exit(EXIT_SKIP);
}

// 递归数一个用例 results 里 steps 树的断言节点数，返回计数与是否走了标题退化识别。
function countAsserts(steps) {
  let n = 0;
  let degraded = false;
  const walk = (list) => {
    if (!Array.isArray(list)) return;
    for (const st of list) {
      if (!st || typeof st !== 'object') continue;
      if (st.category === 'assert') n++;
      else if (
        st.category === undefined &&
        typeof st.title === 'string' &&
        (st.title.startsWith('expect(') || st.title.startsWith('expect '))
      ) {
        n++;
        degraded = true;
      }
      walk(st.steps);
    }
  };
  walk(steps);
  return { n, degraded };
}

// 递归展开套件树，specs 自身不带完整路径，点名时把沿途套件标题拼进标题路径，
// 输出形如 文件 加 套件链与用例标题，便于直接定位到具体用例。
function iterSpecs(suite, inheritedFile, titlePrefix, out) {
  const file = suite.file || inheritedFile || suite.title || '未知文件';
  const prefix = suite.title ? [...titlePrefix, suite.title] : titlePrefix;
  for (const spec of suite.specs || []) {
    out.push({ file: spec.file || file, titlePath: [...prefix, spec.title], spec });
  }
  for (const child of suite.suites || []) iterSpecs(child, file, prefix, out);
}

// 判据二，零断言用例点名。
function modeZeroAssertion(opts) {
  const dirs = collectShardDirs(opts.artifacts);
  if (dirs === null) {
    failNamedSkip(`分片产物根目录 ${opts.artifacts} 不存在，零断言点名记 SKIP`);
  }
  if (dirs.length === 0) {
    failNamedSkip(`${opts.artifacts} 下没有分片子目录，报告制品没下载下来，零断言点名记 SKIP`);
  }
  const reports = [];
  for (const d of dirs) {
    for (const j of findJsonReports(d.dir)) reports.push({ ...j, shard: d.shard });
  }
  if (reports.length === 0) {
    failNamedSkip(
      '当前分片产物内没有 Playwright JSON 报告（跑测 reporter 配置只有 html 与 line，轻量报告制品只含文本输出），零断言判定需要逐用例 steps 树，文本没有该数据，记 SKIP 而不是默认通过；分片跑测补出 json reporter 后本判据自动生效',
    );
  }
  const offenders = [];
  let anyDegraded = false;
  let executedTests = 0;
  for (const { file, report } of reports) {
    const specs = [];
    for (const root of report.suites) iterSpecs(root, null, [], specs);
    for (const { file: specFile, titlePath, spec } of specs) {
      for (const test of spec.tests || []) {
        for (const result of test.results || []) {
          // 只判实际执行且通过（或 flaky 最终通过）的用例，失败与跳过用例的步骤树
          // 本就不完整，把它们算进零断言名单是制造噪声。
          if (result.status !== 'expected' && result.status !== 'flaky') continue;
          executedTests++;
          const { n, degraded } = countAsserts(result.steps);
          if (degraded) anyDegraded = true;
          if (n === 0) {
            offenders.push({ file: specFile, title: titlePath.join(' > '), report: file });
          }
        }
      }
    }
  }
  if (anyDegraded) {
    log('E2E-ZERO-ASSERT-NOTE 部分步骤缺 category 字段，已按标题 expect 前缀退化识别');
  }
  if (offenders.length > 0) {
    for (const o of offenders) {
      failNamed(`零断言用例，文件=${o.file} 标题=${o.title}（取自报告 ${o.report}）`);
    }
    log(`E2E-ZERO-ASSERT-COUNT ${offenders.length}`);
    process.exit(EXIT_FAIL);
  }
  log(`零断言用例点名 PASS，共核 ${executedTests} 个实际执行且通过的用例，全部含断言节点`);
  process.exit(EXIT_PASS);
}

// 字符串感知的注释剔除状态机，行注释与块注释内容清空，字符串内容原样保留，
// 目的是让注释里转述的 test.skip() 字样不进计数。不处理正则字面量内部引号。
function stripComments(src) {
  let out = '';
  let mode = 'code';
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    const d = src[i + 1];
    if (mode === 'code') {
      if (c === '/' && d === '/') {
        mode = 'line';
        i += 2;
        continue;
      }
      if (c === '/' && d === '*') {
        mode = 'block';
        i += 2;
        continue;
      }
      if (c === "'") mode = 'sq';
      else if (c === '"') mode = 'dq';
      else if (c === '`') mode = 'tpl';
      out += c;
      i++;
      continue;
    }
    if (mode === 'line') {
      if (c === '\n') {
        mode = 'code';
        out += c;
      }
      i++;
      continue;
    }
    if (mode === 'block') {
      if (c === '*' && d === '/') {
        mode = 'code';
        i += 2;
        continue;
      }
      if (c === '\n') out += c;
      i++;
      continue;
    }
    // 三种引号状态，反斜杠转义整体保留，闭合引号后回代码态。
    out += c;
    if (c === '\\') {
      out += d ?? '';
      i += 2;
      continue;
    }
    if ((mode === 'sq' && c === "'") || (mode === 'dq' && c === '"') || (mode === 'tpl' && c === '`')) {
      mode = 'code';
    }
    i++;
  }
  return out;
}

function countMatches(re, text) {
  let m;
  let n = 0;
  while ((m = re.exec(text)) !== null) n++;
  return n;
}

const SKIP_EXCLUDE_DIRS = new Set(['node_modules', 'test-results', '.auth', 'playwright-report', 'blob-report']);

function collectTsFiles(root) {
  const out = [];
  const walk = (d) => {
    for (const ent of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, ent.name);
      if (ent.isDirectory()) {
        if (!SKIP_EXCLUDE_DIRS.has(ent.name)) walk(p);
      } else if (ent.name.endsWith('.ts')) {
        out.push(p);
      }
    }
  };
  walk(root);
  return out.sort();
}

function relKey(file) {
  return path.relative(process.cwd(), file).split(path.sep).join('/');
}

function scanSourceCounts(e2eSrc) {
  const files = collectTsFiles(e2eSrc);
  const rows = [];
  for (const f of files) {
    let content = '';
    try {
      content = fs.readFileSync(f, 'utf8');
    } catch {
      continue;
    }
    const code = stripComments(content);
    const skipCount =
      countMatches(/test\.skip\s*\(/g, code) +
      countMatches(/test\.fixme\s*\(/g, code) +
      countMatches(/describe\.skip\s*\(/g, code);
    const onlyCount = countMatches(/\.only\s*\(/g, code);
    if (skipCount > 0 || onlyCount > 0) rows.push({ file: f, key: relKey(f), skipCount, onlyCount });
  }
  return { totalTsFiles: files.length, rows };
}

// 判据三，skip 与 only 对账，附带报告侧 skipped 数打印。
function modeSkipOnly(opts) {
  if (!fs.existsSync(opts.e2eSrc) || !fs.statSync(opts.e2eSrc).isDirectory()) {
    failNamed(`源码扫描目录 ${opts.e2eSrc} 不存在，判据三失去扫描面，按数据异常判红而不是默认通过`);
    process.exit(EXIT_FAIL);
  }
  const { totalTsFiles, rows } = scanSourceCounts(opts.e2eSrc);
  if (totalTsFiles === 0) {
    failNamed(`${opts.e2eSrc} 下没有任何 .ts 文件，扫描面为空，判红`);
    process.exit(EXIT_FAIL);
  }
  const skipMap = {};
  let skipTotal = 0;
  let onlyTotal = 0;
  for (const r of rows) {
    log(`E2E-SKIP-ONLY file=${r.key} skip=${r.skipCount} only=${r.onlyCount}`);
    if (r.skipCount > 0) {
      skipMap[r.key] = r.skipCount;
      skipTotal += r.skipCount;
    }
    onlyTotal += r.onlyCount;
  }

  if (opts.writeBaseline) {
    const payload = {
      description: '由 scripts/ci/check-e2e-shard-red-forensics.mjs 的 write-baseline 模式生成，skip 类逐文件计数基线，只降不升',
      generatedBy: 'node scripts/ci/check-e2e-shard-red-forensics.mjs --mode skip-only --write-baseline',
      files: skipMap,
      total: skipTotal,
    };
    fs.mkdirSync(path.dirname(opts.baseline), { recursive: true });
    fs.writeFileSync(opts.baseline, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
    log(`基线已写入 ${opts.baseline}，skip 类总数 ${skipTotal}，登记文件 ${Object.keys(skipMap).length} 个`);
    process.exit(EXIT_PASS);
  }

  // 报告侧 skipped 数打印，供与源码侧对账，当前文本口径只能拿到分片合计而不是逐文件，
  // 所以不作硬判据，缺失也不改变下面两条硬判定的结果。
  const dirs = collectShardDirs(opts.artifacts);
  if (dirs === null || dirs.length === 0) {
    log('E2E-SKIP-REPORT-NOTE 分片报告目录不可得，报告侧 skipped 数本轮不打印（数据缺失已在判据一点名）');
  } else {
    let reportSkipped = 0;
    for (const d of dirs) {
      const rec = shardStatsRecord(d.dir);
      if (rec) reportSkipped += rec.skipped;
    }
    log(`E2E-SKIP-REPORT 报告侧 skipped 合计 ${reportSkipped}（供对账参考）`);
  }

  let isFail = false;
  const leaks = rows.filter((r) => r.onlyCount > 0);
  if (leaks.length > 0) {
    for (const r of leaks) {
      failNamed(`only 泄漏，文件=${r.key} 计数=${r.onlyCount}，test.only 或 .only 会吞掉其余用例，直接判红且不设基线`);
    }
    isFail = true;
  }

  if (!fs.existsSync(opts.baseline)) {
    failNamed(`skip 基线文件 ${opts.baseline} 缺失，无法执行只降不升比对，按门禁自身数据缺失判红`);
    process.exit(EXIT_FAIL);
  }
  let baseline;
  try {
    baseline = JSON.parse(fs.readFileSync(opts.baseline, 'utf8'));
  } catch {
    failNamed(`skip 基线文件 ${opts.baseline} 不是合法 JSON，判红`);
    process.exit(EXIT_FAIL);
  }
  const baseMap = (baseline && typeof baseline.files === 'object' && baseline.files) || {};
  const overs = [];
  for (const r of rows) {
    if (r.skipCount === 0) continue;
    const base = typeof baseMap[r.key] === 'number' ? baseMap[r.key] : 0;
    if (r.skipCount > base) overs.push({ key: r.key, current: r.skipCount, base, over: r.skipCount - base });
  }
  if (overs.length > 0) {
    for (const o of overs) {
      failNamed(`skip 计数超基线，文件=${o.key} 当前=${o.current} 基线=${o.base} 超出=${o.over}`);
    }
    isFail = true;
  }
  if (isFail) process.exit(EXIT_FAIL);
  if (skipTotal < (typeof baseline.total === 'number' ? baseline.total : skipTotal)) {
    log(`提示，当前 skip 总数 ${skipTotal} 已低于基线记录的 ${baseline.total}，可重生成基线让棘轮真实缩短，本轮判定不受影响`);
  }
  log(`skip 与 only 对账 PASS，扫描 ${totalTsFiles} 个 .ts 文件，skip 总数 ${skipTotal}，基线总数 ${baseline.total ?? '未登记'}`);
  process.exit(EXIT_PASS);
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (opts.mode === 'shard-stats') modeShardStats(opts);
  else if (opts.mode === 'zero-assertion') modeZeroAssertion(opts);
  else modeSkipOnly(opts);
}

main();
