/**
 * 客户信用“客户名称 / 信用等级”出参形状锁
 * （后端实体载体 ↔ 前端接口声明 ↔ 视图绑定 三向互证）。
 *
 * 钉住的事实：
 * 1) 列表/详情/创建/更新/评分端点的行载体都是 backend/src/models/customer_credit.rs
 *    的 Model 整行直序列化（derive Serialize、无 rename/rename_all），出参键集恒为那 15 个
 *    snake_case 键；其中等级键是 credit_level（字母档），没有 credit_rating，
 *    也没有 valid_from/valid_to/remarks（表 customer_credit_ratings 无此三列）。
 * 2) credit_rating 只存在于评估端点出参 CreditEvaluationResult
 *    （services/customer_credit_service.rs）——它是另一个载体，不是列表行；
 *    把两者混用同一个 TS 接口正是等级列被错绑成 credit_rating 的通道。
 * 3) 前端 CustomerCredit 声明键集合与后端 Model 精确相等；NOT NULL 列不得标 ?；
 *    Decimal 列（rust_decimal 只启 serde feature）出参是字符串，声明必须是 string 不是 number。
 * 4) 请求侧入参键集合 ⊆ 对应 DTO 字段集（SET/UPDATE/QUERY/EVALUATE 各自一把锁），
 *    杜绝 status（UpdateCreditDto 无此字段）、keyword（CreditQuery 无此字段）、
 *    评估请求多带 id 这类"前端以为写了、后端 serde 丢了"的幽灵写/查键。
 * 5) 视图侧：表格列 prop 与行载体读取键（row 点号与 detailRecord 点号形态）都必须 ⊆ Model 键集；
 *    无数据源的假字段列（生效/失效日期、备注）不得留在界面上显示空白；
 *    等级为 null 必须走可区分的"未评"文案（中英双侧齐全，英文侧不得是中文），
 *    不得塌成 0，也不得复用 -/“未知”把"后端没这个键"藏起来。
 * 6) 信用评级/编辑表单不得用 0 作未填初值（async-validator 的 required 认 0 为有效值，
 *    会把"没填"当"用户输入了 0"落库）。
 *
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内源码文本，不执行后端代码、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import zhCN from '@/locales/zh-CN';
import enUS from '@/locales/en-US';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、CI 检出为 LF，
 * 本锁的字面量夹具与逐行正则对行尾形态敏感，不归一会让检测力自证被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r?\n/g, '\n');
}

const CREDIT_MODEL_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/customer_credit.rs')
);
const CREDIT_HANDLER_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/handlers/customer_credit_handler.rs')
);
const CREDIT_SERVICE_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/customer_credit_service.rs')
);
const CREDIT_LIMIT_SERVICE_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/customer_credit_limit.rs')
);
const CREDIT_DDL_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/migration/src/domain/business/m0012_add_ap_ar_finance_analysis.rs')
);
const CREDIT_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/customer-credit.ts'));
const CREDIT_VIEW_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/customer-credit/index.vue')
);
const RATING_TAB_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/customer-credit/tabs/RatingDialogTab.vue')
);

/** Model 出参里后端从不输出的幽灵键：声明或视图绑定中出现即判红 */
const GHOST_MODEL_KEYS = ['credit_rating', 'valid_from', 'valid_to', 'remarks'] as const;

/** customer_credit_ratings 的 NOT NULL 列（DDL 逐列对过）：前端接口里不得标 ? */
const NOT_NULL_COLUMNS = [
  'id',
  'customer_id',
  'credit_limit',
  'used_credit',
  'available_credit',
  'status',
  'created_at',
  'updated_at',
] as const;

/** rust_decimal（backend/Cargo.toml 只启 serde feature，未启 serde-float）NOT NULL 出参 = 十进制字符串 */
const DECIMAL_WIRE_COLUMNS = ['credit_limit', 'used_credit', 'available_credit'] as const;

/** 期望的 Model 键集（= models/customer_credit.rs 的 15 个字段），新增/删除字段须同步修订本锁 */
const EXPECTED_MODEL_KEYS = [
  'available_credit',
  'created_at',
  'created_by',
  'credit_days',
  'credit_level',
  'credit_limit',
  'credit_score',
  'customer_id',
  'customer_name',
  'id',
  'last_assessment_date',
  'next_assessment_date',
  'status',
  'updated_at',
  'used_credit',
] as const;

/** 截取 TS interface 正文（含字段类型原文）；接口被删除或改名即抛错（防判据静默失去覆盖） */
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

/** 提取 TS 接口块内的声明键 */
function tsInterfaceKeys(src: string, name: string): string[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}(?:readonly\s+)?([a-z][A-Za-z0-9_]*)\??\s*:/gm)].map(m => m[1]);
}

/** 截取 Rust struct 正文；结构体改名/删除即抛错（防判据静默失效） */
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

/** 提取 Rust struct 字段名（= serde snake_case 序列化键名；两处载体均无 rename/rename_all） */
function rustStructFields(body: string): string[] {
  return [...body.matchAll(/pub\s+([a-z_][a-z0-9_]*)\s*:/g)].map(m => m[1]);
}

/**
 * 提取模板中 el-table-column 标签的 prop 值（行数据字段绑定）。
 * 只认表格列标签：el-form-item 的 prop 是校验字段名（表单输入合法载体），不得混为一谈误判。
 */
function tableColumnProps(tplSrc: string): string[] {
  return [...tplSrc.matchAll(/<el-table-column\b[^>]*>/gs)]
    .map(tag => tag[0].match(/\sprop="([^"]+)"/)?.[1])
    .filter((p): p is string => Boolean(p));
}

/**
 * 取 .vue 的模板区（第一个 <template> 到 <script setup 之前）。
 * 行绑定与控件属性只在模板里；把整份文件（含 script 的 x.value 与解释性注释）
 * 一起喂给提取器，会把 ref 赋值与文档示例误判成数据绑定。
 */
function templateSectionOf(viewSrc: string): string {
  const start = viewSrc.indexOf('<template>');
  const end = viewSrc.indexOf('<script setup');
  if (start === -1 || end === -1 || end < start) {
    throw new Error('视图里未找到 <template>…<script setup> 边界——文件结构漂移，锁需同步修订');
  }
  return viewSrc.slice(start, end);
}

/**
 * 提取行载体读取点（row.<键> / detailRecord.<键>）与表单 v-model 绑定键
 * （v-model="editForm.<键>"）。
 * 只认这三种表达式形态，不看注释：本域视图里保留了"哪个键没有载体"的契约说明，
 * 用整文件子串匹配会把解释当成数据绑定误判。
 */
function rowBoundKeys(viewSrc: string): string[] {
  return [
    ...[...viewSrc.matchAll(/\b(?:row|detailRecord)\.([a-z_][a-z0-9_]*)/g)].map(m => m[1]),
    ...[...viewSrc.matchAll(/v-model="editForm\.([a-z_][a-z0-9_]*)"/g)].map(m => m[1]),
  ];
}

/** 按点分路径取语言包里的值；取不到返回 undefined（由调用方断言存在性） */
function localeValue(bundle: unknown, dottedKey: string): unknown {
  return dottedKey.split('.').reduce<unknown>((acc, seg) => {
    if (acc !== null && typeof acc === 'object' && seg in (acc as Record<string, unknown>)) {
      return (acc as Record<string, unknown>)[seg];
    }
    return undefined;
  }, bundle);
}

const modelKeys = rustStructFields(rustStructBody(CREDIT_MODEL_SRC, 'Model'));
const declaredKeys = tsInterfaceKeys(CREDIT_API_SRC, 'CustomerCredit');
const setRatingDtoKeys = rustStructFields(
  rustStructBody(CREDIT_HANDLER_SRC, 'CreditRatingRequestDto')
);
const updateDtoKeys = rustStructFields(rustStructBody(CREDIT_HANDLER_SRC, 'UpdateCreditDto'));
const queryDtoKeys = rustStructFields(rustStructBody(CREDIT_HANDLER_SRC, 'CreditQuery'));
const evaluateReqDtoKeys = rustStructFields(
  rustStructBody(CREDIT_HANDLER_SRC, 'CreditEvaluationRequest')
);
const evaluateResultKeys = rustStructFields(
  rustStructBody(CREDIT_SERVICE_SRC, 'CreditEvaluationResult')
);
/** 出参载体键集（读模型 CustomerCreditView 才是当前列表/详情真正序列化的结构） */
const viewKeys = rustStructFields(rustStructBody(CREDIT_SERVICE_SRC, 'CustomerCreditView'));

/** 视图模板区（数据绑定只在这里发生） */
const CREDIT_VIEW_TEMPLATE = templateSectionOf(CREDIT_VIEW_SRC);
const RATING_TAB_TEMPLATE = templateSectionOf(RATING_TAB_SRC);

/** 新增/改动的用户可见文案键：中英双侧必须齐全（check-i18n 只查对称与存在，语种地板在这里补） */
const NEW_I18N_KEYS = [
  'customerCredit.index.creditLevel.none',
  'customerCredit.index.edit.creditLimitRequired',
  'customerCredit.index.message.missingCustomerId',
  'customerCredit.index.log.missingCustomerId',
  'customerCredit.index.log.submitEditBlocked',
  'customerCredit.rating.validation.incompleteForm',
  'customerCredit.rating.log.submissionBlocked',
] as const;

/** 随假字段一起收口的文案键：两侧都不得再留（留了就是孤儿，check-i18n 棘轮同判红） */
const DEAD_I18N_KEYS = [
  'customerCredit.index.detail.validFrom',
  'customerCredit.index.detail.validTo',
  'customerCredit.index.detail.remarks',
] as const;

describe('客户信用出参形状锁（后端实体载体 ↔ 前端声明 ↔ 视图绑定）', () => {
  it('后端事实基准：列表/详情载体是 CustomerCreditView 读模型，键集 15 个且无幽灵键', () => {
    // 读模型 derive Serialize（附加 FromQueryResult），无 rename/rename_all ⇒ 出参键 = 字段名
    expect(CREDIT_SERVICE_SRC).toMatch(
      /#\[derive\(Debug, Clone, Serialize, FromQueryResult\)\]\s*pub struct CustomerCreditView \{/
    );
    expect(rustStructBody(CREDIT_SERVICE_SRC, 'CustomerCreditView')).not.toMatch(
      /rename|skip_serializing_if/
    );
    // 实体 Model 同样无 rename，且读模型键集与 Model 逐键同源（任一侧单边漂移即判红）
    expect(CREDIT_MODEL_SRC).toMatch(/pub struct Model \{/);
    expect(CREDIT_MODEL_SRC).not.toMatch(/rename_all|skip_serializing_if/);
    expect([...viewKeys].sort()).toEqual([...modelKeys].sort());
    // 列表处理器：data 载荷是裸数组，读模型直装
    expect(CREDIT_HANDLER_SRC).toContain('Result<Json<ApiResponse<Vec<CustomerCreditView>>>');
    // 详情/评分出参载体同为读模型；创建/更新经 serde_json::to_value(read_back_view(..)) 装同一模型
    expect(CREDIT_HANDLER_SRC).toContain('Result<Json<ApiResponse<CustomerCreditView>>');
    expect(CREDIT_HANDLER_SRC).toContain(
      'serde_json::to_value(read_back_view(&service, credit.customer_id)'
    );
    // 键集精确相等（后端增删字段会在此判红，须同步修订本锁与前端声明）
    expect([...viewKeys].sort()).toEqual([...EXPECTED_MODEL_KEYS].sort());
    expect(viewKeys.length).toBe(15);
    // 等级真实键存在，且幽灵键在载体侧确实不存在（证明删的是"无此出参键"，不是解析失效）
    expect(viewKeys).toContain('credit_level');
    expect(viewKeys).toContain('customer_name');
    for (const key of GHOST_MODEL_KEYS) {
      expect(viewKeys, `CustomerCreditView 不应含键 ${key}`).not.toContain(key);
      expect(modelKeys, `customer_credit::Model 不应含键 ${key}`).not.toContain(key);
    }
    // DDL 侧同证：表里根本没有 valid_from/valid_to/remarks 三列，等级列名就是 credit_level
    const ddlAt = CREDIT_DDL_SRC.indexOf('CREATE TABLE IF NOT EXISTS "customer_credit_ratings"');
    expect(ddlAt, '未找到 customer_credit_ratings 建表语句——迁移已漂移').toBeGreaterThan(-1);
    const ddl = CREDIT_DDL_SRC.slice(ddlAt, ddlAt + 1200);
    expect(ddl).toMatch(/"credit_level" VARCHAR\(20\),/);
    expect(ddl).not.toMatch(/"valid_from"/);
    expect(ddl).not.toMatch(/"valid_to"/);
    expect(ddl).not.toMatch(/"remarks?"/);
    // customer_name 是唯一可空列且真值来自 LEFT JOIN（缺失时如实 null，不得前端拼名）
    expect(ddl).toMatch(/"customer_name" VARCHAR\(200\),/);
    expect(CREDIT_SERVICE_SRC).toMatch(
      /\.column_as\(customer::Column::CustomerName, "customer_name"\)/
    );
    expect(CREDIT_SERVICE_SRC).toMatch(/\.join\(JoinType::LeftJoin, customer_rel\)/);
    // 冗余实体列被显式剔除，本表写入路径也从不填它 ⇒ 名字只有一个来源，不会随列序漂移
    expect(CREDIT_LIMIT_SERVICE_SRC).not.toMatch(/\.customer_name = Set\(/);
    const viewSelectBody = CREDIT_SERVICE_SRC.slice(CREDIT_SERVICE_SRC.indexOf('fn view_select'));
    expect(viewSelectBody, '未找到 view_select——富化查询基底漂移').not.toBe(CREDIT_SERVICE_SRC);
    expect(viewSelectBody).toMatch(/\.select_only\(\)/);
    expect(viewSelectBody).not.toMatch(/\.column\(customer_credit::Column::CustomerName\)/);
    expect(viewSelectBody).toMatch(
      /\.column_as\(customer::Column::CustomerName, "customer_name"\)/
    );
  });

  it('credit_rating 只属于评估出参载体，不得回流到列表/详情声明', () => {
    // 评估端点出参确实有 credit_rating（等级字母由分数换算，evaluate.rs）
    expect(evaluateResultKeys).toContain('credit_rating');
    expect(CREDIT_SERVICE_SRC).toMatch(/pub credit_rating: String,/);
    // 它是独立载体：前端必须有自己的接口名，出参类型不得谎称是 CustomerCredit
    expect(CREDIT_API_SRC).toMatch(/export interface CreditEvaluationResult \{/);
    expect(CREDIT_API_SRC).toMatch(
      /evaluateCustomerCredit = \(\s*data: CreditEvaluationRequest\s*\): Promise<ApiResponse<CreditEvaluationResult>>/
    );
    expect(CREDIT_API_SRC).not.toMatch(
      /evaluateCustomerCredit[\s\S]{0,200}Promise<ApiResponse<CustomerCredit>/
    );
    // 而列表/详情载体声明里它必须是缺席的（下一条 it 里做差集精确锁）
    expect(declaredKeys).not.toContain('credit_rating');
  });

  it('前端声明：CustomerCredit 键集合与 Model 精确相等，NOT NULL 不可选，Decimal 出参是 string', () => {
    expect(declaredKeys.length).toBeGreaterThanOrEqual(15);
    // 键集合精确相等：既拦幽灵键回流，也拦"后端有键前端不声明"导致的消费方无从得知
    expect(declaredKeys.filter(k => !viewKeys.includes(k))).toEqual([]);
    expect(viewKeys.filter(k => !declaredKeys.includes(k))).toEqual([]);
    for (const key of GHOST_MODEL_KEYS) {
      expect(declaredKeys, `CustomerCredit 不得声明后端不输出的键 ${key}`).not.toContain(key);
    }
    const body = tsInterfaceBody(CREDIT_API_SRC, 'CustomerCredit');
    // NOT NULL 列不得写 ?（那是把"后端没下发这个键"合法化成"可能没有"）
    for (const key of NOT_NULL_COLUMNS) {
      expect(modelKeys, `后端实体已无字段 ${key}，本锁需同步修订`).toContain(key);
      expect(body, `NOT NULL 列 ${key} 不得声明为可选`).toMatch(new RegExp(`^ {2}${key}: `, 'm'));
      expect(body, `NOT NULL 列 ${key} 出现可选标记即判红`).not.toMatch(
        new RegExp(`^ {2}${key}\\?:`, 'm')
      );
    }
    // 可空列：声明为 string | null / number | null，键本身必填（后端键恒存在，值可为 null）
    for (const key of ['customer_name', 'credit_level', 'credit_score', 'credit_days']) {
      expect(body, `可空列 ${key} 应声明为 必填键 + | null`).toMatch(
        new RegExp(`^ {2}${key}: (string|number) \\| null;$`, 'm')
      );
    }
    // Decimal 出参 = 十进制字符串；声明成 number 就是类型谎言（列渲染与比较都会静默走偏）
    const cargoToml = readRepoSource(path.join(REPO_ROOT, 'backend/Cargo.toml'));
    expect(cargoToml).toMatch(/^rust_decimal = \{[^\n]*features = \["serde"\][^\n]*\}$/m);
    expect(cargoToml).not.toContain('serde-float');
    for (const key of DECIMAL_WIRE_COLUMNS) {
      expect(CREDIT_MODEL_SRC, `后端 ${key} 应为 Decimal`).toMatch(
        new RegExp(`pub ${key}: Decimal,`)
      );
      expect(body, `api 声明 ${key} 应为 string`).toMatch(new RegExp(`^ {2}${key}: string;$`, 'm'));
      expect(body, `api 声明 ${key} 混入 number（类型谎言回潮）`).not.toMatch(
        new RegExp(`^ {2}${key}\\??:[^;]*number[^;]*;$`, 'm')
      );
    }
    // 出参实体里根本没有备注键（remark 只是入参 DTO 字段，且后端当前不落库）
    expect(body).not.toMatch(/^ {2}remark/);
  });

  it('入参声明：请求体/查询参数键集合 ⊆ 对应后端 DTO，杜绝幽灵写键与幽灵查询键', () => {
    const setKeys = tsInterfaceKeys(CREDIT_API_SRC, 'SetCreditRatingInput');
    expect(setKeys.filter(k => !setRatingDtoKeys.includes(k))).toEqual([]);
    for (const key of ['customer_id', 'credit_level', 'credit_score', 'credit_limit', 'remark']) {
      expect(setRatingDtoKeys, `CreditRatingRequestDto 需同步修订（缺 ${key}）`).toContain(key);
      expect(setKeys, `SetCreditRatingInput 缺 DTO 真实字段 ${key}`).toContain(key);
    }

    const updateKeys = tsInterfaceKeys(CREDIT_API_SRC, 'UpdateCustomerCreditInput');
    expect(updateKeys.filter(k => !updateDtoKeys.includes(k))).toEqual([]);
    expect([...updateKeys].sort()).toEqual([...updateDtoKeys].sort());
    // status 不在 UpdateCreditDto 里：编辑表单曾带它提交，serde 丢弃 = 静默无效的假控件
    expect(updateDtoKeys).not.toContain('status');
    expect(updateKeys, 'UpdateCustomerCreditInput 不得声明 PUT 不接收的 status').not.toContain(
      'status'
    );
    expect(CREDIT_API_SRC).toMatch(
      /updateCustomerCredit = \(\s*customerId: number,\s*data: UpdateCustomerCreditInput/
    );
    expect(CREDIT_API_SRC).not.toContain('Partial<CustomerCredit>');

    const queryKeys = tsInterfaceKeys(CREDIT_API_SRC, 'CustomerCreditQueryParams');
    expect(queryKeys.filter(k => !queryDtoKeys.includes(k))).toEqual([]);
    expect(queryDtoKeys).not.toContain('keyword');
    expect(queryKeys, '后端 CreditQuery 无 keyword，筛选键不得留下空壳').not.toContain('keyword');

    const evalReqKeys = tsInterfaceKeys(CREDIT_API_SRC, 'CreditEvaluationRequest');
    expect(evalReqKeys.filter(k => !evaluateReqDtoKeys.includes(k))).toEqual([]);
    // 评估请求 DTO 只有 customer_id/evaluation_date，且该路由无路径参数
    expect(evalReqKeys.sort()).toEqual(['customer_id', 'evaluation_date']);
    expect(CREDIT_HANDLER_SRC).toMatch(
      /pub struct CreditEvaluationRequest \{\n {4}pub customer_id: i32,\n {4}pub evaluation_date: String,\n\}/
    );
  });

  it('视图绑定：列 prop 与行读取键都 ⊆ Model 键集，无数据源假字段列已删除，等级 null 显式「未评」', () => {
    const props = tableColumnProps(CREDIT_VIEW_SRC);
    expect(props).toEqual(expect.arrayContaining(['customer_name', 'credit_level', 'status']));
    for (const p of props) {
      expect(viewKeys, `表格列绑定了出参载体不输出的键 ${p}`).toContain(p);
    }
    for (const key of GHOST_MODEL_KEYS) {
      expect(props, `等级列假键 ${key} 不得回流到表格列绑定`).not.toContain(key);
    }
    const bound = rowBoundKeys(CREDIT_VIEW_TEMPLATE);
    expect(bound.length).toBeGreaterThan(0);
    for (const k of bound) {
      expect(viewKeys, `视图模板读取了出参载体不输出的键 row./detailRecord.${k}`).toContain(k);
    }
    for (const key of GHOST_MODEL_KEYS) {
      expect(bound, `视图不得再读取无载体键 ${key}`).not.toContain(key);
    }
    // 无数据源的三项（生效日期/失效日期/备注）已从详情与编辑表单删除，不留恒空壳
    expect(CREDIT_VIEW_SRC).not.toMatch(/detailRecord\.valid_(from|to)/);
    expect(CREDIT_VIEW_SRC).not.toMatch(/editForm\.valid_(from|to)/);
    expect(CREDIT_VIEW_SRC).not.toMatch(/(detailRecord|editForm)\.remarks/);
    expect(CREDIT_VIEW_SRC).not.toMatch(
      /t\('customerCredit\.index\.detail\.(validFrom|validTo|remarks)'\)/
    );
    // 等级未评：走 i18n 显式文案，不许塌成 0，也不许用 '-'/'未知' 把缺键压平
    expect(CREDIT_VIEW_SRC).toContain(`t('customerCredit.index.creditLevel.none')`);
    expect(CREDIT_VIEW_SRC).not.toMatch(/row\.credit_level\s*\|\|/);
    expect(CREDIT_VIEW_SRC).not.toMatch(/credit_level[^)\n]*\?\?\s*0/);
    expect(CREDIT_VIEW_SRC).not.toMatch(/>\s*-\s*<\/el-tag>/);
    // 详情载荷不再用 || {} 兜底（会把信封漂移伪装成空记录）
    expect(CREDIT_VIEW_SRC).not.toMatch(/detailRecord\.value = res\.data \|\| \{\}/);
    // 端点句柄：本域路径参数语义是 customer_id，不得再拿信用行主键 id 当句柄
    expect(CREDIT_VIEW_SRC).not.toMatch(/currentCustomerId\.value = row\.id/);
    expect(CREDIT_VIEW_SRC).toMatch(
      /(getCustomerCredit|deleteCustomerCredit|deactivateCredit|updateCustomerCredit)\(\s*(customerId|currentCustomerId\.value)/
    );
    expect(
      (CREDIT_VIEW_SRC.match(/requireCustomerId\(row\)/g) || []).length
    ).toBeGreaterThanOrEqual(8);
    expect(CREDIT_VIEW_SRC).not.toMatch(/evaluateCustomerCredit\(\{\s*\n\s*id:/);
    // 金额列参与比较必须走 Decimal 归一函数，不在模板里拿字符串跟数字比
    expect(CREDIT_VIEW_SRC).toContain('decimalWireToNumber(row.available_credit) > 0');
    expect(CREDIT_VIEW_SRC).not.toMatch(/row\.available_credit > 0/);
    expect(CREDIT_VIEW_SRC).toContain("import { decimalWireToNumber } from '@/utils/money'");
  });

  it('评级表单：未填态是 null 而不是 0，required 只拦未填、0 视为用户显式输入', () => {
    // 初值 0 + required 认 0 为有效值时，"没填"会被当"输入了 0"落库——本锁拒任何 0 初值回潮
    expect(RATING_TAB_SRC).not.toMatch(/creditScore: 0/);
    expect(RATING_TAB_SRC).not.toMatch(/creditLimit: 0/);
    expect(RATING_TAB_SRC).not.toMatch(/creditDays: 0/);
    expect(RATING_TAB_SRC).not.toMatch(/form\.credit(Score|Limit|Days) = 0/);
    expect(RATING_TAB_SRC).toMatch(/creditScore: null as number \| null/);
    expect(RATING_TAB_SRC).toMatch(/creditLimit: null as number \| null/);
    expect(RATING_TAB_SRC).toMatch(/creditDays: null as number \| null/);
    expect(RATING_TAB_SRC).toMatch(/form\.creditScore = null/);
    // 清空输入框回到未填态，而不是被 :min="0" 拍平成 0
    // 只在模板区计数：script 的说明注释里也出现过这个属性名，整文件匹配会多数一次
    expect(
      (RATING_TAB_TEMPLATE.match(/:value-on-clear="null"/g) || []).length,
      '三个数字输入框都必须显式把清空态定为 null'
    ).toBe(3);
    // 提交侧：拿不到数值时显式失败并留日志，不补默认值
    expect(RATING_TAB_SRC).toContain('function buildRatingPayload');
    expect(RATING_TAB_SRC).toContain('incompleteForm');
    expect(RATING_TAB_SRC).not.toMatch(/credit_score: form\.creditScore \?\? 0/);
    expect(RATING_TAB_SRC).not.toMatch(/credit_limit: form\.creditLimit \?\? 0/);
    // 提交键集合 = 后端 CreditRatingRequestDto 的键（不再丢 credit_score/credit_days）
    const submitted = [
      ...RATING_TAB_SRC.matchAll(
        /^\s{4}(customer_id|credit_level|credit_score|credit_limit|credit_days|remark):/gm
      ),
    ].map(m => m[1]);
    expect([...submitted].sort()).toEqual(
      [
        'credit_days',
        'credit_limit',
        'credit_level',
        'credit_score',
        'customer_id',
        'remark',
      ].sort()
    );
    // 等级档位取自单一真相源，不再在下拉里各写一套取值
    expect(RATING_TAB_SRC).toContain('v-for="level in CREDIT_LEVEL_VALUES"');
    expect(RATING_TAB_SRC).not.toMatch(/<el-option label="AAA" value="AAA" \/>/);
  });

  it('i18n：新增文案中英双侧齐全，英文侧不得塞中文；被删假字段的文案不留孤儿', () => {
    for (const key of NEW_I18N_KEYS) {
      const zh = localeValue(zhCN, key);
      const en = localeValue(enUS, key);
      // 缺一侧 check-i18n 就判中英不对称；这里按解析后的对象路径核，不靠缩进正则
      expect(typeof zh === 'string' && zh.length > 0, `zh-CN 缺文案 ${key}`).toBe(true);
      expect(typeof en === 'string' && en.length > 0, `en-US 缺文案 ${key}`).toBe(true);
      // 英文侧不得写中文：本仓 check-i18n 不查语种，只会让英文界面显示中文，故在此补语种地板
      expect(en, `en-US 的 ${key} 混入中文字符（英文态会漏译显示）`).not.toMatch(/[\u4e00-\u9fff]/);
    }
    const zhNone = localeValue(zhCN, 'customerCredit.index.creditLevel.none');
    const enNone = localeValue(enUS, 'customerCredit.index.creditLevel.none');
    // “未评”两侧都必须是有效文案，且不得复用"未知"这类把缺键压平的词
    expect(typeof zhNone === 'string' && /[\u4e00-\u9fff]/.test(zhNone)).toBe(true);
    expect(typeof enNone === 'string' && (enNone as string).length > 0).toBe(true);
    for (const flat of [zhNone, enNone]) {
      expect(flat, '无等级文案不得写成"未知"（会掩盖"后端没这个键"）').not.toMatch(/未知|unknown/i);
      expect(flat, '无等级文案不得是数字占位').not.toMatch(/^\s*0\s*$/);
    }
    // 后端不产出的三项文案不得留在语言包里当孤儿（孤儿棘轮会判红，这里同锁）
    for (const deadKey of DEAD_I18N_KEYS) {
      expect(localeValue(zhCN, deadKey), `zh-CN 仍留着无数据源的 ${deadKey} 文案`).toBeUndefined();
      expect(localeValue(enUS, deadKey), `en-US 仍留着无数据源的 ${deadKey} 文案`).toBeUndefined();
    }
    // 视图里不得再引用这些已收口的键（引用不存在的键时 vue-i18n 会把键名本身画到界面上）
    for (const deadKey of DEAD_I18N_KEYS) {
      expect(CREDIT_VIEW_SRC, `视图仍引用无数据源的文案键 ${deadKey}`).not.toContain(
        `t('${deadKey}')`
      );
    }
  });

  it('检测力自证：复现历史幽灵形态的夹具必被上述判据抓到（防判据静默失效）', () => {
    // 夹具 A：把后端不输出的 credit_rating 加回 CustomerCredit，差集判据必须抓到
    const mutatedGhost = CREDIT_API_SRC.replace(
      '  credit_level: string | null;',
      '  credit_level: string | null;\n  credit_rating: string;'
    );
    expect(mutatedGhost, '夹具替换本身必须生效').not.toBe(CREDIT_API_SRC);
    const ghostKeys = tsInterfaceKeys(mutatedGhost, 'CustomerCredit');
    expect(ghostKeys.filter(k => !modelKeys.includes(k))).toContain('credit_rating');
    expect(GHOST_MODEL_KEYS.some(k => ghostKeys.includes(k))).toBe(true);
    // 夹具 B：valid_to 回流（曾在基线里挂了很久的存量豁免键），声明判据必须抓到
    const mutatedValidTo = CREDIT_API_SRC.replace(
      '  credit_days: number | null;',
      '  credit_days: number | null;\n  valid_to?: string;'
    );
    expect(mutatedValidTo, '夹具替换本身必须生效').not.toBe(CREDIT_API_SRC);
    expect(tsInterfaceKeys(mutatedValidTo, 'CustomerCredit')).toContain('valid_to');
    // 夹具 C：把假键写回表格列绑定，列 prop 判据必须抓到
    const mutatedView = CREDIT_VIEW_SRC.replace(
      '<el-table-column\n          prop="credit_level"',
      '<el-table-column\n          prop="credit_rating" />\n        <el-table-column\n          prop="credit_level"'
    );
    expect(mutatedView, '夹具替换本身必须生效').not.toBe(CREDIT_VIEW_SRC);
    expect(tableColumnProps(mutatedView), '假键列回流必须被表格列绑定判据抓到').toContain(
      'credit_rating'
    );
    // 夹具 D：详情里再读无载体键，rowBoundKeys 判据必须抓到（且不受解释性注释干扰）
    const mutatedDetail = CREDIT_VIEW_SRC.replace(
      'detailRecord.credit_limit',
      'detailRecord.remarks'
    );
    expect(mutatedDetail, '夹具替换本身必须生效').not.toBe(CREDIT_VIEW_SRC);
    expect(rowBoundKeys(mutatedDetail), '无载体读取点回流必须被行绑定判据抓到').toContain(
      'remarks'
    );
    // 夹具 E：Decimal 出参声明回退成 number（类型谎言），string 锁与 anti-number 锁都要失败
    const mutatedDecimal = CREDIT_API_SRC.replace(
      '  credit_limit: string;',
      '  credit_limit: number;'
    );
    expect(mutatedDecimal, '夹具替换本身必须生效').not.toBe(CREDIT_API_SRC);
    const decimalBody = tsInterfaceBody(mutatedDecimal, 'CustomerCredit');
    expect(decimalBody).not.toMatch(/^ {2}credit_limit: string;$/m);
    expect(decimalBody).toMatch(/^ {2}credit_limit: number;$/m);
    // 夹具 F：NOT NULL 列改回可选，可选标记判据必须抓到
    const mutatedOptional = CREDIT_API_SRC.replace('  status: string;', '  status?: string;');
    expect(mutatedOptional, '夹具替换本身必须生效').not.toBe(CREDIT_API_SRC);
    expect(tsInterfaceBody(mutatedOptional, 'CustomerCredit')).not.toMatch(/^ {2}status: /m);
    // 夹具 G：评级表单初值退回 0，0 兜底判据必须抓到
    const mutatedZero = RATING_TAB_SRC.replace(
      'creditScore: null as number | null,',
      'creditScore: 0,'
    );
    expect(mutatedZero, '夹具替换本身必须生效').not.toBe(RATING_TAB_SRC);
    expect(mutatedZero).toMatch(/creditScore: 0/);
    expect(mutatedZero).not.toMatch(/creditScore: null as number \| null/);
    // 夹具 H：编辑表单再带上 PUT 不接收的 status 键，入参差集锁必须抓到
    const mutatedStatus = CREDIT_API_SRC.replace(
      '  credit_score?: number | null;\n  credit_limit?: number | null;\n  credit_days?: number | null;\n  remark?: string | null;\n}\n\nexport interface CreditAdjustment',
      '  credit_score?: number | null;\n  credit_limit?: number | null;\n  credit_days?: number | null;\n  remark?: string | null;\n  status?: string;\n}\n\nexport interface CreditAdjustment'
    );
    expect(mutatedStatus, '夹具替换本身必须生效').not.toBe(CREDIT_API_SRC);
    const statusKeys = tsInterfaceKeys(mutatedStatus, 'UpdateCustomerCreditInput');
    expect(statusKeys.filter(k => !updateDtoKeys.includes(k))).toContain('status');
    // 反向自证：后端结构体块改名/删除时解析必须抛错而非静默空集
    expect(() => rustStructFields(rustStructBody('pub struct Renamed {}\n', 'Model'))).toThrow();
    expect(() => tsInterfaceKeys('export const nothing = 1;\n', 'CustomerCredit')).toThrow();
  });
});
