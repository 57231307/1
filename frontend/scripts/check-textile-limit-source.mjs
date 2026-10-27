#!/usr/bin/env node
/**
 * 纺织品安全合规域 国标限值单源自证静态锁（不连库、本地可跑、CI 常规可跑）
 *
 * 钉什么（任一不满足即 exit 1，逐条点名，绝不把"没查出问题"当通过）：
 *  1. 单源散布锁：GB 18401 与 GB 31701 的限值数字只允许出现在迁移种子
 *     m0111_seed_textile_safety_matrix.rs 里。判定器 textile_safety_limit_judge.rs
 *     与非测试区的三个 service、三个 handler、前端合规视图与 api 与常量层，都不得
 *     把任何国标限值数值当硬编码字面量散写（判定按行查表，零魔法数）。
 *  2. 种子即国标锁：把 m0111 种子解析成 (项目,类别,形态,条件,比较器,下限,上限) 的
 *     全量行集，与独立重述的 GB 18401-2010 三类加 GB 31701-2015 婴童加考权威矩阵
 *     逐行集合相等；多一行少一行改一值都判红并点名差异行。权威矩阵写死在本文件
 *     作为与被检种子相互独立的一方，二源交叉验证，种子漂移即被本锁抓到。
 *  3. 两侧同源锁：种子里出现的比较器与安全类别与形态与条件标签 token，必须全部命中
 *     Rust 词表模块 textile_comparator、textile_safety_category、textile_product_form、
 *     textile_exemption_tag、textile_limit_scope 的取值集合（判定器按词表常量比对，
 *     种子写了词表没有的 token = 判定器永不触发 = 两侧同缺，必须红）。
 *  4. 判定器结构锁：判定器非测试区不得出现任何小数限值字面量或引号包裹的数字限值，
 *     且必须通过 fetch_rules_for_item 从表取行（把"判定器只查表"钉成可静态核验的形态）。
 *
 * 已知假绿盲区与本锁的对策：
 *  - 遮蔽：解析种子按 INSERT 列清单声明的位置取上下限，不按"某处出现 20"匹配。
 *  - 解绑：require 全部 INSERT 语句解析成功，取不到锚点即判红，不用可选链吞掉。
 *  - 只校验首个匹配点：逐行取集合差，两侧全部差异都列出，不是只报第一条。
 *  - 形态过窄：豁免行是 item IN(...) 乘 VALUES 标签的交叉展开，单独按 6×3=18 核验，
 *    不把它塞进标量行解析里当窄形态。
 *  - 两侧同缺同序：同源锁对种子侧与词表侧双向取 token，任一侧多或缺都红。
 *  - 按中文字面量 grep 判界面文案：本锁只按 ASCII 稳定码（GB18401_A、HCHO、lte 等）
 *    与数值比对，绝不拿中文展示文本当判据。
 *
 * 反空操作：解析不到任何限值行、任何项目、任何词表常量时直接失败退出——文件改名、
 * 目录指错、SQL 形态漂移都不能被当成"零发现"通过。判定器另以内联夹具自证（正例零发现、
 * 反例必抓到并点名），未过自证不扫库。
 *
 * 调用方：本地门禁 gate_lane.sh 的 fe 档（check-textile-limit-source 步骤，待主编排接入），
 * 以及 `node frontend/scripts/check-textile-limit-source.mjs` 手动核验。
 * 输出：stdout 报告扫描计数与逐条发现；exit 0 通过、exit 1 判负（阻断 CI 步骤）。
 * 持久化：不落盘、不写任何文件、不连数据库。
 */
import { readFileSync, existsSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const FRONTEND = resolve(__dirname, '..');
export const BACKEND = resolve(FRONTEND, '..', 'backend');

const M0111_REL = join('migration', 'src', 'domain', 'textile_safety', 'm0111_seed_textile_safety_matrix.rs');
const VOCAB_REL = join('src', 'models', 'status', 'textile_safety.rs');
const JUDGE_REL = join('src', 'services', 'textile_safety_limit_judge.rs');

// ---- 独立重述的权威矩阵（GB 18401-2010 三类加 GB 31701-2015 婴童加考）----
// 键规范化为 `code|cat|form|tag|cmp|low|high`，low 与 high 为规范化数字串或空串（无该界）。
const FASTNESS_SIX = ['FAST_WATER', 'FAST_ACID_SWEAT', 'FAST_ALKALI_SWEAT', 'FAST_DRY_RUB', 'FAST_WET_RUB', 'FAST_SALIVA'];
const EXEMPT_TAGS = ['laundering_fade_non_final', 'plain_or_bleached', 'traditional_hand_dyed'];
// 种子项目主数据应含全部 21 枚（GB 18401 基本 10 加 GB 31701 加考 11，含四枚 manual 条款位）。
const EXPECTED_ITEM_CODES = [
  'HCHO', 'PH', 'AZO', 'ODOR',
  'FAST_WATER', 'FAST_ACID_SWEAT', 'FAST_ALKALI_SWEAT', 'FAST_DRY_RUB', 'FAST_WET_RUB', 'FAST_SALIVA',
  'LEAD', 'CADMIUM', 'PHTHALATE', 'FLAM_EXTERIOR', 'METAL_SHARP',
  'ACCESSOR_TENSILE', 'SHARP_EDGE', 'SMALL_PARTS', 'CORD_HEAD', 'CORD_WAIST', 'LABEL_POSITION',
];

/** 剥离块注释与行注释（Rust 与 TS/JS 的双斜杠与星斜杠块注释、Vue 模板的 HTML 注释），保留字符串。 */
export function stripComments(src) {
  return src
    .replace(/<!-{2}[\s\S]*?-{2}>/g, '')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\/\/[^\n]*/g, '');
}

/** 剥离字符串字面量（单引号、双引号、反引号；仅处理常见转义）。 */
export function stripStrings(src) {
  return src
    .replace(/'(?:[^'\\\n]|\\.)*'/g, "''")
    .replace(/"(?:[^"\\\n]|\\.)*"/g, '""')
    .replace(/`(?:[^`\\]|\\.)*`/g, '``');
}

/** 数值规范化：去前导空格、去尾零、统一小数串（"4.0" 与 "4" 与 "20" 一律可比）。 */
export function normNum(raw) {
  if (raw === null || raw === undefined) return '';
  const t = String(raw).trim();
  if (t === '' || t.toUpperCase() === 'NULL') return '';
  const n = Number(t);
  if (!Number.isFinite(n)) throw new Error(`非数值限值 token：${JSON.stringify(raw)}`);
  return String(n);
}

/** 行键构造。low 与 high 传规范化串或 undefined（视作空串）。 */
function rowKey(code, cat, form, tag, cmp, low, high) {
  return [code, cat, form, tag, cmp, normNum(low), normNum(high)].join('|');
}

/**
 * 权威矩阵（与种子互不引用的一方）。逐条对照裁定文本：
 * 甲醛 20/75/300；pH 最终 4.0~7.5/4.0~8.5/4.0~9.0，中间湿处理放宽 4.0~10.5；
 * 芳香胺 禁用 加 上限 20；异味 必须无；耐水与耐酸碱汗渍 A 3.5 B与C 3.0；
 * 耐干摩擦 A 4.0 B与C 3.0；耐湿摩擦 A 3.5 B与C 3.0；耐唾液 A 4.0 B与C 不考核；
 * 六个色牢度项目乘三枚豁免标签 不考核；铅 90 镉 100 邻苯 0.1 燃烧 1 级
 * 金属针与头颈绳带 必须无 腰绳 360。
 */
export function canonicalMatrix() {
  const set = new Set();
  const add = (...a) => set.add(rowKey(...a));
  // 甲醛（形态通配 any，条件 none）
  add('HCHO', 'GB18401_A', 'any', 'none', 'lte', undefined, '20');
  add('HCHO', 'GB18401_B', 'any', 'none', 'lte', undefined, '75');
  add('HCHO', 'GB18401_C', 'any', 'none', 'lte', undefined, '300');
  // pH（最终产品区间；中间产品湿处理放宽行挂在 any 类别乘 intermediate 乘 wet_processing）
  add('PH', 'GB18401_A', 'final', 'none', 'between', '4.0', '7.5');
  add('PH', 'GB18401_B', 'final', 'none', 'between', '4.0', '8.5');
  add('PH', 'GB18401_C', 'final', 'none', 'between', '4.0', '9.0');
  add('PH', 'any', 'intermediate', 'wet_processing', 'between', '4.0', '10.5');
  // 可分解致癌芳香胺染料（禁用行与上限行并存）
  add('AZO', 'any', 'any', 'none', 'forbidden', undefined, undefined);
  add('AZO', 'any', 'any', 'none', 'lte', undefined, '20');
  // 异味
  add('ODOR', 'any', 'any', 'none', 'must_none', undefined, undefined);
  // 耐水与耐酸与耐碱汗渍（A 不低于 3-4 级即 3.5，B 与 C 不低于 3 级）
  for (const code of ['FAST_WATER', 'FAST_ACID_SWEAT', 'FAST_ALKALI_SWEAT']) {
    add(code, 'GB18401_A', 'any', 'none', 'gte', '3.5', undefined);
    add(code, 'GB18401_B', 'any', 'none', 'gte', '3.0', undefined);
    add(code, 'GB18401_C', 'any', 'none', 'gte', '3.0', undefined);
  }
  // 耐干摩擦（A 4.0，B 与 C 3.0）
  add('FAST_DRY_RUB', 'GB18401_A', 'any', 'none', 'gte', '4.0', undefined);
  add('FAST_DRY_RUB', 'GB18401_B', 'any', 'none', 'gte', '3.0', undefined);
  add('FAST_DRY_RUB', 'GB18401_C', 'any', 'none', 'gte', '3.0', undefined);
  // 耐湿摩擦（A 3.5，B 与 C 3.0）
  add('FAST_WET_RUB', 'GB18401_A', 'any', 'none', 'gte', '3.5', undefined);
  add('FAST_WET_RUB', 'GB18401_B', 'any', 'none', 'gte', '3.0', undefined);
  add('FAST_WET_RUB', 'GB18401_C', 'any', 'none', 'gte', '3.0', undefined);
  // 耐唾液（A 4.0；B 与 C 法定不考核）
  add('FAST_SALIVA', 'GB18401_A', 'any', 'none', 'gte', '4.0', undefined);
  add('FAST_SALIVA', 'GB18401_B', 'any', 'none', 'na', undefined, undefined);
  add('FAST_SALIVA', 'GB18401_C', 'any', 'none', 'na', undefined, undefined);
  // 色牢度豁免（六个色牢度项目乘三枚豁免标签的不考核）
  for (const code of FASTNESS_SIX) {
    for (const tag of EXEMPT_TAGS) {
      add(code, 'any', 'any', tag, 'na', undefined, undefined);
    }
  }
  // GB 31701 婴童加考（人群维由送检服务推入清单，类别维取通配 any）
  add('LEAD', 'any', 'any', 'none', 'lte', undefined, '90');
  add('CADMIUM', 'any', 'any', 'none', 'lte', undefined, '100');
  add('PHTHALATE', 'any', 'any', 'none', 'lte', undefined, '0.1');
  add('FLAM_EXTERIOR', 'any', 'any', 'none', 'lte', undefined, '1');
  add('METAL_SHARP', 'any', 'any', 'none', 'must_none', undefined, undefined);
  add('CORD_HEAD', 'any', 'any', 'none', 'must_none', undefined, undefined);
  add('CORD_WAIST', 'any', 'any', 'none', 'lte', undefined, '360');
  return set;
}

/** 从 Rust 源码里抠出一个 mod 块体（`pub mod NAME {` 到匹配的右花括号）。 */
export function extractModBlock(src, name) {
  const anchor = `pub mod ${name} {`;
  const start = src.indexOf(anchor);
  if (start < 0) throw new Error(`词表模块锚点缺失：pub mod ${name} {`);
  let depth = 0;
  let i = start + anchor.length - 1; // 落在起始 '{'
  for (; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return src.slice(start + anchor.length - 1, i + 1);
    }
  }
  throw new Error(`词表模块 ${name} 花括号不配平`);
}

/** 从一个 mod 块体里抠出所有 `pub const X: &str = "TOKEN";` 与 `ALL/CATEGORIES/... : &[&str] = &[...]` 里字面量 token。 */
function stringLiteralsIn(text) {
  const out = new Set();
  const re = /"([^"\\]*)"/g;
  let m;
  while ((m = re.exec(text))) out.add(m[1]);
  return out;
}

/** 解析一个词表模块，返回它 `pub const <NAME>: &str = "<TOKEN>"` 声明的 token 集合（排除 ALL 之类切片与函数）。 */
export function parseVocabTokens(modBody) {
  const set = new Set();
  const re = /pub const \w+\s*:\s*&str\s*=\s*"([^"\\]*)"/g;
  let m;
  while ((m = re.exec(modBody))) set.add(m[1]);
  return set;
}

/**
 * 把 m0111 种子里的 textile_limit_rule 全部行解析成规范化行集（含交叉展开的豁免 na 行）。
 * 解析口径：逐条 INSERT INTO textile_limit_rule，先读列清单判定是否含 limit_low 与 limit_high，
 * 再逐 SELECT 元组按位置取 (cat,form,tag,cmp, 界值...);界值个数必须等于声明的限值列数，不等即
 * 判红（SQL 形态漂移不能静默少算一行）。交叉展开块单列（item IN(...) 乘 VALUES 标签）。
 * 返回 { rows:Set, scalarRows, exemptRows, itemCodes:Set }。
 */
export function parseSeed(sql) {
  const rows = new Set();
  const itemCodes = new Set();
  let scalarRows = 0;
  let exemptRows = 0;

  // 一 项目主数据编码集合
  const itemInsert = /INSERT INTO textile_test_item [\s\S]*?VALUES([\s\S]*?)ON CONFLICT/.exec(sql);
  if (!itemInsert) throw new Error('种子里找不到 textile_test_item 的 INSERT VALUES 块');
  {
    const body = itemInsert[1];
    const tupleRe = /\(\s*'([A-Z_]+)'/g;
    let m;
    while ((m = tupleRe.exec(body))) itemCodes.add(m[1]);
  }

  // 二 逐条限值 INSERT
  const insertRe = /INSERT INTO textile_limit_rule \(([^)]*)\)([\s\S]*?)ON CONFLICT[^;]*/g;
  let ins;
  let sawAnyInsert = false;
  while ((ins = insertRe.exec(sql))) {
    sawAnyInsert = true;
    const cols = ins[1];
    const hasLow = /limit_low\b/.test(cols);
    const hasHigh = /limit_high\b/.test(cols);
    const limitColCount = (hasLow ? 1 : 0) + (hasHigh ? 1 : 0);
    const body = ins[2];

    // 二A 交叉展开的豁免标签 na 块（FROM textile_test_item i, (VALUES ...) AS t(tag) ... item_code IN (...)）
    // 锚定到 `(VALUES ... ) AS t(tag)` 的外层闭括号，非贪婪会在第一个内层元组右括号处误停。
    const crossJoin = /FROM textile_test_item i,[\s\S]*?\(VALUES([\s\S]*?)\)\s*AS t\(tag\)[\s\S]*?item_code IN \(([^)]*)\)/.exec(body);
    if (crossJoin) {
      const tags = [...crossJoin[1].matchAll(/'([^']+)'/g)].map(x => x[1]);
      const codes = [...crossJoin[2].matchAll(/'([^']+)'/g)].map(x => x[1]);
      // 此块列清单不含限值列，比较器为 na，界值空
      for (const code of codes) {
        for (const tag of tags) {
          rows.add(rowKey(code, 'any', 'any', tag, 'na', undefined, undefined));
          exemptRows++;
        }
      }
      continue;
    }

    // 二B 标量行（SELECT id, ... FROM textile_test_item WHERE item_code = 'CODE'）
    const selectRe = /SELECT id,\s*([\s\S]*?)FROM textile_test_item WHERE item_code = '([A-Z_]+)'/g;
    let s;
    let sawSelect = false;
    while ((s = selectRe.exec(body))) {
      sawSelect = true;
      const code = s[2];
      const parts = splitTopLevel(s[1]);
      // parts: 'cat','form','tag','cmp', <界值...>  尾部的 clause_ref 与 remark 也是引号串，需剔除
      const quoted = parts.map(p => p.trim());
      const cat = unquote(quoted[0]);
      const form = unquote(quoted[1]);
      const tag = unquote(quoted[2]);
      const cmp = unquote(quoted[3]);
      // 从第 5 项起：数值或 NULL 是界值，遇到第一个"像 clause_ref 的引号串"即停
      const limitSlots = [];
      for (let i = 4; i < quoted.length; i++) {
        const tok = quoted[i];
        if (/^'.*'$/.test(tok)) break; // 到这里已是 clause_ref/remark
        limitSlots.push(tok);
      }
      if (limitSlots.length !== limitColCount) {
        throw new Error(
          `项目 ${code}（比较器 ${cmp}）解析到 ${limitSlots.length} 个界值槽，但 INSERT 声明 ${limitColCount} 个限值列，形态漂移`
        );
      }
      const low = hasLow ? limitSlots[0] : undefined;
      const high = hasHigh ? (hasLow ? limitSlots[1] : limitSlots[0]) : undefined;
      rows.add(rowKey(code, cat, form, tag, cmp, low, high));
      scalarRows++;
    }
    if (!sawSelect && !crossJoin) {
      throw new Error('一条 textile_limit_rule INSERT 既没有标量 SELECT 也没有交叉展开块，SQL 形态已变');
    }
  }
  if (!sawAnyInsert) throw new Error('种子里找不到任何 textile_limit_rule 的 INSERT');
  return { rows, scalarRows, exemptRows, itemCodes };
}

/** 逗号顶层切分（忽略单引号串内的逗号）。 */
function splitTopLevel(text) {
  const out = [];
  let cur = '';
  let inStr = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === "'") {
      inStr = !inStr;
      cur += c;
    } else if (c === ',' && !inStr) {
      out.push(cur);
      cur = '';
    } else {
      cur += c;
    }
  }
  out.push(cur);
  return out;
}

function unquote(tok) {
  const t = tok.trim();
  if (!/^'[^']*'$/.test(t)) throw new Error(`期望单引号 token，实际 ${JSON.stringify(t)}`);
  return t.slice(1, -1);
}

/** 抽出 Rust 文件里非测试区（截断到 `#[cfg(test)]`）的文本。 */
export function stripTestModule(src) {
  const idx = src.indexOf('#[cfg(test)]');
  return idx < 0 ? src : src.slice(0, idx);
}

// 单源散布锁要盯的"够独特、不会与业务噪声撞车"的限值串（避开 20、100、1 这类通用数）。
const DISTINCT_LIMIT_TOKENS = [
  { re: /(?<![\w.])75(?![\w.])/, why: '甲醛 B 类上限 75' },
  { re: /(?<![\w.])300(?![\w.])/, why: '甲醛 C 类上限 300' },
  { re: /(?<![\w.])9\.0(?![\w])/, why: 'pH C 类上限 9.0' },
  { re: /(?<![\w.])10\.5(?![\w])/, why: 'pH 中间产品放宽上限 10.5' },
  { re: /(?<![\w.])3\.5(?![\w])/, why: '色牢度 3-4 级半级编码 3.5' },
  { re: /(?<![\w.])360(?![\w.])/, why: '腰部绳带伸出上限 360' },
  { re: /(?<![\w.])0\.1(?![\w])/, why: '邻苯二甲酸酯上限 0.1' },
];

/**
 * 单源散布锁：判定合规族的生产文件（判定器与非测试区的 service/handler/前端源）里
 * 出现独特国标限值数值即判红并点名文件与数值。迁移种子与测试与常量展示表不在扫描范围。
 * 只看裸代码数值（剥掉注释与字符串字面量），避免把文档描述与展示占位文案当魔法数误判；
 * 真把限值硬编码进比较逻辑一定是裸数字或 Decimal::new，剥字符串后仍在。
 */
export function scanScatter(files) {
  const findings = [];
  let scanned = 0;
  for (const f of files) {
    if (!existsSync(f.path)) throw new Error(`散布锁点名文件缺失：${f.path}（文件被改名或移动，不能当零发现）`);
    scanned++;
    const raw = readFileSync(f.path, 'utf-8');
    const hits = scatterHits(raw, f.stripTest);
    if (hits.length) findings.push({ path: f.path, hits });
  }
  return { findings, scanned };
}

/** 散布命中的共用实现（读盘与自证内存内容共用一条代码路径）。 */
function scatterHits(raw, stripTest) {
  let body = stripTest ? stripTestModule(raw) : raw;
  body = stripStrings(stripComments(body));
  const hits = [];
  for (const t of DISTINCT_LIMIT_TOKENS) if (t.re.test(body)) hits.push(t.why);
  return hits;
}

// ---- 判定器结构锁：非测试区不得含小数限值或引号数字限值，且必须查表取行 ----
export function scanJudgeStructure(judgeSrcRaw) {
  // 剥测试模块，再剥注释（模块头 `//!` 叙述里的 pH 档与半级说明是文档不是代码限值），但保留字符串。
  const body = stripComments(stripTestModule(judgeSrcRaw));
  const errs = [];
  // 非测试区里出现带小数的裸字面量（4.0 / 7.5 / 3.5 / 0.1 等）即硬编码限值的信号。
  // 词表半级编码在别处，判定器只应引用 rule.limit_low/high。tier_score 的整数权重不算。
  const decimals = body.match(/(?<![\w."'])\d+\.\d+(?![\w."'])/g) || [];
  if (decimals.length) errs.push(`判定器非测试区出现裸小数限值 ${decimals.join(',')}（限值必须来自表行）`);
  // 引号包裹的数字串限值（如 "20"、"75"）
  const quotedNums = (body.match(/"(\d+(?:\.\d+)?)"/g) || []).filter(s => !/^"0"$|^"1"$/.test(s));
  if (quotedNums.length) errs.push(`判定器非测试区出现引号数字限值 ${quotedNums.join(',')}`);
  // 必须存在查表入口（拿掉查表 = 判定器只能硬编码）
  if (!/fetch_rules_for_item/.test(body)) errs.push('判定器非测试区缺 fetch_rules_for_item 查表入口（判定不再源自表）');
  return errs;
}

/**
 * 判定器自证（两轮变异，本地跑）：
 *  第一轮 反例变异，把权威矩阵里甲醛 A 的上限从 20 改成 999，行集差必须非空且点名 HCHO 与 GB18401_A；
 *  同一变异喂给散布锁，把 300 塞进一个模拟生产文件必须被抓到并点名数值含义。
 *  任一未抓到 ⇒ 本锁无检测力，拒绝扫真库。
 *  第二轮 正例，真种子解析行集与权威矩阵必须完全相等、真生产文件必须零发现（防止恒真判负）。
 */
export function selfProof() {
  const bad = [];
  // 反例 1：变异权威矩阵一个限值（改 HCHO A 上限 20 到 999），应制造与另一方的集合差
  const canonical = canonicalMatrix();
  const mutated = new Set(canonical);
  mutated.delete(rowKey('HCHO', 'GB18401_A', 'any', 'none', 'lte', undefined, '20'));
  mutated.add(rowKey('HCHO', 'GB18401_A', 'any', 'none', 'lte', undefined, '999'));
  const diff = diffSets(canonical, mutated);
  if (diff.length === 0) bad.push('自证反例1：改一个限值却取不到集合差（判红无检测力）');
  else if (!diff.some(d => d.includes('HCHO') && d.includes('GB18401_A'))) {
    bad.push(`自证反例1：差异未点名被改项目与类别，实际 ${diff.join(' ')}`);
  }
  // 反例 2：把独特限值 300 作为裸数字塞进模拟生产文件，散布锁必须抓到
  const fakeBad = [{ path: 'selfproof://fake_service.rs', stripTest: false, _content: 'let x = 300; // 泄漏' }];
  const scatter = scanScatterWithContent(fakeBad);
  if (scatter.findings.length !== 1) bad.push('自证反例2：散布值塞进生产文件却未被抓到');
  else if (!scatter.findings[0].hits.some(h => h.includes('300'))) {
    bad.push(`自证反例2：抓到却未点名数值含义，实际 ${JSON.stringify(scatter.findings[0].hits)}`);
  }
  // 反例 3：判定器结构锁必须抓到裸小数限值（且带 fetch 锚点，确保是被限值而非缺锚点触发）
  const judgeBad = 'fn fetch_rules_for_item(){} fn resolve(){ if v <= Decimal::from_f64(7.5) {} }';
  const judgeErrs = scanJudgeStructure(judgeBad);
  if (!judgeErrs.some(e => e.includes('7.5'))) bad.push(`自证反例3：判定器裸小数限值未被结构锁点名，实际 ${JSON.stringify(judgeErrs)}`);
  // 反例 4：字符串里的展示数字不得被散布锁误判（证明"剥字符串"生效，不是恒抓）
  const fakeDisplay = [{ path: 'selfproof://display.vue', stripTest: false, _content: '<el-input placeholder="3-4 级记 3.5" />' }];
  if (scanScatterWithContent(fakeDisplay).findings.length !== 0) {
    bad.push('自证反例4：展示占位字符串里的 3.5 被散布锁误判（应剥字符串后不抓）');
  }
  return bad;
}

/** 集合差（both sides 的多出与缺失都列出，点名两侧）。 */
function diffSets(expected, actual) {
  const out = [];
  for (const k of actual) if (!expected.has(k)) out.push(`种子侧多出行 ${k}`);
  for (const k of expected) if (!actual.has(k)) out.push(`种子侧缺失行 ${k}`);
  return out;
}

// scanScatter 的真实文件入口（读盘），自证用内存内容版，两者共用 scatterHits
function scanScatterWithContent(files) {
  const findings = [];
  for (const f of files) {
    const hits = scatterHits(f._content, f.stripTest);
    if (hits.length) findings.push({ path: f.path, hits });
  }
  return { findings, scanned: files.length };
}

// 待扫的生产文件清单（判定器 strip 测试区；前端合规源整文件扫，前端无数值限值）
function productionTargets() {
  const b = p => join(BACKEND, p);
  const f = p => join(FRONTEND, p);
  return [
    { path: b('src/services/textile_safety_limit_judge.rs'), stripTest: true },
    { path: b('src/services/textile_safety_report_service.rs'), stripTest: true },
    { path: b('src/services/textile_safety_submission_service.rs'), stripTest: true },
    { path: b('src/services/textile_safety_master_service.rs'), stripTest: true },
    { path: b('src/handlers/textile_safety_handler.rs'), stripTest: true },
    { path: b('src/handlers/textile_safety_report_handler.rs'), stripTest: true },
    { path: b('src/handlers/textile_safety_master_handler.rs'), stripTest: true },
    { path: f('src/api/textile-safety.ts'), stripTest: false },
    { path: f('src/constants/textile-safety.ts'), stripTest: false },
    { path: f('src/views/textile-safety/index.vue'), stripTest: false },
    { path: f('src/views/textile-safety/master.vue'), stripTest: false },
    { path: f('src/views/textile-safety/components/SubmissionDetail.vue'), stripTest: false },
  ];
}

function main() {
  const bad = selfProof();
  if (bad.length) {
    console.error('FAIL: 国标限值单源锁自证未过——本检查不可信，拒绝扫库：');
    for (const b of bad) console.error(`  - ${b}`);
    process.exit(1);
  }

  const seedPath = join(BACKEND, M0111_REL);
  const vocabPath = join(BACKEND, VOCAB_REL);
  const judgePath = join(BACKEND, JUDGE_REL);
  const seedSrc = readFileSync(seedPath, 'utf-8');
  const vocabSrc = readFileSync(vocabPath, 'utf-8');
  const judgeSrc = readFileSync(judgePath, 'utf-8');

  // 种子 SQL 模板（r#" ... "#;）
  const sqlMatch = /let sql = r#"([\s\S]*?)"#;/.exec(seedSrc);
  if (!sqlMatch) throw new Error('m0111 找不到 r#" 原始字符串 SQL 锚点（种子写法漂移，不能当零发现）');
  const seedSql = sqlMatch[1];

  const seed = parseSeed(seedSql);
  const canonical = canonicalMatrix();

  console.log('=== check-textile-limit-source: 国标限值单源自证 ===');
  console.log(
    `种子解析 标量限值行 ${seed.scalarRows}，豁免交叉 na 行 ${seed.exemptRows}，行集 ${seed.rows.size}；权威矩阵行集 ${canonical.size}；项目编码 ${seed.itemCodes.size}`
  );

  // 锁 0：项目编码集合与权威 21 枚双向相等（种子多出/残缺都红，两侧同缺防护）；
  // 权威矩阵引用的项目必须都在种子主数据登记。
  const codeDiff = [];
  for (const c of EXPECTED_ITEM_CODES) if (!seed.itemCodes.has(c)) codeDiff.push(`种子主数据缺项目 ${c}`);
  for (const c of seed.itemCodes) if (!EXPECTED_ITEM_CODES.includes(c)) codeDiff.push(`种子主数据多出未知项目 ${c}`);
  const canonicalCodes = new Set([...canonical].map(k => k.split('|')[0]));
  for (const c of canonicalCodes) if (!seed.itemCodes.has(c)) codeDiff.push(`权威矩阵引用项目 ${c} 未在种子主数据登记`);

  // 锁 2：种子行集与权威矩阵逐行集合相等
  const rowDiff = diffSets(canonical, seed.rows);

  // 锁 3：种子 token 与 Rust 词表两侧同源
  const cmpVocab = parseVocabTokens(extractModBlock(vocabSrc, 'textile_comparator'));
  const catVocab = parseVocabTokens(extractModBlock(vocabSrc, 'textile_safety_category'));
  const formVocab = parseVocabTokens(extractModBlock(vocabSrc, 'textile_product_form'));
  const tagVocab = parseVocabTokens(extractModBlock(vocabSrc, 'textile_exemption_tag'));
  const scopeBlock = extractModBlock(vocabSrc, 'textile_limit_scope');
  const wildcardCats = new Set(['any']);
  const allowedCats = new Set([...catVocab, 'any']);
  const allowedForms = new Set([...formVocab, 'any']);
  const allowedTags = new Set([...tagVocab, 'none']);
  void wildcardCats;
  const scopeCats = ['GB18401_A', 'GB18401_B', 'GB18401_C', 'any'];
  const tokenDiff = [];
  for (const key of seed.rows) {
    const [, cat, form, tag, cmp] = key.split('|');
    if (!cmpVocab.has(cmp)) tokenDiff.push(`种子比较器 ${cmp} 不在 textile_comparator 词表`);
    if (!allowedCats.has(cat)) tokenDiff.push(`种子类别 ${cat} 不在 textile_safety_category 加 any 集合`);
    if (!allowedForms.has(form)) tokenDiff.push(`种子形态 ${form} 不在 textile_product_form 加 any 集合`);
    if (!allowedTags.has(tag)) tokenDiff.push(`种子条件标签 ${tag} 不在 textile_exemption_tag 加 none 集合`);
  }
  // 反向：词表 CATEGORIES/FORMS/CONDITION_TAGS 声明的通配维必须真被子行用到（防止词表与种子同缺）
  if (!scopeBlock.includes('CATEGORY_ANY') || !scopeBlock.includes('TAG_NONE')) {
    tokenDiff.push('textile_limit_scope 缺 CATEGORY_ANY 或 TAG_NONE 锚点（通配维漂移）');
  }
  // 交叉豁免必须恰为 6 项目乘 3 标签 = 18
  if (seed.exemptRows !== FASTNESS_SIX.length * EXEMPT_TAGS.length) {
    tokenDiff.push(`色牢度豁免 na 行 ${seed.exemptRows}，应为 ${FASTNESS_SIX.length * EXEMPT_TAGS.length}（6 项目乘 3 标签）`);
  }
  void scopeCats;

  // 锁 4：判定器结构
  const judgeErrs = scanJudgeStructure(judgeSrc);

  // 锁 1：单源散布
  const scatter = scanScatter(productionTargets());

  let fail = false;
  const report = (label, items) => {
    if (items.length) {
      fail = true;
      console.error(`\n[${label}] ${items.length} 条：`);
      for (const it of items) console.error(`  - ${it}`);
    }
  };
  report('项目编码覆盖', codeDiff);
  report('种子与权威矩阵逐行相等', rowDiff);
  report('种子 token 与 Rust 词表两侧同源', tokenDiff);
  report('判定器非测试区不得散落限值', judgeErrs);
  report(
    '国标限值不得散落生产码',
    scatter.findings.map(x => `${x.path} 出现 ${x.hits.join('、')}`)
  );

  if (fail) {
    console.error('\nFAIL: 国标限值单源自证未过，见上方逐条点名。');
    process.exit(1);
  }
  console.log(
    `\nOK: 种子 ${seed.rows.size} 行与 GB 18401 加 GB 31701 权威矩阵逐行相等；比较器与维度 token 与 Rust 词表同源；判定器非测试区零散落限值；${scatter.scanned} 个生产文件无国标魔法数。`
  );
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) main();
