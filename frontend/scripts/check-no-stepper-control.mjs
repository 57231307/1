#!/usr/bin/env node
/**
 * 全项目禁步进数字输入框 静态判定器（棘轮门禁）。
 *
 * 用户 2026-10-25 硬红线 创建单据的数量 单价 税率 金额一律纯手写
 * 不允许任何带点击加减按钮的数字输入框，也不允许自造的加减步进交互
 * 只加 CSS 隐藏 el-input-number 的原生按钮算假修 键盘上下键与长按仍在就说明控件没换
 * 因此这里从标签层面判定 控件必须换成 el-input 加校验器 范围与精度靠 rules 不靠按钮
 *
 * 判两件事
 * 一 规则 tag .vue 模板里出现 el-input-number 开标签（Element Plus 原生步进控件 全禁）
 * 二 规则 hand 自造加减步进按钮的模板迹象 点击把某个数值加一减一或按步长增减
 *      判据只认模板指令值里的显式增减操作与步进动词调用 以及 Minus 图标按钮
 *      不吃 i18n 里的 increase decrease 文案 不吃空单元格占位短横 不吃 Plus 新增行按钮
 *      避免把正常的增删行按钮与单选文案误判成步进
 *
 * 判负口径 存量按 文件 到 违规计数 记入基线 只允许下降 不允许上升
 * 单文件上升即失败并打印两类计数 新增文件出现违规同样失败
 * 某文件已收敛但基线未下调也判失败 逼着把基线跟着改紧
 *
 * 用法 node scripts/check-no-stepper-control.mjs [--write-baseline] [--verbose]
 *       node scripts/check-no-stepper-control.mjs --self-test
 *       self-test 用内联夹具双向自证 正例必抓 反例必不报 不扫仓库
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND = path.resolve(HERE, '..');
const SRC = path.join(FRONTEND, 'src');
const BASELINE = path.join(HERE, 'no-stepper-control-baseline.json');

const SKIP_DIRS = new Set(['node_modules', 'dist', 'target']);

function walkVue(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP_DIRS.has(e.name) || e.name.startsWith('.')) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walkVue(p, out);
    else if (e.name.endsWith('.vue')) out.push(p);
  }
  return out;
}

// ---------- 规则 tag el-input-number 开标签 ----------
// 只数开标签（含自闭合），闭合标签 </el-input-number 以斜杠起头不计入 一处使用记一次违规
const TAG_RE = /<el-input-number\b/g;

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
 * 扫描单个 .vue 源文本 返回两类违规计数
 * @param {string} source 文件全文
 * @returns {{ tag: number, hand: number, handHits: string[] }}
 */
export function scanVueSource(source) {
  const tag = (source.match(TAG_RE) || []).length;

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
  const hand = handHits.length;

  return { tag, hand, handHits };
}

/**
 * 遍历真实源码树 汇总每个有违规的文件 键为相对 frontend 根的路径（正斜杠）
 */
export function scanTree() {
  const files = walkVue(SRC);
  const result = {};
  for (const abs of files) {
    const rel = path.relative(FRONTEND, abs).split(path.sep).join('/');
    const { tag, hand } = scanVueSource(fs.readFileSync(abs, 'utf8'));
    if (tag > 0 || hand > 0) result[rel] = { tag, hand, count: tag + hand };
  }
  return result;
}

// ---------- 内联夹具自证 ----------
function runSelfTest() {
  const cases = [
    {
      name: '正例 el-input-number 开标签必被抓到',
      src: `<el-form-item label="数量">
  <el-input-number v-model="form.qty" :min="0" :precision="2" />
</el-form-item>
<el-input-number v-model="form.price" :min="0" />
`,
      expect: { tag: 2, hand: 0 },
    },
    {
      name: '正例 自造点击加一减一指令必被抓到',
      src: `<button @click="form.qty++">+</button>
<button @click="form.qty--">-</button>
<button @click="form.price += 1">提价</button>
`,
      expect: { tag: 0, hand: 3 },
    },
    {
      name: '正例 步进动词函数调用必被抓到',
      src: `<el-button @click="stepUp('quantity')">加</el-button>
<el-button @click="increaseQty(index)">加数量</el-button>
`,
      expect: { tag: 0, hand: 2 },
    },
    {
      name: '正例 Minus 图标步进按钮必被抓到',
      src: `<el-button :icon="Minus" @click="onDec" />
<el-button><Minus /></el-button>
`,
      expect: { tag: 0, hand: 2 },
    },
    {
      name: '反例 普通 el-input 不得被 tag 规则误报',
      src: `<el-input v-model="form.name" />
<el-input v-model="form.remark" type="textarea" />
`,
      expect: { tag: 0, hand: 0 },
    },
    {
      name: '反例 i18n increase decrease 单选文案不得被 hand 规则误报',
      src: `<el-radio-group v-model="form.adjustmentType">
  <el-radio value="increase">{{ t('opt.increase') }}</el-radio>
  <el-radio value="decrease">{{ t('opt.decrease') }}</el-radio>
</el-radio-group>
<el-input v-model="form.amount" />
`,
      expect: { tag: 0, hand: 0 },
    },
    {
      name: '反例 空单元格占位短横与 Plus 新增行按钮不得被 hand 规则误报',
      src: `<span v-else>-</span>
<el-button type="primary" :icon="Plus" @click="addItem">新增明细</el-button>
<el-button type="danger" :icon="Delete" @click="removeItem(index)" />
`,
      expect: { tag: 0, hand: 0 },
    },
    {
      name: '反例 分页页码 change 自减以外的普通赋值不得误报',
      src: `<el-pagination @current-change="p = $event" :current-page="p" />
`,
      expect: { tag: 0, hand: 0 },
    },
  ];
  let pass = 0;
  let fail = 0;
  console.log('=== self-test check-no-stepper-control 双向自证 ===');
  for (const c of cases) {
    const got = scanVueSource(c.src);
    const okTag = got.tag === c.expect.tag;
    const okHand = got.hand === c.expect.hand;
    if (okTag && okHand) {
      pass++;
      console.log(
        `  PASS  ${c.name}  [tag=${got.tag} hand=${got.hand}]`
      );
    } else {
      fail++;
      console.log(
        `  FAIL  ${c.name}  -> 期望 tag=${c.expect.tag} hand=${c.expect.hand} 实际 tag=${got.tag} hand=${got.hand}`
      );
      if (got.handHits.length) console.log(`        handHits=${JSON.stringify(got.handHits)}`);
    }
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
  const current = scanTree();
  const files = Object.keys(current).sort();
  let totalTag = 0;
  let totalHand = 0;
  for (const f of files) {
    totalTag += current[f].tag;
    totalHand += current[f].hand;
  }
  const total = totalTag + totalHand;

  if (process.argv.includes('--write-baseline')) {
    const payload = {
      note: '全项目禁步进数字输入框棘轮基线 开工时全库违规快照 只减不增 规则 tag=el-input-number 标签 hand=自造加减步进按钮',
      total,
      files: Object.fromEntries(
        files.map((f) => [f, { tag: current[f].tag, hand: current[f].hand, count: current[f].count }])
      ),
    };
    fs.writeFileSync(BASELINE, JSON.stringify(payload, null, 2) + '\n');
    console.log(
      `[no-stepper] 基线已写入 违规 ${total}（tag ${totalTag} + hand ${totalHand}）分布于 ${files.length} 个文件`
    );
    process.exit(0);
  }

  if (!fs.existsSync(BASELINE)) {
    console.error('[no-stepper] 缺基线文件 scripts/no-stepper-control-baseline.json 先跑 --write-baseline');
    process.exit(1);
  }
  const baseline = JSON.parse(fs.readFileSync(BASELINE, 'utf8'));
  const baseFiles = baseline.files || {};

  const failures = [];
  const reduced = [];
  for (const f of files) {
    const c = current[f];
    const b = baseFiles[f];
    if (b === undefined) {
      failures.push(`新增违规文件 ${f} tag=${c.tag} hand=${c.hand} 基线内不存在`);
    } else if (c.count > b.count) {
      failures.push(
        `${f} 违规由 ${b.count}（tag ${b.tag} hand ${b.hand}）增至 ${c.count}（tag ${c.tag} hand ${c.hand}）`
      );
    } else if (c.count < b.count) {
      reduced.push(`${f} ${b.count} -> ${c.count}（请连同基线一起下调）`);
    }
  }
  // 基线里记录但当前已完全消失的文件 也要求从基线移除
  for (const f of Object.keys(baseFiles)) {
    if (current[f] === undefined) reduced.push(`${f} ${baseFiles[f].count} -> 0（已从源码消失 请从基线移除）`);
  }

  if (process.argv.includes('--verbose')) {
    for (const f of files) console.log(`  ${f}: tag=${current[f].tag} hand=${current[f].hand}`);
  }
  console.log(
    `[no-stepper] 当前违规 ${total}（tag ${totalTag} + hand ${totalHand}）文件 ${files.length} 基线总计 ${baseline.total}`
  );

  if (failures.length) {
    console.error(`\n[no-stepper] 失败 ${failures.length} 个文件出现新增步进控件违规`);
    for (const m of failures) console.error(`  ${m}`);
    console.error('\n口径 全项目禁止带加减按钮的数字输入框 el-input-number 一律换 el-input 加校验器');
    console.error('自造点击加减步进交互同禁 新增数值录入项从落笔起用 el-input 加 rules');
    process.exit(1);
  }
  if (reduced.length) {
    console.error(
      '\n[no-stepper] 有文件已收敛但基线未下调 请跑 node scripts/check-no-stepper-control.mjs --write-baseline'
    );
    for (const m of reduced) console.error(`  已收敛 ${m}`);
    process.exit(1);
  }
  console.log('[no-stepper] 无新增步进控件违规');
  process.exit(0);
}

const invokedDirectly =
  process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  if (process.argv.includes('--self-test')) runSelfTest();
  else main();
}
