#!/usr/bin/env node
/**
 * 全项目禁步进数字输入框 静态判定器（零出现锁）。
 *
 * 用户 2026-10-25 硬红线 创建单据的数量 单价 税率 金额一律纯手写
 * 不允许任何带点击加减按钮的数字输入框，也不允许自造的加减步进交互
 * 只加 CSS 隐藏步进控件的原生按钮算假修 键盘上下键与长按仍在就说明控件没换
 * 因此这里从标签层面判定 控件必须换成 el-input 加校验器 范围与精度靠 rules 不靠按钮
 *
 * 判两件事
 * 一 规则 tag 全库扫描面里出现步进控件名 含模板标签 注释 CSS 选择器 动态组件名字符串
 *    e2e 定位器与注释 出现即违规 逐条点名 文件 行号 与该行内容
 *    扫描面 frontend/src frontend/e2e 以及存在时的 mobile-app
 *    目标为零 不设白名单 不设基线 任何一次出现都判红
 *    步进控件名由分段拼接构造 本文件自身源码不携带该字面量 无需自豁免特判
 * 二 规则 hand 自造加减步进按钮的模板迹象 点击把某个数值加一减一或按步长增减
 *    判据只认模板指令值里的显式增减操作与步进动词调用 以及 Minus 图标按钮
 *    不吃 i18n 里的 increase decrease 文案 不吃空单元格占位短横 不吃 Plus 新增行按钮
 *    避免把正常的增删行按钮与单选文案误判成步进
 *
 * 判负口径 两类计数之和为 0 才绿 否则点名全部违规行并退码 1
 *
 * 用法 node scripts/check-no-stepper-control.mjs [--verbose]
 *       node scripts/check-no-stepper-control.mjs --self-test
 *       self-test 用内联夹具双向自证 正例必抓 反例必不报 不扫仓库
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND = path.resolve(HERE, '..');
const REPO = path.resolve(FRONTEND, '..');

// 扫描根 前端源码与前端 e2e 以及存在时的移动端应用目录
const SCAN_ROOTS = [path.join(FRONTEND, 'src'), path.join(FRONTEND, 'e2e')];
const MOBILE = path.join(REPO, 'mobile-app');
if (fs.existsSync(MOBILE)) SCAN_ROOTS.push(MOBILE);

// 步进控件名的禁用形态 分段拼接构造，本文件自身源码不携带任何完整形态，无需自豁免特判
// 一 连字符族 小写中划线写法，含其子类增减按钮的类名与 CSS 选择器前缀点号写法
// 二 驼峰族 PascalCase 与 camelCase 两种大小写形态，覆盖自动导入写法 显式导入清单里的
//    组件名 动态组件名字符串 以及共享定位器的 kind 判别值
//    此前只按连字符族判定，驼峰写法整片隐形，实测漏掉四处真实步进框
const KEBAB_FORM = ['input', '-', 'number'].join('');
const CAMEL_FORM = ['Input', 'Number'].join('').toLowerCase();
const NAME_FORMS = [KEBAB_FORM, CAMEL_FORM];
// 夹具与文案沿用 kebab 族做示例，取形同旧常量的别名，避免判定器源码出现完整字面量
const STEPPER_LITERAL = KEBAB_FORM;

/**
 * 行级判定 命中任一禁用形态即违规 大小写不敏感
 * 调用方 scanFile 与 runSelfTest 的夹具，两侧共用同一谓词，保证自证的就是线上逻辑
 * 入参 line 单行原文
 * 传给谁 只回布尔给调用方计数与点名，不产数据不落库
 */
export function tagHit(line) {
  const lower = line.toLowerCase();
  return NAME_FORMS.some(form => lower.includes(form));
}

const SKIP_DIRS = new Set(['node_modules', 'dist', 'target', 'coverage']);
const TEXT_EXT = new Set([
  '.vue',
  '.ts',
  '.tsx',
  '.js',
  '.jsx',
  '.mjs',
  '.cjs',
  '.css',
  '.scss',
  '.html',
  '.json',
]);

function walkFiles(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP_DIRS.has(e.name) || e.name.startsWith('.')) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walkFiles(p, out);
    else if (TEXT_EXT.has(path.extname(e.name).toLowerCase())) out.push(p);
  }
  return out;
}

// ---------- 规则 hand 自造加减步进按钮 ----------
// 指令值里的显式自增自减与加减赋值（点击加一减一 正是被禁的交互）
const HANDLER_ASSIGN_RE =
  /@(?:click|mousedown|change)="([^"]*)"|v-on:(?:click|mousedown)="([^"]*)"/g;
const STEP_ASSIGN_RE = /(\+\+|--|\+=\s*[-\w.]+|-=\s*[-\w.]+)/;
// 指令值里调用步进动词函数（把某个数值字段按步长增减）
const STEP_VERB_RE =
  /@(?:click|mousedown)="[^"]*\b(step[Uu]p|step[Dd]own|increment\w*|decrement\w*|nudge\w*|bump[Uu]p|bump[Dd]own|incQty|decQty|increaseQty|decreaseQty|plusOne|minusOne)\s*\(/g;
// Element Plus Minus 图标按钮 本仓库 Minus 图标仅用于步进 出现即判步进
const MINUS_ICON_RE = /(?::icon|icon)\s*=\s*["']Minus["']|<Minus\b/g;

/**
 * 扫描单个 .vue 源文本的自造步进迹象
 * @param {string} source 文件全文
 * @returns {{ hand: number, handHits: string[] }}
 */
export function scanHandRules(source) {
  const handHits = [];
  for (const m of source.matchAll(HANDLER_ASSIGN_RE)) {
    const val = m[1] ?? m[2] ?? '';
    // 只对含加减赋值且像数值步进的处理函数计一次
    if (STEP_ASSIGN_RE.test(val)) handHits.push(val.trim());
  }
  for (const m of source.matchAll(STEP_VERB_RE)) {
    handHits.push(`${m[1]}() 调用`);
  }
  for (const m of source.matchAll(MINUS_ICON_RE)) {
    handHits.push(m[0].trim());
  }
  return { hand: handHits.length, handHits };
}

/**
 * 扫描单个文件全文 产出两类违规
 * @param {string} abs 文件绝对路径
 * @returns {{ tagLines: Array<{line: number, text: string}>, handHits: string[] }}
 */
export function scanFile(abs) {
  const source = fs.readFileSync(abs, 'utf8');
  const lines = source.split(/\r?\n/);
  const tagLines = [];
  for (let i = 0; i < lines.length; i++) {
    // 行级判定 同一行多处只点一次名 足以定位
    if (tagHit(lines[i])) {
      tagLines.push({ line: i + 1, text: lines[i].trim().slice(0, 160) });
    }
  }
  const { handHits } = abs.endsWith('.vue') ? scanHandRules(source) : { handHits: [] };
  return { tagLines, handHits };
}

/**
 * 遍历全部扫描根 汇总每个有违规的文件 键为相对 frontend 根的路径（正斜杠）
 */
export function scanTree() {
  const result = {};
  for (const root of SCAN_ROOTS) {
    for (const abs of walkFiles(root)) {
      const { tagLines, handHits } = scanFile(abs);
      if (!tagLines.length && !handHits.length) continue;
      const rel = path.relative(FRONTEND, abs).split(path.sep).join('/');
      result[rel] = { tagLines, handHits, count: tagLines.length + handHits.length };
    }
  }
  return result;
}

// ---------- 内联夹具自证 ----------
function runSelfTest() {
  const tag = '<' + STEPPER_LITERAL;
  const cases = [
    {
      name: '正例 步进控件开标签必被抓到',
      src: `<el-form-item label="数量">
  ${tag} v-model="form.qty" :min="0" :precision="2" />
</el-form-item>
`,
      expectTag: 1,
      expectHand: 0,
    },
    {
      name: '正例 注释与 CSS 选择器与定位器里的控件名必被抓到',
      src: `/* 残留提及 ${STEPPER_LITERAL} 也算假修未净 */
.foo .${STEPPER_LITERAL} input { width: 100px; }
const sel = '.${STEPPER_LITERAL} input';
`,
      expectTag: 3,
      expectHand: 0,
    },
    {
      name: '正例 动态组件工厂返回控件名字符串必被抓到',
      src: `switch (field.type) {
  case 'number':
    return '${STEPPER_LITERAL}';
}
`,
      expectTag: 1,
      expectHand: 0,
    },
    {
      name: '正例 自造点击加一减一指令必被抓到',
      src: `<button @click="form.qty++">+</button>
<button @click="form.qty--">-</button>
<button @click="form.price += 1">提价</button>
`,
      expectTag: 0,
      expectHand: 3,
    },
    {
      name: '正例 步进动词函数调用必被抓到',
      src: `<el-button @click="stepUp('quantity')">加</el-button>
<el-button @click="increaseQty(index)">加数量</el-button>
`,
      expectTag: 0,
      expectHand: 2,
    },
    {
      name: '正例 Minus 图标步进按钮必被抓到',
      src: `<el-button :icon="Minus" @click="onDec" />
<el-button><Minus /></el-button>
`,
      expectTag: 0,
      expectHand: 2,
    },
    {
      name: '反例 普通 el-input 不得被 tag 规则误报',
      src: `<el-input v-model="form.name" />
<el-input v-model="form.remark" type="textarea" />
`,
      expectTag: 0,
      expectHand: 0,
    },
    {
      name: '反例 i18n increase decrease 单选文案不得被 hand 规则误报',
      src: `<el-radio-group v-model="form.adjustmentType">
  <el-radio value="increase">{{ t('opt.increase') }}</el-radio>
  <el-radio value="decrease">{{ t('opt.decrease') }}</el-radio>
</el-radio-group>
<el-input v-model="form.amount" />
`,
      expectTag: 0,
      expectHand: 0,
    },
    {
      name: '反例 空单元格占位短横与 Plus 新增行按钮不得被 hand 规则误报',
      src: `<span v-else>-</span>
<el-button type="primary" :icon="Plus" @click="addItem">新增明细</el-button>
<el-button type="danger" :icon="Delete" @click="removeItem(index)" />
`,
      expectTag: 0,
      expectHand: 0,
    },
    {
      name: '反例 分页页码 change 普通赋值不得误报',
      src: `<el-pagination @current-change="p = $event" :current-page="p" />
`,
      expectTag: 0,
      expectHand: 0,
    },
    {
      name: '正例 PascalCase 开标签与显式导入组件名必被抓到',
      src: `<script setup>
import { ${['El', 'Input', 'Number'].join('')}, ElButton } from 'element-plus';
</script>
<template>
  <${['El', 'Input', 'Number'].join('')} v-model="form.orderId" :min="0" />
</template>
`,
      expectTag: 2,
      expectHand: 0,
    },
    {
      name: '正例 共享定位器的 kind 判别值与动态组件名字符串同族必被抓到',
      src: `type Field = { kind: '${['input', 'Number'].join('')}'; label: string };
const target: Field = { kind: '${['input', 'Number'].join('')}', label: '数量', value: 100 };
`,
      expectTag: 2,
      expectHand: 0,
    },
    {
      name: '正例 只隐藏步进原生按钮的 CSS 假修必被抓到',
      src: `/* 假修 按钮仍在只是看不见 */
.${STEPPER_LITERAL}__increase,
.${STEPPER_LITERAL}__decrease {
  display: none;
}
`,
      expectTag: 2,
      expectHand: 0,
    },
    {
      name: '反例 替代控件与 inputmode 属性不得被误报',
      src: `<NumericInput v-model="form.qty" integer :min="0" />
<el-input v-model="form.code" inputmode="numeric" />
// 数值录入统一走 NumericInput，底层是 el-input
`,
      expectTag: 0,
      expectHand: 0,
    },
  ];
  let pass = 0;
  let fail = 0;
  console.log('=== self-test check-no-stepper-control 双向自证 ===');
  for (const c of cases) {
    // 夹具走与仓库扫描完全相同的行级判定 保证自证的就是线上逻辑
    const gotTag = c.src.split(/\r?\n/).filter(l => tagHit(l)).length;
    const gotHand = scanHandRules(c.src).hand;
    if (gotTag === c.expectTag && gotHand === c.expectHand) {
      pass++;
      console.log(`  PASS  ${c.name}  [tag=${gotTag} hand=${gotHand}]`);
    } else {
      fail++;
      console.log(
        `  FAIL  ${c.name}  -> 期望 tag=${c.expectTag} hand=${c.expectHand} 实际 tag=${gotTag} hand=${gotHand}`
      );
    }
  }
  // 自检 判定器自身源码必须不携带任何禁用形态 否则零出现锁永无绿日
  const selfSource = fs.readFileSync(fileURLToPath(import.meta.url), 'utf8').toLowerCase();
  const selfCarried = NAME_FORMS.filter(form => selfSource.includes(form));
  if (selfCarried.length) {
    fail++;
    console.log(`  FAIL  判定器自身源码携带禁用形态 ${selfCarried.length} 类 无法自证豁免`);
  } else {
    pass++;
    console.log('  PASS  判定器自身源码不携带任何禁用形态');
  }
  // 地板 禁用形态集合为空说明判定器被改坏 必须判红而不是"零违规"
  if (NAME_FORMS.length < 2) {
    fail++;
    console.log('  FAIL  禁用形态集合少于两类 kebab 与 PascalCase 族缺位，零出现锁无检测力');
  } else {
    pass++;
    console.log('  PASS  地板 禁用形态 >= 2  [forms=' + NAME_FORMS.length + ']');
  }
  console.log(`---- self-test 结论 ${pass} 通过 / ${fail} 失败 ----`);
  if (fail) {
    console.error('[no-stepper] 判定器自证失败 存在漏报或误报 门禁不可信');
    process.exit(1);
  }
  console.log('[no-stepper] 判定器自证通过 正例必抓 反例必不报');
  process.exit(0);
}

// ---------- 主流程 ----------
function main() {
  if (!fs.existsSync(SCAN_ROOTS[0])) {
    console.error('[no-stepper] 扫描根不存在 ' + SCAN_ROOTS[0]);
    process.exit(1);
  }
  const current = scanTree();
  const files = Object.keys(current).sort();
  let totalTag = 0;
  let totalHand = 0;
  for (const f of files) {
    totalTag += current[f].tagLines.length;
    totalHand += current[f].handHits.length;
  }
  const total = totalTag + totalHand;

  if (process.argv.includes('--verbose')) {
    for (const f of files) {
      for (const t of current[f].tagLines) {
        console.log(`  ${f}:${t.line}: ${t.text}`);
      }
      for (const h of current[f].handHits) {
        console.log(`  ${f}: hand 自造步进 ${h}`);
      }
    }
  }
  console.log(
    `[no-stepper] 全库违规 ${total}（tag ${totalTag} + hand ${totalHand}）分布于 ${files.length} 个文件`
  );

  if (total > 0) {
    console.error(`\n[no-stepper] 失败 目标为 0 出现 实际 ${total} 处 逐条点名如下`);
    for (const f of files) {
      for (const t of current[f].tagLines) {
        console.error(`  ${f}:${t.line}: ${t.text}`);
      }
      for (const h of current[f].handHits) {
        console.error(`  ${f}: hand 自造步进 ${h}`);
      }
    }
    console.error(
      `\n口径 全项目禁止带加减按钮的数字输入框 ${STEPPER_LITERAL} 一律换 el-input 加校验器`
    );
    console.error('注释 CSS 选择器 动态组件名 e2e 定位器里的残留提及同样判红 假修不接受');
    console.error('自造点击加减步进交互同禁 新增数值录入项从落笔起用 el-input 加 rules');
    process.exit(1);
  }
  console.log('[no-stepper] 全库 0 出现 锁绿');
  process.exit(0);
}

const invokedDirectly =
  process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  if (process.argv.includes('--self-test')) runSelfTest();
  else main();
}
