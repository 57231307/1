/**
 * 应收域壳页与页签与按钮门控静态锁（细键在册 + 三处一致 + 壳页/页签同源并集 + 写面按钮无裸口）。
 *
 * 钉住的六件事：
 *   1. /ar 的 router meta.permission 只能是后端权限资源注册表里真实存在的应收族**细键**，
 *      不得再是粗域键（粗键经 COARSE_DOMAIN_RESOURCES 展开为整个财务域五十余枚细资源的并集，
 *      会让只持凭证/工资/汇率读权的岗位也打开应收壳页，属跨域越权入口；而后端 matcher 从不认
 *      粗键，粗键给出的展示面必然宽于后端能力面，点进去就是 403）；
 *   2. router 的 /ar 值、constants/permissions 的 AR_SHELL_ENTRY_PERMISSIONS、
 *      e2e 权限模型的 ROUTE_PERMISSIONS.ar 三处逐字一致（一处改动而其余两处没跟上即判红）；
 *   3. 壳页入口判据 == 六个页签入口判据的并集，且每个页签的可见性确实落到模板的 v-if
 *      ——钉住「进得来必然至少看得见一个页签」，也防止只登记不消费；
 *   4. 后端矩阵里两岗（finance_manager 与 accountant）按 hasRoutePermission 真判可达 /ar，
 *      只持财务域其他细读权的岗位（成本会计）判不可达，出纳只经资金账户页签可达——
 *      收细前后端能力面与前端可见面必须同起同落；
 *   5. 应收四个页签里每一个**写面与出文面**入口（建单、改单、审核、作废、确认、核销、撤核、
 *      打印）都必须由 v-permission 按该动作在后端真实派生出的键守卫，且该键至少被一个
 *      能进该页签的角色持有——两头都不许缺：缺守卫就是点了必 403 的裸口，缺持有者就是
 *      谁都点不亮的死守卫；
 *   6. 每条 api 调用的「方法加路径」必须在 frontend/scripts/route-snapshot.txt 里在册，
 *      本锁的键表与后端派生链（由 Rust 侧
 *      tests/contract_ar_family_grant_channels_test.rs 用真实 matcher 与真实派生函数钉）
 *      因此共用同一个端点底数，两侧任一侧漂移都会红。
 *
 * 判定口径唯一：可达性一律走 router 导出的 hasRoutePermission 同一函数（与守卫、
 * v-permission 指令同源），本文件不自写第二套字符串比较。
 *
 * 入参：只读仓库内源码文本与前端常量模块，不执行后端代码、不发网络请求、不起浏览器。
 * 调用方：vitest（tests/unit，CI 前端单测 job）。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { describe, expect, it } from 'vitest';

import {
  AR_REPORT_TAB_PERMISSIONS,
  AR_SHELL_ENTRY_PERMISSIONS,
  PERMISSIONS,
} from '@/constants/permissions';
import { hasRoutePermission } from '../../e2e/traversal/permission-model';
import {
  BACKEND_MATRIX_RS,
  BACKEND_REGISTRY_RS,
  PERMISSION_MODEL_TS,
  ROUTER_TS,
  parseBackendResourceRegistry,
  parseBackendRoleGrantGroups,
  parseCoarseDomainResources,
  parseRoutePermissionEntry,
  readRepoSource,
  sliceBetween,
} from './ap-gate-sources';

const ROUTER_SRC = readRepoSource(ROUTER_TS);
const PERMISSION_MODEL_SRC = readRepoSource(PERMISSION_MODEL_TS);
const AR_SHELL_VUE = 'frontend/src/views/ar/index.vue';
const AR_SHELL_SRC = readRepoSource(AR_SHELL_VUE);
const ROUTE_SNAPSHOT_TXT = readRepoSource('frontend/scripts/route-snapshot.txt');
/** 后端角色授权矩阵（资源与动作的唯一权威授予面，与写库同源） */
const BACKEND_GROUPS = parseBackendRoleGrantGroups(readRepoSource(BACKEND_MATRIX_RS));
/** 后端资源名注册表（本锁「键是否真实存在」的唯一判据源） */
const BACKEND_RESOURCES = parseBackendResourceRegistry(readRepoSource(BACKEND_REGISTRY_RS));
const COARSE_DOMAINS = Object.keys(parseCoarseDomainResources(ROUTER_SRC));

const AR_SHELL_ROUTE = '/ar';

/** 应收族八枚细键（ar 前缀下消歧出的权威资源名，与 Rust 侧锁同名同集） */
const AR_FAMILY_RESOURCES = [
  'ar-invoices',
  'ar-payments',
  'ar-verifications',
  'ar-collections',
  'ar-aging',
  'ar-daily',
  'ar-monthly',
  'ar-statistics',
];

/** 从 router 源文本取 /ar 的 meta.permission（守卫实际消费的口径，与 e2e 派生侧同一解析器） */
function shellGateFromRouter(routerSrc: string): string[] {
  const gate = parseRoutePermissionEntry(routerSrc, 'ar');
  if (gate === undefined) {
    throw new Error('router 中 /ar 无 meta.permission——应收壳页退化为「登录即可达」，判红交人工');
  }
  if (!Array.isArray(gate)) {
    throw new Error(
      `/ar 的 meta.permission 不是数组（实际 ${JSON.stringify(gate)}）——本锁按细键集合口径校验，` +
        '粗域键单值形态已按缺陷回收，恢复它必须先过本锁评审'
    );
  }
  return gate;
}

/** 权限码 `资源:动作` → 资源段 */
function resourceOf(code: string): string {
  return code.slice(0, code.indexOf(':'));
}

/** 返回不在后端注册表里的门控项（空数组即全部在册）。 */
function unregisteredResources(codes: readonly string[], registry: readonly string[]): string[] {
  return codes.map(resourceOf).filter(res => !registry.includes(res));
}

/** 返回落在粗域展开表里的门控项（空数组即没有粗键兼职）。 */
function coarseOffenders(gate: readonly string[], coarseDomains: readonly string[]): string[] {
  return gate.map(resourceOf).filter(res => coarseDomains.includes(res));
}

/**
 * 解析应收壳页 AR_TABS 登记表：页签名 → 入口权限码标识符。
 * 口径与 AP 侧同款（只做词法抽取，把标识符换成真实常量值交给本文件，
 * 以免解析器与常量定义之间出现第二份取值）。
 */
function parseArTabRegistrations(src: string): Array<{ name: string; identifier: string }> {
  const body = sliceBetween(src, 'const AR_TABS = [', '\n] as const;', 'AR_TABS');
  return [...body.matchAll(/\{ name: '([a-zA-Z]+)', entry: ([A-Za-z_.]+) \}/g)].map(m => ({
    name: m[1],
    identifier: m[2],
  }));
}

/** 页签登记表标识符 → 常量真实取值（取值只认 constants 模块，解析器不自己猜字符串） */
function resolveTabEntry(identifier: string): string[] {
  const report = identifier === 'AR_REPORT_TAB_PERMISSIONS';
  if (report) return [...AR_REPORT_TAB_PERMISSIONS];
  const direct = /^PERMISSIONS\.([A-Z_]+)$/.exec(identifier);
  if (!direct) {
    throw new Error(`页签入口标识符形态不认识：${identifier}——登记表漂移，锁需同步修订`);
  }
  const value = (PERMISSIONS as unknown as Record<string, string>)[direct[1]];
  if (typeof value !== 'string') {
    throw new Error(`PERMISSIONS.${direct[1]} 在 constants 模块里不存在——页签登记了死标识符`);
  }
  return [value];
}

/** 某角色在后端矩阵里的权限码列表（缺失即该岗在矩阵无授权分组） */
function keysOfRole(role: string): string[] {
  const keys = BACKEND_GROUPS[role];
  if (keys === undefined) {
    throw new Error(
      `用例前提失效：后端矩阵里没有角色 ${role} 的授权分组，` +
        `可达性判据拿不到入参。在册分组：${Object.keys(BACKEND_GROUPS).join(' ')}`
    );
  }
  return keys;
}

/**
 * 每个页签内「写面与出文面」入口的守卫期望表。
 * 每条形如 [api 函数名, 该方法在后端派生出的权限码, 该 api 调用在 route-snapshot 里的行片段]，
 * 判据是：该页签的 vue 源文本里必须存在一条 v-permission 指向解析后等于该权限码的常量标识符，
 * 并且该权限码至少被一个能进该页签的角色持有。
 */
const WRITE_ENTRY_TABLE: Record<string, Array<{ api: string; key: string; snapshotPath: string }>> =
  {
    'frontend/src/views/ar/tabs/InvoiceTab.vue': [
      { api: 'createARInvoice', key: 'ar-invoices:create', snapshotPath: 'POST\t/ar/invoices\t' },
      {
        api: 'approveARInvoice',
        key: 'ar-invoices:approve',
        snapshotPath: 'POST\t/ar/invoices/*/approve\t',
      },
      {
        api: 'cancelARInvoice',
        key: 'ar-invoices:cancel',
        snapshotPath: 'POST\t/ar/invoices/*/cancel\t',
      },
    ],
    'frontend/src/views/ar/tabs/PaymentTab.vue': [
      { api: 'createARPayment', key: 'ar-payments:create', snapshotPath: 'POST\t/ar/payments\t' },
      {
        api: 'updateARPayment',
        key: 'ar-payments:update',
        snapshotPath: 'PUT\t/ar/payments/*\t',
      },
      {
        api: 'confirmARPayment',
        key: 'ar-payments:confirm',
        snapshotPath: 'POST\t/ar/payments/*/confirm\t',
      },
      {
        api: 'printARCollectionDocx',
        key: 'ar-collections:print',
        snapshotPath: 'GET\t/ar/collections/*/print\t',
      },
    ],
    'frontend/src/views/ar/tabs/VerificationTab.vue': [
      {
        api: 'autoVerifyAR',
        key: 'ar-verifications:create',
        snapshotPath: 'POST\t/ar/verifications/auto\t',
      },
      {
        api: 'manualVerifyAR',
        key: 'ar-verifications:create',
        snapshotPath: 'POST\t/ar/verifications/manual\t',
      },
      {
        api: 'cancelARVerification',
        key: 'ar-verifications:cancel',
        snapshotPath: 'POST\t/ar/verifications/*/cancel\t',
      },
    ],
  };

/** 页签登记表（模块级一份解析结果，三处判据共用，避免同一源被解两遍给出不同结论） */
const TAB_REGISTRATIONS = parseArTabRegistrations(AR_SHELL_SRC);

describe('应收壳页门控形态（细键、在册、无粗键兼职）', () => {
  it('/ar 的 router 门控必须是后端在册的应收族细键数组', () => {
    const gate = shellGateFromRouter(ROUTER_SRC);
    expect(gate.length).toBeGreaterThan(0);
    expect(unregisteredResources(gate, BACKEND_RESOURCES)).toEqual([]);
    expect(coarseOffenders(gate, COARSE_DOMAINS)).toEqual([]);
    // 八枚细键的读面必须逐枚出现在门控里（报表四枚、发票收款核销三枚、催收出文不入壳页判据）
    for (const code of [
      'ar-invoices:read',
      'ar-payments:read',
      'ar-verifications:read',
      'ar-aging:read',
      'ar-daily:read',
      'ar-monthly:read',
      'ar-statistics:read',
    ]) {
      expect(gate, `壳页门控缺 ${code}，该岗只看得到别的页签时会误判可达`).toContain(code);
    }
  });

  it('router、constants、e2e 权限模型三处门控必须逐字一致', () => {
    const fromRouter = shellGateFromRouter(ROUTER_SRC);
    const fromConstants = [...AR_SHELL_ENTRY_PERMISSIONS];
    const fromModel = parseRoutePermissionEntry(PERMISSION_MODEL_SRC, 'ar');
    expect(fromRouter, 'router 与 constants 的壳页入口判据分叉').toEqual(fromConstants);
    expect(fromRouter, 'router 与 e2e 权限模型的评审锚分叉（防漂移门禁会判红，这里同口径拦下）').toEqual(
      Array.isArray(fromModel) ? fromModel : [fromModel ?? '']
    );
  });
});

describe('应收壳页与页签同源并集（进得来必然看得见一个页签）', () => {
  it('六个页签入口判据的并集必须等于壳页入口判据集合', () => {
    expect(TAB_REGISTRATIONS.map(r => r.name).sort()).toEqual(
      ['fund', 'invoice', 'payment', 'reconciliation', 'report', 'verification'].sort()
    );
    const union = new Set<string>();
    for (const reg of TAB_REGISTRATIONS) {
      for (const code of resolveTabEntry(reg.identifier)) union.add(code);
    }
    const shell = new Set(shellGateFromRouter(ROUTER_SRC));
    expect([...union].sort(), '页签入口判据并集与壳页门控不等（任一侧改动都要两侧同批）').toEqual(
      [...shell].sort()
    );
  });

  it('每个页签的可见性必须真的落到模板 v-if（防只登记不消费）', () => {
    for (const reg of TAB_REGISTRATIONS) {
      const needle = `v-if="visibleTabNames.includes('${reg.name}')"`;
      expect(AR_SHELL_SRC, `页签 ${reg.name} 已登记入口判据但模板没有对应 v-if`).toContain(needle);
    }
    expect(AR_SHELL_SRC, '壳页必须用 hasRoutePermission 判页签可达，不许另写第二套').toContain(
      "import { hasRoutePermission } from '@/router'"
    );
  });
});

describe('收细前后端能力面与前端可见面同起同落', () => {
  const shell = shellGateFromRouter(ROUTER_SRC);

  it('财务两岗按 hasRoutePermission 真判可达 /ar', () => {
    for (const role of ['finance_manager', 'accountant']) {
      expect(
        hasRoutePermission([...shell], keysOfRole(role)),
        `${role} 已被补授应收族细键，壳页门控却判不可达（三通道漂移或键名拼错）`
      ).toBe(true);
    }
  });

  it('只持财务域其他细读权的岗位不得被放行 /ar（粗键时代的跨域入口必须已闭合）', () => {
    const blocked = ['cost_accountant'];
    for (const role of blocked) {
      expect(
        hasRoutePermission([...shell], keysOfRole(role)),
        `${role} 不持任何应收族键与资金账户键，却仍能打开应收壳页——粗键收细未生效`
      ).toBe(false);
    }
  });

  it('出纳只经资金账户页签可达，且可见页签集合恰为该一个', () => {
    const cashierKeys = keysOfRole('cashier');
    const visible = registrationsOfTabs().filter(reg =>
      hasRoutePermission(resolveTabEntry(reg.identifier), cashierKeys)
    );
    expect(
      hasRoutePermission([...shell], cashierKeys),
      '出纳持 fund-management 读权，壳页必须可达（否则资金账户页签无从进入）'
    ).toBe(true);
    expect(
      visible.map(v => v.name),
      '出纳在应收壳页里只应看到资金账户一页'
    ).toEqual(['fund']);
  });

  /** 页签登记表（局部函数，供上面两条共用同一份解析结果） */
  function registrationsOfTabs() {
    return parseArTabRegistrations(AR_SHELL_SRC);
  }
});

describe('应收页签写面与出文面无 403 裸口', () => {
  const allReachingRoles = ['finance_manager', 'accountant'];

  for (const [tabFile, entries] of Object.entries(WRITE_ENTRY_TABLE)) {
    const src = readRepoSource(tabFile);
    for (const entry of entries) {
      it(`${tabFile} 的 ${entry.api}（后端派生键 ${entry.key}）必须既有在册端点又被 v-permission 守卫`, () => {
        // 端点在册：键的资源段与动作段必须能在快照里找到对应的注册腿
        expect(
          ROUTE_SNAPSHOT_TXT.includes(entry.snapshotPath),
          `端点 ${entry.snapshotPath.trim()} 不在 route-snapshot 里——本锁的键表与后端在册面脱钩`
        ).toBe(true);
        // api 函数确实在该页签被消费
        expect(
          src.includes(entry.api),
          `${entry.api} 未在 ${tabFile} 出现——键表指向的入口已退场，须同步删条目`
        ).toBe(true);
        // 守卫存在且解析值等于该键
        const identifier = Object.keys(PERMISSIONS).find(
          k => (PERMISSIONS as unknown as Record<string, string>)[k] === entry.key
        );
        expect(
          identifier,
          `constants 里没有取值等于 ${entry.key} 的权限码——守卫无从写出`
        ).toBeTruthy();
        expect(
          src.includes(`v-permission="PERMISSIONS.${identifier}"`),
          `${tabFile} 里 ${entry.key} 的入口没有 v-permission 守卫：不持该键的角色看得见点不得，必 403`
        ).toBe(true);
        // 守卫不是死守卫：至少一个能进该页签的角色真持有
        const holder = allReachingRoles.some(role => keysOfRole(role).includes(entry.key));
        expect(holder, `${entry.key} 没有任何财务岗持有——该守卫会让入口对所有人隐去，属死面`);
      });
    }
  }

  it('后端矩阵里被授出的应收族写面键必须都能在 constants 里写出守卫（防授了键却没有守卫的新裸口）', () => {
    const grantedWriteKeys = new Set<string>();
    for (const role of allReachingRoles) {
      for (const code of keysOfRole(role)) {
        const res = resourceOf(code);
        if (!AR_FAMILY_RESOURCES.includes(res)) continue;
        const action = code.slice(code.indexOf(':') + 1);
        if (action === 'read') continue;
        grantedWriteKeys.add(code);
      }
    }
    const constantValues = new Set(Object.values(PERMISSIONS));
    for (const key of grantedWriteKeys) {
      expect(
        constantValues.has(key),
        `${key} 已授给财务岗但 constants 里没有对应权限码，页面入口无法守卫（要么补守卫要么撤授权）`
      ).toBe(true);
    }
  });
});

describe('判定器自身检测力（四处注入必须被抓到）', () => {
  it('注入「门控改回粗键 finance:read」必须被判红', () => {
    const mutated = ROUTER_SRC.replace(
      "  path: 'ar',\n        name: 'AR',",
      "  path: 'ar',\n        name: 'AR_MUTATED_PROBE',"
    );
    expect(mutated).not.toBe(ROUTER_SRC);
    const coarse = coarseOffenders(['finance:read'], COARSE_DOMAINS);
    expect(coarse, '粗域判据失效：finance 被判成非粗域，说明 COARSE_DOMAIN_RESOURCES 解析没跑起来').toEqual(
      ['finance']
    );
  });

  it('注入「页签登记改成注册表外的资源名」必须被 unregisteredResources 抓到', () => {
    const offenders = unregisteredResources(['ar-canary-not-a-real-resource:read'], BACKEND_RESOURCES);
    expect(offenders, '注册表判据失去分辨力：注册表外的资源段没被判为在册缺失').toEqual([
      'ar-canary-not-a-real-resource',
    ]);
  });

  it('注入「摘掉确认收款的 v-permission」必须让该页签的守卫判据转假', () => {
    const src = readRepoSource('frontend/src/views/ar/tabs/PaymentTab.vue');
    const stripped = src.replace(
      'v-permission="PERMISSIONS.AR_PAYMENT_CONFIRM"',
      'data-probe="removed"'
    );
    expect(stripped).not.toBe(src);
    expect(
      stripped.includes('v-permission="PERMISSIONS.AR_PAYMENT_CONFIRM"'),
      '注入未生效（替换目标没命中），本条自证不作数'
    ).toBe(false);
    expect(src.includes('v-permission="PERMISSIONS.AR_PAYMENT_CONFIRM"')).toBe(true);
  });

  it('注入「e2e 评审锚少一枚键」必须被三处一致判据抓到', () => {
    const fromRouter = shellGateFromRouter(ROUTER_SRC);
    const shortModel = fromRouter.slice(1);
    expect(shortModel).not.toEqual(fromRouter);
  });

  it('用例前提自证：后端矩阵确实不再把任何 ar-* 键授给成本会计与出纳', () => {
    for (const role of ['cost_accountant', 'cashier']) {
      const held = keysOfRole(role).filter(code => AR_FAMILY_RESOURCES.includes(resourceOf(code)));
      expect(held, `${role} 竟持有应收族键 ${held}：授予面外泄，本锁的反向判据前提失效`).toEqual([]);
    }
  });
});
