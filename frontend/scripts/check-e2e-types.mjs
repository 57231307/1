#!/usr/bin/env node
/**
 * e2e 类型棘轮门禁。
 *
 * 背景：`tsconfig.json` 只含 `src/**`，所以 `vue-tsc` 从不检查 e2e；本仓一度有
 * 6 条真实类型错误（用例按后端根本不存在的 `items` 键断言、`expect(locator).first()`
 * 这类会直接判负或运行期失败的写法）长期无人发现——它们就藏在未接 CI 的 `tsconfig.e2e.json` 里。
 *
 * 口径：
 * - 跑 `tsc --noEmit -p tsconfig.e2e.json`，按 `文件|TS错误码` 计数与基线比对；
 * - 新增键或计数上升 → 失败（贴出原始错误行，便于直接修）；
 * - 计数下降但基线未下调 → 失败（棘轮只准收紧，防止"存量一直挂着"）；
 * - tsc 自身非 0 且拿不到可解析的错误行 → 失败（解析不出即失败，不静默放行）。
 *
 * 存量 93 条属同一根因：未安装 `@types/node`，`process/fs/path/crypto/require` 全部无法解析。
 * 引入 `@types/node` 会连带把此前不判定的表达式纳入检查，需单独 PR 逐个判责，
 * 故此处按 `文件|错误码` 记账，不在本 PR 处理。
 *
 * usage: node scripts/check-e2e-types.mjs [--write-baseline]
 */
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const FE = join(dirname(fileURLToPath(import.meta.url)), '..');
const BASELINE = join(FE, 'scripts', 'e2e-types-baseline.json');
const WRITE = process.argv.includes('--write-baseline');

let stdout = '';
let tscFailed = false;
try {
  // 直接调本地 typescript 的入口脚本：`npx` 在 Windows 上是 .cmd，execFileSync 无法 spawn；
  // 走 node + 包内 bin 在两侧行为一致，且不依赖 PATH 上是否有 npx。
  stdout = execFileSync(
    process.execPath,
    [join(FE, 'node_modules', 'typescript', 'bin', 'tsc'), '--noEmit', '-p', 'tsconfig.e2e.json'],
    { cwd: FE, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }
  );
} catch (err) {
  tscFailed = true;
  stdout = `${err.stdout ?? ''}${err.stderr ?? ''}`;
  // tsc 因任何原因没吐出可解析的错误行时，不能当成"0 错误"放行
  if (!/error TS/.test(stdout)) {
    console.error('[e2e-types] tsc 执行异常且无错误行可解析，判负：');
    console.error(stdout.slice(0, 4000) || String(err));
    process.exit(1);
  }
}

const errors = [];
for (const line of stdout.split(/\r?\n/)) {
  const m = line.match(/^(.+?)\((\d+),(\d+)\): error (TS\d+): (.*)$/);
  if (m) errors.push({ file: m[1].replace(/\\/g, '/'), code: m[4], text: line });
}

const current = {};
for (const e of errors) {
  const k = `${e.file}|${e.code}`;
  current[k] = (current[k] ?? 0) + 1;
}

if (WRITE) {
  writeFileSync(BASELINE, JSON.stringify(current, null, 2) + '\n', 'utf8');
  console.log(`[e2e-types] 基线已写入：${errors.length} 条，分布于 ${Object.keys(current).length} 个 文件|错误码`);
  process.exit(0);
}

if (!existsSync(BASELINE)) {
  console.error('[e2e-types] 缺少基线 scripts/e2e-types-baseline.json，请先跑 --write-baseline');
  process.exit(1);
}
const baseline = JSON.parse(readFileSync(BASELINE, 'utf8'));

const worse = [];
const better = [];
for (const [k, n] of Object.entries(current)) {
  const b = baseline[k] ?? 0;
  if (n > b) worse.push(`${k}: ${b} → ${n}`);
  else if (n < b) better.push(`${k}: ${b} → ${n}`);
}
for (const [k, b] of Object.entries(baseline)) {
  if (!(k in current) && b > 0) better.push(`${k}: ${b} → 0`);
}

const sample = new Map();
for (const e of errors) {
  const k = `${e.file}|${e.code}`;
  if (worse.some(w => w.startsWith(k)) && !sample.has(k)) sample.set(k, e.text);
}

console.log(
  `[e2e-types] e2e 类型错误 ${errors.length} 条（基线 ${Object.values(baseline).reduce((a, b) => a + b, 0)}），` +
    `恶化 ${worse.length}，已收敛 ${better.length}`
);
if (worse.length) {
  console.error('[e2e-types] ❌ 新增/上升的类型错误:');
  for (const w of worse) {
    console.error(`  - ${w}`);
    if (sample.has(w.split(':')[0])) console.error(`      ${sample.get(w.split(':')[0])}`);
  }
  process.exit(1);
}
if (better.length) {
  console.error(
    '[e2e-types] ❌ 有文件已收敛但基线未下调，请跑 node scripts/check-e2e-types.mjs --write-baseline'
  );
  for (const b of better) console.error(`  已收敛 ${b}`);
  process.exit(1);
}
console.log('[e2e-types] ✅ 无新增类型错误');
