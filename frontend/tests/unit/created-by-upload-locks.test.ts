/**
 * 创建端点请求侧身份键锁（操作人一律由服务端按会话派生，请求体不得承载身份键）。
 * 功能：对以下 9 个 POST 创建端点，钉死「载荷接口声明键 ↔ 视图提交字面量」两层
 * 均不出现 created_by / createdBy / created_by_name——防止界面重新采集身份、
 * 或类型漂移后再次上送一个服务端只会忽略的假字段：
 * - /environmental-tax/discharge-records
 * - /export-refunds/customs-declarations
 * - /period-adjustments
 * - /labor-contracts
 * - /occupational-health/health-exams
 * - /occupational-health/ppe-distributions
 * - /occupational-health/hazard-monitorings
 * - /social-insurance
 * - /pollution-monitoring/solid-waste-disposals（前端尚未接入，以正向形态锁住
 *   「任何提及该端点的 src 文件不得携带身份键」，接入时自动纳入检测）
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取 frontend/src 内源码文本，不发网络请求、不执行后端代码。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
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

/** 提取 TS 接口块内声明键（跳过 `[key: string]: unknown` 索引签名行） */
function tsInterfaceKeys(src: string, name: string): string[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}(?:readonly\s+)?([a-z][A-Za-z0-9_]*)\??\s*:/gm)].map(m => m[1]);
}

/** 接口声明键中命中的身份键（正向判据：必须为空） */
function interfaceIdentityViolations(src: string, name: string): string[] {
  return tsInterfaceKeys(src, name).filter(k => IDENTITY_KEYS.includes(k));
}

/** 提取载荷对象字面量正文里的键（值段无嵌套花括号，`[^{}]*` 即整块） */
function literalKeys(literalBody: string): string[] {
  return [...literalBody.matchAll(/(?:^|[,{\n])\s*([A-Za-z_][A-Za-z0-9_]*)\s*:(?!:)/g)].map(
    m => m[1]
  );
}

/**
 * 抓取 `fn({ ... })` 直传字面量块（每块必须非空，防正则空转）。
 * kind='call' 匹配 createX({..})；kind='payload-var' 匹配 `const payload = {..}`
 * （labor-contracts 视图先组装 payload 变量再传入 create/update 两个分支）。
 */
function payloadLiteralBodies(src: string, name: string, kind: 'call' | 'payload-var'): string[] {
  const pattern =
    kind === 'call'
      ? new RegExp(`${name}\\(\\{([^{}]*)\\}\\)`, 'g')
      : /const payload = \{([^{}]*)\};/g;
  const bodies = [...src.matchAll(pattern)].map(m => m[1]);
  if (bodies.length === 0) {
    throw new Error(`未找到 ${name} 的提交字面量——调用点被重写或改名，身份锁失效`);
  }
  return bodies;
}

/** 9 端点中前端已接入的 8 个创建载荷接口 */
const PAYLOAD_INTERFACES: Array<{ file: string; iface: string }> = [
  { file: 'api/export-compliance.ts', iface: 'CreateDischargeRecordPayload' },
  { file: 'api/tax-rebate.ts', iface: 'CreateCustomsDeclarationPayload' },
  { file: 'api/period-adjustment.ts', iface: 'CreatePeriodAdjustmentPayload' },
  { file: 'api/labor-contract.ts', iface: 'CreateLaborContractPayload' },
  { file: 'api/occupational-health.ts', iface: 'CreateHealthExamPayload' },
  { file: 'api/occupational-health.ts', iface: 'CreateHazardMonitoringPayload' },
  { file: 'api/occupational-health.ts', iface: 'CreatePpeDistributionPayload' },
  { file: 'api/social-insurance.ts', iface: 'CreateSocialInsurancePayload' },
];

/** 视图提交点：文件 ↔ 被调创建函数 ↔ 提交形态（直传字面量 / 组装 payload 变量） */
const VIEW_CALL_SITES: Array<{ file: string; fn: string; kind: 'call' | 'payload-var' }> = [
  { file: 'views/export-compliance/index.vue', fn: 'createDischargeRecord', kind: 'call' },
  { file: 'views/export-compliance/index.vue', fn: 'createCustomsDeclaration', kind: 'call' },
  { file: 'views/tax-rebates/index.vue', fn: 'createCustomsDeclaration', kind: 'call' },
  { file: 'views/period-adjustments/index.vue', fn: 'createPeriodAdjustment', kind: 'call' },
  { file: 'views/labor-contracts/index.vue', fn: 'createLaborContract', kind: 'payload-var' },
  { file: 'views/occupational-health/index.vue', fn: 'createHealthExam', kind: 'call' },
  { file: 'views/occupational-health/index.vue', fn: 'createHazardMonitoring', kind: 'call' },
  { file: 'views/occupational-health/index.vue', fn: 'createPpeDistribution', kind: 'call' },
  { file: 'views/social-insurance/index.vue', fn: 'createSocialInsurance', kind: 'call' },
];

/** 递归列出 src/api 与 src/views 下全部 .ts/.vue 源文件（绝对路径） */
function listSourceFiles(): string[] {
  const out: string[] = [];
  const walk = (dir: string): void => {
    for (const entry of readdirSync(dir)) {
      const abs = path.join(dir, entry);
      if (statSync(abs).isDirectory()) {
        walk(abs);
      } else if (/\.(ts|vue)$/.test(entry)) {
        out.push(abs);
      }
    }
  };
  walk(path.join(FRONTEND_ROOT, 'src', 'api'));
  walk(path.join(FRONTEND_ROOT, 'src', 'views'));
  return out;
}

/**
 * 对一个源文件做通用身份扫描：所有 *Payload 接口声明块 + 所有 createX({..}) 直传字面量，
 * 返回命中的身份键列表。供尚未接入端点（solid-waste-disposals）的正向锁与夹具自证复用。
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

describe('9 创建端点请求侧身份键锁', () => {
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

  it('solid-waste-disposals 正向锁：任何提及该端点的 src 文件不得携带身份键', () => {
    const mentioning = listSourceFiles().filter(f => readRepoSource(f).includes('solid-waste'));
    // 当前前端未接入该端点（提及文件集为空集，接入后自动纳入扫描而不需改本锁）
    for (const f of mentioning) {
      const src = readRepoSource(f);
      expect(identityViolationsInSource(src), `${f} 携带身份键`).toEqual([]);
    }
  });

  it('检测力自证：注入身份键的夹具必须被上述判据抓到，而非静默通过', () => {
    // 夹具 1：往真实接口文本里塞 created_by 声明，接口判据必须点名
    const mutated = frontendSrc('api/labor-contract.ts').replace(
      'export interface CreateLaborContractPayload {\n  worker_id: number;',
      'export interface CreateLaborContractPayload {\n  worker_id: number;\n  created_by: number;'
    );
    expect(mutated).not.toBe(frontendSrc('api/labor-contract.ts')); // 夹具替换本身生效
    expect(interfaceIdentityViolations(mutated, 'CreateLaborContractPayload')).toEqual([
      'created_by',
    ]);

    // 夹具 2：往提交字面量里塞身份键，字面量判据必须点名
    const callFixture =
      'await createSocialInsurance({\n  worker_id: 1,\n  created_by_name: form.realName,\n});';
    const fixtureKeys = literalKeys(
      payloadLiteralBodies(callFixture, 'createSocialInsurance', 'call')[0]
    );
    expect(fixtureKeys.filter(k => IDENTITY_KEYS.includes(k))).toEqual(['created_by_name']);

    // 夹具 3：尚未接入端点的正向扫描同样有检测力
    const wasteFixture =
      'export interface CreateSolidWasteDisposalPayload {\n  waste_type: string;\n  created_by: number;\n}\n' +
      'const ok = createSolidWasteDisposal({\n  waste_type: t,\n  createdBy: u,\n});';
    expect(identityViolationsInSource(wasteFixture).sort()).toEqual(['createdBy', 'created_by']);
    // 反向对照：干净夹具零命中，证明上述点名非正则误报
    expect(
      identityViolationsInSource(
        'export interface CreateSolidWasteDisposalPayload {\n  waste_type: string;\n}'
      )
    ).toEqual([]);
  });
});
