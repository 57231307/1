/**
 * 供应商对比形状锁（后端评分聚合载体 ↔ 前端声明 ↔ 对比页消费三向互证）。
 * 功能：
 * 1) 钉住对比入口是按关键字向服务端取候选的多选远程搜索下拉，不要求手填数字主键；
 * 2) 钉住并排的四个指标键与后端 SupplierScoreResponse 字段逐键一致，写成驼峰或自创键即列恒空；
 * 3) 钉住对比行的名称取供应商主数据出参的 supplier_name，不得用编码或主键拼假名；
 * 4) 钉住后端明确无评估记录与真实失败分列、结论词表中英双侧齐全、缺数据位不拿 0 或空数组兜底；
 * 5) 钉住对比不引入新端点：既有的供应商对比路径在后端路由快照里本就不存在，页面也不得自行拼出。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内源码文本，不执行后端代码、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、CI 检出为 LF，
 * 本锁的字面量夹具与逐行正则对行尾形态敏感，不归一会让检测力自证被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const EVAL_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/supplier-evaluation/index.vue')
);
const API_EVAL_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/supplier-evaluation.ts'));
const ZH_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/locales/zh-CN.ts'));
const EN_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/locales/en-US.ts'));
const ROUTE_SNAPSHOT_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'scripts/route-snapshot.txt'));
const BACKEND_SERVICE_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/supplier_evaluation_service.rs')
);

/**
 * 被禁的步进数字录入控件标签名。全项目禁止带加减按钮的数字输入框，本锁用它钉住对比入口
 * 不得退回该控件。分段拼接避免本锁源码自身携带这个字面量，口径与静态判定器一致。
 */
const FORBIDDEN_STEPPER_TAG = ['el-input-', 'number'].join('');

/** 对比要并排的四个指标键，权威定义在后端 SupplierScoreResponse */
const COMPARE_METRIC_KEYS = [
  'rating',
  'average_score',
  'total_records',
  'latest_evaluation_date',
] as const;

/** 取数结论词表：与后端 get_supplier_score 的 not_found 业务态一一对应 */
const OUTCOME_TOKENS = ['loaded', 'noRecord', 'failed'] as const;

/** 截取 Rust struct 正文；结构体改名或删除即抛错（防判据静默失去覆盖） */
function rustStructBody(src: string, name: string): string {
  const start = src.indexOf(`pub struct ${name} {`);
  if (start === -1) {
    throw new Error(`后端源码中未找到 pub struct ${name}——载体已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`pub struct ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 提取 Rust struct 字段名（该结构无 rename_all，序列化键名即字段名） */
function rustStructFields(body: string): string[] {
  return [...body.matchAll(/pub\s+([a-z_][a-z0-9_]*)\s*:/g)].map(m => m[1]);
}

/** 截取 TS interface 正文；接口被删除或改名即抛错 */
function tsInterfaceBody(src: string, name: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`TS 源码中未找到 interface ${name}——结构已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 对比页模板区域：从对比页签注释到该页签闭合 */
function compareTemplateRegion(src: string): string {
  const start = src.indexOf('<!-- 供应商对比');
  if (start === -1) {
    throw new Error('对比页签模板不存在或注释锚点漂移——判据失去抓手');
  }
  const end = src.indexOf('</el-tab-pane>', start);
  if (end === -1) {
    throw new Error('对比页签模板未闭合，无法界定区域');
  }
  return src.slice(start, end);
}

/** 对比页脚本区域：从对比分节注释到下一个分节（保存评估） */
function compareScriptRegion(src: string): string {
  const start = src.indexOf('// ============== 供应商对比 ==============');
  if (start === -1) {
    throw new Error('对比脚本分节不存在或锚点漂移——判据失去抓手');
  }
  const end = src.indexOf('const handleSaveRecord', start);
  if (end === -1) {
    throw new Error('对比脚本分节没有后继边界，无法界定区域');
  }
  return src.slice(start, end);
}

/** 绑定对比选中集合的控件标签名 */
function compareIdsBoundTag(src: string): string {
  const hit = src.match(/<([a-z][a-z0-9-]*)[^>]*?v-model="compareIds"/);
  if (hit === null) {
    throw new Error('没有任何控件绑定 compareIds——对比入口形态已漂移');
  }
  return hit[1];
}

/** 对比入口是否为多选的关键字远程搜索下拉 */
function compareEntryIsRemoteMultiPicker(template: string): boolean {
  return (
    compareIdsBoundTag(template) === 'el-select' &&
    /(^|\s)multiple(\s|=|>)/.test(template) &&
    /(^|\s)remote(\s|=|>)/.test(template) &&
    template.includes(':remote-method="searchCompareSuppliers"') &&
    template.includes(':loading="comparePickerLoading"') &&
    !template.includes(':filter-method=') &&
    !template.includes(FORBIDDEN_STEPPER_TAG)
  );
}

/** 提取模板里实际消费的评分聚合键（行级可选链取值） */
function consumedMetricKeys(template: string): string[] {
  return [...template.matchAll(/row\.score\?\.([a-z_][a-z0-9_]*)/g)].map(m => m[1]);
}

/** 行名称是否只取主数据出参的 supplier_name，且没有用编码或主键拼假名 */
function nameComesFromMasterData(template: string, script: string): boolean {
  const usesSupplierNameColumn = /\{\{ row\.supplierName \?\? '-' \}\}/.test(template);
  const sourcedFromPickedCandidates =
    /comparePickedSuppliers\.value\.map\(s => \[s\.id, s\.supplier_name\]\)/.test(script);
  const noFabricatedName =
    !/supplier_code/.test(script) && !/\$\{\s*id\s*\}/.test(script) && !/Supplier #/.test(script);
  return usesSupplierNameColumn && sourcedFromPickedCandidates && noFabricatedName;
}

/** 结论词表三值是否齐全，且中英双侧都给了文案 */
function outcomeVocabularyIsComplete(metaBlock: string, zh: string, en: string): boolean {
  const allTokensDeclared = OUTCOME_TOKENS.every(token =>
    new RegExp(
      `^  ${token}: \\{ labelKey: 'supplierEvaluation\\.index\\.compare\\.outcome\\.${token}'`,
      'm'
    ).test(metaBlock)
  );
  const bothLocales = OUTCOME_TOKENS.every(token => {
    const block = (src: string): boolean => {
      const start = src.indexOf('      compare: {');
      if (start === -1) return false;
      return new RegExp(`outcome: \\{[^}]*${token}: '[^']+'`).test(src.slice(start, start + 1600));
    };
    return block(zh) && block(en);
  });
  return allTokensDeclared && bothLocales;
}

/** 后端明确无评估记录与真实失败是否分列处置，且真实失败既留日志又给界面提示 */
function failurePathsAreDistinguished(script: string): boolean {
  return (
    /=== 404/.test(script) &&
    /outcome: 'noRecord'/.test(script) &&
    /outcome: 'failed'/.test(script) &&
    /logAuxLoadFailure\(/.test(script) &&
    /compareRows\.value\.some\(row => row\.outcome === 'failed'\)/.test(script)
  );
}

/** 缺数据位是否拒绝用 0 或空数组兜底掩盖 */
function hasNoZeroFallback(script: string): boolean {
  return !/\?\? 0\b/.test(script) && !/\|\| 0\b/.test(script) && !/\?\? \[\]/.test(script);
}

const scoreStructKeys = rustStructFields(
  rustStructBody(BACKEND_SERVICE_SRC, 'SupplierScoreResponse')
);
const declaredScoreKeys = [
  ...tsInterfaceBody(API_EVAL_SRC, 'SupplierScore').matchAll(/^ {2}([a-z][A-Za-z0-9_]*)\??\s*:/gm),
].map(m => m[1]);
const templateRegion = compareTemplateRegion(EVAL_SRC);
const scriptRegion = compareScriptRegion(EVAL_SRC);
const consumedKeys = consumedMetricKeys(templateRegion);
const outcomeMetaBlock = scriptRegion.slice(
  scriptRegion.indexOf('const COMPARE_OUTCOME_META'),
  scriptRegion.indexOf('const compareIds')
);

describe('供应商对比形状锁（后端载体 ↔ 前端声明 ↔ 对比页消费）', () => {
  it('后端事实基准：四个指标键是评分聚合出参的真实字段，且不存在任何对比端点', () => {
    expect(scoreStructKeys.length).toBeGreaterThanOrEqual(6);
    for (const key of COMPARE_METRIC_KEYS) {
      expect(scoreStructKeys, `后端 SupplierScoreResponse 应含键 ${key}`).toContain(key);
    }
    // 路由快照里没有任何 compare 路径：对比只能用既有端点，凭空拼出的 URL 会被本锁与 check-api-paths 双侧拒收
    expect(ROUTE_SNAPSHOT_SRC).not.toMatch(/supplier-evaluations\/compare/);
    expect(EVAL_SRC).not.toMatch(/supplier-evaluations\/compare/);
  });

  it('对比入口是多选远程搜索下拉，不再要求手填数字主键', () => {
    expect(compareEntryIsRemoteMultiPicker(templateRegion)).toBe(true);
  });

  it('消费的指标键与后端逐键一致，声明侧把后端必出键写成非可选', () => {
    expect(consumedKeys.length).toBeGreaterThanOrEqual(COMPARE_METRIC_KEYS.length);
    for (const key of consumedKeys) {
      expect(scoreStructKeys, `对比页消费了后端不输出的键 ${key}`).toContain(key);
    }
    for (const key of COMPARE_METRIC_KEYS) {
      expect(consumedKeys, `对比页必须消费指标键 ${key}`).toContain(key);
    }
    // 前端声明与后端载体同名同形：驼峰或自创键即列恒空
    for (const key of COMPARE_METRIC_KEYS) {
      expect(declaredScoreKeys, `SupplierScore 声明缺键 ${key}`).toContain(key);
    }
    expect(declaredScoreKeys).not.toContain('averageScore');
    // average_score 线上是字符串形态（rust_decimal 仅开 serde 特性），声明不得退回 number
    expect(tsInterfaceBody(API_EVAL_SRC, 'SupplierScore')).toMatch(/^ {2}average_score: string;$/m);
  });

  it('对比行名称取主数据出参的 supplier_name，不用编码或主键拼假名', () => {
    expect(nameComesFromMasterData(templateRegion, scriptRegion)).toBe(true);
  });

  it('取数结论词表三值齐全且中英双侧都有文案，无记录与真实失败分列', () => {
    expect(outcomeVocabularyIsComplete(outcomeMetaBlock, ZH_SRC, EN_SRC)).toBe(true);
    expect(failurePathsAreDistinguished(scriptRegion)).toBe(true);
  });

  it('缺数据位不拿 0 或空数组兜底，可空日期按空值显式占位', () => {
    expect(hasNoZeroFallback(scriptRegion)).toBe(true);
    expect(templateRegion).toMatch(/row\.score\?\.latest_evaluation_date \?\? '-'/);
  });

  it('检测力自证：退化入口、拼假名、合并失败分支、删词表项与 0 兜底都必被判据抓到', () => {
    // 夹具 A：把多选远程搜索退化成手填数字主键，入口判据必须抓到
    const manualEntry = templateRegion.replace(
      '              v-model="compareIds"\n              multiple\n              filterable\n              remote',
      '              v-model="compareIds"\n              filterable'
    );
    expect(manualEntry).not.toBe(templateRegion);
    expect(compareEntryIsRemoteMultiPicker(manualEntry)).toBe(false);
    // 夹具 B：把行名称来源换成供应商编码，取名判据必须抓到
    const codedName = scriptRegion.replace(
      'comparePickedSuppliers.value.map(s => [s.id, s.supplier_name])',
      'comparePickedSuppliers.value.map(s => [s.id, s.supplier_code])'
    );
    expect(codedName).not.toBe(scriptRegion);
    expect(nameComesFromMasterData(templateRegion, codedName)).toBe(false);
    // 夹具 C：把后端无评估记录并入真实失败（去掉 404 分列），失败分列判据必须抓到
    const mergedFailure = scriptRegion.replace(
      `    if (isNoEvaluationRecord(error)) {
      return { supplierId, supplierName, score: null, outcome: 'noRecord' };
    }
`,
      ''
    );
    expect(mergedFailure).not.toBe(scriptRegion);
    expect(failurePathsAreDistinguished(mergedFailure)).toBe(false);
    // 夹具 D：把结论词表里的无记录一项删掉，词表齐全判据必须抓到
    const droppedToken = outcomeMetaBlock.replace(
      "  noRecord: { labelKey: 'supplierEvaluation.index.compare.outcome.noRecord', tagType: 'info' },\n",
      ''
    );
    expect(droppedToken).not.toBe(outcomeMetaBlock);
    expect(outcomeVocabularyIsComplete(droppedToken, ZH_SRC, EN_SRC)).toBe(false);
    // 夹具 E：给平均分加 0 兜底，缺数据不兜底判据必须抓到
    const zeroFallback = scriptRegion.replace(
      "score: res.data, outcome: 'loaded'",
      "score: res.data ?? 0, outcome: 'loaded'"
    );
    expect(zeroFallback).not.toBe(scriptRegion);
    expect(hasNoZeroFallback(zeroFallback)).toBe(false);
    // 夹具 F：把消费的指标键改成驼峰，逐键一致判据必须抓到
    const camelMetric = templateRegion.replace(
      "row.score?.average_score ?? '-'",
      "row.score?.averageScore ?? '-'"
    );
    expect(camelMetric).not.toBe(templateRegion);
    expect(consumedMetricKeys(camelMetric).every(key => scoreStructKeys.includes(key))).toBe(false);
    // 反向自证：后端结构块改名时解析必须抛错而不是静默空集
    expect(() =>
      rustStructFields(rustStructBody('pub Other {}', 'SupplierScoreResponse'))
    ).toThrow();
    expect(() => compareTemplateRegion('<template></template>')).toThrow();
    expect(() => compareScriptRegion('const a = 1;')).toThrow();
    expect(() => compareIdsBoundTag('<div></div>')).toThrow();
  });
});
