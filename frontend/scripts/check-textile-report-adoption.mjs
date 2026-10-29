#!/usr/bin/env node
/**
 * 纺织品安全合规域 第三方报告采信规则静态锁（不连库、本地可跑、CI 常规可跑）
 *
 * 钉什么（任一不满足即 exit 1，逐条点名，绝不把"没查出问题"当通过）：
 *  1. 定义唯一锁：采信函数 `select_adopted_reports` 与有效期窗口函数 `validity_window_on`
 *     只允许定义在 `backend/src/services/textile_safety_report_service.rs` 一处；
 *     其它生产文件出现同名定义即红（第二份实现就是第二套采信规则）。
 *  2. 采信单源锁：送检服务与三个 handler 里不得出现第二套受理判式或第二套日期比法
 *     （`report_acceptance_status::ACCEPTED` 自写分支、`today >` 一类裸日期比较、
 *     送检服务里直接取 `.valid_until` 列）；采信结论只能来自上面那两个函数。
 *  3. 至多一份锁：`select_adopted_reports` 体内必须同时出现 `.first()`、`.skip(1)`
 *     与 `report_adoption_reason::SUPERSEDED`——即"排序后首位唯一采信、其余逐枚点名替代"，
 *     少其一就意味着要么多份同时采信，要么被替代的报告被静默丢掉。
 *  4. 有效期双列锁：`validity_window_on` 必须同时消费 `issue_date` 与 `valid_until`
 *     并产出 `NotYetValid` 档；报告级判定 `judge_report` 必须调用本函数
 *     （过期报告不得被判定器采信，且"出具日参与判定"不能只写在注释里）。
 *  5. 排除必点名锁：窗口三档失败（Expired、WindowMissing、NotYetValid）都必须映射到
 *     `report_adoption_reason` 词表常量；未受理档必须点名 `NOT_ACCEPTED`；
 *     `run_acceptance_gate` 的资质档必须按"两枚都缺才拒且逐枚点名"的形态，
 *     有效期档必须调用 `validity_window_on` 而不是裸比 `valid_until`。
 *  6. 词表同源锁：Rust `report_adoption_reason` 模块声明的取值集与本锁独立重述的六枚
 *     双向相等，且 `display_label` 必须覆盖每一枚；前端 `ADOPTION_REASON_LABELS` 的键集
 *     与 Rust 侧双向相等（三端同源，前端不得多一枚或少一枚）。
 *  7. 前端只消费锁：送检详情视图必须消费后端 `adoption` 结论，且不得自行按出具日挑最新
 *     （出现对 issue_date 排序或取最大的本地计算即红）。
 *
 * 已知假绿盲区与对策：
 *  - 注释谎报：所有判据都先剥注释（含 `//!` 与 `///` 与块注释），注释里写"已实现采信"不算数。
 *  - 按中文字面量判行为：本锁只按 ASCII 符号名（函数名、常量路径、方法名）比对，
 *    绝不拿中文文案当判据。
 *  - 只校验首个匹配点：逐条锁把全部发现都列出，不是报第一条就停。
 *  - 文件改名或目录指错：点名文件缺失直接抛错退出，不被当成"零发现"通过。
 *  - 恒真判负：`--self-test` 用五组变异反例证明每条锁真有检测力，再加一组正例证明不误判；
 *    自证不过即拒绝扫真库。
 *
 * 调用方：本地门禁与 CI 常规步骤；手动核验 `node frontend/scripts/check-textile-report-adoption.mjs --self-test`。
 * 输出：stdout 报告逐条锁的判定与发现；exit 0 通过、exit 1 判负（阻断 CI 步骤）。
 * 持久化：不落盘、不写任何文件、不连数据库。
 */
import { readFileSync, existsSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const FRONTEND = resolve(__dirname, '..');
export const BACKEND = resolve(FRONTEND, '..', 'backend');

const REL = {
  reportService: join('src', 'services', 'textile_safety_report_service.rs'),
  submissionService: join('src', 'services', 'textile_safety_submission_service.rs'),
  submissionHandler: join('src', 'handlers', 'textile_safety_handler.rs'),
  reportHandler: join('src', 'handlers', 'textile_safety_report_handler.rs'),
  masterHandler: join('src', 'handlers', 'textile_safety_master_handler.rs'),
  vocab: join('src', 'models', 'status', 'textile_safety.rs'),
  constants: join('src', 'constants', 'textile-safety.ts'),
  detailView: join('src', 'views', 'textile-safety', 'components', 'SubmissionDetail.vue'),
};

/** 独立重述的采信原因六枚（与 Rust 词表互不引用的一方，交叉验证用） */
export const CANONICAL_ADOPTION_REASONS = [
  'adopted',
  'superseded',
  'expired',
  'valid_window_missing',
  'not_yet_valid',
  'not_accepted',
];

/** 采信函数与窗口函数的定义文件（其余生产文件只准消费，不准另写） */
export const DEFINITION_FILE = 'src/services/textile_safety_report_service.rs';

/** 剥注释（Rust 与 TS/JS 的双斜杠与星斜杠、Vue 的 HTML 注释），保留字符串。 */
export function stripComments(src) {
  return src
    .replace(/<!-{2}[\s\S]*?-{2}>/g, '')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^\s*\/\/[^\n]*$/gm, '')
    .replace(/\/\/[^\n]*/g, '');
}

/** 截断到测试区之前（Rust 的 `#[cfg(test)]` 之后都是测试代码，不参与生产判据）。 */
export function stripTestModule(src) {
  const idx = src.indexOf('#[cfg(test)]');
  return idx < 0 ? src : src.slice(0, idx);
}

/** 归一空白（把跨行签名与多行条件压成单行形态比对，避免排版差异造成漏判或误判）。 */
export function normSpace(text) {
  return text.replace(/\s+/g, ' ').trim();
}

/**
 * 从 Rust 源码里按函数名抠出函数体（锚点后第一个 `{` 起做花括号配平）。
 * 抠不到即抛错——函数被改名或删除不能当"零发现"通过。
 */
export function extractFnBody(src, anchor) {
  const start = src.indexOf(anchor);
  if (start < 0) throw new Error(`函数锚点缺失：${anchor}`);
  const braceAt = src.indexOf('{', start + anchor.length);
  if (braceAt < 0) throw new Error(`函数 ${anchor} 找不到函数体起始花括号`);
  let depth = 0;
  for (let i = braceAt; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return src.slice(braceAt + 1, i);
    }
  }
  throw new Error(`函数 ${anchor} 花括号不配平`);
}

/** 抠 `pub mod NAME { ... }` 块体。 */
export function extractModBlock(src, name) {
  const anchor = `pub mod ${name} {`;
  const start = src.indexOf(anchor);
  if (start < 0) throw new Error(`词表模块锚点缺失：${anchor}`);
  let depth = 0;
  let i = start + anchor.length - 1;
  for (; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return src.slice(start + anchor.length - 1, i + 1);
    }
  }
  throw new Error(`词表模块 ${name} 花括号不配平`);
}

/** 取一个 mod 块体里 `pub const NAME: &str = "TOKEN";` 声明的 token 集合。 */
export function parseVocabTokens(modBody) {
  const set = new Set();
  const re = /pub const \w+\s*:\s*&str\s*=\s*"([^"\\]*)"/g;
  let m;
  while ((m = re.exec(modBody))) set.add(m[1]);
  return set;
}

/** 取 TS 对象字面量 `KEY_LABELS = { a: '...', b: '...' }` 的键集合。 */
export function parseTsObjectKeys(src, objectName) {
  const anchorRe = new RegExp(`${objectName}\\s*(?::[^=]+)?=\\s*\\{`);
  const m = anchorRe.exec(src);
  if (!m) throw new Error(`前端字典锚点缺失：${objectName}`);
  const start = m.index + m[0].length - 1;
  let depth = 0;
  let end = -1;
  for (let i = start; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end < 0) throw new Error(`前端字典 ${objectName} 花括号不配平`);
  const body = src.slice(start + 1, end);
  const keys = new Set();
  const re = /^\s*([A-Za-z_][\w]*)\s*:/gm;
  let k;
  while ((k = re.exec(body))) keys.add(k[1]);
  return keys;
}

/** 装配全部待检文本（一次读盘，各锁共用；自证走 fixtures 注入同一批锁函数）。 */
export function readTargets(root = BACKEND, feRoot = FRONTEND) {
  const paths = {
    reportService: join(root, REL.reportService),
    submissionService: join(root, REL.submissionService),
    submissionHandler: join(root, REL.submissionHandler),
    reportHandler: join(root, REL.reportHandler),
    masterHandler: join(root, REL.masterHandler),
    vocab: join(root, REL.vocab),
    constants: join(feRoot, REL.constants),
    detailView: join(feRoot, REL.detailView),
  };
  const texts = {};
  for (const [key, path] of Object.entries(paths)) {
    if (!existsSync(path)) throw new Error(`采信锁点名文件缺失：${path}（文件被改名或移动，不能当零发现）`);
    texts[key] = readFileSync(path, 'utf-8');
  }
  return { paths, texts };
}

/**
 * 锁 1 定义唯一：采信与窗口函数只准在定义文件里出现一次定义。
 * fixtures.reportServiceOverride 用于自证把定义塞进别的文件。
 */
export function lockDefinitionUnique(texts) {
  const errs = [];
  const defs = ['pub fn select_adopted_reports', 'pub fn validity_window_on'];
  for (const key of Object.keys(texts)) {
    if (!key.endsWith('Service') && !key.endsWith('Handler') && key !== 'vocab') continue;
    const body = stripTestModule(texts[key]);
    for (const anchor of defs) {
      const hits = body.split(anchor).length - 1;
      if (key === 'reportService' && hits !== 1) {
        errs.push(`${REL.reportService} 里 ${anchor} 定义出现 ${hits} 次（必须恰好一次）`);
      }
      if (key !== 'reportService' && hits > 0) {
        errs.push(`${key} 出现 ${anchor} 的第二份定义（采信判据只准住在 ${DEFINITION_FILE}）`);
      }
    }
  }
  return errs;
}

/** 锁 2 采信单源：消费面不得自写受理分支、裸日期比法或直接取有效期列。 */
export function lockSingleSource(texts) {
  const errs = [];
  const consumers = {
    submissionService: REL.submissionService,
    submissionHandler: REL.submissionHandler,
    reportHandler: REL.reportHandler,
    masterHandler: REL.masterHandler,
  };
  for (const [key, label] of Object.entries(consumers)) {
    const body = stripTestModule(stripComments(texts[key]));
    if (/report_acceptance_status::ACCEPTED/.test(body)) {
      errs.push(`${label} 自写受理判式 report_acceptance_status::ACCEPTED（受理档只能由 select_adopted_reports 判）`);
    }
    if (/today\s*[<>]=?|[<>]=?\s*today/.test(body)) {
      errs.push(`${label} 自写裸日期比较（有效期窗口只能由 validity_window_on 判）`);
    }
    if (key === 'submissionService' && /\.valid_until/.test(body)) {
      errs.push(`${label} 直接消费 .valid_until 列（必须消费采信决策，不再自己读有效期列）`);
    }
  }
  return errs;
}

/** 锁 3 至多一份：采信函数体必须首位唯一采信、其余逐枚点名替代。 */
export function lockAtMostOne(texts) {
  const body = extractFnBody(stripTestModule(texts.reportService), 'pub fn select_adopted_reports');
  const errs = [];
  if (!/\.first\(\)/.test(body)) errs.push('select_adopted_reports 未取 .first()（会出现多份报告同时被采信）');
  if (!/\.skip\(1\)/.test(body)) errs.push('select_adopted_reports 未把其余窗口内报告 .skip(1) 逐枚点名（被替代的报告会被静默继续采信）');
  if (!/report_adoption_reason::SUPERSEDED/.test(body)) errs.push('select_adopted_reports 未点名 SUPERSEDED（同批次多份报告时排除原因缺失）');
  if (!/report_adoption_reason::NOT_ACCEPTED/.test(body)) errs.push('select_adopted_reports 未点名 NOT_ACCEPTED（未受理报告被静默跳过）');
  if (/\.filter\(/.test(body)) errs.push('select_adopted_reports 内出现 .filter(（丢弃报告必须走点名分支，不允许静默过滤）');
  return errs;
}

/** 锁 4 有效期双列：窗口函数必须同时消费出具日与有效期，报告级判定必须调用它。 */
export function lockValidityTwoColumns(texts) {
  const fnBody = extractFnBody(stripTestModule(texts.reportService), 'pub fn validity_window_on');
  const judgeBody = extractFnBody(stripTestModule(texts.reportService), 'pub async fn judge_report');
  const errs = [];
  if (!/issue_date/.test(fnBody)) errs.push('validity_window_on 未消费 issue_date（出具日不参与有效期判定）');
  if (!/valid_until/.test(fnBody)) errs.push('validity_window_on 未消费 valid_until');
  if (!/NotYetValid/.test(fnBody)) errs.push('validity_window_on 缺 NotYetValid 档（出具日在未来的报告无法被点名）');
  if (!/validity_window_on\(/.test(judgeBody)) errs.push('judge_report 未调用 validity_window_on（过期报告仍可被判定器采信）');
  if (!/WindowMissing/.test(fnBody)) errs.push('validity_window_on 缺 WindowMissing 档（缺有效期被当成永不过期）');
  return errs;
}

/** 锁 5 排除必点名：三档窗口失败与未受理都要映射到词表常量；完整性门形态不得回退。 */
export function lockNamedExclusion(texts) {
  const body = extractFnBody(stripTestModule(texts.reportService), 'pub fn select_adopted_reports');
  const errs = [];
  for (const [arm, code] of [
    ['ValidityWindow::Expired', 'report_adoption_reason::EXPIRED'],
    ['ValidityWindow::WindowMissing', 'report_adoption_reason::VALID_WINDOW_MISSING'],
    ['ValidityWindow::NotYetValid', 'report_adoption_reason::NOT_YET_VALID'],
  ]) {
    if (!body.includes(arm)) errs.push(`select_adopted_reports 未处理窗口档 ${arm}`);
    if (!body.includes(code)) errs.push(`select_adopted_reports 未把 ${arm} 点名成 ${code}`);
  }
  const gate = extractFnBody(stripTestModule(texts.reportService), 'pub async fn run_acceptance_gate');
  const flat = normSpace(gate);
  // 资质档必须"两枚都缺才拒"且逐枚点名（派工口径 CMA 或 CNAS）
  if (!/if !report\.has_cma && !report\.has_cnas \{/.test(flat)) {
    errs.push('完整性门资质档形态不是"两枚都缺才拒"（缺任一即拒会把只持 CMA 的合法报告判死）');
  }
  for (const code of ['report_element_code::CMA', 'report_element_code::CNAS']) {
    if (!gate.includes(code)) errs.push(`完整性门资质档未逐枚点名 ${code}`);
  }
  if (!/validity_window_on\(/.test(gate)) {
    errs.push('完整性门有效期档未调用 validity_window_on（出具日不参与受理判定）');
  }
  if (!/report_element_code::NOT_YET_VALID/.test(gate)) {
    errs.push('完整性门未点名 not_yet_valid（出具日未到达的报告没有独立缺项码）');
  }
  return errs;
}

/** 锁 6 词表同源：Rust 六枚、本锁独立重述六枚、前端标签六枚，三方两两双向相等。 */
export function lockVocabSameSource(texts) {
  const modBody = extractModBlock(texts.vocab, 'report_adoption_reason');
  const rustSet = parseVocabTokens(modBody);
  const labelBody = parseTsObjectKeys(stripComments(texts.constants), 'ADOPTION_REASON_LABELS');
  const canon = new Set(CANONICAL_ADOPTION_REASONS);
  const errs = [];
  const diff = (name, a, b) => {
    for (const t of a) if (!b.has(t)) errs.push(`${name} 多出 ${t}`);
    for (const t of b) if (!a.has(t)) errs.push(`${name} 缺失 ${t}`);
  };
  diff('Rust report_adoption_reason 与采信锁独立重述', rustSet, canon);
  diff('前端 ADOPTION_REASON_LABELS 与 Rust 词表', labelBody, rustSet);
  if (!/pub fn display_label/.test(modBody)) {
    errs.push('report_adoption_reason 缺 display_label（点名文案没有单源换算处）');
  } else {
    const labelBlock = modBody.slice(modBody.indexOf('pub fn display_label'));
    for (const code of CANONICAL_ADOPTION_REASONS) {
      const constName = code.toUpperCase();
      if (!labelBlock.includes(constName)) {
        errs.push(`display_label 未覆盖 ${code}（该原因码回显会退化成裸英文 token）`);
      }
    }
  }
  // 完整性门缺项码：not_yet_valid 必须已登记进 ALL（否则拒绝明细里的码不在取值域内）
  const elementMod = extractModBlock(texts.vocab, 'report_element_code');
  if (!/"not_yet_valid"/.test(elementMod)) {
    errs.push('report_element_code 缺 not_yet_valid 取值（完整性门会写出取值域外的缺项码）');
  }
  return errs;
}

/** 锁 7 前端只消费：详情视图必须消费后端 adoption，不得自己按出具日挑最新。 */
export function lockFrontendConsumer(texts) {
  const view = stripComments(texts.detailView);
  const errs = [];
  if (!/\.adoption/.test(view)) {
    errs.push(`${REL.detailView} 未消费后端 adoption 结论（前端自己在猜采信哪份）`);
  }
  if (!/ADOPTION_REASON_LABELS/.test(view)) {
    errs.push(`${REL.detailView} 未引用 ADOPTION_REASON_LABELS（排除原因没有走单源字典）`);
  }
  if (/\.sort\([^;]{0,200}issue_date/.test(view) || /Math\.max\([^;]{0,200}issue_date/.test(view)) {
    errs.push(`${REL.detailView} 自行按出具日挑最新（采信判据只能在后端 select_adopted_reports）`);
  }
  return errs;
}

/** 全量跑七条锁。 */
export function runAllLocks(texts) {
  const findings = [];
  const push = (label, items) => {
    for (const it of items) findings.push(`[${label}] ${it}`);
  };
  push('定义唯一', lockDefinitionUnique(texts));
  push('采信单源', lockSingleSource(texts));
  push('至多一份', lockAtMostOne(texts));
  push('有效期双列', lockValidityTwoColumns(texts));
  push('排除必点名', lockNamedExclusion(texts));
  push('词表同源', lockVocabSameSource(texts));
  push('前端只消费', lockFrontendConsumer(texts));
  return findings;
}

/**
 * 变异自证（本地跑，`--self-test`）：五组反例证明每条锁真有检测力，一组正例证明不误判。
 * 任一变异未被抓到即判本锁无检测力，拒绝扫真库。
 */
export function selfProof() {
  const bad = [];
  const { texts } = readTargets();

  // 反例 1 定义唯一：把采信函数复制进送检服务，必须抓到第二份定义
  {
    const mutant = { ...texts, submissionService: texts.submissionService + '\npub fn select_adopted_reports() {}' };
    const hits = lockDefinitionUnique(mutant);
    if (!hits.some((h) => h.includes('submission_service') || h.includes('第二份定义'))) {
      bad.push(`自证反例1：复制定义未被抓到，实际 ${JSON.stringify(hits)}`);
    }
  }
  // 反例 2 采信单源：在送检服务塞自写受理分支与裸日期比法，必须两条都点名
  {
    const injected = texts.submissionService.replace(
      'let adoption = select_adopted_reports(',
      'let _x = report_acceptance_status::ACCEPTED; let today = 1; let _y = today > 2; let adoption = select_adopted_reports('
    );
    const hits = lockSingleSource({ ...texts, submissionService: injected });
    if (hits.length < 2) bad.push(`自证反例2：自写受理与裸日期比法未被逐条点名，实际 ${JSON.stringify(hits)}`);
  }
  // 反例 3 至多一份：把 skip(1) 的点名循环删掉（多份同时存活），必须抓到
  {
    const stripped = texts.reportService.replace(
      /for stale in in_window\.iter\(\)\.skip\(1\) \{[\s\S]*?\n    \}/,
      ''
    );
    if (stripped === texts.reportService) bad.push('自证反例3：变异锚点失效（skip(1) 点名循环形态已变，锁的判据需同步）');
    else if (!lockAtMostOne({ ...texts, reportService: stripped }).some((h) => h.includes('skip(1)'))) {
      bad.push('自证反例3：删掉被替代报告的点名循环却未被抓到');
    }
  }
  // 反例 4 有效期双列：从窗口函数里删掉 issue_date 判据，必须抓到
  {
    const stripped = texts.reportService.replace(
      /if today < issue_date \{\s*\n\s*return ValidityWindow::NotYetValid;\s*\n\s*\}/,
      ''
    );
    if (stripped === texts.reportService) bad.push('自证反例4：变异锚点失效（出具日判据形态已变）');
    else {
      const hits = lockValidityTwoColumns({ ...texts, reportService: stripped });
      if (!hits.some((h) => h.includes('issue_date') || h.includes('NotYetValid'))) {
        bad.push(`自证反例4：删掉出具日判据未被点名，实际 ${JSON.stringify(hits)}`);
      }
    }
  }
  // 反例 5 词表同源：前端字典少一枚，必须抓到缺失
  {
    const stripped = texts.constants.replace(/\n\s*not_accepted:[^\n]*\n/, '\n');
    if (stripped === texts.constants) bad.push('自证反例5：变异锚点失效（前端 not_accepted 行形态已变）');
    else if (!lockVocabSameSource({ ...texts, constants: stripped }).some((h) => h.includes('not_accepted'))) {
      bad.push('自证反例5：前端少一枚原因码却未被点名');
    }
  }
  // 反例 6 完整性门口径：把资质档改回"缺任一即拒"，必须被形态锁抓到
  {
    const mutated = texts.reportService.replace(
      'if !report.has_cma && !report.has_cnas {',
      'if !report.has_cma {'
    );
    if (mutated === texts.reportService) bad.push('自证反例6：变异锚点失效（资质档形态已变）');
    else if (!lockNamedExclusion({ ...texts, reportService: mutated }).some((h) => h.includes('两枚都缺'))) {
      bad.push('自证反例6：资质档退回"缺任一即拒"未被抓到');
    }
  }
  // 反例 7 前端自算采信：视图里加一句按出具日排序，必须被抓到
  {
    const injected = texts.detailView.replace('const judging = ref(false);', "const judging = ref(false);\nreports.sort((a, b) => a.issue_date.localeCompare(b.issue_date));");
    if (!lockFrontendConsumer({ ...texts, detailView: injected }).some((h) => h.includes('出具日'))) {
      bad.push('自证反例7：前端自行按出具日排序未被抓到');
    }
  }
  // 正例：真文件必须零发现（防止恒真判负）
  const positives = runAllLocks(texts);
  if (positives.length) bad.push(`自证正例：真源码被判出 ${positives.length} 条（说明锁在误判）\n  ${positives.join('\n  ')}`);
  return bad;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--self-test')) {
    const bad = selfProof();
    if (bad.length) {
      console.error('FAIL: 采信规则静态锁自证未过——本锁不可信：');
      for (const b of bad) console.error(`  - ${b}`);
      process.exit(1);
    }
    console.log('OK: 采信规则静态锁自证通过（七组变异反例全部被抓并点名，真源码正例零发现）');
    return;
  }

  const bad = selfProof();
  if (bad.length) {
    console.error('FAIL: 采信规则静态锁自证未过——本检查不可信，拒绝扫库：');
    for (const b of bad) console.error(`  - ${b}`);
    process.exit(1);
  }

  const { texts } = readTargets();
  console.log('=== check-textile-report-adoption: 第三方报告采信规则静态锁 ===');
  const findings = runAllLocks(texts);
  if (findings.length) {
    console.error(`\nFAIL: 采信规则被破坏，${findings.length} 条：`);
    for (const f of findings) console.error(`  - ${f}`);
    process.exit(1);
  }
  console.log(
    `OK: 采信与有效期窗口定义唯一在 ${DEFINITION_FILE}；至多一份加逐枚点名排除成立；有效期同时消费出具日与有效期且报告级判定已接窗口；` +
      `完整性门资质档为"两枚都缺才拒并逐枚点名"；Rust 与前端与锁内重述三方原因码同源；前端只消费后端采信结论。`
  );
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) main();
