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
 * 字符串字面量与 `--`、`/* *​/` 注释先剔除，避免把文案里的 "IF" 当关键字。
 *
 * 反空操作：扫不到任何 `$$` 块时直接失败退出——目录指错/文件改名不能让本检查"零异常"
 * 地被当成通过。
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

export function checkPlpgsqlBlocks(files) {
  const findings = [];
  let blocks = 0;
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
  return { blocks, findings };
}

function main() {
  const roots = [join(BACKEND, 'migration', 'src'), join(BACKEND, 'src'), join(BACKEND, 'tests')];
  const files = roots
    .filter(d => statSync(d, { throwIfNoEntry: false })?.isDirectory())
    .flatMap(collectRs);
  const { blocks, findings } = checkPlpgsqlBlocks(files);
  console.log('=== check-migration-sql: PL/pgSQL 块配平 ===');
  console.log(`扫描 .rs ${files.length} 个（含 $$ 的文件已筛），PL/pgSQL 块 ${blocks} 个`);
  if (!blocks) {
    console.error('FAIL: 一个 $$ 块都没扫到——目录指错/迁移改名，拒绝把"空集"当通过');
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
  console.log(`\nOK: ${blocks} 个 PL/pgSQL 块 IF/END IF 与 END 收尾全部配平。`);
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) main();
