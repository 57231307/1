/**
 * 应付壳页 /ap 门控收细键的可达集复算（纯函数，不连浏览器、不连后端）。
 *
 * 用途：这条改动改变"角色可达集"，角色权限矩阵基线轨据此判漂移，漂移由人工重采签署。
 * 本用例把"谁失去 /ap、谁保留、由哪一枚键解释"从手抄数字变成可复算判据，并与**已重采的基线**
 * 双向对账：按新口径（九枚应付细键）复算出的可达性必须逐角色等于基线现值；按旧口径（粗域键
 * finance:read）复算时，与基线的差异集必须恰好等于本次点名的六个收权岗。任一方向不成立，都说明
 * 本用例的解析口径或权限模型已与真实守卫分叉，先停下交人工而不是改期望值。
 *
 * 输入（三件事，来源各不相同）：
 *   1. 角色现有权限码：解析后端 init_service_ops 的角色授权矩阵源文本；
 *   2. 前置变更的授予集 AP_FAMILY_GRANTS_FROM_PR_1008：这是**前提声明**而非仓库事实——
 *      前置那支（应付族授权面）未合入 main 之前，本用例算的是它合入后的矩阵状态，这也是本支的合并前置；
 *      它合入后矩阵文本自带这些键，与本表取并集仍然等价（集合并集幂等），用例不会因此变红；
 *   3. 新旧门控：旧 = 粗域键 finance:read，新 = 从 router 源文本解析的 /ap meta 现值。
 *
 * 判定用 e2e 权限模型的 hasRoutePermission（router 守卫的同源镜像：通配、read↔view 等价、
 * 粗域键展开），不在本文件里另写第二套比较。
 *
 * 调用方：vitest（tests/unit，CI 前端单测 job）。
 * 入参：只读仓库内源码文本与基线 JSON，不执行后端代码、不发网络请求、不写任何文件。
 * 传给谁：断言结果进 vitest 报告，漂移表同时打进标准输出供基线重采逐份核对；不落盘。
 */
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import {
  AP_REPORT_TAB_PERMISSIONS,
  AP_SHELL_ENTRY_PERMISSIONS,
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
/** 后端角色授权矩阵现值（角色码 → 资源:动作 列表） */
const MATRIX_GROUPS = parseBackendRoleGrantGroups(readRepoSource(BACKEND_MATRIX_RS));
/** 粗域键 finance 展开后的细资源段集（旧口径"持该域任一细资源读权即放行整域壳页"的判据来源） */
const FINANCE_COARSE_RESOURCES = parseCoarseDomainResources(ROUTER_SRC).finance;
const COARSE_DOMAIN_KEYS = Object.keys(parseCoarseDomainResources(ROUTER_SRC));

/** /ap 收细键后的新门控（守卫实际消费的同一份值，由 e2e 解析器从 router 源文本取） */
const NEW_SHELL_GATE: string[] = (() => {
  const gate = parseRouterPermissions(ROUTER_SRC)['/ap'];
  if (!Array.isArray(gate) || gate.length === 0) {
    throw new Error(
      `/ap 的 meta.permission 不是非空数组（实际 ${JSON.stringify(gate)}）——收细键未落地，复算无输入`
    );
  }
  return gate as string[];
})();

/** 改动前的门控（粗域键）：只在复算里出现一次，作为"旧口径"的显式输入 */
const OLD_SHELL_GATE = 'finance:read';

/**
 * 前置那支的应付族授予集（前置声明，见文件头"输入 2"）。
 * 口径为该 PR 合入 main 后的最终矩阵：会计拿制单与记账面十四枚（含导出面），
 * 财务经理拿审核与报表面十一枚，出纳不持任何应付族键（显式空数组，
 * 而不是"查不到就当作 0 枚"）。
 * 导出面不是壳页或页签的入口判据，因此它只影响 InvoiceTab 的两枚按钮可见性，
 * 不改变本表的 /ap 存废结论。
 */
const AP_FAMILY_GRANTS_FROM_PR_1008: Record<string, readonly string[]> = {
  accountant: [
    'ap-invoices:read',
    'ap-invoices:create',
    'ap-invoices:update',
    'ap-invoices:export',
    'ap-payments:read',
    'ap-verifications:read',
    'ap-verifications:create',
    'ap-verifications:cancel',
    'ap-payment-requests:read',
    'ap-payment-requests:create',
    'ap-payment-requests:update',
    'ap-payment-requests:submit',
    'ap-reconciliations:read',
    'ap-reconciliations:create',
  ],
  finance_manager: [
    'ap-invoices:read',
    'ap-invoices:approve',
    'ap-payments:read',
    'ap-verifications:read',
    'ap-payment-requests:read',
    'ap-reconciliations:read',
    'ap-reconciliations:confirm',
    'ap-aging:read',
    'ap-daily:read',
    'ap-monthly:read',
    'ap-statistics:read',
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

const BASELINE_REACHES_AP = new Map<string, boolean>(
  BASELINE_ROLES.map(role => [role, baselineReachable(role).includes('/ap')])
);

/**
 * 可复算角色 = 权限码能从三条真实输入源推出的那批基线角色
 * （后端授权矩阵在册 ∪ 超管通配 ∪ 应付族授予集点名）。
 * 其余基线角色属前端镜像岗位与边界岗位：它们的权限码不来自后端矩阵，
 * 后端授权面改动对它们无意义，由"它们今天进不来 /ap"那条断言单独兜住。
 */
const RECOMPUTABLE_ROLES = BASELINE_ROLES.filter(
  role =>
    Boolean(MATRIX_GROUPS[role]) ||
    Boolean(WILDCARD_ROLES[role]) ||
    Object.prototype.hasOwnProperty.call(AP_FAMILY_GRANTS_FROM_PR_1008, role)
);

/** 某角色在"矩阵现值 ∪ 前置授予集 ∪ 通配"下的权限码 */
function permissionsOf(role: string): string[] {
  return [
    ...new Set([
      ...(MATRIX_GROUPS[role] ?? []),
      ...(AP_FAMILY_GRANTS_FROM_PR_1008[role] ?? []),
      ...(WILDCARD_ROLES[role] ?? []),
    ]),
  ];
}

/** 旧口径下真正让该角色进得来 /ap 的解释键（粗域展开后与 read 等价成立的项） */
function explainingKeysOfOldGate(role: string): string[] {
  const holders: string[] = [];
  for (const code of permissionsOf(role)) {
    const idx = code.indexOf(':');
    const resource = idx > 0 ? code.slice(0, idx) : code;
    const action = idx > 0 ? code.slice(idx + 1) : '';
    if (!FINANCE_COARSE_RESOURCES.includes(resource)) continue;
    if (action === '*' || action === 'read' || action === 'view') holders.push(code);
  }
  return holders.sort();
}

type DriftRow = { role: string; before: boolean; after: boolean; explain: string };

function buildDriftTable(): DriftRow[] {
  return RECOMPUTABLE_ROLES.map(role => {
    const before = hasRoutePermission(OLD_SHELL_GATE, permissionsOf(role));
    const after = hasRoutePermission(NEW_SHELL_GATE, permissionsOf(role));
    const heldNew = permissionsOf(role).filter(code => NEW_SHELL_GATE.includes(code));
    const explain = after
      ? heldNew.length > 0
        ? `保留：持应付族入口键 ${heldNew.join(', ')}`
        : '保留：*:* 通配（超管）'
      : before
        ? `失去：旧口径靠 ${explainingKeysOfOldGate(role).join(', ')} 被财务域并集放行，本岗不持任何应付族键`
        : '本就被拒，无变化';
    return { role, before, after, explain };
  });
}

describe('复算输入自洽：前置授予集与门控本身必须成立', () => {
  it('应付族授予集每一枚键的资源段都在后端注册表在册（不许拿注册表外的键做复算输入）', () => {
    for (const [role, grants] of Object.entries(AP_FAMILY_GRANTS_FROM_PR_1008)) {
      const offenders = grants.filter(
        code => !BACKEND_RESOURCES.includes(code.slice(0, code.indexOf(':')))
      );
      expect(offenders, `${role} 的授予集含注册表外键`).toEqual([]);
    }
  });

  it('授予集枚数与前置 PR 口径一致（该 PR 的集合一旦改动，本用例先红，逼人工重看漂移表）', () => {
    expect(AP_FAMILY_GRANTS_FROM_PR_1008.accountant).toHaveLength(14);
    expect(AP_FAMILY_GRANTS_FROM_PR_1008.finance_manager).toHaveLength(11);
    expect(AP_FAMILY_GRANTS_FROM_PR_1008.cashier).toEqual([]);
    expect(
      [...Object.values(AP_FAMILY_GRANTS_FROM_PR_1008).flat()].filter(
        k => k === 'ap-invoices:export'
      )
    ).toEqual(['ap-invoices:export']);
  });

  it('新门控九枚全部在册且没有一枚落在粗域展开表上', () => {
    expect(NEW_SHELL_GATE).toHaveLength(9);
    expect(
      NEW_SHELL_GATE.filter(code => !BACKEND_RESOURCES.includes(code.slice(0, code.indexOf(':'))))
    ).toEqual([]);
    expect(
      NEW_SHELL_GATE.filter(code => COARSE_DOMAIN_KEYS.includes(code.slice(0, code.indexOf(':'))))
    ).toEqual([]);
  });
});

describe('复算与已重采基线对账（新口径复算等于基线现值，旧口径差异集恰为收权岗）', () => {
  it('可复算角色逐角色一致：新口径复算 = 基线是否含 /ap', () => {
    const mismatches: string[] = [];
    for (const role of RECOMPUTABLE_ROLES) {
      const derivedNew = hasRoutePermission(NEW_SHELL_GATE, permissionsOf(role));
      const observed = BASELINE_REACHES_AP.get(role);
      if (observed !== derivedNew) mismatches.push(`${role} 基线=${observed} 复算=${derivedNew}`);
    }
    expect(
      mismatches,
      `复算模型与已重采基线分叉（解析口径或权限模型已漂）：\n${mismatches.join('\n')}`
    ).toEqual([]);
    expect(RECOMPUTABLE_ROLES.length).toBeGreaterThanOrEqual(20);
  });

  // 旧口径仍是口径自证的一环：它与重采后基线的差异只允许出现在本次点名的六个收权岗上，
  // 多一个岗位或少一个岗位都说明复算模型与真实守卫已经分叉，而不是基线采错。
  it('旧口径与重采基线的差异集恰为六个收权岗，其余岗位未被牵动', () => {
    const diverged: string[] = [];
    for (const role of RECOMPUTABLE_ROLES) {
      const derivedOld = hasRoutePermission(OLD_SHELL_GATE, permissionsOf(role));
      if (derivedOld !== BASELINE_REACHES_AP.get(role)) diverged.push(role);
    }
    expect(diverged.sort(), `差异集漂了：${diverged.join(', ')}`).toEqual([
      'auditor',
      'cashier',
      'cost_accountant',
      'purchase_manager',
      'qc_manager',
      'sales_manager',
    ]);
  });

  it('不可复算角色（前端镜像岗位与边界岗位）今天都进不来 /ap ⇒ 本次改动不会牵动它们的基线', () => {
    const others = BASELINE_ROLES.filter(role => !RECOMPUTABLE_ROLES.includes(role));
    const reaching = others.filter(role => BASELINE_REACHES_AP.get(role) === true);
    expect(reaching, `这些非后端矩阵岗位的基线竟含 /ap：${reaching.join(', ')}`).toEqual([]);
    expect(others.length).toBeGreaterThan(0);
  });
});

describe('收细键后的 /ap 可达集漂移表（前提：应付族授权面已合入）', () => {
  it('恰好六份基线失去 /ap，三份保留（含超管），无岗位新增', () => {
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
      'cashier',
      'cost_accountant',
      'purchase_manager',
      'qc_manager',
      'sales_manager',
    ]);
    expect(keepers).toEqual(['accountant', 'admin', 'finance_manager']);
    expect(gainers).toEqual([]);
  });

  it('出纳失去 /ap 的解释键不含任何应付族键（收口的是跨域并集放行，不是本岗 AP 读权）', () => {
    const explain = explainingKeysOfOldGate('cashier');
    expect(explain.length, '出纳在旧口径下没有放行键，与它的基线含 /ap 矛盾').toBeGreaterThan(0);
    expect(explain.filter(k => k.startsWith('ap-'))).toEqual([]);
    expect(hasRoutePermission(NEW_SHELL_GATE, permissionsOf('cashier'))).toBe(false);
  });

  it('会计与财务经理的保留由应付族读面键解释（不再依赖粗域并集）', () => {
    for (const role of ['accountant', 'finance_manager']) {
      const held = permissionsOf(role).filter(code => NEW_SHELL_GATE.includes(code));
      expect(held.length, `${role} 未持任何应付族入口键`).toBeGreaterThan(0);
      expect(
        held.every(k => k.startsWith('ap-')),
        `${role} 的保留键不全是应付族细键`
      ).toBe(true);
    }
  });

  it('页签级：会计看不到报表中心页签，财务经理六个页签全可见，出纳一个都看不到', () => {
    // 页签入口判据直接取 constants 单一真相源，不在本文件重述字面量
    const tabGate: Record<string, readonly string[]> = {
      invoice: [PERMISSIONS.AP_INVOICE_READ],
      payment: [PERMISSIONS.AP_PAYMENT_READ],
      paymentRequest: [PERMISSIONS.AP_PAYMENT_REQUEST_READ],
      verification: [PERMISSIONS.AP_VERIFICATION_READ],
      reconciliation: [PERMISSIONS.AP_RECONCILIATION_READ],
      report: AP_REPORT_TAB_PERMISSIONS,
    };
    // 与壳页门控同源核对：页签判据并集必须等于 AP_SHELL_ENTRY_PERMISSIONS
    const tabUnion = [...new Set(Object.values(tabGate).flat())].sort();
    expect(tabUnion).toEqual([...AP_SHELL_ENTRY_PERMISSIONS].sort());

    const visibleTabs = (role: string) =>
      Object.entries(tabGate)
        .filter(([, keys]) => hasRoutePermission([...keys], permissionsOf(role)))
        .map(([name]) => name)
        .sort();
    // 壳页可达 ⇒ 至少一个页签可见（该角色可进的页签数恒 > 0）
    for (const role of RECOMPUTABLE_ROLES) {
      if (hasRoutePermission(NEW_SHELL_GATE, permissionsOf(role))) {
        expect(visibleTabs(role).length, `${role} 进得来 /ap 却零页签可见`).toBeGreaterThan(0);
      }
    }
    expect(visibleTabs('accountant').sort()).toEqual(
      ['invoice', 'payment', 'paymentRequest', 'reconciliation', 'verification'].sort()
    );
    expect(visibleTabs('finance_manager').sort()).toEqual(Object.keys(tabGate).sort());
    expect(visibleTabs('cashier')).toEqual([]);
  });
});
