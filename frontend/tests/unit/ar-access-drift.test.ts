/**
 * 应收壳页 /ar 门控收细键的可达集复算（纯函数，不连浏览器、不连后端）。
 *
 * 用途：本支把 /ar 的入口判据从粗域键 finance:read 收成九枚细键，同时给财务两岗补授应收族
 * 键——这既改变"谁能进壳页"，也改变"谁进得去哪些页签"。角色权限矩阵基线轨据此判漂移，
 * 本用例把"谁失去 /ar、谁保留、由哪一枚键解释"从手抄数字变成可复算判据，并与**已重采的基线**
 * 双向对账：按新口径复算出的可达性必须逐角色等于基线现值；按旧口径（粗域键 finance:read）
 * 复算时，与基线的差异集必须恰好等于本次点名的五个收权岗。任一方向不成立，都说明本用例的
 * 解析口径或权限模型已与真实守卫分叉，先停下交人工而不是改期望值。
 *
 * 输入（三件事，来源各不相同）：
 *   1. 角色权限码：解析后端 init_service_ops/permission.rs 的角色授权矩阵源文本（通道①），
 *      并按本支补授集做前后态分离——本支的补授与收细在同一个改动面上，因此
 *      "补授后的矩阵"直接取自文本现值，"补授前的矩阵"由现值减去本支点名的补授集得到；
 *      两态都从同一份文本派生，不手抄第二份权限表。
 *   2. 新旧门控：旧 = 粗域键 finance:read，新 = 从 router 源文本解析的 /ar meta 现值；
 *   3. 超管 *:* 通配：后端为 admin 注入，不在矩阵文本里，但可达集必须算进来。
 *
 * 判定用 e2e 权限模型的 hasRoutePermission（router 守卫的同源镜像：通配、read 与 view 等价、
 * 粗域键展开），不在本文件里另写第二套比较。
 *
 * 调用方：vitest（tests/unit，CI 前端单测 job）。
 * 入参：只读仓库内源码文本与 e2e/traversal/access-map-baseline 的基线 JSON；
 * 传给谁：断言结果进 vitest 报告，漂移表同时打进标准输出供基线重采逐份核对；
 * 存什么与存哪里：不落盘、不写候选基线、不改任何仓库文件。
 */
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import {
  AR_REPORT_TAB_PERMISSIONS,
  AR_SHELL_ENTRY_PERMISSIONS,
  PERMISSIONS,
} from '@/constants/permissions';
import { hasRoutePermission, parseRouterPermissions } from '../../e2e/traversal/permission-model';
import {
  BACKEND_MATRIX_RS,
  BACKEND_REGISTRY_RS,
  ROUTER_TS,
  parseBackendResourceRegistry,
  parseBackendRoleGrantGroups,
  parseCoarseDomainResources,
  readRepoSource,
} from './ap-gate-sources';

const ROUTER_SRC = readRepoSource(ROUTER_TS);
const BACKEND_RESOURCES = parseBackendResourceRegistry(readRepoSource(BACKEND_REGISTRY_RS));
/** 后端角色授权矩阵现值（角色码 到 资源:动作 列表），已含本支补授 */
const MATRIX_GROUPS = parseBackendRoleGrantGroups(readRepoSource(BACKEND_MATRIX_RS));
/** 粗域键 finance 展开后的细资源段集（旧口径"持该域任一细资源读权即放行整域壳页"的判据来源） */
const FINANCE_COARSE_RESOURCES = parseCoarseDomainResources(ROUTER_SRC).finance;
const COARSE_DOMAIN_KEYS = Object.keys(parseCoarseDomainResources(ROUTER_SRC));

/** /ar 收细键后的新门控（守卫实际消费的同一份值，由 e2e 解析器从 router 源文本取） */
const NEW_SHELL_GATE: string[] = (() => {
  const gate = parseRouterPermissions(ROUTER_SRC)['/ar'];
  if (!Array.isArray(gate) || gate.length === 0) {
    throw new Error(
      `/ar 的 meta.permission 不是非空数组（实际 ${JSON.stringify(gate)}）——收细键未落地，复算无输入`
    );
  }
  return gate;
})();

/** 改动前的门控（粗域键）：只在复算里出现一次，作为"旧口径"的显式输入 */
const OLD_SHELL_GATE = 'finance:read';

/**
 * 本支补授的应收族键集（通道① 的 finance_manager 与 accountant 两分组，
 * 与通道② 迁移 GRANTS 逐字符同源，由 backend/tests/
 * contract_ar_family_grant_channels_test.rs 钉住同源）。
 * 口径：会计拿制单与记账面十枚，财务经理拿审核、报表与催收出文面十一枚，
 * 出纳不持任何应收族键（显式空数组，而不是"查不到就当作 0 枚"）。
 * 本表有两个用途：一是复算"补授前的矩阵"（现值减去本表），二是反向钉住本支确实把这些键
 * 落进了通道①（缺一枚即说明矩阵与迁移分叉，存量库与新装库的授权面会不一致）。
 */
const AR_FAMILY_GRANTS_THIS_BATCH: Record<string, readonly string[]> = {
  accountant: [
    'ar-invoices:read',
    'ar-invoices:create',
    'ar-invoices:update',
    'ar-payments:read',
    'ar-payments:create',
    'ar-payments:update',
    'ar-verifications:read',
    'ar-verifications:create',
    'ar-verifications:cancel',
    'ar-collections:print',
  ],
  finance_manager: [
    'ar-invoices:read',
    'ar-invoices:approve',
    'ar-invoices:cancel',
    'ar-payments:read',
    'ar-payments:confirm',
    'ar-verifications:read',
    'ar-collections:print',
    'ar-aging:read',
    'ar-daily:read',
    'ar-monthly:read',
    'ar-statistics:read',
  ],
  cashier: [],
};

/** 超管由后端注入 *:* 通配（不进授权矩阵文本，但可达集必须把它算进来） */
const WILDCARD_ROLES: Record<string, readonly string[]> = { admin: ['*:*'] };

// ===== 基线目录（复算的对账对象与角色论域） =====

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const BASELINE_DIR = path.join(FRONTEND_ROOT, 'e2e/traversal/access-map-baseline');

const BASELINE_ROLES: string[] = readdirSync(BASELINE_DIR)
  .filter(f => f.endsWith('.json'))
  .map(f => f.replace(/\.json$/, ''))
  .sort();

/** 读一份已签署基线的可达集（只读取，绝不写候选、不自动生成） */
function baselineReachable(role: string): string[] {
  const parsed = JSON.parse(readFileSync(path.join(BASELINE_DIR, `${role}.json`), 'utf8')) as {
    role?: unknown;
    reachableRoutes?: unknown;
  };
  if (parsed.role !== role || !Array.isArray(parsed.reachableRoutes)) {
    throw new Error(
      `基线 ${role}.json schema 不符（role 与文件名不同源或 reachableRoutes 非数组）`
    );
  }
  return parsed.reachableRoutes as string[];
}

const BASELINE_REACHES_AR = new Map<string, boolean>(
  BASELINE_ROLES.map(role => [role, baselineReachable(role).includes('/ar')])
);

/**
 * 可复算角色 = 权限码能从真实输入源推出的那批基线角色
 * （后端授权矩阵在册 ∪ 超管通配 ∪ 本支补授集点名）。
 * 其余基线角色属前端镜像岗位与边界岗位：它们的权限码不来自后端矩阵，
 * 后端授权面改动对它们无意义，由"它们今天进不来 /ar"那条断言单独兜住。
 */
const RECOMPUTABLE_ROLES = BASELINE_ROLES.filter(
  role =>
    Boolean(MATRIX_GROUPS[role]) ||
    Boolean(WILDCARD_ROLES[role]) ||
    Object.prototype.hasOwnProperty.call(AR_FAMILY_GRANTS_THIS_BATCH, role)
);

/** 某角色在"矩阵现值 ∪ 通配"下的权限码（补授后世界，即本支合入后的守卫输入） */
function permissionsOf(role: string): string[] {
  return [...new Set([...(MATRIX_GROUPS[role] ?? []), ...(WILDCARD_ROLES[role] ?? [])])];
}

/** 某角色在"矩阵现值减去本支补授 ∪ 通配"下的权限码（补授前世界，即基线采集时的守卫输入） */
function permissionsBeforeBatch(role: string): string[] {
  const granted = new Set(AR_FAMILY_GRANTS_THIS_BATCH[role] ?? []);
  return [...new Set(permissionsOf(role).filter(code => !granted.has(code)))];
}

/** 旧口径下真正让该角色进得来 /ar 的解释键（粗域展开后与 read 等价成立的项） */
function explainingKeysOfOldGate(role: string): string[] {
  const holders: string[] = [];
  for (const code of permissionsBeforeBatch(role)) {
    const idx = code.indexOf(':');
    const resource = idx > 0 ? code.slice(0, idx) : code;
    const action = idx > 0 ? code.slice(idx + 1) : '';
    if (!FINANCE_COARSE_RESOURCES.includes(resource)) continue;
    if (action === '*' || action === 'read' || action === 'view') holders.push(code);
  }
  return holders.sort();
}

/** 应收六个页签的入口判据（取 constants 单一真相源，不在本文件重述字面量） */
const AR_TAB_GATE: Record<string, readonly string[]> = {
  invoice: [PERMISSIONS.AR_INVOICE_READ],
  payment: [PERMISSIONS.AR_PAYMENT_READ],
  verification: [PERMISSIONS.AR_VERIFICATION_READ],
  reconciliation: [PERMISSIONS.AR_RECONCILIATION_READ],
  fund: [PERMISSIONS.FUND_MANAGEMENT_READ],
  report: AR_REPORT_TAB_PERMISSIONS,
};

function visibleTabs(role: string): string[] {
  return Object.entries(AR_TAB_GATE)
    .filter(([, keys]) => hasRoutePermission([...keys], permissionsOf(role)))
    .map(([name]) => name)
    .sort();
}

type DriftRow = { role: string; before: boolean; after: boolean; explain: string };

function buildDriftTable(): DriftRow[] {
  return RECOMPUTABLE_ROLES.map(role => {
    const before = hasRoutePermission(OLD_SHELL_GATE, permissionsBeforeBatch(role));
    const after = hasRoutePermission(NEW_SHELL_GATE, permissionsOf(role));
    const heldNew = permissionsOf(role).filter(code => NEW_SHELL_GATE.includes(code));
    const explain = after
      ? heldNew.length > 0
        ? `保留：持应收族或资金账户入口键 ${heldNew.join(', ')}`
        : '保留：*:* 通配（超管）'
      : before
        ? `失去：旧口径靠 ${explainingKeysOfOldGate(role).join(', ')} 被财务域并集放行，本岗不持任何应收族键与资金账户键`
        : '本就被拒，无变化';
    return { role, before, after, explain };
  });
}

describe('复算输入自洽：补授集、门控与矩阵三者必须先成立', () => {
  it('本支补授集每一枚键的资源段都在后端注册表在册（不许拿注册表外的键做复算输入）', () => {
    for (const [role, grants] of Object.entries(AR_FAMILY_GRANTS_THIS_BATCH)) {
      const offenders = grants.filter(
        code => !BACKEND_RESOURCES.includes(code.slice(0, code.indexOf(':')))
      );
      expect(offenders, `${role} 的补授集含注册表外键`).toEqual([]);
    }
  });

  it('补授集枚数与本支口径一致（该集合一旦改动，本用例先红，逼人工重看漂移表）', () => {
    expect(AR_FAMILY_GRANTS_THIS_BATCH.accountant).toHaveLength(10);
    expect(AR_FAMILY_GRANTS_THIS_BATCH.finance_manager).toHaveLength(11);
    expect(AR_FAMILY_GRANTS_THIS_BATCH.cashier).toEqual([]);
  });

  it('补授集已逐枚落进通道① 矩阵（矩阵缺任一枚即通道① 与通道② 分叉，新老库授权面会不一致）', () => {
    for (const [role, grants] of Object.entries(AR_FAMILY_GRANTS_THIS_BATCH)) {
      const missing = grants.filter(code => !permissionsOf(role).includes(code));
      expect(missing, `通道① 矩阵的 ${role} 分组缺这些本支补授键`).toEqual([]);
    }
  });

  it('新门控九枚全部在册、没有一枚落在粗域展开表上，且等于 constants 的壳页入口判据', () => {
    expect(NEW_SHELL_GATE).toHaveLength(9);
    expect(
      NEW_SHELL_GATE.filter(code => !BACKEND_RESOURCES.includes(code.slice(0, code.indexOf(':'))))
    ).toEqual([]);
    expect(
      NEW_SHELL_GATE.filter(code => COARSE_DOMAIN_KEYS.includes(code.slice(0, code.indexOf(':'))))
    ).toEqual([]);
    expect([...NEW_SHELL_GATE].sort(), 'router 门控与 constants 的并集分叉').toEqual(
      [...AR_SHELL_ENTRY_PERMISSIONS].sort()
    );
  });

  it('六个页签入口判据的并集等于壳页门控（进得来必然看得见一个页签的前提）', () => {
    expect([...new Set(Object.values(AR_TAB_GATE).flat())].sort()).toEqual(
      [...AR_SHELL_ENTRY_PERMISSIONS].sort()
    );
  });
});

describe('复算与已重采基线对账（新口径复算等于基线现值，旧口径差异集恰为收权岗）', () => {
  it('可复算角色逐角色一致：新口径复算 = 基线是否含 /ar', () => {
    const mismatches: string[] = [];
    for (const role of RECOMPUTABLE_ROLES) {
      const derivedNew = hasRoutePermission(NEW_SHELL_GATE, permissionsOf(role));
      const observed = BASELINE_REACHES_AR.get(role);
      if (observed !== derivedNew) mismatches.push(`${role} 基线=${observed} 复算=${derivedNew}`);
    }
    expect(
      mismatches,
      `复算模型与已重采基线分叉（解析口径或权限模型已漂）：\n${mismatches.join('\n')}`
    ).toEqual([]);
    expect(RECOMPUTABLE_ROLES.length).toBeGreaterThanOrEqual(20);
  });

  // 旧口径仍是口径自证的一环：它与重采后基线的差异只允许出现在本次点名的五个收权岗上，
  // 多一个岗位或少一个岗位都说明复算模型与真实守卫已经分叉，而不是基线采错。
  it('旧口径与重采基线的差异集恰为五个收权岗，其余岗位未被牵动', () => {
    const diverged: string[] = [];
    for (const role of RECOMPUTABLE_ROLES) {
      const derivedOld = hasRoutePermission(OLD_SHELL_GATE, permissionsBeforeBatch(role));
      if (derivedOld !== BASELINE_REACHES_AR.get(role)) diverged.push(role);
    }
    expect(diverged.sort(), `差异集漂了：${diverged.join(', ')}`).toEqual([
      'auditor',
      'cost_accountant',
      'purchase_manager',
      'qc_manager',
      'sales_manager',
    ]);
  });

  it('不可复算角色（前端镜像岗位与边界岗位）今天都进不来 /ar ⇒ 本次改动不会牵动它们的基线', () => {
    const others = BASELINE_ROLES.filter(role => !RECOMPUTABLE_ROLES.includes(role));
    const reaching = others.filter(role => BASELINE_REACHES_AR.get(role) === true);
    expect(reaching, `这些非后端矩阵岗位的基线竟含 /ar：${reaching.join(', ')}`).toEqual([]);
    expect(others.length).toBeGreaterThan(0);
  });
});

describe('收细键后的 /ar 可达集漂移表', () => {
  it('恰好五份基线失去 /ar，四份保留（含超管），无岗位新增', () => {
    const table = buildDriftTable();
    // 漂移表进 CI 日志：基线重采按这张表逐份核对，不再另抄清单
    console.table(table);
    const losers = table
      .filter(r => r.before && !r.after)
      .map(r => r.role)
      .sort();
    const keepers = table
      .filter(r => r.before && r.after)
      .map(r => r.role)
      .sort();
    const gainers = table
      .filter(r => !r.before && r.after)
      .map(r => r.role)
      .sort();
    expect(losers).toEqual([
      'auditor',
      'cost_accountant',
      'purchase_manager',
      'qc_manager',
      'sales_manager',
    ]);
    expect(keepers).toEqual(['accountant', 'admin', 'cashier', 'finance_manager']);
    expect(gainers).toEqual([]);
  });

  it('四个收权岗失去 /ar 的解释键不含任何应收族键（收口的是跨域并集放行，不是本岗读权）', () => {
    for (const role of [
      'auditor',
      'cost_accountant',
      'purchase_manager',
      'qc_manager',
      'sales_manager',
    ]) {
      const explain = explainingKeysOfOldGate(role);
      expect(
        explain.length,
        `${role} 在旧口径下没有放行键，与它的基线曾含 /ar 矛盾`
      ).toBeGreaterThan(0);
      expect(
        explain.filter(k => k.startsWith('ar-')),
        `${role} 的旧口径放行键里出现了应收族键`
      ).toEqual([]);
      expect(
        hasRoutePermission(NEW_SHELL_GATE, permissionsOf(role)),
        `${role} 新口径仍可达 /ar`
      ).toBe(false);
    }
  });

  it('会计与财务经理的保留由应收族读面键解释（不再依赖粗域并集）', () => {
    for (const role of ['accountant', 'finance_manager']) {
      const held = permissionsOf(role).filter(code => NEW_SHELL_GATE.includes(code));
      expect(held.length, `${role} 未持任何应收族或资金账户入口键`).toBeGreaterThan(0);
      expect(
        held.every(k => k.startsWith('ar-')),
        `${role} 的保留键不全是应收族细键`
      ).toBe(true);
    }
  });

  it('出纳的保留只由资金账户读面键解释，且它不持任何应收族键', () => {
    const held = permissionsOf('cashier').filter(code => NEW_SHELL_GATE.includes(code));
    expect(held, '出纳的 /ar 可达只应由 fund-management:read 解释').toEqual([
      'fund-management:read',
    ]);
    expect(
      permissionsOf('cashier').filter(code => code.startsWith('ar-')),
      '出纳竟持有应收族键，授予面外泄'
    ).toEqual([]);
  });

  it('页签级：会计四个页签、财务经理六个全可见，出纳只见资金账户，收权岗零可见', () => {
    expect(visibleTabs('accountant')).toEqual(
      ['invoice', 'payment', 'reconciliation', 'verification'].sort()
    );
    expect(visibleTabs('finance_manager')).toEqual(Object.keys(AR_TAB_GATE).sort());
    expect(visibleTabs('cashier')).toEqual(['fund']);
    for (const role of [
      'auditor',
      'cost_accountant',
      'purchase_manager',
      'qc_manager',
      'sales_manager',
    ]) {
      expect(visibleTabs(role), `${role} 不该看见任何应收页签`).toEqual([]);
    }
  });

  it('壳页可达必然至少一个页签可见（防"进得来但整页空白"的门控分叉）', () => {
    for (const role of RECOMPUTABLE_ROLES) {
      if (hasRoutePermission(NEW_SHELL_GATE, permissionsOf(role))) {
        expect(visibleTabs(role).length, `${role} 进得来 /ar 却零页签可见`).toBeGreaterThan(0);
      }
    }
  });
});
