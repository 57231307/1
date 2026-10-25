/**
 * 应付域壳页与页签门控静态锁（细键在册 + 三处一致 + 壳页/页签同源并集）。
 *
 * 钉住的四件事：
 *   1. /ap 的 router meta.permission 只能是后端权限资源注册表里真实存在的应付族**细键**，
 *      不得再是粗域键（粗键经 COARSE_DOMAIN_RESOURCES 展开为整个财务域的并集，
 *      会让只持凭证/工资/汇率读权的岗位也打开应付壳页，属跨域越权入口）；
 *   2. router 的 /ap 值、constants/permissions 的 AP_SHELL_ENTRY_PERMISSIONS、
 *      e2e 权限模型的 ROUTE_PERMISSIONS 评审锚三处逐字一致（一处改动而其余两处没跟上即判红）；
 *   3. router 与 e2e 权限模型两份 COARSE_DOMAIN_RESOURCES 是人工镜像关系，必须逐字相同；
 *   4. 壳页入口判据 == 六个页签入口判据的并集，且每个页签的可见性确实落到模板的 v-if
 *      ——钉住"进得来必然至少看得见一个页签"，也防止只登记不消费。
 *
 * 入参：只读仓库内源码文本与前端常量模块，不执行后端代码、不发网络请求、不起浏览器。
 * 调用方：vitest（tests/unit，CI 前端单测 job）。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { describe, expect, it } from 'vitest';

import {
  AP_REPORT_TAB_PERMISSIONS,
  AP_SHELL_ENTRY_PERMISSIONS,
  PERMISSIONS,
} from '@/constants/permissions';
import { parseRouterPermissions } from '../../e2e/traversal/permission-model';
import {
  AP_SHELL_VUE,
  BACKEND_REGISTRY_RS,
  PERMISSION_MODEL_TS,
  ROUTER_TS,
  parseApTabRegistrations,
  parseBackendResourceRegistry,
  parseCoarseDomainResources,
  parseRoutePermissionEntry,
  readRepoSource,
} from './ap-gate-sources';

const ROUTER_SRC = readRepoSource(ROUTER_TS);
const PERMISSION_MODEL_SRC = readRepoSource(PERMISSION_MODEL_TS);
const AP_SHELL_SRC = readRepoSource(AP_SHELL_VUE);
/** 后端资源名注册表（本锁的"键是否真实存在"唯一判据源） */
const BACKEND_RESOURCES = parseBackendResourceRegistry(readRepoSource(BACKEND_REGISTRY_RS));

/** 从 router 源文本取 /ap 的 meta.permission（守卫实际消费的口径，与 e2e 派生侧同一解析器） */
function shellGateFromRouter(routerSrc: string): string[] {
  const gate = parseRouterPermissions(routerSrc)['/ap'];
  if (gate === null || gate === undefined) {
    throw new Error('router 中 /ap 无 meta.permission——应付壳页退化为"登录即可达"，判红交人工');
  }
  if (!Array.isArray(gate)) {
    throw new Error(
      `/ap 的 meta.permission 不是数组（实际 ${JSON.stringify(gate)}）——本锁按细键集合口径校验`
    );
  }
  return gate;
}

/** 权限码 `资源:动作` → 资源段 */
function resourcesOf(codes: readonly string[]): string[] {
  return codes.map(code => code.slice(0, code.indexOf(':')));
}

/** 返回不在后端注册表里的资源段（空数组即全部在册）。 */
function unregisteredResources(codes: readonly string[], registry: readonly string[]): string[] {
  return resourcesOf(codes).filter(res => !registry.includes(res));
}

/** 返回落在粗域展开表里的门控项（空数组即没有粗键兼职）。 */
function coarseOffenders(gate: readonly string[], coarseDomains: readonly string[]): string[] {
  return resourcesOf(gate).filter(res => coarseDomains.includes(res));
}

/** 报表面四枚入口判据（页签并集算式的右半边，锁与夹具共用同一份值） */
const REPORT_TAB_KEYS: readonly string[] = [...AP_REPORT_TAB_PERMISSIONS];

/** 页签登记表：标识符 → 常量真实取值（取值只认 constants 模块，解析器不自己猜字符串） */
function resolveTabEntry(identifier: string): string[] {
  const direct = /^PERMISSIONS\.([A-Z_]+)$/.exec(identifier);
  if (direct) {
    const value = (PERMISSIONS as unknown as Record<string, string>)[direct[1]];
    if (typeof value !== 'string') {
      throw new Error(`constants/permissions 里没有 ${identifier}——页签登记引用了不存在的常量`);
    }
    return [value];
  }
  if (identifier === 'AP_REPORT_TAB_PERMISSIONS') return [...REPORT_TAB_KEYS];
  throw new Error(
    `页签入口判据 ${identifier} 不是本锁认识的常量形态（PERMISSIONS.X 或 AP_REPORT_TAB_PERMISSIONS）`
  );
}

describe('应付壳页 /ap 门控：只认后端在册细键', () => {
  const gate = shellGateFromRouter(ROUTER_SRC);

  it('门控为非空细键集合，且不含任何粗域键', () => {
    expect(gate.length, '应付壳页门控为空集等于把页面锁死给超管').toBeGreaterThan(0);
    const coarseDomains = Object.keys(parseCoarseDomainResources(ROUTER_SRC));
    const offenders = coarseOffenders(gate, coarseDomains);
    expect(
      offenders,
      `/ap 仍引用粗域键（会被展开为整域并集放行）：${offenders.join(', ')}`
    ).toEqual([]);
    for (const code of gate) {
      expect(code, `/ap 门控项 ${code} 不是 "资源:动作" 形态`).toContain(':');
      expect(code.split(':')[1], `/ap 门控项 ${code} 缺动作段`).not.toBe('');
    }
  });

  it('每一枚键的资源段都在后端 PERMISSION_RESOURCES 注册表在册（同源文本核对，非另抄清单）', () => {
    expect(BACKEND_RESOURCES.length, '后端资源注册表解析条数为 0——解析口径已漂移').toBeGreaterThan(
      0
    );
    expect(unregisteredResources(gate, BACKEND_RESOURCES), `/ap 门控含注册表外编造键`).toEqual([]);
  });

  it('router /ap == constants 的 AP_SHELL_ENTRY_PERMISSIONS == ROUTE_PERMISSIONS 评审锚（三处逐字一致）', () => {
    expect(gate).toEqual([...AP_SHELL_ENTRY_PERMISSIONS]);
    const declared = parseRoutePermissionEntry(PERMISSION_MODEL_SRC, 'ap');
    expect(declared, 'e2e 权限模型 ROUTE_PERMISSIONS 里没有 ap 条目').toBeDefined();
    expect(declared).toEqual(gate);
  });

  it('router 与 e2e 权限模型的 COARSE_DOMAIN_RESOURCES 逐字相同（人工镜像表不得分叉）', () => {
    expect(parseCoarseDomainResources(PERMISSION_MODEL_SRC)).toEqual(
      parseCoarseDomainResources(ROUTER_SRC)
    );
  });
});

describe('应付页签级可达性门控：与壳页门控同源且真被模板消费', () => {
  const gate = shellGateFromRouter(ROUTER_SRC);
  const tabs = parseApTabRegistrations(AP_SHELL_SRC);

  it('六个页签全部登记入口判据，每枚键都在册', () => {
    expect(tabs.map(t => t.name)).toEqual([
      'invoice',
      'payment',
      'paymentRequest',
      'verification',
      'reconciliation',
      'report',
    ]);
    for (const tab of tabs) {
      const entries = resolveTabEntry(tab.identifier);
      expect(entries.length, `页签 ${tab.name} 的入口判据为空`).toBeGreaterThan(0);
      expect(
        unregisteredResources(entries, BACKEND_RESOURCES),
        `页签 ${tab.name} 含注册表外键`
      ).toEqual([]);
    }
  });

  it('壳页入口判据 == 六个页签入口判据的并集（进得来必然至少看得见一个页签）', () => {
    const union = [...new Set(tabs.flatMap(tab => resolveTabEntry(tab.identifier)))].sort();
    const shell = [...new Set(gate)].sort();
    expect(union).toEqual(shell);
  });

  it('每个页签的可见性判定写在模板 v-if 上（结构性门控真实生效，不是只登记）', () => {
    for (const tab of tabs) {
      expect(
        AP_SHELL_SRC.includes(`v-if="visibleTabNames.includes('${tab.name}')"`),
        `页签 ${tab.name} 未接结构性可见性判定`
      ).toBe(true);
    }
    // 面板级 v-permission 指令移除的是面板 DOM，页签标签由 el-tabs 另一棵子树渲染，
    // 会留下"标签在、点开空白"的假收敛——应付壳页禁止对 el-tab-pane 用该指令。
    expect(
      /<el-tab-pane[^>]*v-permission/.test(AP_SHELL_SRC),
      'el-tab-pane 上出现 v-permission'
    ).toBe(false);
  });
});

describe('检测力自证：四类单边改动必须被上面的锁抓到', () => {
  it('夹具一：把 /ap 里一枚细键换回粗域键 finance:read，粗域键判据与三处一致判据同时点名', () => {
    // 夹具本身必须生效：'ap-invoices:read' 在 router 文本里只出现在 /ap 门控数组内
    const mutated = ROUTER_SRC.replace("'ap-invoices:read',", "'finance:read',");
    expect(mutated, '夹具替换未生效（router 文本形态已变，本自证需同步修订）').not.toBe(ROUTER_SRC);
    const mutatedGate = shellGateFromRouter(mutated);
    const coarseDomains = Object.keys(parseCoarseDomainResources(ROUTER_SRC));
    expect(coarseDomains).toContain('finance');
    expect(
      coarseOffenders(mutatedGate, coarseDomains),
      '粗域键混进 /ap 门控却没被判据抓到'
    ).toEqual(['finance']);
    expect(mutatedGate).not.toEqual([...AP_SHELL_ENTRY_PERMISSIONS]);
    // 现值必须干净，否则上面两条判据在没有回归时也"恒红"，自证就失去意义
    expect(coarseOffenders(shellGateFromRouter(ROUTER_SRC), coarseDomains)).toEqual([]);
  });

  it('夹具二：注册表删掉 ap-aging，在册判据必须点名该键（编造/失注册键的检出力）', () => {
    const gate = shellGateFromRouter(ROUTER_SRC);
    expect(unregisteredResources(gate, BACKEND_RESOURCES)).toEqual([]);
    expect(
      unregisteredResources(
        gate,
        BACKEND_RESOURCES.filter(r => r !== 'ap-aging')
      )
    ).toEqual(['ap-aging']);
    // 反向：注册表里根本没有的名字，加进门控即被点名
    expect(unregisteredResources(['ap-nonexistent:read'], BACKEND_RESOURCES)).toEqual([
      'ap-nonexistent',
    ]);
  });

  it('夹具三：页签登记与壳页并集分叉（摘掉报表页签登记），并集判据必须失守', () => {
    const mutated = AP_SHELL_SRC.replace(
      "  { name: 'report', entry: AP_REPORT_TAB_PERMISSIONS },\n",
      ''
    );
    expect(mutated, '夹具替换未生效（AP_TABS 形态已变，本自证需同步修订）').not.toBe(AP_SHELL_SRC);
    const brokenUnion = [
      ...new Set(parseApTabRegistrations(mutated).flatMap(tab => resolveTabEntry(tab.identifier))),
    ].sort();
    expect(brokenUnion).not.toEqual([...new Set(shellGateFromRouter(ROUTER_SRC))].sort());
    expect(
      REPORT_TAB_KEYS.every(k => shellGateFromRouter(ROUTER_SRC).includes(k)),
      '报表面本就未进壳页门控'
    ).toBe(true);
  });

  it('夹具四：把某页签的 v-if 摘掉（只登记不消费），模板消费判据必须点名', () => {
    const mutated = AP_SHELL_SRC.replace(
      `v-if="visibleTabNames.includes('verification')"`,
      `v-if="true"`
    );
    expect(mutated, '夹具替换未生效（模板形态已变，本自证需同步修订）').not.toBe(AP_SHELL_SRC);
    expect(
      mutated.includes(`v-if="visibleTabNames.includes('verification')"`),
      '摘掉 v-if 后消费判据无从失守，说明夹具没改动到判据读取的那段文本'
    ).toBe(false);
    expect(
      shellGateFromRouter(ROUTER_SRC).includes(PERMISSIONS.AP_VERIFICATION_READ),
      '核销页签的入口键本就受壳页门控放行（说明该页签不在并集里，自证需同步修订）'
    ).toBe(true);
  });
});
