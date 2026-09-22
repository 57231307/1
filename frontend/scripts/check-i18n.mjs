#!/usr/bin/env node
/**
 * i18n key 完整性检查（CI 静态门）
 *
 * 校验三件事，任一不通过即 exit 1：
 * 1. 代码引用的 key 在 zh-CN / en-US 中是否都存在且指向文案字符串
 *    —— zh-CN 缺失会把界面渲染成 `apModule.paymentRequest.requestNo` 这类原始 key；
 *       en-US 缺失由 fallbackLocale 静默回退中文。两者都不抛异常，运行时零信号。
 * 2. 语言包内是否存在重复 key（后者覆盖前者，前一份文案变成永不生效的死文案）
 * 3. 每条文案是否能被 vue-i18n 的消息编译器编译
 *    —— `@` 是 linked message 语法、`{ }` 是插值语法，字面量里直接写 `!@#$%`、`{"k":1}`
 *       这类内容会让编译器报错；生产构建下抛出的就是 `SyntaxError`，message 只有错误码
 *       （如 10 = INVALID_LINKED_FORMAT），压缩栈里连是哪条文案都看不到。
 *       编译期才暴露的问题必须在这里挡住，不能等页面渲染时崩。
 *
 * 用 TypeScript AST 而非 `new Function` 求值：eval 会直接丢掉了重复 key 的旧值，
 * 无法报告覆盖；AST 保留全部属性节点，两种问题都能查出。
 *
 * 自检：若出现本检查无法表达的结构（计算键、展开、方法简写、非字符串值），
 * 属性计数会对不上并直接报错退出，避免"解析不到"被误当成"不存在问题"。
 * 模板串/拼接的文案取不到字面值，只统计条数并打印，不参与编译校验。
 *
 * 历史缺陷：旧版用 `[$\s.(]t\(` 抓 key，漏掉 `:label="t('a.b')"` 这类引号后调用
 * （旧版抓到 6018 个 key，收紧后 9636 个，多出 3618 个从未校验过），且只比对 zh-CN。
 * 收紧后暴露 15 个中文界面原始 key 与 117 个英文界面回退中文，已在同批次清零。
 * 另一起：`/security/change-password` 渲染期抛 `SyntaxError {message:10}`，根因是
 * `security.changePassword.tips.special` 文案里的 `!@#$%` 被当作 linked message 解析；
 * 当时本脚本只做 1、2 两项，故完全无信号——据此补上第 3 项。
 */
import { readFileSync, readdirSync } from 'fs';
import { join, dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import ts from 'typescript';
// vue-i18n 运行时用的就是这一个编译器，用它校验即与线上行为一致
import { baseCompile } from '@intlify/message-compiler';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FRONTEND = resolve(__dirname, '..');
const LOCALES = join(FRONTEND, 'src', 'locales');
const NAMES = ['zh-CN', 'en-US'];

/* ---------- 语言包解析 ---------- */

function parseLocale(name) {
  const file = join(LOCALES, name + '.ts');
  const sf = ts.createSourceFile(
    file,
    readFileSync(file, 'utf-8'),
    ts.ScriptTarget.ESNext,
    true,
    ts.ScriptKind.TS
  );
  const values = new Map(); // 点分路径 -> 文案
  const groups = new Set(); // 指向对象的点分路径
  const literals = []; // 字面量文案 {path, text, line}，逐条送消息编译器
  const dynamic = []; // 模板串/拼接文案，取不到字面值，只计数
  const dups = [];
  let props = 0; // 已识别属性数

  const visit = (node, prefix) => {
    const seen = new Map();
    for (const prop of node.properties) {
      if (ts.isSpreadAssignment(prop) || ts.isShorthandPropertyAssignment(prop)) {
        throw new Error(
          `${name}.ts 第 ${lineOf(sf, prop)} 行为展开/简写，静态检查无法覆盖，需人工核对`
        );
      }
      if (!ts.isPropertyAssignment(prop)) continue;
      const key = literalKey(prop);
      if (key === null) {
        throw new Error(
          `${name}.ts 第 ${lineOf(sf, prop)} 行为计算键或非字面量键，静态检查无法覆盖`
        );
      }
      props++;
      const path = prefix ? prefix + '.' + key : key;
      if (seen.has(key)) dups.push({ path, first: seen.get(key), line: lineOf(sf, prop) });
      else seen.set(key, lineOf(sf, prop));

      const init = prop.initializer;
      if (ts.isObjectLiteralExpression(init)) {
        groups.add(path);
        visit(init, path);
      } else if (ts.isStringLiteral(init) || ts.isNoSubstitutionTemplateLiteral(init)) {
        values.set(path, init.text);
        literals.push({ path, text: init.text, line: lineOf(sf, init) });
      } else if (ts.isTemplateExpression(init) || ts.isBinaryExpression(init)) {
        // 拼接文案：存在性可判定，字面值不比对，也就无法编译校验
        values.set(path, null);
        dynamic.push({ path, line: lineOf(sf, init) });
      } else {
        throw new Error(
          `${name}.ts 第 ${lineOf(sf, prop)} 行 ${path} 的值不是字符串字面量（${ts.SyntaxKind[init.kind]}），` +
            '静态检查无法确认文案存在性'
        );
      }
    }
  };
  visit(findRootObject(sf), '');

  return { values, groups, literals, dynamic, dups, props };
}

function literalKey(prop) {
  const n = prop.name;
  if (!n) return null;
  if (ts.isIdentifier(n) || ts.isStringLiteral(n) || ts.isNumericLiteral(n)) return n.text;
  return null;
}
function findRootObject(sf) {
  let root = null;
  (function walk(n) {
    if (root) return;
    if (ts.isExportAssignment(n) && ts.isObjectLiteralExpression(n.expression)) root = n.expression;
    else ts.forEachChild(n, walk);
  })(sf);
  if (!root) throw new Error('未找到 `export default { ... }`');
  return root;
}
function lineOf(sf, node) {
  return sf.getLineAndCharacterOfPosition(node.getStart(sf)).line + 1;
}

/* ---------- 代码侧 key 抓取 ---------- */

/**
 * t('a.b') / $t('a.b') / msg.translate('a.b')
 * 前置断言用「非标识符字符」，覆盖 `:label="t('a.b')"`、`{{ t('a.b') }}`、`|| t('a.b')` 等
 * 旧版的 `[$\s.(]` 要求前置换行/空格/点/括号，导致模板属性里的调用整体漏检。
 */
const REF_PATTERNS = [
  { re: /(?:^|[^\w$])\$?t\(\s*(['"])([^'"]+)\1/g, prefix: '' },
  { re: /(?:^|[^\w$])msg\.translate\(\s*(['"])([^'"]+)\1/g, prefix: 'message.' },
  // 模板串调用：含 ${} 的为动态 key，只能统计不能判定；无 ${} 的按字面 key 校验
  { re: /(?:^|[^\w$])\$?t\(\s*`([^`]*)`/g, prefix: '' },
];
const isDynamic = k => k.includes('$') || k.endsWith('.');

function collectRefs(dir, out, dynamic) {
  for (const f of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, f.name);
    if (f.isDirectory()) {
      if (f.name === 'node_modules' || f.name === 'dist') continue;
      collectRefs(p, out, dynamic);
      continue;
    }
    if (!/\.(vue|ts)$/.test(f.name) || /[/\\]locales[/\\]/.test(p)) continue;
    const src = readFileSync(p, 'utf-8');
    const rel = p.slice(FRONTEND.length + 1);
    for (const { re, prefix } of REF_PATTERNS) {
      for (const m of src.matchAll(re)) {
        const key = prefix + m[m.length - 1];
        if (isDynamic(key)) dynamic.add(rel + '  ' + key);
        else out.push({ key, where: rel });
      }
    }
  }
}

/* ---------- 主流程 ---------- */

const locales = new Map();
for (const name of NAMES) locales.set(name, parseLocale(name));

const refs = [];
const dynamicRefs = new Set();
collectRefs(join(FRONTEND, 'src'), refs, dynamicRefs);
const firstRef = new Map();
for (const r of refs) if (!firstRef.has(r.key)) firstRef.set(r.key, r.where);

const violations = [];
for (const [key, where] of firstRef) {
  for (const name of NAMES) {
    const { values, groups } = locales.get(name);
    if (values.has(key)) continue;
    violations.push(
      groups.has(key)
        ? `${name}: ${key} 指向分组而非文案，界面会显示原始 key  首次引用 ${where}`
        : `${name}: 缺失 key ${key}  首次引用 ${where}`
    );
  }
}
for (const name of NAMES) {
  for (const d of locales.get(name).dups) {
    violations.push(`${name}.ts: 重复 key ${d.path}，第 ${d.first} 行被第 ${d.line} 行覆盖`);
  }
}

/* 文案语法：逐条字面量文案过一遍消息编译器，编译期报错的一律拦下 */
let compiled = 0;
for (const name of NAMES) {
  for (const { path, text, line } of locales.get(name).literals) {
    compiled++;
    const errors = [];
    try {
      baseCompile(text, { onError: err => errors.push(...[err].flat()) });
    } catch (err) {
      errors.push(err); // 默认 onError 直接抛 SyntaxError，与收集到的错误一并报告
    }
    for (const err of errors) {
      violations.push(
        `${name}.ts:${line} ${path} 文案编译失败 code=${err.code ?? '?'} ${err.message ?? ''}` +
          `（生产构建下界面会抛 SyntaxError: ${err.code ?? '?'}）原文: ${JSON.stringify(text)}`
      );
    }
  }
}

/* 兜底自检：文案与重复项之和不应超过已识别属性数，否则解析器统计有缺陷 */
for (const name of NAMES) {
  const l = locales.get(name);
  const counted = l.values.size + l.dups.length;
  if (counted > l.props) {
    throw new Error(`${name}.ts 统计异常：计入 ${counted} > 属性 ${l.props}，解析器存在缺陷`);
  }
}

/* 存量挂账：修复一条删一条，禁止新增 */
const ALLOWLIST = new Set();

const blocked = violations.filter(v => !ALLOWLIST.has(v));
const uncompilable = NAMES.reduce((n, name) => n + locales.get(name).dynamic.length, 0);
console.log(
  `[i18n] 引用字面 key ${firstRef.size} 个（动态拼接 ${dynamicRefs.size} 处不判定），` +
    `zh-CN 文案 ${locales.get('zh-CN').values.size}，en-US 文案 ${locales.get('en-US').values.size}，` +
    `消息编译 ${compiled} 条（模板串/拼接 ${uncompilable} 条取不到字面值，不编译），` +
    `问题 ${violations.length}，挂账 ${violations.length - blocked.length}`
);
if (blocked.length) {
  console.error(
    `[i18n] ❌ 未通过（${blocked.length} 项）:\n${blocked.map(v => '  - ' + v).join('\n')}`
  );
  process.exit(1);
}
console.log('[i18n] ✅ 通过');
