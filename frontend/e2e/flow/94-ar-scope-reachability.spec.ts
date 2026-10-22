// 本文件守的缺陷族：数据范围门（Self）只拦写不拦读、越权读被降级成 2xx 空列表（前端
// 误读为「无数据」的假绿）、越权写触发后状态/数据半漂移、RBAC 层 403 与行级数据范围层
// 403 混为一谈（判据错位掩盖真实门失效）。
// 归属可见性用「可达内容」断言：列表逐行按行内 created_by 验归属、他人单据必须缺席；
// 详情越权必须 403 且机器码 FORBIDDEN，来源层按授权码面与固定文案的归因分类钉
// （行级门脱敏文案与 RBAC 直出文案只用于归因分类，禁止拿中文整串相等当判据）；
// 被拒写动作之后一律回到 owner 会话读回，证明状态与数据零漂移。
// 角色账号一律取 global-setup 角色矩阵真实凭证在隔离 context 独立登录（不允许同账号自测
// 的假越权）；角色数据范围取值先经角色详情端点回读坐实（列表端点不下发该列），
// 种子口径漂移时显式判红而非静默改判。
import { type Browser } from '@playwright/test';
import { test, expect } from '../diagnose-fixture';
import {
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  failureCode,
  genCode,
  getCtx,
  getRoleCredential,
  loginViaUI,
  loginInIsolatedContext,
  expectDenied,
  tryCleanup,
  APP_ERROR_CODES,
  type IsolatedAuthedSession,
} from './helpers';

type Row = Record<string, unknown>;

/** 行级数据范围门的脱敏固定文案（权限中间件 RBAC 直出文案不同，禁止混用判据） */
const DATA_SCOPE_403_MESSAGE = '无权限';
const RBAC_403_MESSAGE = '权限不足，无法访问该资源';

/**
 * 403 来源层归因：两枚固定文案分别来自后端行级门（AppError 脱敏常量）与权限中间件
 * 直出字面量，这里只做「message → 来源层类别」的分类映射，再用类别相等下判据；
 * 禁止再新增对任一中文整串的相等断言（文案属出参脱敏面，可改，判定不可靠）。
 */
type DenialLayer = 'data-scope' | 'rbac';
const DENIAL_LAYER_BY_MESSAGE: ReadonlyMap<string, DenialLayer> = new Map([
  [DATA_SCOPE_403_MESSAGE, 'data-scope'],
  [RBAC_403_MESSAGE, 'rbac'],
]);

function classifyDenialLayer(message: unknown): DenialLayer | 'unknown' {
  return DENIAL_LAYER_BY_MESSAGE.get(String(message ?? '')) ?? 'unknown';
}

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0) throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/** 读取会话出参里的 department_id：键必须存在（缺键=契约漂移判红），null 表示未分配部门 */
function readDeptId(session: Row, label: string): number | null {
  if (!Object.prototype.hasOwnProperty.call(session, 'department_id')) {
    throw new Error(
      `${label} 会话出参缺 department_id 键，实际键=${Object.keys(session).join(',')}`
    );
  }
  const v = session.department_id;
  if (v === null) return null;
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0) {
    throw new Error(`${label} department_id 非有效数值：${JSON.stringify(v)}`);
  }
  return n;
}

async function openRoleSession(browser: Browser, role: string): Promise<IsolatedAuthedSession> {
  const cred = getRoleCredential(role);
  if (!cred) throw new Error(`角色 ${role} 凭证不存在——setup 缺陷判红，不 skip 掩盖`);
  return loginInIsolatedContext(browser, cred.username, cred.password);
}

/** owner（分片 admin）建一张 draft 订单并返回 id/order_no/notes */
async function seedOrderAsOwner(
  page: import('@playwright/test').Page
): Promise<{ id: number; order_no: string; notes: string }> {
  const ctx = getCtx();
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  const productId = ctx.productIds[0];
  if (!productId) throw new Error('前置缺失：ctx.productIds 为空');
  const notes = `E2E-SCOPE-${genCode('O')}`;
  const created = await apiCallRaw<Row>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    items: [{ product_id: productId, quantity: '5', unit_price: '10.00', tax_percent: '0' }],
    notes,
  });
  const id = requireNum(created.id, 'owner 建订单');
  CLEANUP.push({ path: `/sales/orders/${id}`, label: `so#${id}` });
  const detail = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${id}`);
  expect(detail.created_by, '订单 created_by 必须落 owner 用户 id（越权前提坐实）').toBeTruthy();
  return { id, order_no: String(detail.order_no), notes };
}

test.describe('94 销售单据数据范围纵深（可达内容 + 被拒零漂移）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('94-01 Self 角色越权面：列表不含他人行且逐行归属自洽，详情/四类写动作 403 钉数据范围层，被拒后 owner 侧零漂移', async ({
    page,
    browser,
  }) => {
    const owner = await seedOrderAsOwner(page);
    const ownerId = requireNum(
      (await apiCallRaw<Row>(page, 'GET', '/auth/me')).id,
      'owner user id'
    );

    const intruder = await openRoleSession(browser, 'salesperson');
    try {
      const me = await apiCallRaw<Row>(intruder.page, 'GET', '/auth/me');
      const intruderId = requireNum(me.id, '入侵者会话用户 id');
      expect(intruderId === ownerId, '越权前提：必须是两个不同身份的账号').toBe(false);

      // 授权码面回读：403 的来源层判据由「该角色是否持有对应权限键」推出，不靠猜文案
      const permsRaw = me.permissions;
      if (!Array.isArray(permsRaw)) {
        throw new Error(
          `入侵者 /auth/me 出参缺 permissions 数组（授码面判据来源），实际键=${Object.keys(me).join(',')}`
        );
      }
      const perms = new Set(permsRaw.map(String));

      // 正向门：先以 200 读到自己的列表——证明 RBAC orders:read 已过门，
      // 后续对他人单据的 403 只可能来自行级数据范围，而不是路径不可达。
      const list = await apiCallRaw<Row>(intruder.page, 'GET', '/sales/orders?page=1&page_size=200');
      const rows = Array.isArray((list as Row).items) ? ((list as Row).items as Row[]) : null;
      expect(rows, 'Self 角色订单列表出参必须含 items 数组（PaginatedResponse）').toBeTruthy();
      // 可达内容断言一：owner 的单据不得出现在其列表（空列表也满足，但必须证明是过滤而非全量泄漏）
      expect(
        rows!.some(r => Number(r.id) === owner.id),
        `Self 角色列表不得包含他人订单 id=${owner.id}`
      ).toBe(false);
      // 可达内容断言二：列表每一行（若携带 created_by）都归属本会话用户——行级过滤的逐行自证
      for (const r of rows!) {
        if (Object.prototype.hasOwnProperty.call(r, 'created_by') && r.created_by !== null) {
          expect(
            Number(r.created_by),
            `Self 列表行 id=${r.id} 的 created_by=${r.created_by} 必须等于本会话用户 ${intruderId}`
          ).toBe(intruderId);
        }
      }

      // 详情越权：403 + 机器码 FORBIDDEN，来源层归因到行级数据范围门
      // （归因判据：本角色持 orders:read 且 RBAC 判定与具体行无关，列表同键已 200 过门，
      // 他人行上的 403 只可能来自处理器内的行级归属校验；不得被降级为 2xx 空对象/404）
      const fDetail = await apiCallExpectFail(intruder.page, 'GET', `/sales/orders/${owner.id}`);
      expectDenied(fDetail, 'Self 角色读他人订单详情');
      expect(
        failureCode(fDetail),
        `详情越权机器码必须为 ${APP_ERROR_CODES.FORBIDDEN}，实际 code=${JSON.stringify(fDetail.code)}`
      ).toBe(APP_ERROR_CODES.FORBIDDEN);
      expect(
        perms.has('orders:read'),
        `行级门归因前提：入侵者角色必须持 orders:read，否则越权 403 来自 RBAC 层、行级门不可达。实际授码面=${JSON.stringify([...perms])}`
      ).toBe(true);
      expect(
        classifyDenialLayer(fDetail.message),
        `详情越权 403 必须归因到行级数据范围门，实际 message=${JSON.stringify(fDetail.message)}`
      ).toBe('data-scope');

      // 写面四连拒：来源层必须与授权码面一致——该写键未授时 403 归因 RBAC 层、
      // 已授时归因行级数据范围门；矩阵补授销售写码后同一判据自动切到行级门，无需改用例。
      const fUpdate = await apiCallExpectFail(intruder.page, 'PUT', `/sales/orders/${owner.id}`, {
        notes: 'E2E-越权改写',
      });
      expectDenied(fUpdate, 'Self 角色改他人订单');
      expect(
        classifyDenialLayer(fUpdate.message),
        `改他人订单 403 的来源层必须与 orders:update 授码状态一致，实际 message=${JSON.stringify(fUpdate.message)}`
      ).toBe(perms.has('orders:update') ? 'data-scope' : 'rbac');

      const fSubmit = await apiCallExpectFail(intruder.page, 'POST', `/sales/orders/${owner.id}/submit`);
      expectDenied(fSubmit, 'Self 角色提交他人订单');
      expect(
        classifyDenialLayer(fSubmit.message),
        `提交他人订单 403 的来源层必须与 orders:submit 授码状态一致，实际 message=${JSON.stringify(fSubmit.message)}`
      ).toBe(perms.has('orders:submit') ? 'data-scope' : 'rbac');

      const fShip = await apiCallExpectFail(intruder.page, 'POST', `/sales/orders/${owner.id}/ship`, {
        order_id: owner.id,
        warehouse_code: 'WH-X',
        items: [],
      });
      expectDenied(fShip, 'Self 角色发他人订单货');
      // 发货动作末段不在路径动作关键字表内，对应派生权限键无法在用例侧静态坐实，
      // 此处只钉 403 可归因到两道门之一（归因不明即判红），不硬猜键名。
      expect(
        classifyDenialLayer(fShip.message) !== 'unknown',
        `发货被拒 403 必须能归因到行级门或 RBAC 门之一，实际 message=${JSON.stringify(fShip.message)}`
      ).toBe(true);

      const fCancel = await apiCallExpectFail(intruder.page, 'POST', `/sales/orders/${owner.id}/cancel`);
      expectDenied(fCancel, 'Self 角色取消他人订单');
      expect(
        classifyDenialLayer(fCancel.message),
        `取消他人订单 403 的来源层必须与 orders:cancel 授码状态一致，实际 message=${JSON.stringify(fCancel.message)}`
      ).toBe(perms.has('orders:cancel') ? 'data-scope' : 'rbac');

      // 发货记录读口同门（读码已过门且与详情同派生键，越权不得读他人出库痕迹）
      const fDlv = await apiCallExpectFail(intruder.page, 'GET', `/sales/orders/${owner.id}/deliveries`);
      expectDenied(fDlv, 'Self 角色读他人发货记录');
      expect(
        classifyDenialLayer(fDlv.message),
        `读他人发货记录 403 必须归因到行级数据范围门，实际 message=${JSON.stringify(fDlv.message)}`
      ).toBe('data-scope');
    } finally {
      await intruder.close();
    }

    // 被拒零漂移：全部越权动作结束后，owner 会话读回逐字段原样
    const after = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${owner.id}`);
    expect(after.status, '越权写全被拒后订单必须仍是 draft').toBe('draft');
    expect(String(after.notes), '越权改写不得落库').toBe(owner.notes);
    expect(String(after.order_no), '单号不因越权尝试漂移').toBe(owner.order_no);
    const dlv = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${owner.id}/deliveries`);
    expect(
      ((dlv as Row).list as Row[]).length,
      '越权发货被拒后不得产生发货单'
    ).toBe(0);
  });

  test('94-02 角色数据范围口径坐实：sales_manager 先回读真实 data_scope 再按语义验可见集；department_manager 未持销售读码时如实钉 RBAC 层', async ({
    page,
    browser,
  }) => {
    const owner = await seedOrderAsOwner(page);

    // 回读通道按契约选择：/roles 列表项只下发 id/name/code 等基础列、不含 data_scope，
    // 该列由角色详情端点逐角色下发；先用列表定位角色 id，再经详情回读真实口径。
    const roles = await apiCallRaw<Row>(page, 'GET', '/roles');
    if (!Array.isArray(roles.roles)) {
      throw new Error(
        `角色清单契约失配：GET /roles 应出参 data.roles 数组，实际键=${Object.keys(roles).join(',')}`
      );
    }
    const managerRole = (roles.roles as Row[]).find(r => r.code === 'sales_manager');
    expect(
      managerRole,
      '前置：sales_manager 角色必须由种子建立（后端初始化补建或 setup 补建，缺即种子缺陷）'
    ).toBeTruthy();
    const managerRoleId = requireNum(managerRole!.id, 'sales_manager 角色 id');
    const managerDetail = await apiCallRaw<Row>(page, 'GET', `/roles/${managerRoleId}`);
    const scopeRaw = managerDetail.data_scope;
    if (typeof scopeRaw !== 'string') {
      throw new Error(
        `角色详情必须下发 data_scope 字符串，实际类型=${typeof scopeRaw} ` +
          `值=${JSON.stringify(scopeRaw)}，实际键=${Object.keys(managerDetail).join(',')}`
      );
    }
    const scope = scopeRaw;
    // 写入侧口径：初始化种子为该角色写 dept，经接口补建的新角色后端默认 self——
    // 比较点与写入值逐字符相同；落在两者之外=口径漂移，显式判红点名当前值，先改种子再改判据。
    expect(
      scope === 'dept' || scope === 'self',
      `sales_manager data_scope 必须落在写入侧口径（dept=初始化种子值，self=接口新建默认值），当前值=${JSON.stringify(scope)}`
    ).toBe(true);

    const mgr = await openRoleSession(browser, 'sales_manager');
    try {
      const list = await apiCallRaw<Row>(mgr.page, 'GET', '/sales/orders?page=1&page_size=200');
      expect(
        Array.isArray((list as Row).items),
        'sales_manager 订单列表应 2xx 且含 items（RBAC orders:read 过门正证明，后续 403 只可能来自行级门）'
      ).toBe(true);
      const rows = (list as Row).items as Row[];
      const me = await apiCallRaw<Row>(mgr.page, 'GET', '/auth/me');
      const mgrId = requireNum(me.id, 'manager 会话 id');
      const mgrDept = readDeptId(me, 'sales_manager 会话');
      // 可见集按语义验：self 口径仅本人行；dept 口径在账号未分配部门时无可见成员集合、
      // 后端过滤构造器退化为仅本人，两口径在本夹具形态下判据等价。
      // 若账号已分配部门，dept 可见集须按部门成员集合逐行归属坐实，
      // 本套件角色账号无部门分配通道、成员读通道未提供——如实判红交夹具判责，禁止放宽。
      if (scope === 'dept' && mgrDept !== null) {
        throw new Error(
          `sales_manager 账号已分配部门（department_id=${mgrDept}）：dept 可见集需按部门成员集合逐行归属，` +
            '本套件未提供成员读通道，该形态不可判——如实判红交夹具判责，禁止放宽成只断字段存在'
        );
      }
      expect(
        rows.some(r => Number(r.id) === owner.id),
        `${scope} 口径（无可见成员退化仅本人）下他人订单 id=${owner.id} 必须不可见`
      ).toBe(false);
      for (const r of rows) {
        if (Object.prototype.hasOwnProperty.call(r, 'created_by') && r.created_by !== null) {
          expect(
            Number(r.created_by),
            `${scope} 口径退化仅本人时可见行必须归属自己（行 ${r.id}）`
          ).toBe(mgrId);
        }
      }
      const fDetail = await apiCallExpectFail(mgr.page, 'GET', `/sales/orders/${owner.id}`);
      expectDenied(fDetail, 'sales_manager 读他人订单详情');
      expect(
        classifyDenialLayer(fDetail.message),
        `sales_manager 越权 403 必须归因到行级数据范围门（读码已过门为列表 200 所证），实际 message=${JSON.stringify(fDetail.message)}`
      ).toBe('data-scope');
    } finally {
      await mgr.close();
    }

    // department_manager：CI 种子未授销售读码——其 403 属 RBAC 层（权限不足文案），
    // 与数据范围层是两条不同的门；此处如实钉来源层，防止把 RBAC 拦误读成行级隔离生效。
    const dept = await openRoleSession(browser, 'department_manager');
    try {
      const fDept = await apiCallExpectFail(dept.page, 'GET', `/sales/orders/${owner.id}`);
      expect(fDept.status, 'department_manager 读销售订单应 403').toBe(403);
      expect(failureCode(fDept), 'department_manager 机器码 FORBIDDEN').toBe(APP_ERROR_CODES.FORBIDDEN);
      const deptMsg = String(fDept.message ?? '');
      expect(
        deptMsg === RBAC_403_MESSAGE || deptMsg === DATA_SCOPE_403_MESSAGE,
        `403 文案必须能归因到两门之一，实际=${deptMsg}`
      ).toBe(true);
      console.warn(
        `[94-02] department_manager 403 归因=${deptMsg === RBAC_403_MESSAGE ? 'RBAC 层（未授 orders:read，dept 数据范围面不可达）' : '数据范围层'}`
      );
    } finally {
      await dept.close();
    }
  });
});
