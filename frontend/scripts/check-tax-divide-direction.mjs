#!/usr/bin/env node
/**
 * 价税分离方向锁的本地镜像判定器（静态段），与后端契约锁
 * backend/tests/contract_tax_divide_reconcile_lock_test.rs 的静态部分同一套规则。
 *
 * 为什么需要它，本地验证分工（用户 2026-10-27 改定）不跑 cargo，
 * 契约锁的静态段因此无法在车道里自证；本判定器把同一组禁列规则与同一张扫描清单
 * 在前台跑一遍，供车道本地做格式与编码类轻判与变异自证。
 * 权威判定仍是那枚 Rust 锁（由推送触发的 CI run 兑付），本文件只做镜像复算，
 * 不新增任何判据，也不替代 Rust 锁。
 *
 * 输入，扫描清单不写第二份，逐条从 Rust 锁源码里的 include_str! 对解析得到，
 * Rust 锁删一条清单这里就少扫一条，两侧不可能漂移出第二个覆盖面。
 * 传给谁，本地自查与变异自证（改回乘法必须判红并点名文件与行号）。
 * 用法，node scripts/check-tax-divide-direction.mjs（扫真值），
 * 加 --self-test 则把 Rust 锁自带的禁止形态与合法形态样本喂给本镜像判定器，
 * 漏判或误判都判红（样本同样从 Rust 锁源码解析，不写第二份样本）。
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const LOCK_FILE = 'backend/tests/contract_tax_divide_reconcile_lock_test.rs';
const FORBIDDEN_TIER_LITERALS = ['1.13', '0.13', '1.09', '1.06', '1.055'];

const repoRoot = process.argv[2] && !process.argv[2].startsWith('--') ? process.argv[2] : '.';
const selfTest = process.argv.includes('--self-test');

/** 从 Rust 锁源码解析扫描清单，出参是 { lockPath, absPath } 数组。 */
function readManifest(root) {
  const src = fs.readFileSync(path.join(root, LOCK_FILE), 'utf8').replace(/\r/g, '');
  const flat = src.replace(/\n/g, ' ');
  const re = /\(\s*"([^"]+)"\s*,\s*include_str!\("([^"]+)"\)/g;
  const out = [];
  let m;
  while ((m = re.exec(flat)) !== null) {
    const rel = m[2].replace(/^\.\.\//, '').replace(/^\.\.\//, '');
    // include_str! 的相对基准是 backend/tests，前端条目形如 ../../frontend/... 归一到仓库根
    const absPath = rel.startsWith('frontend/') ? path.join(root, rel) : path.join(root, 'backend', rel);
    out.push({ lockPath: m[1], absPath });
  }
  if (out.length === 0) {
    throw new Error('扫描清单解析为空，Rust 锁形态变更需同步本镜像判定器');
  }
  // 条数自证，Rust 锁里每枚 include_str! 都必须被解析到（漏解析等于镜像覆盖面比权威锁窄）
  const needle = (src.match(/include_str!\(/g) || []).length;
  if (needle !== out.length) {
    throw new Error(`扫描清单解析条数 ${out.length} 与 Rust 锁的 include_str! 出现数 ${needle} 不一致`);
  }
  return out;
}

/** 块注释闭合标记的最早出现处，与 Rust 锁的 block_closer 同一判据。 */
function blockCloser(text) {
  const star = text.indexOf('*/');
  const html = text.indexOf('-->');
  const cands = [];
  if (star >= 0) cands.push({ pos: star, len: 2 });
  if (html >= 0) cands.push({ pos: html, len: 3 });
  if (cands.length === 0) return null;
  cands.sort((a, b) => a.pos - b.pos);
  return cands[0];
}

/** 剥掉一行的注释部分，与 Rust 锁的 code_part_of_line 同一状态机。 */
function codePartOfLine(line, state) {
  let rest = line;
  if (state.inBlock) {
    const closer = blockCloser(rest);
    if (!closer) return '';
    state.inBlock = false;
    rest = rest.slice(closer.pos + closer.len);
  }
  for (;;) {
    const trimmed = rest.replace(/^\s+/, '');
    const indent = rest.length - trimmed.length;
    if (trimmed.startsWith('//')) return '';
    let openLen = null;
    if (trimmed.startsWith('/*')) openLen = 2;
    else if (trimmed.startsWith('<!--')) openLen = 4;
    if (openLen !== null) {
      const closer = blockCloser(rest.slice(indent + openLen));
      if (closer) {
        rest = rest.slice(indent + openLen + closer.pos + closer.len);
        continue;
      }
      state.inBlock = true;
      return '';
    }
    const cands = [
      { pos: rest.indexOf('//') < 0 ? Infinity : rest.indexOf('//'), len: 2, isBlock: false },
      { pos: rest.indexOf('/*') < 0 ? Infinity : rest.indexOf('/*'), len: 2, isBlock: true },
      { pos: rest.indexOf('<!--') < 0 ? Infinity : rest.indexOf('<!--'), len: 4, isBlock: true },
    ];
    let earliest = cands[0];
    for (const c of cands) if (c.pos < earliest.pos) earliest = c;
    if (earliest.pos === Infinity) return rest;
    if (!earliest.isBlock) return rest.slice(0, earliest.pos);
    const closer = blockCloser(rest.slice(earliest.pos + earliest.len));
    if (closer) {
      return (
        rest.slice(0, earliest.pos) + ' ' + rest.slice(earliest.pos + earliest.len + closer.pos + closer.len)
      );
    }
    state.inBlock = true;
    return rest.slice(0, earliest.pos);
  }
}

function codeLines(src) {
  const state = { inBlock: false };
  const out = [];
  src
    .split('\n')
    .forEach((line, idx) => {
      const code = codePartOfLine(line, state);
      if (code.trim() !== '') out.push({ no: idx + 1, code });
    });
  return out;
}

const compact = s => s.replace(/\s+/g, '');
const windowOf = s => [...s].slice(0, 48).join('');

/** 乘号后紧跟括号且括号内是 1 加税率 形态，逐 *( 出现点判定。 */
function mulOnePlusHit(c) {
  let idx = c.indexOf('*(');
  while (idx >= 0) {
    const rest = windowOf(c.slice(idx + 2));
    if (rest.startsWith('1+') || rest.includes('Decimal::ONE+')) return idx;
    idx = c.indexOf('*(', idx + 2);
  }
  return null;
}

/** 乘号括号窗口内税率与正 1 加数反序并存，逐 *( 出现点判定。 */
function mulTaxReverseAddendHit(c) {
  let idx = c.indexOf('*(');
  while (idx >= 0) {
    const rest = windowOf(c.slice(idx + 2));
    const lower = rest.toLowerCase();
    if (lower.includes('tax')) {
      const plusOne =
        rest.includes('+1') &&
        [...rest.matchAll(/\+1/g)].some(m => {
          const after = rest.slice(m.index + 2);
          return after === '' || !/^[0-9]/.test(after);
        });
      if (plusOne || lower.includes('+decimal::one')) return idx;
    }
    idx = c.indexOf('*(', idx + 2);
  }
  return null;
}

/** 乘号后直接跟 tax 开头标识符（含点号接收者与驼峰），逐乘号出现点判定。 */
function mulByTaxIdentifierHit(c) {
  for (let i = 0; i < c.length; i += 1) {
    if (c[i] !== '*') continue;
    let j = i + 1;
    for (;;) {
      const segStart = j;
      while (j < c.length && /[A-Za-z0-9_]/.test(c[j])) j += 1;
      if (j === segStart) break;
      if (c.slice(segStart, j).toLowerCase().startsWith('tax')) return i;
      if (c[j] === '.' && j + 1 < c.length && /[A-Za-z0-9_]/.test(c[j + 1])) {
        j += 1;
        continue;
      }
      break;
    }
  }
  return null;
}

function forwardFormulaHits(c) {
  const hits = [];
  const a = mulOnePlusHit(c);
  if (a !== null) hits.push({ form: '乘 1 加税率 造含税（含限定前缀与空格形态）', at: a });
  const b = mulTaxReverseAddendHit(c);
  if (b !== null) hits.push({ form: '乘括号窗口内税率与正 1 加数反序并存', at: b });
  const d = mulByTaxIdentifierHit(c);
  if (d !== null) hits.push({ form: '乘 tax 开头标识符造税额', at: d });
  return hits;
}

function containsTierLiteral(c, lit) {
  let idx = c.indexOf(lit);
  while (idx >= 0) {
    const beforeOk = idx === 0 || !/[0-9]/.test(c[idx - 1]);
    const afterIdx = idx + lit.length;
    const afterOk = afterIdx >= c.length || !/[0-9]/.test(c[afterIdx]);
    if (beforeOk && afterOk) return true;
    idx = c.indexOf(lit, idx + 1);
  }
  return false;
}

/** 对一枚扫描对象施加逐行与跨行拼串两层判定，出参是违规清单。 */
function scan(lockPath, absPath) {
  const src = fs.readFileSync(absPath, 'utf8').replace(/\r/g, '');
  const lines = codeLines(src);
  const problems = [];
  if (lines.length === 0) {
    problems.push(`${lockPath} 剥注释后没有可扫描代码行，扫描对象被掏空等于锁失效`);
  }
  for (const { no, code } of lines) {
    const c = compact(code);
    for (const hit of forwardFormulaHits(c)) {
      problems.push(`${lockPath} 第 ${no} 行出现「${hit.form}」的正向公式: ${code.trim()}`);
    }
    for (const lit of FORBIDDEN_TIER_LITERALS) {
      if (containsTierLiteral(c, lit)) {
        problems.push(`${lockPath} 第 ${no} 行出现硬编码增值税档位 ${lit}: ${code.trim()}`);
      }
    }
  }
  const joined = lines.map(l => compact(l.code)).join('');
  for (const hit of forwardFormulaHits(joined)) {
    problems.push(`${lockPath} 跨行拼接后成立「${hit.form}」正向公式`);
  }
  return problems;
}

/** 从 Rust 锁源码里解析形态自检样本（forbidden 与 legal 两个字面量数组）。 */
function readJudgeSamples(root) {
  const src = fs.readFileSync(path.join(root, LOCK_FILE), 'utf8').replace(/\r/g, '');
  const grab = name => {
    const start = src.indexOf(`let ${name}: &[&str] = &[`);
    if (start < 0) throw new Error(`Rust 锁缺 let ${name} 样本数组，镜像判定器无法复算形态自检`);
    const end = src.indexOf('];', start);
    const body = src.slice(start, end);
    return [...body.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map(m =>
      m[1].replace(/\\u\{([0-9a-fA-F]+)\}/g, (_, h) => String.fromCodePoint(parseInt(h, 16)))
    );
  };
  return { forbidden: grab('forbidden'), legal: grab('legal') };
}

function runSelfTest(root) {
  const { forbidden, legal } = readJudgeSamples(root);
  const failures = [];
  if (forbidden.length === 0 || legal.length === 0) {
    failures.push(`形态自检样本解析条数为 0（forbidden=${forbidden.length} legal=${legal.length}）`);
  }
  for (const form of forbidden) {
    const c = compact(form);
    const caught =
      forwardFormulaHits(c).length > 0 || FORBIDDEN_TIER_LITERALS.some(lit => containsTierLiteral(c, lit));
    if (!caught) failures.push(`镜像判定器漏判禁止形态 ${form}`);
  }
  for (const form of legal) {
    const c = compact(form);
    const falseHit = forwardFormulaHits(c);
    const tier = FORBIDDEN_TIER_LITERALS.some(lit => containsTierLiteral(c, lit));
    if (falseHit.length > 0 || tier) {
      failures.push(
        `镜像判定器误判合法形态 ${form}（命中 ${falseHit.map(h => h.form).join('、')} 档位命中 ${tier}）`
      );
    }
  }
  const split = 'fn f() {\n    let inclusive = net\n        * (Decimal::ONE + tax_percent / PERCENT);\n}\n';
  const lines = codeLines(split);
  if (!lines.some(l => forwardFormulaHits(compact(l.code)).length === 1)) {
    failures.push('行首乘号的算子续行被注释判定吞掉，跨行遮蔽的假绿盲区复活');
  }
  const doc = '/*\n * 说明 禁止 net * (Decimal::ONE + tax_percent / PERCENT) 这种写法\n */\nlet net = inclusive;\n';
  const docLines = codeLines(doc);
  if (docLines.length !== 1 || docLines.some(l => forwardFormulaHits(compact(l.code)).length > 0)) {
    failures.push('块注释里的说明性公式混进扫描层，注释形态会产误报');
  }
  if (failures.length > 0) {
    for (const f of failures) console.log(`JUDGE_RED ${f}`);
    console.log('_TAX_DIRECTION_JUDGE_SELFTEST_RED_');
    return 1;
  }
  console.log(`JUDGE_SELFTEST forbidden=${forbidden.length} legal=${legal.length}`);
  console.log('_TAX_DIRECTION_JUDGE_SELFTEST_OK_');
  return 0;
}

/**
 * 从 Rust 锁源码解析两枚权威钉列（PINNED_BACKEND_TAX_LEGS 与 PINNED_FRONTEND_TAX_LEGS），
 * 用来与 include_str! 解析出的扫描清单做集合与条数双向核对。
 * 判据不新增基线：钉列本身就是权威锁里的钉列，这里只是把「清单被摘掉一条而钉列还在」
 * 这类两侧同缺同序的假绿形态在本地复算一遍，Rust 锁里对应的逐条核对仍由 CI 兑付。
 */
function readPinnedLegs(root) {
  const src = fs.readFileSync(path.join(root, LOCK_FILE), 'utf8').replace(/\r/g, '');
  const grab = name => {
    const start = src.indexOf(`const ${name}: &[&str] = &[`);
    if (start < 0) throw new Error(`Rust 锁缺 const ${name} 钉列数组，镜像判定器无法复算两侧同序核对`);
    const end = src.indexOf('\n];', start);
    if (end < 0) throw new Error(`Rust 锁的 const ${name} 钉列数组没有闭合，无法解析`);
    return [...src.slice(start, end).matchAll(/"([^"]+)"/g)].map(m => m[1]);
  };
  return [...grab('PINNED_BACKEND_TAX_LEGS'), ...grab('PINNED_FRONTEND_TAX_LEGS')];
}

/** 扫描清单与权威钉列的集合与条数核对，出参是不一致清单（空数组即两侧同集合同条数）。 */
function crossCheckManifest(root, manifest) {
  const pinned = readPinnedLegs(root);
  const scanned = manifest.map(m => m.lockPath);
  const problems = [];
  const pinnedSet = new Set(pinned);
  const scannedSet = new Set(scanned);
  for (const p of pinnedSet) if (!scannedSet.has(p)) problems.push(`权威钉列有 ${p} 但扫描清单里没有`);
  for (const s of scannedSet) if (!pinnedSet.has(s)) problems.push(`扫描清单有 ${s} 但权威钉列里没有`);
  if (pinned.length !== scanned.length) {
    problems.push(`钉列 ${pinned.length} 条与扫描清单 ${scanned.length} 条不等`);
  }
  return problems;
}

function main() {
  const root = path.resolve(repoRoot);
  if (selfTest) return runSelfTest(root);
  const manifest = readManifest(root);
  const problems = crossCheckManifest(root, manifest);
  for (const { lockPath, absPath } of manifest) {
    if (!fs.existsSync(absPath)) {
      problems.push(`${lockPath} 在 Rust 锁清单里但磁盘上不存在: ${absPath}`);
      continue;
    }
    problems.push(...scan(lockPath, absPath));
  }
  console.log(`SCAN_FILES=${manifest.length}`);
  if (problems.length > 0) {
    for (const p of problems) console.log(`TAX_DIRECTION_RED ${p}`);
    console.log('_TAX_DIRECTION_RED_');
    return 1;
  }
  console.log('_TAX_DIRECTION_OK_');
  return 0;
}

process.exit(main());
