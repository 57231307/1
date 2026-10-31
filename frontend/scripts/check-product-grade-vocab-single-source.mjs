#!/usr/bin/env node
/**
 * 产品等级词表四方同源对账锁（纸面锁，钉 m0122 与 m0123 的取值域承诺）。
 *
 * 为什么需要这一支。products.product_grade 的取值口径同时写在四个地方，后端 Rust 常量、
 * 前端 TS 常量、迁移 m0122 的点名谓词、迁移 m0123 施加的 CHECK 字面量。两支迁移的文件头都
 * 写着"由纸面锁钉彼此相等"，但对账判据本身不存在，等于把一致性交给注释自觉。任何一侧单独
 * 改动都会静默成立，而库里会出现一个 UI 给不出、API 认不下、CHECK 又拦不住的取值。
 *
 * 判什么。四侧解析出的有序取值清单必须逐项逐字符相等，且长度为 3、无重复、无空串。
 * 第五侧一并钉住，产品表单必须引用前端词表常量，不得再抄一份中文字面量，否则表单与
 * 词表会在某次只改一处的提交里分家。
 *
 * 调用方 主编排与人工复核，后续接入 CI 静态门时按同一入口调用。入参 无路径参数，
 * 仓库根由本文件位置推导。传给谁 只往 stdout 与 stderr 打印结论并以退出码交判，
 * 不产数据不落库。退出码 0 四侧同源，1 存在漂移或解析失败，8 目标文件缺失。
 *
 * 自证 用 --self-test 走内联夹具双向自证，四侧各自的漂移形态必抓、对齐形态必不报、
 * 解析空集必判红而不是"零问题"，夹具走与线上完全相同的解析与比对函数。
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND = path.resolve(HERE, '..');
const REPO = path.resolve(FRONTEND, '..');

const SOURCES = {
  rustVocab: path.join(REPO, 'backend/src/models/status/general.rs'),
  tsVocab: path.join(FRONTEND, 'src/constants/product-spec-vocab.ts'),
  migrationReport: path.join(
    REPO,
    'backend/migration/src/domain/product_grade_vocab_check/m0122_report_product_grade_vocab_dirty_values.rs'
  ),
  migrationApply: path.join(
    REPO,
    'backend/migration/src/domain/product_grade_vocab_check/m0123_apply_product_grade_vocab_check.rs'
  ),
  form: path.join(FRONTEND, 'src/views/product/tabs/ProductFormDialogTab.vue'),
};

/**
 * 从后端 general.rs 取产品等级有序清单，先读三档常量再按 ALL_GRADES 的引用顺序解析。
 * 取的是引用序而非声明序，词表顺序以 ALL_GRADES 为准，避免有人在常量区插一档就改到清单顺序。
 * 必须先切出 product_grade 子模块再解析，该文件里存在多个同名 ALL_GRADES（色牢度与库存等级
 * 各有一份），全文匹配会取到别的词表，实测第一处命中的是上一子模块的多行形态。
 */
export function parseRustVocab(text) {
  const segments = text.split(/\npub mod /);
  const mod = segments.find(seg => seg.startsWith('product_grade'));
  if (!mod) return null;
  const body = mod.split('\n}')[0];
  const named = new Map();
  for (const m of body.matchAll(/pub const ([A-Z_]+):\s*&str\s*=\s*"([^"]*)";/g)) {
    named.set(m[1], m[2]);
  }
  const list = body.match(/pub const ALL_GRADES:\s*&\[&str\]\s*=\s*&\[([\s\S]*?)\];/);
  if (!list) return null;
  const items = list[1]
    .split(',')
    .map(s => s.trim())
    .filter(Boolean);
  if (items.some(name => !named.has(name))) return null;
  return items.map(name => named.get(name));
}

/** 从前端词表取有序清单，按 PRODUCT_GRADE_VALUES 的引用序解析 PRODUCT_GRADE 的取值。 */
export function parseTsVocab(text) {
  const block = text.match(/export const PRODUCT_GRADE\s*=\s*\{([\s\S]*?)\}\s*as const;/);
  const list = text.match(/export const PRODUCT_GRADE_VALUES[^=]*=\s*\[([\s\S]*?)\];/);
  if (!block || !list) return null;
  const named = new Map();
  for (const m of block[1].matchAll(/^\s*([a-zA-Z_][\w]*)\s*:\s*'([^']*)',/gm)) {
    named.set(m[1], m[2]);
  }
  const items = list[1]
    .split(',')
    .map(s => s.trim())
    .filter(Boolean);
  if (!items.length) return null;
  return items.map(ref => {
    const key = ref.split('.').pop();
    return named.has(key) ? (named.get(key) ? named.get(key) : null) : null;
  });
}

/** 从迁移取 SQL 字面量清单，两侧共用本函数保证两侧判据同形。 */
export function parseMigrationVocab(text) {
  const m = text.match(/const PRODUCT_GRADE_ALLOWED:\s*&str\s*=\s*"([^"]*)";/);
  if (!m) return null;
  const inner = m[1].trim();
  if (!inner.length) return [];
  return inner.split(',').map(part => {
    const one = part.trim();
    const q = one.match(/^'([\s\S]*)'$/);
    return q ? q[1] : one;
  });
}

/**
 * 比对四侧清单与表单引用形态，返回全部问题点。
 * 空集或解析不出不返回"无问题"，一律记为判据失效，防判定器瞎掉时报绿。
 */
export function compareAll(parsed, formText) {
  const problems = [];
  const entries = Object.entries(parsed);
  for (const [name, list] of entries) {
    if (!Array.isArray(list) || !list.length) {
      problems.push(`${name} 解析不到词表清单，判据失效不得视为通过`);
    }
  }
  if (problems.length) return problems;

  const [firstName, firstList] = entries[0];
  if (firstList.length !== 3)
    problems.push(`${firstName} 档位数为 ${firstList.length}，本批定案为 3 档`);
  if (new Set(firstList).size !== firstList.length) problems.push(`${firstName} 词表存在重复档位`);
  if (firstList.some(v => !v || !v.trim())) problems.push(`${firstName} 词表存在空档或纯空白档`);

  for (const [name, list] of entries.slice(1)) {
    if (list.length !== firstList.length) {
      problems.push(`${name} 档位序数 ${list.length} 与 ${firstName} 的 ${firstList.length} 不等`);
      continue;
    }
    for (let i = 0; i < firstList.length; i++) {
      if (list[i] !== firstList[i]) {
        problems.push(
          `${name} 第 ${i + 1} 档为 "${list[i]}"，与 ${firstName} 的 "${firstList[i]}" 不逐字相等`
        );
      }
    }
  }

  if (!formText.includes('PRODUCT_GRADE_VALUES')) {
    problems.push('产品表单未引用前端词表常量 PRODUCT_GRADE_VALUES，等级清单已另立一套');
  }
  for (const value of firstList) {
    if (new RegExp(`['"]${value}['"]`).test(formText)) {
      problems.push(`产品表单出现中文字面量 "${value}"，词表改动时该处不会随动`);
    }
  }
  return problems;
}

function readOrAbort(file) {
  if (!fs.existsSync(file)) {
    console.error(`[grade-vocab-lock] 目标文件缺失 ${file}`);
    process.exit(8);
  }
  return fs.readFileSync(file, 'utf8');
}

function loadRepo() {
  const rustText = readOrAbort(SOURCES.rustVocab);
  const tsText = readOrAbort(SOURCES.tsVocab);
  return {
    parsed: {
      rust: parseRustVocab(rustText),
      ts: parseTsVocab(tsText),
      migrationReport: parseMigrationVocab(readOrAbort(SOURCES.migrationReport)),
      migrationApply: parseMigrationVocab(readOrAbort(SOURCES.migrationApply)),
    },
    formText: readOrAbort(SOURCES.form),
  };
}

function runSelfTest() {
  const rustGood =
    'pub mod stock_grade {\n    pub const ALL_GRADES: &[&str] = &["干扰项"];\n}\npub mod product_grade {\n    pub const A: &str = "甲";\n    pub const B: &str = "乙";\n    pub const C: &str = "丙";\n    pub const ALL_GRADES: &[&str] = &[A, B, C];\n}';
  const tsGood =
    "export const PRODUCT_GRADE = {\n  a: '甲',\n  b: '乙',\n  c: '丙',\n} as const;\nexport const PRODUCT_GRADE_VALUES: ProductGradeValue[] = [PRODUCT_GRADE.a, PRODUCT_GRADE.b, PRODUCT_GRADE.c];";
  const migGood = "const PRODUCT_GRADE_ALLOWED: &str = \"'甲','乙','丙'\";";
  const formGood =
    "import { PRODUCT_GRADE_VALUES } from '@/constants/product-spec-vocab';\nconst options = PRODUCT_GRADE_VALUES;";

  const cases = [
    {
      name: '反例 四侧逐项相等不得报',
      rust: rustGood,
      ts: tsGood,
      migA: migGood,
      migB: migGood,
      form: formGood,
      expectProblems: 0,
    },
    {
      name: '正例 后端少一档必抓',
      rust: 'pub mod stock_grade {\n    pub const ALL_GRADES: &[&str] = &["干扰项"];\n}\npub mod product_grade {\n    pub const A: &str = "甲";\n    pub const B: &str = "乙";\n    pub const C: &str = "丙";\n    pub const ALL_GRADES: &[&str] = &[A, B];\n}',
      ts: tsGood,
      migA: migGood,
      migB: migGood,
      form: formGood,
      // 少一档会级联成自身档位数不符加其余三侧序数不等，共四处点名，缺一即漏报
      expectProblems: 4,
    },
    {
      name: '正例 迁移字面量改序必抓',
      rust: rustGood,
      ts: tsGood,
      migA: "const PRODUCT_GRADE_ALLOWED: &str = \"'乙','甲','丙'\";",
      migB: migGood,
      form: formGood,
      expectProblems: 2,
    },
    {
      name: '正例 施加支与点名支两侧字面量不一致必抓',
      rust: rustGood,
      ts: tsGood,
      migA: migGood,
      migB: "const PRODUCT_GRADE_ALLOWED: &str = \"'甲','乙','丙丁'\";",
      form: formGood,
      expectProblems: 1,
    },
    {
      name: '正例 前端把末档改成带尾空格的近形词必抓',
      rust: rustGood,
      ts: "export const PRODUCT_GRADE = {\n  a: '甲',\n  b: '乙',\n  c: '丙 ',\n} as const;\nexport const PRODUCT_GRADE_VALUES: ProductGradeValue[] = [PRODUCT_GRADE.a, PRODUCT_GRADE.b, PRODUCT_GRADE.c];",
      migA: migGood,
      migB: migGood,
      form: formGood,
      // 档位数一致而第三档多一个尾空格，逐项比较必须点到该档，否则库里会出现 CHECK 拦不下的值
      expectProblems: 1,
    },
    {
      name: '正例 表单退回硬编码中文必抓',
      rust: rustGood,
      ts: tsGood,
      migA: migGood,
      migB: migGood,
      form: "<el-option label='甲' value='甲' />",
      expectProblems: 2,
    },
    {
      name: '地板 词表解析空集必须判红而不是零问题',
      rust: '// 词表常量不在这份文件里',
      ts: tsGood,
      migA: migGood,
      migB: migGood,
      form: formGood,
      expectProblems: 1,
    },
  ];

  let pass = 0;
  let fail = 0;
  console.log('=== self-test check-product-grade-vocab-single-source 双向自证 ===');
  for (const c of cases) {
    const parsed = {
      rust: parseRustVocab(c.rust),
      ts: parseTsVocab(c.ts),
      migrationReport: parseMigrationVocab(c.migA),
      migrationApply: parseMigrationVocab(c.migB),
    };
    const problems = compareAll(parsed, c.form);
    const label = problems.length === c.expectProblems ? 'PASS' : 'FAIL';
    if (label === 'PASS') pass++;
    else fail++;
    console.log(`  ${label}  ${c.name}  [问题=${problems.length} 期望=${c.expectProblems}]`);
    if (label === 'FAIL') {
      for (const p of problems) console.log(`        -> ${p}`);
    }
  }
  console.log(`---- self-test 结论 ${pass} 通过 / ${fail} 失败 ----`);
  if (fail) {
    console.error('[grade-vocab-lock] 判定器自证失败，存在漏报或误报，锁不可信');
    process.exit(1);
  }
  console.log('[grade-vocab-lock] 判定器自证通过，四侧漂移形态必抓、对齐形态必不报');
  process.exit(0);
}

function main() {
  const { parsed, formText } = loadRepo();
  const problems = compareAll(parsed, formText);
  console.log(
    `[grade-vocab-lock] 后端 ${JSON.stringify(parsed.rust)} 前端 ${JSON.stringify(parsed.ts)} ` +
      `m0122 ${JSON.stringify(parsed.migrationReport)} m0123 ${JSON.stringify(parsed.migrationApply)}`
  );
  if (problems.length) {
    console.error(`\n[grade-vocab-lock] 失败 词表四方同源不成立，共 ${problems.length} 处`);
    for (const p of problems) console.error(`  - ${p}`);
    console.error('\n口径 四侧取值清单必须逐项逐字符相等且同为三档，表单只准引用前端词表常量');
    console.error('任一侧单独改动即判红，改词表要同批改后端常量 前端常量 两支迁移与 DB CHECK');
    process.exit(1);
  }
  console.log('[grade-vocab-lock] 四侧清单逐项相等且表单引用词表，锁绿');
  process.exit(0);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.includes('--self-test')) runSelfTest();
  else main();
}
