/**
 * 六业务域（化学品/工资/委外/lab 打样/流转卡；能源域前端当前无写入面）写入端点
 * 请求侧身份键锁：操作人一律由服务端按会话派生，提交载荷不得承载 created_by 等身份键。
 * 功能：对六域已声明的提交载荷接口、视图提交字面量与表单模型初始值三层做静态扫描，
 * 钉死其均不出现 created_by / createdBy / created_by_name——防止界面重新采集身份、
 * 或类型漂移后再次上送一个服务端只会忽略的假字段；并锁定「扫描到的写端点数量 ≥ 地板值」，
 * 扫描面塌缩到取不到即判红，防止判据静默失去覆盖。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取 frontend/src 内源码文本，不发网络请求、不执行后端代码。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、CI 检出为 LF，
 * 夹具的字面量 replace（含 `\n`）对行尾形态敏感，不归一会使检测力自证静默落空。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

function frontendSrc(rel: string): string {
  return readRepoSource(path.join(FRONTEND_ROOT, 'src', rel));
}

/** 请求侧禁止出现的身份键（操作人由后端会话注入，前端上送即假语义） */
const IDENTITY_KEYS: readonly string[] = ['created_by', 'createdBy', 'created_by_name'];

/** 六域 API 源文件（能源域无前端 API 文件，写入面为零，不列入） */
const DOMAIN_API_FILES: readonly string[] = [
  'api/chemical.ts',
  'api/wage.ts',
  'api/outsourcing.ts',
  'api/lab-dip.ts',
  'api/flow-card.ts',
];

/**
 * 截取 TS 接口块正文。入参：源码文本 + 接口名；找不到即抛错（防判据静默失去覆盖）。
 */
function tsInterfaceBody(src: string, name: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`源码中未找到 interface ${name}——类型被删除或改名，身份锁失效`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 提取 TS 接口块内声明键（`[key: string]: unknown` 索引签名行不匹配键形态，自然跳过） */
function tsInterfaceKeys(src: string, name: string): string[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}(?:readonly\s+)?([a-z][A-Za-z0-9_]*)\??\s*:/gm)].map(m => m[1]);
}

/** 接口声明键中命中的身份键（正向判据：必须为空） */
function interfaceIdentityViolations(src: string, name: string): string[] {
  return tsInterfaceKeys(src, name).filter(k => IDENTITY_KEYS.includes(k));
}

/** 提取对象字面量正文里的键（值段无嵌套花括号，`[^{}]*` 即整块） */
function literalKeys(literalBody: string): string[] {
  return [...literalBody.matchAll(/(?:^|[,{\n])\s*([A-Za-z_][A-Za-z0-9_]*)\s*:(?!:)/g)].map(
    m => m[1]
  );
}

/**
 * 抓取提交字面量块（每块必须非空，防正则空转）。
 * kind='call' 匹配 createX(...{..})（允许 createX(firstArg, {..}) 双参形态；
 * 首参段不含花括号，`[^{}]*` 即整段）；kind='payload-var' 匹配视图先组装的
 * `const payload = {..}` 变量（chemicals 视图 create/update 两分支共用同一字面量）。
 */
function payloadLiteralBodies(src: string, name: string, kind: 'call' | 'payload-var'): string[] {
  const pattern =
    kind === 'call'
      ? new RegExp(`${name}\\([^{}]*\\{([^{}]*)\\}\\)`, 'g')
      : /const payload = \{([^{}]*)\};/g;
  const bodies = [...src.matchAll(pattern)].map(m => m[1]);
  if (bodies.length === 0) {
    throw new Error(`未找到 ${name} 的提交字面量——调用点被重写或改名，身份锁失效`);
  }
  return bodies;
}

/** 抓取 `const formName = reactive({..})` 表单模型初始值块（整表单直传形态的载荷来源） */
function reactiveFormBodies(src: string, formName: string): string[] {
  const pattern = new RegExp(`const ${formName} = reactive\\(\\{([^{}]*)\\}\\)`, 'g');
  const bodies = [...src.matchAll(pattern)].map(m => m[1]);
  if (bodies.length === 0) {
    throw new Error(`未找到 const ${formName} = reactive({...})——表单模型被重写，身份锁失效`);
  }
  return bodies;
}

/**
 * 对一个源文件做通用身份扫描：所有 *Payload 接口声明块 + 所有 createX({..}) 直传字面量。
 * 出参实体接口（如 OutsourcingOrder/OutsourcingReceipt 的 created_by 只读列）名称不以
 * Payload 结尾，天然不在请求侧判据范围内——响应声明与提交载荷各自归位。
 */
function identityViolationsInSource(src: string): string[] {
  const bad: string[] = [];
  for (const m of src.matchAll(/export interface (\w+Payload) \{/g)) {
    bad.push(...interfaceIdentityViolations(src, m[1]));
  }
  for (const m of src.matchAll(/\bcreate[A-Z]\w*\(\{([^{}]*)\}\)/g)) {
    bad.push(...literalKeys(m[1]).filter(k => IDENTITY_KEYS.includes(k)));
  }
  return bad;
}

/** 六域全部已声明的提交载荷接口（含状态机迁移带体端点的载荷） */
const PAYLOAD_INTERFACES: Array<{ file: string; iface: string }> = [
  { file: 'api/chemical.ts', iface: 'CreateChemicalPayload' },
  { file: 'api/wage.ts', iface: 'CreateWageRatePayload' },
  { file: 'api/wage.ts', iface: 'CreateWageRecordPayload' },
  { file: 'api/outsourcing.ts', iface: 'CreateOutsourcingOrderPayload' },
  { file: 'api/outsourcing.ts', iface: 'UpdateOutsourcingOrderPayload' },
  { file: 'api/outsourcing.ts', iface: 'UpdateOutsourcingReceiptPayload' },
  { file: 'api/lab-dip.ts', iface: 'CreateLabDipRequestPayload' },
  { file: 'api/lab-dip.ts', iface: 'UpdateLabDipRequestPayload' },
  { file: 'api/lab-dip.ts', iface: 'ApproveLabDipPayload' },
  { file: 'api/lab-dip.ts', iface: 'CompleteLabDipPayload' },
  { file: 'api/flow-card.ts', iface: 'CreateFlowCardPayload' },
  { file: 'api/flow-card.ts', iface: 'ScheduleFlowCardPayload' },
  { file: 'api/flow-card.ts', iface: 'CompletePreparingPayload' },
];

/** 视图提交点：文件 ↔ 被调创建函数 ↔ 提交形态 */
const VIEW_CALL_SITES: Array<{ file: string; fn: string; kind: 'call' | 'payload-var' }> = [
  { file: 'views/lab-dip/index.vue', fn: 'createLabDipRequest', kind: 'call' },
  { file: 'views/outsourcing/index.vue', fn: 'createOutsourcingOrder', kind: 'call' },
  { file: 'views/outsourcing/index.vue', fn: 'createOutsourcingReceipt', kind: 'call' },
  { file: 'views/outsourcing/index.vue', fn: 'createOutsourcingItem', kind: 'call' },
  { file: 'views/wage/index.vue', fn: 'createWageRecord', kind: 'call' },
  { file: 'views/flow-cards/index.vue', fn: 'createFlowCard', kind: 'call' },
  { file: 'views/chemicals/index.vue', fn: 'createChemical', kind: 'payload-var' },
];

/** 整表单直传形态：提交实参是 reactive 模型，锁其初始值键集 */
const FORM_VAR_SITES: Array<{ file: string; form: string }> = [
  { file: 'views/chemicals/index.vue', form: 'lotForm' },
  { file: 'views/chemicals/index.vue', form: 'categoryForm' },
  { file: 'views/wage/index.vue', form: 'rateForm' },
];

/** 写端点数量地板：六域 API 文件内 request.post/request.put 调用总数与单域最小值 */
const WRITE_ENDPOINT_TOTAL_MIN = 40;
const WRITE_ENDPOINT_PER_FILE_MIN = 3;

describe('六域写入端点请求侧身份键锁', () => {
  it('载荷接口声明键均不含 created_by/createdBy/created_by_name', () => {
    for (const { file, iface } of PAYLOAD_INTERFACES) {
      const src = frontendSrc(file);
      const keys = tsInterfaceKeys(src, iface);
      // 正向非空控制：接口解析不出任何键 = 判据空转
      expect(keys.length, `${file} 的 ${iface} 未解析出声明键`).toBeGreaterThan(0);
      expect(interfaceIdentityViolations(src, iface), `${file} 的 ${iface} 携带身份键`).toEqual([]);
    }
  });

  it('视图提交字面量逐键显式列举，不含身份键', () => {
    for (const { file, fn, kind } of VIEW_CALL_SITES) {
      const src = frontendSrc(file);
      const bodies = payloadLiteralBodies(src, fn, kind);
      for (const body of bodies) {
        const keys = literalKeys(body);
        expect(keys.length, `${file} 中 ${fn} 的提交字面量未解析出键`).toBeGreaterThan(0);
        const violations = keys.filter(k => IDENTITY_KEYS.includes(k));
        expect(violations, `${file} 中 ${fn} 的提交载荷携带身份键`).toEqual([]);
      }
    }
  });

  it('整表单直传形态的 reactive 初始值不含身份键', () => {
    for (const { file, form } of FORM_VAR_SITES) {
      const src = frontendSrc(file);
      for (const body of reactiveFormBodies(src, form)) {
        const keys = literalKeys(body);
        expect(keys.length, `${file} 中 ${form} 未解析出初始键`).toBeGreaterThan(0);
        const violations = keys.filter(k => IDENTITY_KEYS.includes(k));
        expect(violations, `${file} 中 ${form} 的表单模型携带身份键`).toEqual([]);
      }
    }
  });

  it('六域 API 文件通用扫描（*Payload 接口 + createX 直传字面量）零命中', () => {
    for (const file of DOMAIN_API_FILES) {
      const src = frontendSrc(file);
      expect(identityViolationsInSource(src), `${file} 的请求侧形态携带身份键`).toEqual([]);
    }
  });

  it('检测力地板：六域写端点扫描数量 ≥ 阈值，扫描面塌缩即判红', () => {
    let total = 0;
    for (const file of DOMAIN_API_FILES) {
      const src = frontendSrc(file);
      const n = [...src.matchAll(/request\.(?:post|put)/g)].length;
      expect(n, `${file} 写端点扫描数低于单域地板，扫描面已塌缩`).toBeGreaterThanOrEqual(
        WRITE_ENDPOINT_PER_FILE_MIN
      );
      total += n;
    }
    expect(total).toBeGreaterThanOrEqual(WRITE_ENDPOINT_TOTAL_MIN);
  });

  it('检测力自证：注入身份键的夹具必须被上述判据抓到，而非静默通过', () => {
    // 夹具 1：往真实接口文本里塞 created_by 声明，接口判据必须点名
    const mutated = frontendSrc('api/lab-dip.ts').replace(
      'export interface CreateLabDipRequestPayload {\n  customer_id?: number;',
      'export interface CreateLabDipRequestPayload {\n  customer_id?: number;\n  created_by: number;'
    );
    expect(mutated).not.toBe(frontendSrc('api/lab-dip.ts')); // 夹具替换本身生效
    expect(interfaceIdentityViolations(mutated, 'CreateLabDipRequestPayload')).toEqual([
      'created_by',
    ]);

    // 夹具 2：往提交字面量里塞身份键（含双参调用形态），字面量判据必须点名
    const callFixture =
      'await createOutsourcingItem(orderId.value.id, {\n  product_id: 1,\n  createdBy: u,\n});';
    const fixtureKeys = literalKeys(
      payloadLiteralBodies(callFixture, 'createOutsourcingItem', 'call')[0]
    );
    expect(fixtureKeys.filter(k => IDENTITY_KEYS.includes(k))).toEqual(['createdBy']);

    // 夹具 3：往 reactive 表单模型塞身份键，表单判据必须点名
    const formFixture = "const demoForm = reactive({\n  lot_no: '',\n  created_by: 7,\n});";
    const formKeys = literalKeys(reactiveFormBodies(formFixture, 'demoForm')[0]);
    expect(formKeys.filter(k => IDENTITY_KEYS.includes(k))).toEqual(['created_by']);

    // 反向对照：干净夹具零命中，证明上述点名非正则误报
    expect(
      identityViolationsInSource(
        'export interface CreateDemoPayload {\n  demo_code: string;\n}\n' +
          'const ok = createDemo({\n  demo_code: t,\n});'
      )
    ).toEqual([]);
  });
});
