#!/usr/bin/env node
/**
 * 迁移/内联 SQL 里 PL/pgSQL 块的静态配平检查（#260 同族：门禁测不到的那类红）
 *
 * 为什么必须有这条：`DO $$ … $$;` 的内容是**文本形态的 PL/pgSQL**，rustfmt 只保证 Rust
 * 能解析、`cargo check` 不校验 SQL 语义，本机又没有 PostgreSQL 可跑迁移。实案：m0076 的
 * 只读点名块漏写 `END IF;`，CI 里 PG 报 `42601 syntax error at end of input`（pl_scanner.c
 * /plpgsql_yyerror），迁移链当场打断 ⇒ 后续表全部缺失（`permission_change_audits`、
 * `users.totp_recovery_codes` 不存在），Setup/flow×20/smoke×5/traversal×5/extras×8/
 * Rust 测试×10/真实性门禁/收尾清理共 60 个 job 齐红——**一个语法错伪装成 60 个缺陷**。
 *
 * 判据（每个 PL/pgSQL 块逐条核，任一不满足即 exit 1）：
 *  1. 语句 `IF … THEN` 的数量 == `END IF;` 的数量。只数"需要 END IF"的 IF：从 IF 向后
 *     在遇到 `;` 之前能看到 `THEN` 才是语句 IF；`ADD COLUMN IF NOT EXISTS "x"`、
 *     `DROP CONSTRAINT IF EXISTS` 这类 DDL 守卫不配 END IF，误计会把全仓既有迁移打成
 *     假红（首版就因此误报 111/117，据此收紧）。
 *  2. 块体最后一个非空 token 必须是 `END`（顶层块收尾），否则视为块被截断/漏写。
 *  3. `RAISE [EXCEPTION|NOTICE|…] '文案'` 里 `%` 占位符的个数必须等于其后顶层逗号分隔的
 *     实参个数（`%%` 是转义的百分号、不算占位符；`USING …` 子句不带实参）。这是同一条
 *     42601 族的第二个入口：实案 m0075:161 文案里一个 `%` 都没写却传了 `col_count`，
 *     PG 在**编译 DO 块时**就报 `too many parameters specified for RAISE`，迁移链在 v15 域
 *     中断 ⇒ 40 个 E2E + 11 个 Rust 测试 job 连带红（CI run #4674 实测原文）。占位符多于
 *     实参同理致命（PG 报 too few parameters），故按"不等即红"双向判定。
 * 字符串字面量与 `--`、`/* *​/` 注释先剔除，避免把文案里的 "IF" 当关键字。
 *
 * 反空操作：扫不到任何 `$$` 块时直接失败退出——目录指错/文件改名不能让本检查"零异常"
 * 地被当成通过。RAISE 语句同理：块里一条 RAISE 都没解析到但源码里出现了 RAISE 字样，
 * 说明解析形态已变，同样拒绝把"没查出问题"当通过。
 */
import { readFileSync, readdirSync, statSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const FRONTEND = resolve(__dirname, '..');
export const BACKEND = resolve(FRONTEND, '..', 'backend');

export function collectRs(dir) {
  const out = [];
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const e of entries) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === 'target' || e.name === 'node_modules') continue;
      out.push(...collectRs(p));
    } else if (e.name.endsWith('.rs')) out.push(p);
  }
  return out;
}

const stripNoise = body =>
  body
    .replace(/'(?:[^']|'')*'/g, "''") // 字符串字面量
    .replace(/--[^\n]*/g, '') // 行注释
    .replace(/\/\*[\s\S]*?\*\//g, ''); // 块注释

/** 语句 IF 个数：IF 之后、下一个 ';' 之前出现 THEN。DDL 的 IF [NOT] EXISTS 不计。 */
export function stmtIfCount(body) {
  let n = 0;
  const re = /\bIF\b/g;
  let m;
  while ((m = re.exec(body))) {
    const from = m.index + m[0].length;
    const semi = body.indexOf(';', from);
    const seg = body.slice(from, semi >= 0 ? semi : body.length);
    if (/\bTHEN\b/.test(seg)) n++;
  }
  return n;
}

/** 从 src[pos]（必须指向左单引号）读一个 SQL 字符串字面量，`''` 为转义。 */
function readSqlString(src, pos) {
  let i = pos + 1;
  let value = '';
  while (i < src.length) {
    const c = src[i];
    if (c === "'") {
      if (src[i + 1] === "'") {
        value += "''";
        i += 2;
        continue;
      }
      return { value, end: i + 1 };
    }
    value += c;
    i++;
  }
  return null; // 未闭合
}

/** 文案中真正的占位符数：单个 % 计一个，%% 是转义的百分号不计。 */
export function raisePlaceholderCount(literal) {
  let n = 0;
  for (let i = 0; i < literal.length; i++) {
    if (literal[i] !== '%') continue;
    if (literal[i + 1] === '%') i++;
    else n++;
  }
  return n;
}

/** 顶层逗号分隔的实参个数（忽略括号与字符串内部的逗号）；空串返回 0。 */
function countTopLevelArgs(exprs) {
  const text = exprs.trim();
  if (!text) return 0;
  let n = 1;
  let depth = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === "'") {
      const lit = readSqlString(text, i);
      if (!lit) return -1;
      i = lit.end - 1;
      continue;
    }
    if (c === '(' || c === '[') depth++;
    else if (c === ')' || c === ']') depth--;
    else if (c === ',' && depth === 0) n++;
  }
  return depth === 0 ? n : -1;
}

const RAISE_LEVELS = /^(EXCEPTION|WARNING|NOTICE|INFO|LOG)\b/i;

/**
 * 只剥注释、**保留字符串字面量**（与 stripNoise 相反）：RAISE 的占位符就在文案里，
 * 用 stripNoise 会把 `%` 一起抹掉。注释用等长空格替换，保证输出与输入**逐字符同长**，
 * 报错行号才能直接映射回原文件。
 */
export function stripCommentsKeepStrings(raw) {
  let out = '';
  let i = 0;
  while (i < raw.length) {
    const c = raw[i];
    if (c === "'") {
      const lit = readSqlString(raw, i);
      if (!lit) {
        out += raw.slice(i);
        break;
      }
      out += raw.slice(i, lit.end);
      i = lit.end;
      continue;
    }
    if (c === '-' && raw[i + 1] === '-') {
      const nl = raw.indexOf('\n', i);
      const end = nl < 0 ? raw.length : nl;
      out += ' '.repeat(end - i);
      i = end;
      continue;
    }
    if (c === '/' && raw[i + 1] === '*') {
      const close = raw.indexOf('*/', i + 2);
      const end = close < 0 ? raw.length : close + 2;
      out += raw.slice(i, end).replace(/[^\n]/g, ' ');
      i = end;
      continue;
    }
    out += c;
    i++;
  }
  return out;
}

/**
 * 逐条核 `RAISE '文案', 实参…`：占位符数必须等于实参数（PG 在编译块时校验，
 * 不匹配即 42601 打断整条迁移链）。返回 {errors, parsed, skipped}，
 * skipped 用于暴露"解析不到文案"的形态变化，不让它静默通过。
 *
 * 扫描器**逐字符**走而不是全局正则找 "RAISE"：文案里合法地会出现 "RAISE" 字样（例：
 * m0077 的点名文案正文写着"m0078 遇其残留将 RAISE EXCEPTION 拒绝收口"），正则会把它
 * 当成一条语句去解析实参，得出假红。遇到字符串字面量整体跳过，只在字面量之外认 RAISE。
 */
export function checkRaiseArity(raw) {
  const errors = [];
  let parsed = 0;
  let skipped = 0;
  const t = stripCommentsKeepStrings(raw);
  const n = t.length;
  let i = 0;
  while (i < n) {
    const c = t[i];
    if (c === "'") {
      const lit = readSqlString(t, i);
      if (!lit) break;
      i = lit.end;
      continue;
    }
    if (c === 'R' && /\bRAISE\b/.test(t.slice(i, i + 6)) && (i === 0 || /\W/.test(t[i - 1]))) {
      i += 5;
      while (i < n && /\s/.test(t[i])) i++;
      const lvl = RAISE_LEVELS.exec(t.slice(i));
      if (lvl) {
        i += lvl[0].length;
        while (i < n && /\s/.test(t[i])) i++;
      }
      if (t[i] === ';') {
        i++;
        continue; // 裸 RAISE; = 重抛，不带文案
      }
      if (t[i] !== "'") {
        skipped++;
        const semi = t.indexOf(';', i);
        i = semi < 0 ? n : semi + 1;
        continue;
      }
      const lit = readSqlString(t, i);
      if (!lit) {
        skipped++;
        break;
      }
      parsed++;
      // 语句终止符：跳过实参里的字符串字面量，只认顶层 ';'
      let j = lit.end;
      let stmtEnd = -1;
      while (j < n) {
        if (t[j] === "'") {
          const inner = readSqlString(t, j);
          if (!inner) break;
          j = inner.end;
          continue;
        }
        if (t[j] === ';') {
          stmtEnd = j;
          break;
        }
        j++;
      }
      const rest = t.slice(lit.end, stmtEnd >= 0 ? stmtEnd : n);
      if (/^\s*USING\b/i.test(rest)) {
        checkOne(raw, i, lit.value, 0, errors);
        i = stmtEnd < 0 ? n : stmtEnd + 1;
        continue;
      }
      // 实参段以分隔逗号开头（PG 语法：'文案', a, b）；先摘掉这个逗号再数顶层逗号，
      // 否则每个单实参语句都会被多数 1 个（首版即因此把全仓迁移打成假红）。
      const trimmed = rest.trim();
      let args;
      if (!trimmed) args = 0;
      else if (trimmed.startsWith(',')) args = countTopLevelArgs(trimmed.slice(1));
      else args = -1; // 既不是 USING 也不是逗号分隔，形态不认识 ⇒ 不猜
      if (args < 0) {
        skipped++;
      } else {
        checkOne(raw, i, lit.value, args, errors);
      }
      i = stmtEnd < 0 ? n : stmtEnd + 1;
      continue;
    }
    i++;
  }
  return { errors, parsed, skipped };
}

function checkOne(raw, at, literal, args, errors) {
  const pct = raisePlaceholderCount(literal);
  if (pct === args) return;
  const line = raw.slice(0, at).split('\n').length;
  const direction =
    args > pct
      ? `实参 ${args} 个多于占位符 ${pct} 个（PG: too many parameters specified for RAISE）`
      : `占位符 ${pct} 个多于实参 ${args} 个（PG: too few parameters specified for RAISE）`;
  errors.push(`第${line}行 RAISE ${direction}：${JSON.stringify(literal.slice(0, 60))}`);
}

export function checkPlpgsqlBlocks(files) {
  const findings = [];
  let blocks = 0;
  let raises = 0;
  let raiseSkipped = 0;
  for (const file of files) {
    const src = readFileSync(file, 'utf-8');
    if (!src.includes('$$')) continue;
    const re = /\$\$([\s\S]*?)\$\$/g;
    let m;
    let idx = 0;
    while ((m = re.exec(src))) {
      idx++;
      const raw = m[1];
      if (!/\bBEGIN\b/.test(raw)) continue; // 非 PL/pgSQL 的 dollar 引号（索引表达式等）
      blocks++;
      const body = stripNoise(raw);
      const need = stmtIfCount(body);
      const have = (body.match(/\bEND\s+IF\s*;/g) || []).length;
      const tail = body.trimEnd().replace(/;+$/, '').trimEnd();
      const errs = [];
      if (need !== have)
        errs.push(
          `语句 IF/THEN=${need} 与 END IF;=${have} 不配对（缺 END IF 会让整条迁移 42601 中止）`
        );
      if (!tail.endsWith('END'))
        errs.push(`块体结尾不是 END（实际尾部 ${JSON.stringify(tail.slice(-28))}）`);
      const r = checkRaiseArity(raw);
      raises += r.parsed;
      raiseSkipped += r.skipped;
      errs.push(...r.errors);
      if (errs.length) {
        findings.push({
          file,
          line: src.slice(0, m.index).split('\n').length,
          no: idx,
          errs,
        });
      }
    }
  }
  return { blocks, findings, raises, raiseSkipped };
}

/**
 * 判定器自证：本检查若因为解析形态变化而什么都不判，输出同样是"0 异常"，与真通过无法区分。
 * 故跑三组内联夹具（1 正例 + 2 反例），要求正例零发现、反例必被抓到，任一不符即拒绝扫库。
 * 夹具全部走真实代码路径（checkRaiseArity/stripCommentsKeepStrings），不是另写一套断言。
 */
export function selfProof() {
  const cases = [
    {
      name: '正例：一个占位符配一个实参',
      block: `BEGIN
    RAISE EXCEPTION '仅 % 列存在，中止。', col_count;
END`,
      expectFindings: 0,
    },
    {
      name: '正例：%% 是转义百分号不算占位符',
      block: `BEGIN
    RAISE NOTICE '完成度 100%%，共 % 行。', total_rows;
END`,
      expectFindings: 0,
    },
    {
      name: '正例：USING 子句不带实参',
      block: `BEGIN
    RAISE EXCEPTION '拒绝收口。' USING HINT = '先按实核实 %, 再重跑', col_count;
END`,
      expectFindings: 0,
    },
    {
      name: '正例：三占位符配三实参（跨行）',
      block: `BEGIN
    RAISE EXCEPTION '原值 ''%'' 行数 % → 口径 %',
        rec.token, rec.cnt, rec.target;
END`,
      expectFindings: 0,
    },
    {
      name: '反例：#4674 原样（文案无占位符却传了实参）',
      block: `BEGIN
    RAISE EXCEPTION '三列 data_type 非 numeric/decimal（期望三列均为 numeric），中止。', col_count;
END`,
      expectFindings: 1,
    },
    {
      name: '反例：占位符多于实参',
      block: `BEGIN
    RAISE NOTICE 'A=% B=%', v_a;
END`,
      expectFindings: 1,
    },
    {
      name: '反例：文案里写着 RAISE 字样（扫描器必须不被它骗）',
      block: `BEGIN
    RAISE NOTICE '未知脏值 ''%'' 行数 % —— 后续迁移遇其残留将 RAISE EXCEPTION 拒绝收口', rec.token, rec.cnt;
END`,
      expectFindings: 0,
    },
  ];
  const bad = [];
  for (const c of cases) {
    const r = checkRaiseArity(c.block);
    if (r.errors.length !== c.expectFindings || r.skipped !== 0 || r.parsed < 1) {
      bad.push(
        `${c.name}：期望发现 ${c.expectFindings} 条，实际 ${r.errors.length} 条（parsed=${r.parsed} skipped=${r.skipped}）`
      );
    }
  }
  return bad;
}

function main() {
  const bad = selfProof();
  if (bad.length) {
    console.error('FAIL: RAISE 配平判定器自证未过——本检查不可信，拒绝扫库：');
    for (const b of bad) console.error(`  - ${b}`);
    process.exit(1);
  }
  const roots = [join(BACKEND, 'migration', 'src'), join(BACKEND, 'src'), join(BACKEND, 'tests')];
  const files = roots
    .filter(d => statSync(d, { throwIfNoEntry: false })?.isDirectory())
    .flatMap(collectRs);
  const { blocks, findings, raises, raiseSkipped } = checkPlpgsqlBlocks(files);
  console.log('=== check-migration-sql: PL/pgSQL 块配平 + RAISE 占位符配平 ===');
  console.log(
    `扫描 .rs ${files.length} 个（含 $$ 的文件已筛），PL/pgSQL 块 ${blocks} 个，RAISE 文案语句 ${raises} 条`
  );
  if (!blocks) {
    console.error('FAIL: 一个 $$ 块都没扫到——目录指错/迁移改名，拒绝把"空集"当通过');
    process.exit(1);
  }
  if (!raises) {
    console.error('FAIL: 块里一条 RAISE 文案都没解析到——解析形态已变，拒绝把"没查出问题"当通过');
    process.exit(1);
  }
  if (raiseSkipped) {
    console.error(
      `FAIL: ${raiseSkipped} 条 RAISE 取不到可静态判定的文案/实参（非常见写法），本检查无法核对，请显式改写为 '文案', 实参 形态`
    );
    process.exit(1);
  }
  if (findings.length) {
    console.error(
      `\n[迁移 SQL 语法] ${findings.length} 个块不配平 —— CI 迁移会 42601 打断整条链：`
    );
    for (const f of findings) {
      console.error(`  - ${f.file}:${f.line} 第${f.no}个 DO 块`);
      for (const e of f.errs) console.error(`      ${e}`);
    }
    process.exit(1);
  }
  console.log(
    `\nOK: ${blocks} 个 PL/pgSQL 块 IF/END IF、END 收尾与 ${raises} 条 RAISE 占位符全部配平。`
  );
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) main();
