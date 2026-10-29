#!/usr/bin/env node
/**
 * e2e spec 内 .el-select-dropdown 裸面板词元 棘轮门禁。
 *
 * 比对的是"每个 spec/fixture 里是否仍在直接写 .el-select-dropdown 类选择器"：
 * Element Plus 的下拉面板 teleport 到 body，同一页面并发多字段时
 * "page.locator('.el-select-dropdown:visible').last()" 会把面板读错到别的字段名下，
 * 与「aria-controls 归属钉定 + dropdownItems 单源取项」的 helper 有本质区别：
 * 前者会跨字段撞名（31c 部门停用词与停用矩阵正则重叠即此族假绿通道），
 * 后者按 aria-controls 精确锚定该字段自己的面板。裸词元一律迁 helper，
 * 唯一允许保留点在 helper 定义文件本身（frontend/e2e/flow/ui-helpers.ts）。
 *
 * 判负口径：存量按 文件 → 裸词元数 记入基线；只允许下降，不允许上升。
 * 单文件上升即失败并点名 文件:行 与原文；新增文件出现裸词元同样失败；
 * 某文件已清零但基线未同步下调也失败（逼着基线跟着收紧，防"清完不删基线"留后门）。
 *
 * 用法：node scripts/check-e2e-select-ratchet.mjs [--write-baseline]
 *       node scripts/check-e2e-select-ratchet.mjs --self-test
 *       （--self-test 用内联夹具双向自证判定器本身：正例必抓、反例必不报，不扫仓库）
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND = path.resolve(HERE, '..');
const REPO = path.resolve(FRONTEND, '..');
const E2E_DIR = path.join(FRONTEND, 'e2e');
const BASELINE = path.join(HERE, 'e2e-select-baseline.json');

/**
 * 唯一允许的裸词元落点：helper 定义文件本身。
 * pickSelect、pickSelectIn、pickV2Remote、pickTreeNode、ownedDropdown、openFieldDropdown、
 * openFieldDropdownByLabel、openScopedDropdown、dropdownItems、expectDropdownClosed 同族 helper
 * 集中定义于此，spec 与 fixture 只允许通过导入调用，不得再各自写裸面板类名。
 */
const WHITELIST = new Set(['frontend/e2e/flow/ui-helpers.ts']);

/** 词元：`.el-select-dropdown` 起头的类名族（含 __item / :visible / .is-loading 等组合形态）。 */
const TOKEN_RE = /\.el-select-dropdown/g;

const relKey = (f) => path.relative(REPO, f).replace(/\\/g, '/');

function walk(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === 'node_modules' || e.name === 'target' || e.name.startsWith('.')) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (/\.ts$/.test(e.name)) out.push(p);
  }
  return out;
}

/**
 * 扫单个 TS 源，返回"注释之外真实出现的裸词元"列表，每项带 文件、行号 与 原文行。
 *
 * 注释剥离口径：
 *  - 行注释（斜杠斜杠起头）之后的内容不计；同 URL 中的 `://` 会被误当行注释剪掉，
 *    但目标词元不会出现在 URL 里，误剪只会造成漏判而非误判，宁可保守。
 *  - 块注释（斜杠星起、星斜杠闭）内部一律不计，跨行状态跟踪；块注释起始行的前半段
 *    与结束行的后半段仍参与扫描。
 *  - JSDoc 续行（首非空字符为星号）随块注释状态一同跳过，不单独特判。
 */
export function scanSource(rel, source) {
  const hits = [];
  const lines = source.split(/\r?\n/);
  let inBlock = false;
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    let line = raw;
    if (inBlock) {
      const end = line.indexOf('*/');
      if (end === -1) continue;
      line = line.slice(end + 2);
      inBlock = false;
    }
    // 循环吃掉同一行内的多个 /* ... */ 段与尾部未闭合的 /*。
    for (;;) {
      const bStart = line.indexOf('/*');
      if (bStart === -1) break;
      const bEnd = line.indexOf('*/', bStart + 2);
      if (bEnd === -1) {
        line = line.slice(0, bStart);
        inBlock = true;
        break;
      }
      line = line.slice(0, bStart) + ' ' + line.slice(bEnd + 2);
    }
    const slash = line.indexOf('//');
    if (slash !== -1) line = line.slice(0, slash);
    for (const m of line.matchAll(TOKEN_RE)) {
      hits.push({ file: rel, line: i + 1, text: raw.trim() });
    }
  }
  return hits;
}

/**
 * 扫整个 e2e 目录，跳过白名单（helper 定义文件本身）。
 */
export function scanE2E(e2eDir) {
  const all = [];
  for (const f of walk(e2eDir)) {
    const key = relKey(f);
    if (WHITELIST.has(key)) continue;
    all.push(...scanSource(key, fs.readFileSync(f, 'utf8')));
  }
  return all;
}

/**
 * 把逐条命中折叠为 { file → count } 与 { file → [file:line 原文] }。
 */
function group(hits) {
  const count = {};
  const lines = {};
  for (const h of hits) {
    count[h.file] = (count[h.file] ?? 0) + 1;
    (lines[h.file] ??= []).push(`${h.file}:${h.line}  ${h.text}`);
  }
  return { count, lines };
}

/**
 * 棘轮比较：current 与 baseline 逐文件比，返回 { failures, reduced }。
 * failures 每项含新增点位的文件:行，供 CI 日志定位。
 */
export function diffAgainstBaseline(currentCount, currentLines, baseline) {
  const failures = [];
  for (const [f, n] of Object.entries(currentCount)) {
    const base = baseline.files[f];
    if (base === undefined) {
      const where = currentLines[f].join(' | ');
      failures.push(`新文件出现 ${n} 处裸 .el-select-dropdown：${f} → ${where}`);
    } else if (n > base) {
      const where = currentLines[f].join(' | ');
      failures.push(`${f}: 裸词元由 ${base} 增至 ${n}（+${n - base}） → ${where}`);
    }
  }
  const reduced = Object.entries(baseline.files).filter(
    ([f, base]) => base > 0 && (currentCount[f] ?? 0) < base
  );
  return { failures, reduced };
}

/**
 * 判定器自证（--self-test）：不落仓库文件，只走 scanSource/diffAgainstBaseline 真实代码路径。
 * 正例（代码里的裸词元）必须被抓到；反例（只在行注释或块注释里出现的词元）必须一条不报。
 * 任一断言不符即退出码 1，防止"检测力丢失后输出 0 错"的假绿。
 */
function runSelfTest() {
  const cases = [
    {
      name: '正例：page.locator 里的 .el-select-dropdown 必须命中',
      src: `const d = page.locator('.el-select-dropdown:visible').last();\n`,
      expect: 1,
    },
    {
      name: '正例：.el-select-dropdown__item 与 BEM 后缀同样命中',
      src: `panel.locator('.el-select-dropdown__item').first();\n`,
      expect: 1,
    },
    {
      name: '正例：同一行两处命中（filter 拼接形态）',
      src: `page.locator('.el-select-dropdown').filter({ has: page.locator('.el-select-dropdown__item') });\n`,
      expect: 2,
    },
    {
      name: '反例：整行 // 注释里的词元不得命中（sku-mapping 现状即此形态）',
      src: `//      option 同为 \`.el-select-dropdown__item\`；helper 已按真实渲染类名定位\n`,
      expect: 0,
    },
    {
      name: '反例：/* */ 块注释内部的词元不得命中',
      src: `/* 保留 .el-select-dropdown 的用法说明 */\nconst x = 1;\n`,
      expect: 0,
    },
    {
      name: '反例：跨行 /* ... */ 块内的词元不得命中，块尾之后仍参与扫描',
      src: `/*\n * 旧写法 .el-select-dropdown:visible\n */\nconst y = page.locator('.el-select-dropdown__item');\n`,
      expect: 1,
    },
    {
      name: '正例：代码后跟 // 注释，代码里的词元命中、注释里的不命中',
      src: `page.locator('.el-select-dropdown').first(); // 旧写法 .el-select-dropdown:visible 已弃用\n`,
      expect: 1,
    },
  ];
  let pass = 0;
  let fail = 0;
  console.log('=== self-test：check-e2e-select-ratchet 扫描双向自证 ===');
  for (const c of cases) {
    const got = scanSource('fixture.ts', c.src).length;
    if (got === c.expect) {
      pass++;
      console.log(`  PASS  ${c.name}  [命中 ${got}]`);
    } else {
      fail++;
      console.log(`  FAIL  ${c.name}  -> 期望 ${c.expect}，实际 ${got}`);
    }
  }

  // 棘轮比较自证：基线 1 处 → 现在 3 处必须判红；现在 0 处必须判"基线未下调"。
  const baseline = { total: 1, files: { 'a.spec.ts': 1 } };
  const up = diffAgainstBaseline(
    { 'a.spec.ts': 3 },
    { 'a.spec.ts': ['a.spec.ts:10 x', 'a.spec.ts:20 y', 'a.spec.ts:30 z'] },
    baseline
  );
  const down = diffAgainstBaseline({}, {}, baseline);
  const same = diffAgainstBaseline({ 'a.spec.ts': 1 }, { 'a.spec.ts': ['a.spec.ts:1 x'] }, baseline);

  const ratchetCases = [
    { name: '正例：新增点位判红并点名 文件:行', okFlag: up.failures.length === 1 && /a\.spec\.ts/.test(up.failures[0]) },
    { name: '正例：清零但基线未下调判"请下调"', okFlag: down.reduced.length === 1 && down.failures.length === 0 },
    { name: '反例：与基线等值不判红不判下调', okFlag: same.failures.length === 0 && same.reduced.length === 0 },
  ];
  for (const c of ratchetCases) {
    if (c.okFlag) {
      pass++;
      console.log(`  PASS  ${c.name}`);
    } else {
      fail++;
      console.log(`  FAIL  ${c.name}`);
    }
  }

  console.log(`---- self-test 结论：${pass} 通过 / ${fail} 失败 ----`);
  if (fail) {
    console.error('[e2e-select] ✗ 判定器自证失败：扫描或棘轮比较存在漏报或误报，门禁不可信');
    process.exit(1);
  }
  console.log('[e2e-select] ✅ 扫描与棘轮自证通过（正例必抓、反例必不报）');
  process.exit(0);
}

function main() {
  const hits = scanE2E(E2E_DIR);
  const { count, lines } = group(hits);
  const total = hits.length;
  const fileCount = Object.keys(count).length;

  if (process.argv.includes('--write-baseline')) {
    fs.writeFileSync(
      BASELINE,
      JSON.stringify(
        {
          note: 'e2e 内 .el-select-dropdown 裸面板词元棘轮基线：spec/fixture 只允许通过 ui-helpers 的 helper 定位下拉面板，唯一保留点在 helper 定义文件本身（frontend/e2e/flow/ui-helpers.ts，本基线不含它）。只准降不准升；清零文件必须同步下调基线。重生成：node scripts/check-e2e-select-ratchet.mjs --write-baseline',
          total,
          files: count,
        },
        null,
        2
      ) + '\n'
    );
    console.log(
      `[e2e-select] 基线已写入：${total} 处裸词元分布于 ${fileCount} 个文件（白名单 ${WHITELIST.size} 个：${[...WHITELIST].join(', ')}）`
    );
    process.exit(0);
  }

  if (!fs.existsSync(BASELINE)) {
    console.error(
      '[e2e-select] ✗ 缺基线文件 scripts/e2e-select-baseline.json（先跑 --write-baseline）'
    );
    process.exit(1);
  }
  const baseline = JSON.parse(fs.readFileSync(BASELINE, 'utf8'));
  const { failures, reduced } = diffAgainstBaseline(count, lines, baseline);

  console.log(
    `[e2e-select] 白名单外扫到裸 .el-select-dropdown ${total} 处 / 基线 ${baseline.total} 处 / 文件 ${fileCount}`
  );
  for (const [f, base] of reduced) {
    console.log(`  已收敛 ${f}: ${base} → ${count[f] ?? 0}（请连同基线一起下调）`);
  }
  if (failures.length) {
    console.error(`\n[e2e-select] ✗ ${failures.length} 个文件出现新的裸面板词元：`);
    for (const m of failures) console.error(`  ${m}`);
    console.error('\n口径：spec/fixture 里的下拉面板一律走 ui-helpers 的 helper（pickSelectIn、pickSelect、pickV2Remote、pickTreeNode、openFieldDropdownByLabel、openScopedDropdown、dropdownItems、expectDropdownClosed 之一），不得再写 .el-select-dropdown 裸类名。');
    console.error('要么迁到 helper，要么由 helper 侧补出你需要的定位形态；不允许在 spec 内留裸面板。');
    process.exit(1);
  }
  if (reduced.length) {
    console.error(
      '\n[e2e-select] ✗ 有文件已收敛但基线未下调，请跑 node scripts/check-e2e-select-ratchet.mjs --write-baseline'
    );
    process.exit(1);
  }
  console.log('[e2e-select] ✅ 无新增裸面板词元');
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
