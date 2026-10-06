#!/usr/bin/env node
/**
 * i18n key 完整性检查（CI 静态门）
 *
 * 校验五件事，任一不通过即 exit 1：
 * 1. 代码引用的 key 在 zh-CN / en-US 中是否都存在且指向文案字符串
 *    —— zh-CN 缺失会把界面渲染成 `apModule.paymentRequest.requestNo` 这类原始 key；
 *       en-US 缺失由 fallbackLocale 静默回退中文。两者都不抛异常，运行时零信号。
 * 2. 语言包内是否存在重复 key（后者覆盖前者，前一份文案变成永不生效的死文案）
 * 3. 每条文案是否能被 vue-i18n 的消息编译器编译
 *    —— `@` 是 linked message 语法、`{ }` 是插值语法，字面量里直接写 `!@#$%`、`{"k":1}`
 *       这类内容会让编译器报错；生产构建下抛出的就是 `SyntaxError`，message 只有错误码
 *       （如 10 = INVALID_LINKED_FORMAT），压缩栈里连是哪条文案都看不到。
 *       编译期才暴露的问题必须在这里挡住，不能等页面渲染时崩。
 * 4. 中英两侧的键路径集合是否逐项对称（文案集与分组集各比一次）
 *    —— 单侧多出来的键是不可达文案（读不到它），单侧缺出来的键在另一种语言下静默回退；
 *       两类都不会被第 1 项抓到，因为状态词表映射（`*_STATUS_LABEL_KEYS` 之类）写的是
 *       **完整键路径字面量**、经 `t(opt.labelKey)` 动态消费，不经过 `t('字面')` 调用。
 * 5. 语言包里的键是否仍有代码引用（孤儿键）
 *    —— 删掉采集/上送逻辑时若忘记删文案，界面不会坏、构建不会红，只留下一份永不渲染的死文案，
 *       且它会继续被当成"现存约定"复制扩散。存量孤儿记入 `scripts/i18n-orphans-baseline.json`
 *       做棘轮：只准降不准升，基线必须与当前孤儿集合精确相等（既拦新孤儿，也拦留在基线里的已修项）。
 *
 * 孤儿判定的"救回"口径（宁可漏判不可误判，避免把动态引用的键冤枉成孤儿）：
 *    R1 `REF_PATTERNS` 命中的字面引用；
 *    R1' `msg.success/error/warning/info(…)` 提示族实参里的裸键名（补 `message.` 前缀，只做救回）；
 *    R2 键路径作为**完整 token** 出现在 src / e2e 任一源码里（覆盖常量表里的完整键、拼接的静态段）；
 *    R3 键的某个祖先路径是动态构造前缀（模板串插值 `t 反引号 x.插值 反引号`，或 `t('x.' + y)` 拼接）。
 *    e2e 只贡献 R2（实测其 `t('a.b')` 形态全部出现在注释里，不能当作"代码在用这个键"的依据）。
 *
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
import { readFileSync, readdirSync, existsSync, writeFileSync } from 'fs';
import { join, dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import ts from 'typescript';
// vue-i18n 运行时用的就是这一个编译器，用它校验即与线上行为一致
import { baseCompile } from '@intlify/message-compiler';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FRONTEND = resolve(__dirname, '..');
const LOCALES = join(FRONTEND, 'src', 'locales');
const NAMES = ['zh-CN', 'en-US'];
// 存量孤儿棘轮基线（形态比照 scripts/api-keys-baseline.json）
const BASELINE = join(__dirname, 'i18n-orphans-baseline.json');

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
/**
 * 引用形态表：`t('a.b')` / `$t('a.b')` / `msg.translate('a.b')` / 反引号 `t(`a.b`)` / `labelKey: 'a.b'`。
 * 前置断言用「非标识符字符」，覆盖 `:label="t('a.b')"`、`{{ t('a.b') }}`、`|| t('a.b')` 等
 * 旧版的 `[$\s.(]` 要求前置换行/空格/点/括号，导致模板属性里的调用整体漏检。
 * 提示族（`msg.success('x')`）不在此表内，见下方 collectMessageRefs 与文件头 R1'。
 */
const REF_PATTERNS = [
  { re: /(?:^|[^\w$])\$?t\(\s*(['"])([^'"]+)\1/g, prefix: '' },
  { re: /(?:^|[^\w$])msg\.translate\(\s*(['"])([^'"]+)\1/g, prefix: 'message.' },
  // 模板串调用：含 ${} 的为动态 key，只能统计不能判定；无 ${} 的按字面 key 校验
  { re: /(?:^|[^\w$])\$?t\(\s*`([^`]*)`/g, prefix: '' },
  // 词表模块里的 labelKey 字面量：形如 { value: '色差', labelKey: 'common.returnReason.colorDifference' }。
  // 这类值不经过 t() 调用，只被 t(opt.labelKey) 动态消费，落在"不判定"里，
  // 写错就是界面上直接露出 key 原文，故在此按引用校验（api/production.ts 状态表同形）。
  { re: /(?:^|[^\w$])labelKey:\s*(['"])([^'"]+)\1/g, prefix: '' },
];
const isDynamic = k => k.includes('$') || k.endsWith('.');

/**
 * 提示族的键补全（R1'：只用于孤儿救回，不进第 1 项缺键校验）。
 * `utils/message.ts` 的 success/error/warning/info 入参是**裸键名**，内部统一补 `message.` 前缀
 * （`const fullKey = 'message.' + key`），所以：
 * - 任何文件里的 `msg.success(cond ? 'approvePassed' : 'approveRejected')` 都是两个活引用
 *   —— 三元/变量实参形态用 REF_PATTERNS 的字面量口径根本抓不到，不补就会把活文案判成孤儿；
 * - `utils/message.ts` 自身还有内部直调（`loadFail: e => error('loadFailed', …)`）与默认参数
 *   （`function success(key = 'operationSuccess')`）两种形态，只在该文件内按活引用算，
 *   免得把别处 `error('DB error')` 这类非文案实参误当引用。
 * 为什么不顺手把它并进第 1 项（缺键校验）：按此口径抓会立刻暴露 20 个 zh/en **双侧都缺**的
 * `message.*` 键（`msg.error('issueFailed')` 等，界面当前就把原始 key 弹给用户），那是另一族缺陷、
 * 涉及他人在途文件与文案取舍，本次不改变第 1 项既有判据，已单列成交主编排项。
 */
function collectMessageRefs(rel, src) {
  const insideHelper = /(^|[/\\])src[/\\]utils[/\\]message\.ts$/.test(rel);
  const callRe = insideHelper
    ? /(?:^|[^\w$])(?:msg\.)?(?:success|error|warning|info|translate)\(([^)]*)\)/g
    : /(?:^|[^\w$])msg\.(?:success|error|warning|info|translate)\(([^)]*)\)/g;
  const out = [];
  const push = k => {
    if (!/^[\w.]+$/.test(k)) return;
    out.push(k.startsWith('message.') ? k : 'message.' + k);
  };
  for (const m of src.matchAll(callRe))
    for (const lit of m[1].matchAll(/(['"])([^'"]+)\1/g)) {
      // 跳过比较右值（`x === 'approve' ? 'a' : 'b'` 里的 'approve' 是业务值不是键名）
      if (/[=!]==?\s*$/.test(m[1].slice(0, lit.index))) continue;
      push(lit[2]);
    }
  if (insideHelper) for (const m of src.matchAll(/key\s*=\s*(['"])([^'"]+)\1/g)) push(m[2]);
  return out;
}

/** 动态构造调用里的静态前缀：模板串取 `${` 之前、拼接取结尾点号之前 */
function dynamicPrefixOf(key) {
  const i = key.indexOf('$');
  return (i >= 0 ? key.slice(0, i) : key).replace(/\.+$/, '');
}
/** 源码中的点分 token（至少一个点；贪婪最长匹配自带边界，`x.a.b` 不会误配成键 `a.b`） */
const TOKEN_RE = /[\w$]+(?:\.[\w$]+)+/g;

function walkSourceFiles(dir, fn) {
  if (!existsSync(dir)) return; // e2e 等目录可缺席，不参与引用统计
  for (const f of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, f.name);
    if (f.isDirectory()) {
      if (f.name === 'node_modules' || f.name === 'dist') continue;
      walkSourceFiles(p, fn);
      continue;
    }
    if (!/\.(vue|ts)$/.test(f.name) || /[/\\]locales[/\\]/.test(p)) continue;
    fn(p);
  }
}

/**
 * @param literals 是否把 REF_PATTERNS 命中计入"代码在引用这个键"。
 *   src 传 true（既有口径，同时喂第 1 项缺键校验）；e2e 传 false —— 实测 e2e 里的
 *   `t('a.b')` 全部写在注释里，只能作为"这个键路径还在被提及"的救回证据（R2 token），
 *   不能当作活引用，否则注释里提一个不存在的键就会把第 1 项判红。
 */
function collectRefs(dir, out, dynamic, { tokens, dynPrefixes, msgRefs, literals }) {
  walkSourceFiles(dir, p => {
    const src = readFileSync(p, 'utf-8');
    const rel = p.slice(FRONTEND.length + 1);
    for (const m of src.matchAll(TOKEN_RE)) tokens.add(m[0]);
    if (!literals) return;
    for (const { re, prefix } of REF_PATTERNS) {
      for (const m of src.matchAll(re)) {
        const key = prefix + m[m.length - 1];
        if (isDynamic(key)) {
          dynamic.add(rel + '  ' + key);
          const pre = dynamicPrefixOf(key);
          if (pre) dynPrefixes.add(pre);
        } else out.push({ key, where: rel });
      }
    }
    for (const k of collectMessageRefs(rel, src)) msgRefs.add(k);
  });
}

/* ---------- 锁① 中英键集合对称 ---------- */

/**
 * 逐键比两侧的文案集与分组集。方向各比一次，因此"单侧多出"和"单侧缺失"都会点名，
 * 同一路径一侧是文案另一侧是分组（类型翻转）也会被抓到（两条相反方向的记录）。
 * 返回结构化条目，判红文案由主流程拼。
 */
function findAsymmetry(aName, a, bName, b) {
  const out = [];
  const kinds = [
    ['文案', l => new Set(l.values.keys())],
    ['分组', l => new Set(l.groups)],
  ];
  for (const [pairName, pick] of kinds) {
    const A = pick(a);
    const B = pick(b);
    for (const key of [...A].filter(k => !B.has(k)).sort())
      out.push({ kind: pairName, key, onlyIn: aName, missingIn: bName });
    for (const key of [...B].filter(k => !A.has(k)).sort())
      out.push({ kind: pairName, key, onlyIn: bName, missingIn: aName });
  }
  return out;
}
const asymmetryText = d =>
  `zh/en 不对称：${d.key} 只有 ${d.onlyIn} 有（${d.kind}）` +
  (d.kind === '文案'
    ? `，${d.missingIn} 侧缺该文案：切到该语言时由 fallbackLocale 静默回退或直接露出原始 key`
    : `，${d.missingIn} 侧无该分组：其下所有键在该语言都取不到`);

/* ---------- 锁② 孤儿键 ---------- */

/** 键的任一祖先路径（含自身）是动态构造前缀 ⇒ 该键可能由插值/拼接拼出，不得判孤儿 */
function underDynamicPrefix(key, dynPrefixes) {
  if (dynPrefixes.has(key)) return true;
  for (let i = key.length - 1; i > 0; i--)
    if (key[i] === '.' && dynPrefixes.has(key.slice(0, i))) return true;
  return false;
}

/** locale 有、src 无引用的文案键（救回口径 R1|R2|R3 见文件头） */
function findOrphans(locale, { refKeys, tokens, dynPrefixes }) {
  const out = [];
  for (const key of [...locale.values.keys()].sort()) {
    if (refKeys.has(key) || tokens.has(key)) continue;
    if (underDynamicPrefix(key, dynPrefixes)) continue;
    out.push(key);
  }
  return out;
}

/**
 * 棘轮：孤儿集合必须被基线**精确**覆盖。
 * added 非空 = 新增孤儿（当场红）；stale 非空 = 基线里还挂着已不存在的孤儿（红，要求下调），
 * 二者合起来使"偷偷往基线里塞一项"和"删了文案却留着基线条目"都过不了。
 */
function compareWithBaseline(orphans, baseline) {
  const known = new Set(baseline.keys);
  const cur = new Set(orphans);
  return {
    added: orphans.filter(k => !known.has(k)),
    stale: [...known].filter(k => !cur.has(k)).sort(),
    over: orphans.length > baseline.total,
  };
}

function loadBaseline() {
  const raw = JSON.parse(readFileSync(BASELINE, 'utf-8'));
  const keys = raw.modules ? Object.values(raw.modules).flatMap(m => m.keys) : (raw.keys ?? []);
  return { total: raw.total, keys };
}

async function writeBaseline(orphans) {
  const modules = {};
  for (const k of orphans) {
    const mod = k.split('.')[0];
    (modules[mod] ??= { count: 0, keys: [] }).keys.push(k);
  }
  for (const m of Object.values(modules)) m.count = m.keys.length;
  const body = {
    note:
      'i18n 存量孤儿键棘轮基线：locale 里有文案、src/e2e 里无任何引用（且不受动态拼接与完整 token 救回）。' +
      '只准降不准升；新增孤儿要么删文案，要么确认它确实被动态拼接消费。' +
      '重生成：node scripts/check-i18n.mjs --write-baseline',
    total: orphans.length,
    modules,
  };
  // 用 prettier 落盘，保证重新生成的文件直接过 G1 格式门（短数组会被折叠，手写 stringify 会漂移）
  const prettier = await import('prettier');
  const config = (await prettier.resolveConfig(BASELINE)) ?? {};
  const formatted = await prettier.format(JSON.stringify(body, null, 2), {
    ...config,
    filepath: BASELINE,
    parser: 'json',
  });
  writeFileSync(BASELINE, formatted);
  return orphans.length;
}

/** 检测力地板：解析不到键 / 引用集或 token 集为空 ⇒ 判定器已失效，必须红而不是静默通过 */
function powerFloor({ leafCount, refCount, tokenCount }) {
  const msgs = [];
  if (!leafCount) msgs.push('未从 locale 解析到任何文案键，检查器失效');
  if (!refCount) msgs.push('引用集为空（REF_PATTERNS 或 src 扫描失效），孤儿锁无检测力');
  if (!tokenCount) msgs.push('token 集为空（源码扫描失效），孤儿锁无检测力');
  return msgs;
}

/* ---------- 两条锁的双向自证（内联夹具，不读仓库状态） ---------- */

function runSelfTest() {
  const mk = (entries, groups = []) => ({
    values: new Map(entries),
    groups: new Set(groups),
    literals: [],
    dynamic: [],
    dups: [],
    props: entries.length,
  });
  const orphanOf = (locale, ctx) => findOrphans(locale, ctx).sort();
  const asymOf = (zh, en) =>
    findAsymmetry('zh-CN', zh, 'en-US', en)
      .map(d => `${d.kind}|${d.key}|${d.onlyIn}`)
      .sort();
  const cases = [
    {
      name: '孤儿锁·正例 无引用文案必抓',
      got: orphanOf(
        mk([
          ['a.b', 'x'],
          ['a.c', 'y'],
        ]),
        {
          refKeys: new Set(['a.b']),
          tokens: new Set(['a.b']),
          dynPrefixes: new Set(),
        }
      ),
      want: ['a.c'],
    },
    {
      name: '孤儿锁·反例 全部键都有字面引用时不误伤',
      got: orphanOf(
        mk([
          ['a.b', 'x'],
          ['a.c', 'y'],
        ]),
        {
          refKeys: new Set(['a.b', 'a.c']),
          tokens: new Set(['a.b', 'a.c']),
          dynPrefixes: new Set(),
        }
      ),
      want: [],
    },
    {
      name: '孤儿锁·反例 动态拼接前缀下的键不误伤（t(`x.y.${s}`) 的静态 0 命中）',
      got: orphanOf(
        mk([
          ['x.y.low', 'a'],
          ['x.y.high', 'b'],
        ]),
        {
          refKeys: new Set(),
          tokens: new Set(),
          dynPrefixes: new Set(['x.y']),
        }
      ),
      want: [],
    },
    {
      name: '孤儿锁·反例 键路径以完整 token 出现在源码常量表里不误伤（*_STATUS_LABEL_KEYS 形态）',
      got: orphanOf(mk([['s.map.rejected', '已驳回']]), {
        refKeys: new Set(),
        tokens: new Set(['s.map.rejected']),
        dynPrefixes: new Set(),
      }),
      want: [],
    },
    {
      name: '孤儿锁·正例 动态前缀的救回不外溢到兄弟命名空间',
      got: orphanOf(
        mk([
          ['x.y.low', 'a'],
          ['p.q', 'b'],
        ]),
        {
          refKeys: new Set(),
          tokens: new Set(),
          dynPrefixes: new Set(['x.y']),
        }
      ),
      want: ['p.q'],
    },
    {
      name: '棘轮·正例 基线外新增孤儿必抓',
      got: (() => {
        const r = compareWithBaseline(['a.c', 'a.d'], { total: 1, keys: ['a.c'] });
        return [...r.added, r.over ? 'over' : '', ...r.stale].filter(Boolean).sort();
      })(),
      want: ['a.d', 'over'],
    },
    {
      name: '棘轮·反例 存量孤儿已在基线内不误伤',
      got: (() => {
        const r = compareWithBaseline(['a.c', 'a.d'], { total: 2, keys: ['a.c', 'a.d'] });
        return [...r.added, r.over ? 'over' : '', ...r.stale].filter(Boolean).sort();
      })(),
      want: [],
    },
    {
      name: '棘轮·正例 基线里挂着已修项（虚增防护）必抓',
      got: (() => {
        const r = compareWithBaseline(['a.c'], { total: 2, keys: ['a.c', 'a.gone'] });
        return [...r.added, r.over ? 'over' : '', ...r.stale].filter(Boolean).sort();
      })(),
      want: ['a.gone'],
    },
    {
      name: '对称锁·正例 单侧多出一个键必抓',
      got: asymOf(mk([['a.b', 'x']]), mk([])),
      want: ['文案|a.b|zh-CN'],
    },
    {
      name: '对称锁·反例 完全对称不误伤',
      got: asymOf(
        mk([
          ['a.b', 'x'],
          ['c.d', 'y'],
        ]),
        mk([
          ['a.b', 'X'],
          ['c.d', 'Y'],
        ])
      ),
      want: [],
    },
    {
      name: '对称锁·正例 同一路径一侧文案一侧分组（类型翻转）必抓两侧',
      got: asymOf(mk([['m.k', 'x']], []), mk([], ['m.k'])),
      want: ['分组|m.k|en-US', '文案|m.k|zh-CN'],
    },
    {
      name: '提示族·正例 msg.success 三元实参里的裸键名要算活引用（补 message. 前缀）',
      got: collectMessageRefs(
        'src\\views\\bpm\\useBpmApProc.ts',
        "msg.success(approveAction.value === 'approve' ? 'approvePassed' : 'approveRejected')"
      ).sort(),
      want: ['message.approvePassed', 'message.approveRejected'],
    },
    {
      name: '提示族·反例 非 message.ts 里的裸 error("DB failed") 不得算文案引用',
      got: collectMessageRefs('src\\api\\request.ts', "logger.error('DB failed, retry')"),
      want: [],
    },
    {
      name: '提示族·正例 message.ts 内部直调与默认参数都算活引用',
      got: [
        ...new Set(
          collectMessageRefs(
            'src\\utils\\message.ts',
            "function success(key = 'operationSuccess'): void {}\n" +
              "export const msg = { loadFail: () => error('loadFailed') };"
          )
        ),
      ].sort(),
      want: ['message.loadFailed', 'message.operationSuccess'],
    },
    {
      name: '地板·正例 引用集为空必须判红而不是"零问题"',
      got: powerFloor({ leafCount: 100, refCount: 0, tokenCount: 50 }),
      want: ['引用集为空（REF_PATTERNS 或 src 扫描失效），孤儿锁无检测力'],
    },
    {
      name: '地板·正例 locale 解析不到任何键必须判红',
      got: powerFloor({ leafCount: 0, refCount: 10, tokenCount: 10 }),
      want: ['未从 locale 解析到任何文案键，检查器失效'],
    },
    {
      name: '地板·反例 正常输入不误伤',
      got: powerFloor({ leafCount: 100, refCount: 10, tokenCount: 50 }),
      want: [],
    },
  ];
  let pass = 0;
  let fail = 0;
  console.log('=== self-test：check-i18n 对称锁 / 孤儿锁 / 基线棘轮 / 检测力地板 双向自证 ===');
  for (const c of cases) {
    const ok = c.got.length === c.want.length && c.got.every((v, i) => v === c.want[i]);
    if (ok) {
      pass++;
      console.log(`  PASS  ${c.name}  [检出 ${JSON.stringify(c.got)}]`);
    } else {
      fail++;
      console.log(
        `  FAIL  ${c.name}  -> 期望 ${JSON.stringify(c.want)}，实际 ${JSON.stringify(c.got)}`
      );
    }
  }
  console.log(`---- self-test 结论：${pass} 通过 / ${fail} 失败 ----`);
  if (fail) {
    console.error('[i18n] ✗ 判定器自证失败：对称锁或孤儿锁存在漏报/误报，门禁不可信');
    process.exit(1);
  }
  console.log(`SELF-TEST OK: ${pass}/${cases.length}`);
  process.exit(0);
}

/* ---------- 主流程 ---------- */

if (process.argv.includes('--self-test')) runSelfTest(); // 内部 exit，不碰仓库状态

const locales = new Map();
for (const name of NAMES) locales.set(name, parseLocale(name));

const refs = [];
const dynamicRefs = new Set();
const tokens = new Set();
const dynPrefixes = new Set();
const msgRefs = new Set();
const scanCtx = { tokens, dynPrefixes, msgRefs };
collectRefs(join(FRONTEND, 'src'), refs, dynamicRefs, { ...scanCtx, literals: true });
collectRefs(join(FRONTEND, 'e2e'), refs, dynamicRefs, { ...scanCtx, literals: false });
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

/* 锁① 中英键集合对称：单侧多/单侧缺/类型翻转都点名 */
const asymmetry = findAsymmetry('zh-CN', locales.get('zh-CN'), 'en-US', locales.get('en-US'));
violations.push(...asymmetry.map(asymmetryText));

/* 锁② 孤儿键：locale 有文案、src/e2e 无引用 */
const refKeys = new Set([...firstRef.keys(), ...msgRefs]);
const orphanSets = new Map(
  NAMES.map(n => [n, new Set(findOrphans(locales.get(n), { refKeys, tokens, dynPrefixes }))])
);
const orphans = [...new Set(NAMES.flatMap(n => [...orphanSets.get(n)]))].sort();

if (process.argv.includes('--write-baseline')) {
  const n = await writeBaseline(orphans);
  console.log(`[i18n] 孤儿基线已写入 scripts/i18n-orphans-baseline.json：存量孤儿 ${n} 个`);
  process.exit(0);
}

/* 检测力地板：解析器或扫描一旦失效，"查不出问题"不等于"没有问题" */
const floor = powerFloor({
  leafCount: NAMES.reduce((n, k) => n + locales.get(k).values.size, 0),
  refCount: firstRef.size,
  tokenCount: tokens.size,
});
if (floor.length) {
  console.error(
    `[i18n] ❌ 检测力失效（判红，不得静默通过）:\n${floor.map(m => '  - ' + m).join('\n')}`
  );
  process.exit(1);
}

if (!existsSync(BASELINE)) {
  console.error(
    '[i18n] ❌ 缺孤儿基线文件 scripts/i18n-orphans-baseline.json（先跑 node scripts/check-i18n.mjs --write-baseline）'
  );
  process.exit(1);
}
const baseline = loadBaseline();
const ratchet = compareWithBaseline(orphans, baseline);
for (const k of ratchet.added) {
  const sides = NAMES.filter(n => orphanSets.get(n).has(k)).join(' + ');
  violations.push(
    `新增孤儿键 ${k}（${sides}）：语言包里有这条文案，但 src/e2e 无任何引用 —— 删除文案，或把它接回真实引用点`
  );
}
if (ratchet.over)
  violations.push(`孤儿键总数 ${orphans.length} 超过基线 ${baseline.total}（棘轮只准降不准升）`);
if (ratchet.stale.length)
  violations.push(
    `基线未收口：${ratchet.stale.length} 项已不再是孤儿（例 ${ratchet.stale.slice(0, 3).join('、')}）` +
      '，请跑 node scripts/check-i18n.mjs --write-baseline 一并下调基线'
  );

/* 存量挂账：修复一条删一条，禁止新增 */
const ALLOWLIST = new Set();

const blocked = violations.filter(v => !ALLOWLIST.has(v));
const uncompilable = NAMES.reduce((n, name) => n + locales.get(name).dynamic.length, 0);
console.log(
  `[i18n] 引用字面 key ${firstRef.size} 个（动态拼接 ${dynamicRefs.size} 处不判定），` +
    `zh-CN 文案 ${locales.get('zh-CN').values.size}，en-US 文案 ${locales.get('en-US').values.size}，` +
    `消息编译 ${compiled} 条（模板串/拼接 ${uncompilable} 条取不到字面值，不编译），` +
    `中英对称差 ${violations.filter(v => v.startsWith('zh/en 不对称')).length}，` +
    `孤儿 ${orphans.length}（基线 ${baseline.total}，动态前缀 ${dynPrefixes.size} 个、token ${tokens.size} 个参与救回），` +
    `问题 ${violations.length}，挂账 ${violations.length - blocked.length}`
);

if (blocked.length) {
  console.error(
    `[i18n] ❌ 未通过（${blocked.length} 项）:\n${blocked.map(v => '  - ' + v).join('\n')}`
  );
  process.exit(1);
}
console.log('[i18n] ✅ 通过');
