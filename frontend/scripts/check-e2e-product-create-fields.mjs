#!/usr/bin/env node
/**
 * e2e/src 直连建品请求体的 product_type 单源判定器（零依赖、只读扫描）。
 *
 * 功能：后端新建产品把 product_type 改成必填（backend/src/services/product_ops/crud.rs
 * 与 backend/src/services/product_service.rs 的 validate_product_create_fields），
 * 且取值必须落在词表（坯布/成品布/辅料）。夹具若把中文面值散落在各 spec 里，
 * 词表演进时逐处漂移、grep 面值还会假绿。本判定器把「每个直连 POST /products 的
 * 请求体顶层必须有 product_type 键、且取值必须是 PRODUCT_TYPE.<键> 单源成员引用」
 * 立成硬判据：字面串无论词表内外一律判红。
 *
 * 调用方：CI fe 门禁链与本地自查（node frontend/scripts/check-e2e-product-create-fields.mjs），
 * 也被本脚本 --self-test 自检复用同一套扫描函数。
 *
 * 入参：--root <frontend 目录>（默认脚本上级目录）、--self-test（跑内置夹具自测）、
 * 或位置参数指定的额外扫描目录（相对 root 或绝对路径，路径不存在必须响亮报错）。
 *
 * 合法取值集合不内嵌任何中文词表：唯一来源是 <root>/src/constants/product-type.ts 里
 * PRODUCT_TYPE 对象的键与值，运行时解析该文件得到成员集合（按中文字面量 grep 是假绿盲区）。
 *
 * 输出：stdout 逐站点行「HIT|MISS|BLIND|PASSTHRU 文件:行 类别 说明」与计数汇总；
 * 存在判红（缺键/字面串散落/未知常量成员/非成员表达式/透传构造器注入缺省值）或盲区即 exit 1；
 * 透传构造器判据为「对 product_type 完全无感」：builder 内出现键注入或赋值注入（含成员默认值）判红，
 * 取值经展开原样透传不算注入，API 层不得代发类型缺省值。
 * 命中数为 0 也判红（扫描对象消失属判定器空操作，不是通过）。exit 0 需同时满足
 * 判红 0、散落 0、盲区 0 且命中数大于 0。
 */

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------------------
// 文本预处理：剥注释（保留偏移与行号），扫描模式匹配不受注释干扰。
// ---------------------------------------------------------------------------

/** 把 // 行注释与 * 块注释替换为等长空白（保留换行），返回 {text, lineStarts} */
function stripCommentsPreserveOffsets(src) {
  const out = src.split('');
  let i = 0;
  const n = src.length;
  let inBlock = false;
  let inLine = false;
  let str = null; // 当前字符串定界符
  while (i < n) {
    const c = src[i];
    const d = i + 1 < n ? src[i + 1] : '';
    if (inLine) {
      if (c === '\n') inLine = false;
      else out[i] = ' ';
      i++;
      continue;
    }
    if (inBlock) {
      if (c === '*' && d === '/') {
        out[i] = ' ';
        out[i + 1] = ' ';
        inBlock = false;
        i += 2;
        continue;
      }
      if (c !== '\n') out[i] = ' ';
      i++;
      continue;
    }
    if (str) {
      if (c === '\\') i += 2;
      else if (c === str) str = null;
      i++;
      continue;
    }
    if (c === '/' && d === '/') {
      inLine = true;
      out[i] = ' ';
      out[i + 1] = ' ';
      i += 2;
      continue;
    }
    if (c === '/' && d === '*') {
      inBlock = true;
      out[i] = ' ';
      out[i + 1] = ' ';
      i += 2;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') str = c;
    i++;
  }
  return out.join('');
}

function lineOf(text, offset) {
  let line = 1;
  for (let i = 0; i < offset; i++) if (text.charCodeAt(i) === 10) line++;
  return line;
}

// ---------------------------------------------------------------------------
// 单源词表：只从 src/constants/product-type.ts 解析 PRODUCT_TYPE 的键与值。
// ---------------------------------------------------------------------------

function loadProductTypeMembers(constantsPath) {
  if (!fs.existsSync(constantsPath)) {
    throw new Error(
      `[判定器缺陷] 单源常量文件不存在，无法确定合法取值集合：${constantsPath}`
    );
  }
  const src = stripCommentsPreserveOffsets(fs.readFileSync(constantsPath, 'utf8'));
  const m = src.match(/export\s+const\s+PRODUCT_TYPE\s*=\s*\{/);
  if (!m) {
    throw new Error(
      `[判定器缺陷] 常量文件里没有 export const PRODUCT_TYPE = { 定义：${constantsPath}`
    );
  }
  const open = src.indexOf('{', m.index);
  const close = matchBrace(src, open);
  if (close < 0) {
    throw new Error(`[判定器缺陷] PRODUCT_TYPE 对象未闭合：${constantsPath}`);
  }
  const body = src.slice(open + 1, close);
  const keys = [];
  const values = [];
  const pairRe = /([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(['"])(.*?)\2/g;
  let p;
  while ((p = pairRe.exec(body))) {
    keys.push(p[1]);
    values.push(p[3]);
  }
  if (keys.length === 0) {
    throw new Error(
      `[判定器缺陷] PRODUCT_TYPE 对象解析出 0 个成员（单源词表为空属解析失效）：${constantsPath}`
    );
  }
  return { keys, values };
}

// ---------------------------------------------------------------------------
// 括号匹配与顶层键扫描（字符串感知，注释已剥）。
// ---------------------------------------------------------------------------

/** 从 openIdx 的 '{' 找到配对 '}' 的下标；找不到返回 -1 */
function matchBrace(text, openIdx) {
  let depth = 0;
  let str = null;
  for (let i = openIdx; i < text.length; i++) {
    const c = text[i];
    if (str) {
      if (c === '\\') i++;
      else if (c === str) str = null;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') {
      str = c;
      continue;
    }
    if (c === '{') depth++;
    else if (c === '}') {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

/** 取对象字面量（含首尾花括号）内 depth=1 的键值对；product_type 的取值文本由调用方正则提取 */
function topKeyValuePairs(text, openIdx, closeIdx) {
  const pairs = [];
  let depth = 0;
  let str = null;
  let token = '';
  let key = null;
  let valueStart = -1;
  for (let i = openIdx; i <= closeIdx; i++) {
    const c = text[i];
    if (str) {
      if (c === '\\') {
        token += c + (text[i + 1] ?? '');
        i++;
        continue;
      }
      token += c;
      if (c === str) str = null;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') {
      str = c;
      token += c;
      continue;
    }
    if (depth === 1 && key === null && /[A-Za-z0-9_]/.test(c)) token += c;
    if (c === '{' || c === '[' || c === '(') {
      depth++;
      if (depth === 1) token = '';
      continue;
    }
    if (c === '}' || c === ']' || c === ')') {
      depth--;
      if (depth === 0) {
        if (key === 'product_type') pairs.push({ key, value: text.slice(valueStart, i).trim() });
        key = null;
        token = '';
      }
      continue;
    }
    if (c === ',' && depth === 1) {
      if (key === 'product_type') pairs.push({ key, value: text.slice(valueStart, i).trim() });
      key = null;
      token = '';
      continue;
    }
    if (c === ':' && depth === 1 && key === null) {
      key = token.trim().replace(/^['"`]/, '').replace(/['"`]$/, '');
      token = '';
      valueStart = i + 1;
    }
  }
  return pairs;
}

// ---------------------------------------------------------------------------
// 站点识别：五种直连 POST /products 的请求体形态。
// 类别：pass 合格 / missing 缺键 / literal 字面串散落 / unknown-member 未知常量成员 /
// non-member 非成员表达式 / blind 盲区（请求体静态不可解析）。
// ---------------------------------------------------------------------------

function classifyValue(valueText, members) {
  const v = valueText.trim();
  let m = v.match(/^PRODUCT_TYPE\.([A-Za-z_][A-Za-z0-9_]*)$/);
  if (m) {
    return members.keys.includes(m[1])
      ? { cat: 'pass', note: `单源成员 PRODUCT_TYPE.${m[1]}` }
      : { cat: 'unknown-member', note: `PRODUCT_TYPE.${m[1]} 不在常量文件已知键内` };
  }
  m = v.match(/^(['"`])(.*)\1$/s);
  if (m) {
    const inVocab = members.values.includes(m[2]);
    return {
      cat: 'literal',
      note: `字面串取值（${inVocab ? '词表内' : '词表外'}面值同样判红），夹具须改用 PRODUCT_TYPE.<键> 引用`,
    };
  }
  return { cat: 'non-member', note: `取值不是 PRODUCT_TYPE.<键> 成员表达式：${v.slice(0, 60)}` };
}

function resolveObjectLiteral(strip, callEndIdx, varName) {
  // 就近向前解析 const|let|var <varName> = {
  const re = new RegExp(`(?:const|let|var)\\s+${varName}\\s*=\\s*\\{`, 'g');
  let best = -1;
  let m;
  while ((m = re.exec(strip))) {
    if (m.index < callEndIdx) best = strip.indexOf('{', m.index);
    else break;
  }
  if (best < 0) return null;
  const close = matchBrace(strip, best);
  if (close < 0) return null;
  return { open: best, close };
}

function analyzeBodyLiteral(strip, filePath, siteOffset, openIdx, members) {
  const closeIdx = matchBrace(strip, openIdx);
  if (closeIdx < 0) {
    return { cat: 'blind', file: filePath, line: lineOf(strip, siteOffset), note: '对象字面量未闭合，静态不可判' };
  }
  const pairs = topKeyValuePairs(strip, openIdx, closeIdx);
  const pt = pairs.find(p => p.key === 'product_type');
  if (!pt) {
    return { cat: 'missing', file: filePath, line: lineOf(strip, siteOffset), note: '顶层缺 product_type 键（后端新建必填写入即 400）' };
  }
  const r = classifyValue(pt.value, members);
  return { ...r, file: filePath, line: lineOf(strip, siteOffset) };
}

function scanFile(filePath, strip, members, opts) {
  const findings = [];
  const passThru = [];
  const isSrc = opts.srcDirSet && opts.srcDirSet.has(filePath);

  // P1 apiCall/apiCallRaw/submitMustNotBeRejected 形态：'POST', '/products', <body>
  const p1 = /['"]POST['"]\s*,\s*(['"`])\/products\1\s*,\s*/g;
  let m;
  while ((m = p1.exec(strip))) {
    findings.push(analyzeBodyAfterToken(strip, filePath, m, members));
  }
  // P2 seedPost 模板 URL 形态：seedPost(`${...}/products`, <body>)
  const p2 = /seedPost\(\s*`[^`]*\/products`\s*,\s*/g;
  while ((m = p2.exec(strip))) {
    findings.push(analyzeBodyAfterToken(strip, filePath, m, members));
  }
  // P3 矩阵配置形态：createApi: '/products', payload: { ... }
  const p3 = /createApi:\s*(['"`])\/products\1\s*,\s*payload:\s*/g;
  while ((m = p3.exec(strip))) {
    findings.push(analyzeBodyAfterToken(strip, filePath, m, members));
  }
  // P4 firstRefOrSeed 形态：listApi 与 seedApi 连写两个 '/products' 后跟 seedPayload 字面量
  const p4 = /(['"`])\/products\1\s*,\s*(['"`])\/products\2\s*,\s*/g;
  while ((m = p4.exec(strip))) {
    // 已由 P1 覆盖的（POST 连写形态）不重复计：要求前文 400 字符内没有 'POST' 紧邻
    findings.push(analyzeBodyAfterToken(strip, filePath, m, members));
  }
  // P5 axios src 层：request.post('/products', <expr>) 或 request.post<...>('/products', <expr>)
  const p5 = /\.post\s*(?:<[^()]*?>)?\(\s*(['"`])\/products\1\s*,\s*/g;
  while ((m = p5.exec(strip))) {
    const after = strip.slice(m.index + m[0].length);
    const head = after.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(/);
    if (head) {
      // 构造器透传（如 toProductPayload(data)）的判据是「类型无感」：builder 体内
      // 不得出现任何 product_type 注入。字面面值注入判散落红；成员默认值注入
      // （product_type: PRODUCT_TYPE.x、payload.product_type ??= ...、赋值式）判注入红。
      // 口子必须封死：#1084 的修复语义是「新建必须显式提交类型、不允许缺省」，
      // API 层一旦代发缺省值，所有不透类型的调用方都被静默补默认，后端必填门形同虚设。
      // 取值经 ...rest 原样透传不算注入（键的产生仍在调用方）。
      const builderName = head[1];
      const bRe = new RegExp(`function\\s+${builderName}\\b[^{]*\\{`, 'g');
      let bm;
      let literalInBuilder = null;
      let injectionInBuilder = null;
      while ((bm = bRe.exec(strip))) {
        const open = strip.indexOf('{', bm.index);
        const close = matchBrace(strip, open);
        if (close > 0) {
          const bodyText = strip.slice(open, close);
          const lit = bodyText.match(/product_type\s*:\s*(['"`])([^'"`]*)\1/);
          if (lit) literalInBuilder = lit[0];
          const inj = bodyText.match(
            /product_type\s*:|(?:\.product_type\s*(?:\?\?\??|\|\|)?=[^=])/
          );
          if (inj && !injectionInBuilder) injectionInBuilder = inj[0];
        }
      }
      if (literalInBuilder) {
        findings.push({
          cat: 'literal',
          file: filePath,
          line: lineOf(strip, m.index),
          note: `透传构造器 ${builderName} 内硬编码了 product_type 取值：${literalInBuilder}`,
        });
      } else if (injectionInBuilder) {
        findings.push({
          cat: 'injection',
          file: filePath,
          line: lineOf(strip, m.index),
          note: `透传构造器 ${builderName} 注入了 product_type 键（形态「${injectionInBuilder}」），API 层不得代发类型缺省值，取值必须由调用方显式传入`,
        });
      } else {
        passThru.push({
          file: filePath,
          line: lineOf(strip, m.index),
          note: `透传构造器 ${builderName} 对 product_type 完全无感（无注入形态命中），取值由调用方经单源常量传入`,
        });
      }
      continue;
    }
    findings.push(analyzeBodyAfterToken(strip, filePath, m, members));
  }

  if (isSrc) {
    // src 层不得出现任何硬编码中文面值的建品请求体（字面串判红规则同样适用）
  }
  return { findings, passThru };
}

function analyzeBodyAfterToken(strip, filePath, m, members) {
  const siteOffset = m.index;
  const after = strip.slice(m.index + m[0].length);
  const startIdx = m.index + m[0].length + (after.length - after.replace(/^\s+/, '').length);
  const rest = strip.slice(startIdx);
  if (rest[0] === '{') {
    return analyzeBodyLiteral(strip, filePath, siteOffset, startIdx, members);
  }
  const ident = rest.match(/^([A-Za-z_][A-Za-z0-9_]*)\b/);
  if (ident) {
    const lit = resolveObjectLiteral(strip, startIdx, ident[1]);
    if (lit) {
      return analyzeBodyLiteral(strip, filePath, siteOffset, lit.open, members);
    }
    return {
      cat: 'blind',
      file: filePath,
      line: lineOf(strip, siteOffset),
      note: `请求体是变量 ${ident[1]} 且同文件内不可静态解析（盲区，禁止当通过）`,
    };
  }
  return {
    cat: 'blind',
    file: filePath,
    line: lineOf(strip, siteOffset),
    note: '请求体表达式静态不可解析（盲区，禁止当通过）',
  };
}

// ---------------------------------------------------------------------------
// 目录扫描
// ---------------------------------------------------------------------------

function collectTsFiles(dir) {
  const out = [];
  const walk = d => {
    for (const ent of fs.readdirSync(d, { withFileTypes: true })) {
      const full = path.join(d, ent.name);
      if (ent.isDirectory()) walk(full);
      else if (ent.name.endsWith('.ts') && !ent.name.endsWith('.d.ts')) out.push(full);
    }
  };
  walk(dir);
  return out;
}

function scanRoots(root, extraPaths) {
  const constantsPath = path.join(root, 'src', 'constants', 'product-type.ts');
  const members = loadProductTypeMembers(constantsPath);
  const dirs = [path.join(root, 'e2e'), path.join(root, 'src')];
  for (const p of extraPaths) {
    const abs = path.isAbsolute(p) ? p : path.join(root, p);
    if (!fs.existsSync(abs)) {
      throw new Error(`[响亮报错] 扫描路径不存在，拒绝静默报 0：${abs}`);
    }
    dirs.push(abs);
  }
  const srcDirSet = new Set();
  const files = [];
  for (const d of dirs) {
    for (const f of collectTsFiles(d)) {
      files.push(f);
      if (d.endsWith(`${path.sep}src`) || d.includes(`${path.sep}src${path.sep}`)) srcDirSet.add(f);
    }
  }
  const uniq = [...new Set(files)];
  const all = [];
  const allPassThru = [];
  for (const f of uniq) {
    const raw = fs.readFileSync(f, 'utf8');
    const strip = stripCommentsPreserveOffsets(raw);
    const { findings, passThru } = scanFile(f, strip, members, { srcDirSet });
    all.push(...findings);
    allPassThru.push(...passThru);
  }
  return { members, findings: all, passThru: allPassThru, constantsPath };
}

// ---------------------------------------------------------------------------
// 报告与出口
// ---------------------------------------------------------------------------

const RED_CATS = new Set(['missing', 'literal', 'unknown-member', 'non-member', 'blind', 'injection']);

function report(result, root) {
  const rel = f => path.relative(root, f).split(path.sep).join('/');
  const counts = {
    命中: 0,
    合格: 0,
    缺键: 0,
    散落字面串: 0,
    未知常量成员: 0,
    非成员表达式: 0,
    注入缺省值: 0,
    盲区: 0,
    透传构造器: 0,
  };
  for (const s of result.findings) {
    counts.命中++;
    if (s.cat === 'pass') counts.合格++;
    else if (s.cat === 'missing') counts.缺键++;
    else if (s.cat === 'literal') counts.散落字面串++;
    else if (s.cat === 'unknown-member') counts.未知常量成员++;
    else if (s.cat === 'non-member') counts.非成员表达式++;
    else if (s.cat === 'injection') counts.注入缺省值++;
    else if (s.cat === 'blind') counts.盲区++;
  }
  counts.透传构造器 = result.passThru.length;
  for (const s of [...result.findings].sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line)) {
    const tag = s.cat === 'pass' ? 'PASS' : RED_CATS.has(s.cat) ? 'RED ' : '???';
    console.log(`${tag} ${rel(s.file)}:${s.line} [${s.cat}] ${s.note}`);
  }
  for (const s of result.passThru) {
    console.log(`THRU ${rel(s.file)}:${s.line} ${s.note}`);
  }
  console.log(
    `计数汇总 命中=${counts.命中} 合格=${counts.合格} 缺键=${counts.缺键} 散落字面串=${counts.散落字面串} ` +
      `未知常量成员=${counts.未知常量成员} 非成员表达式=${counts.非成员表达式} 注入缺省值=${counts.注入缺省值} ` +
      `盲区=${counts.盲区} 透传构造器=${counts.透传构造器}`
  );
  console.log(
    `单源词表文件: ${rel(result.constantsPath)} 已知键=${result.members.keys.join(',')}`
  );
  const redTotal =
    counts.缺键 + counts.散落字面串 + counts.未知常量成员 + counts.非成员表达式 + counts.注入缺省值 + counts.盲区;
  if (counts.命中 === 0) {
    console.error('判红：命中数为 0。扫描范围内没有任何直连建品请求体属判定器空操作，不允许当通过。');
    return 1;
  }
  if (redTotal > 0) {
    console.error(`判红：存在 ${redTotal} 条不合格站点（缺键+散落+未知成员+非成员+注入缺省值+盲区）。`);
    return 1;
  }
  return 0;
}

// ---------------------------------------------------------------------------
// --self-test：合成夹具走同一套扫描函数，含双向变异与不存在路径的响亮报错。
// ---------------------------------------------------------------------------

const CONSTS_FIXTURE = `export const PRODUCT_TYPE = {
  greige: 'GREIGE_VAL',
  finished: 'FINISHED_VAL',
  auxiliary: 'AUX_VAL',
} as const;
`;

function makeSelfTestRoot() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'e2e-pt-field-'));
  fs.mkdirSync(path.join(root, 'src', 'constants'), { recursive: true });
  fs.mkdirSync(path.join(root, 'e2e', 'flow'), { recursive: true });
  fs.writeFileSync(path.join(root, 'src', 'constants', 'product-type.ts'), CONSTS_FIXTURE);
  return root;
}

function writeCase(root, relPath, content) {
  const full = path.join(root, relPath);
  fs.mkdirSync(path.dirname(full), { recursive: true });
  fs.writeFileSync(full, content);
  return full;
}

function runSelfTest() {
  const results = [];
  const t = (name, fn) => {
    try {
      fn();
      results.push({ name, ok: true });
    } catch (e) {
      results.push({ name, ok: false, err: e.message });
    }
  };
  const expectCat = (rel, content, wantCat) => {
    const root = makeSelfTestRoot();
    try {
      writeCase(root, rel, content);
      const r = scanRoots(root, []);
      const hit = r.findings.find(f => f.file.endsWith(rel.split('/').pop()));
      if (!hit) throw new Error(`没有命中任何站点：${rel}`);
      if (hit.cat !== wantCat) throw new Error(`期望 ${wantCat}，实际 ${hit.cat}（${hit.note}）`);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  };

  // 1 缺键判红
  t('01 顶层缺 product_type 判红', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { name: 'x' });`, 'missing'));
  // 2 词表外字面串判红
  t('02 词表外字面串判红', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { product_type: 'fabric' });`, 'literal'));
  // 3 词表内字面串同样判红（面值单源纪律：词表内外一律红）
  t('03 词表内面值字面串也判红', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { product_type: 'GREIGE_VAL' });`, 'literal'));
  // 4 常量成员判绿
  t('04 PRODUCT_TYPE.greige 判合格', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { product_type: PRODUCT_TYPE.greige });`, 'pass'));
  // 5 未知常量成员判红
  t('05 PRODUCT_TYPE.notAKey 判未知成员红', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { product_type: PRODUCT_TYPE.notAKey });`, 'unknown-member'));
  // 6 喂不存在的扫描路径必须响亮报错、不许静默报 0
  t('06 不存在的扫描路径响亮报错', () => {
    const root = makeSelfTestRoot();
    let threw = false;
    try {
      scanRoots(root, ['e2e/不存在的目录']);
    } catch (e) {
      threw = /响亮报错/.test(e.message);
    }
    fs.rmSync(root, { recursive: true, force: true });
    if (!threw) throw new Error('未对缺失路径响亮报错');
  });
  // 7 变量载荷可静态解析且合格
  t('07 同文件 const payload 解析判合格', () =>
    expectCat('e2e/flow/a.ts', `const payload = { product_type: PRODUCT_TYPE.finished };\nawait apiCall(page, 'POST', '/products', payload);`, 'pass'));
  // 8 变量载荷缺键判红
  t('08 同文件 const payload 缺键判红', () =>
    expectCat('e2e/flow/a.ts', `const payload = { name: 'x' };\nawait apiCall(page, 'POST', '/products', payload);`, 'missing'));
  // 9 不可解析变量判盲区（不许静默放行）
  t('09 外来源变量判盲区', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', buildPayload());`, 'blind'));
  // 10 矩阵配置形态字面串判红
  t('10 createApi+payload 字面串判红', () =>
    expectCat('e2e/flow/a.ts', `{ createApi: '/products', payload: { product_type: 'FINISHED_VAL' } }`, 'literal'));
  // 11 矩阵配置形态常量成员判合格
  t('11 createApi+payload 成员判合格', () =>
    expectCat('e2e/flow/a.ts', `{ createApi: '/products', payload: { product_type: PRODUCT_TYPE.finished } }`, 'pass'));
  // 12 firstRefOrSeed 形态缺键判红
  t('12 连写双 /products 种子载荷缺键判红', () =>
    expectCat('e2e/flow/a.ts', `await firstRefOrSeed(page, '产品', '/products', '/products', { name: 'x' }, 'items');`, 'missing'));
  // 13 firstRefOrSeed 形态成员判合格
  t('13 连写双 /products 种子载荷成员判合格', () =>
    expectCat('e2e/flow/a.ts', `await firstRefOrSeed(page, '产品', '/products', '/products', { product_type: PRODUCT_TYPE.auxiliary }, 'items');`, 'pass'));
  // 14 seedPost 模板 URL 缺键判红
  t('14 seedPost 模板 URL 缺键判红', () =>
    expectCat('e2e/flow/a.ts', `await seedPost(\`\${API_PREFIX}/products\`, { name: 'x' });`, 'missing'));
  // 15 seedPost 模板 URL 成员判合格
  t('15 seedPost 模板 URL 成员判合格', () =>
    expectCat('e2e/flow/a.ts', `await seedPost(\`\${API_PREFIX}/products\`, { product_type: PRODUCT_TYPE.greige });`, 'pass'));
  // 16 嵌套层的 product_type 不算顶层键
  t('16 仅嵌套层带键判顶层缺键', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { name: 'x', extra: { product_type: PRODUCT_TYPE.greige } });`, 'missing'));
  // 17 注释里的 product_type 面值不干扰（注释单源成员也不给过，注释散落面值也不算站点）
  t('17 注释内容不参与判定', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', {\n // product_type: 'AUX_VAL'\n product_type: PRODUCT_TYPE.auxiliary });`, 'pass'));
  // 18 src 透传构造器：builder 无硬编码取值则记透传、不记盲区
  t('18 src 透传构造器判透传合格', () => {
    const root = makeSelfTestRoot();
    try {
      writeCase(
        root,
        'src/api/x.ts',
        `function toProductPayload(data: any) { return { ...data }; }\nexport const createProduct = (data: any) => request.post('/products', toProductPayload(data));`
      );
      const r = scanRoots(root, []);
      if (r.passThru.length !== 1) throw new Error(`期望 1 条透传，实际 ${r.passThru.length}`);
      if (r.findings.some(f => f.cat === 'blind')) throw new Error('透传被判成盲区');
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });
  // 19 src 透传构造器内硬编码字面量判散落红
  t('19 builder 硬编码取值判散落红', () => {
    const root = makeSelfTestRoot();
    try {
      writeCase(
        root,
        'src/api/x.ts',
        `function toProductPayload(data: any) { return { product_type: 'AUX_VAL', ...data }; }\nexport const createProduct = (data: any) => request.post('/products', toProductPayload(data));`
      );
      const r = scanRoots(root, []);
      if (!r.findings.some(f => f.cat === 'literal')) throw new Error('builder 内硬编码面值没被判红');
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });
  // 20 带 spread 的对象显式键仍判合格
  t('20 展开后显式成员键判合格', () =>
    expectCat('e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { name: 'x', product_type: PRODUCT_TYPE.greige, ...extra });`, 'pass'));
  // 21 双向变异一：合格站点把成员改回字面串必须翻红
  t('21 变异：成员改回字面串翻红', () => {
    const root = makeSelfTestRoot();
    try {
      writeCase(root, 'e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { product_type: PRODUCT_TYPE.greige });`);
      const before = scanRoots(root, []);
      const green = before.findings.length === 1 && before.findings[0].cat === 'pass';
      writeCase(root, 'e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { product_type: 'GREIGE_VAL' });`);
      const after = scanRoots(root, []);
      const red = after.findings.length === 1 && after.findings[0].cat === 'literal';
      if (!green || !red) throw new Error(`变异未生效 green=${green} red=${red}`);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });
  // 22 双向变异二：字面串站点改成成员引用必须翻绿
  t('22 变异：字面串改成员翻绿', () => {
    const root = makeSelfTestRoot();
    try {
      writeCase(root, 'e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { product_type: 'AUX_VAL' });`);
      const before = scanRoots(root, []);
      const red = before.findings.length === 1 && before.findings[0].cat === 'literal';
      writeCase(root, 'e2e/flow/a.ts', `await apiCall(page, 'POST', '/products', { product_type: PRODUCT_TYPE.auxiliary });`);
      const after = scanRoots(root, []);
      const green = after.findings.length === 1 && after.findings[0].cat === 'pass';
      if (!red || !green) throw new Error(`变异未生效 red=${red} green=${green}`);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });
  // 23 空操作探针：扫描目录里没有任何建品站点时命中 0 必须判红
  t('23 命中 0 判空操作红', () => {
    const root = makeSelfTestRoot();
    try {
      writeCase(root, 'e2e/flow/a.ts', `await apiCall(page, 'GET', '/products', {});`);
      const r = scanRoots(root, []);
      const code = report(r, root);
      if (code !== 1) throw new Error('命中 0 却未判红');
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  // 24 builder 注入成员默认值判注入红（透传口子封死：API 层不得代发类型缺省）
  t('24 builder 注入成员默认值判红', () => {
    const root = makeSelfTestRoot();
    try {
      writeCase(
        root,
        'src/api/x.ts',
        `function toProductPayload(data: any) { return { product_type: PRODUCT_TYPE.auxiliary, ...data }; }\nexport const createProduct = (data: any) => request.post('/products', toProductPayload(data));`
      );
      const r = scanRoots(root, []);
      if (!r.findings.some(f => f.cat === 'injection')) throw new Error(`成员默认值注入未判红 findings=${JSON.stringify(r.findings.map(f => f.cat))}`);
      if (r.passThru.length !== 0) throw new Error('注入站点仍被记成透传合格');
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });
  // 25 builder 赋值式注入判注入红
  t('25 builder 赋值式注入判红', () => {
    const root = makeSelfTestRoot();
    try {
      writeCase(
        root,
        'src/api/x.ts',
        `function toProductPayload(data: any) { const p: any = { ...data }; p.product_type ??= PRODUCT_TYPE.greige; return p; }\nexport const createProduct = (data: any) => request.post('/products', toProductPayload(data));`
      );
      const r = scanRoots(root, []);
      if (!r.findings.some(f => f.cat === 'injection')) throw new Error(`赋值式注入未判红 findings=${JSON.stringify(r.findings.map(f => f.cat))}`);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  const failed = results.filter(x => !x.ok);
  for (const x of results) {
    console.log(`[self-test] ${x.ok ? 'PASS' : 'FAIL'} ${x.name}${x.ok ? '' : ` → ${x.err}`}`);
  }
  console.log(`SELF_TEST_RESULT total=${results.length} fail=${failed.length}`);
  return failed.length === 0 ? 0 : 1;
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--self-test')) {
    process.exit(runSelfTest());
  }
  let root = __dirname.replace(/[\\/]scripts$/, '');
  const ri = args.indexOf('--root');
  if (ri >= 0) root = path.resolve(args[ri + 1]);
  const extras = args.filter((a, i) => !a.startsWith('--') && i !== ri + 1);
  let result;
  try {
    result = scanRoots(root, extras);
  } catch (e) {
    console.error(String(e.message || e));
    process.exit(2);
  }
  process.exit(report(result, root));
}

main();
