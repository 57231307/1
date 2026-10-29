// 供应商联系人/资质子资源 与 启停开关 的写改删契约 E2E — 建后回读逐字段真值、删除后库里消失、非法手机号被公开格式规则点名拒绝
//
// 本文件守的账：子资源写端点必须真落到库并按父资源归属可读回；更新只改提交的列、其余原样保留；
// 删除链路断言行确实不在列表里，不以「请求返回 200」当作已删除；父供应商启停开关同时落到
// is_enabled 布尔列与 status 文本列，两列必须同源不得各说各话。
// 负例锁真实拒绝：非法手机号经服务端格式校验（公开规则）被拦下回 VALIDATION_ERROR，并在拒绝后
// 按父供应商重新列表探针断零写入，绝不把未注册/被拒当健康。
// 数据自建自清：供应商用唯一名称自建，联系人/资质按父归属删除，供应商在 afterEach 逆序清理。
import { test, expect } from '../diagnose-fixture';
import { type Page } from '@playwright/test';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  APP_ERROR_CODES,
  genCode,
  tryCleanup,
} from './helpers';
import { pickListArray } from './ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

const isoDate = (offsetDays: number): string =>
  new Date(Date.now() + offsetDays * 86_400_000).toISOString().slice(0, 10);

function requireId(row: Record<string, unknown>, label: string): number {
  const id = Number(row.id);
  if (!Number.isInteger(id) || id <= 0) {
    throw new Error(`${label}：未取到有效 id，实际 ${JSON.stringify(row)}`);
  }
  return id;
}

/** 自建唯一命名供应商，返回 id 并登记 afterEach 清理 */
async function newSupplier(page: Page, tag: string): Promise<number> {
  const name = `151供应商${tag}${genCode('S')}`;
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/suppliers', {
    supplier_name: name,
    supplier_type: 'normal',
  });
  const id = requireId(created, `供应商 ${name}`);
  CLEANUP.push({ path: `/purchase/suppliers/${id}`, label: `supplier#${id}` });
  return id;
}

/** 在已取回的裸数组行里按 id 精确定位，取不到即判红不兜底 */
function rowById(rows: Array<Record<string, unknown>>, id: number, label: string): Record<string, unknown> {
  const hit = rows.find(r => Number(r.id) === id);
  if (!hit) {
    throw new Error(`${label}：列表未找到 id=${id}，实际 id 集=${rows.map(r => r.id).join(',') || '(空)'}`);
  }
  return hit;
}

test.describe('151 供应商子资源与启停写改删契约：逐字段回读 + 删除断消失 + 非法手机号拒绝', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('151-01 联系人建改删：建后回读逐字段，改后仅改提交列，删后库里消失，非法手机号回校验码且零写入', async ({
    page,
  }) => {
    const supplierId = await newSupplier(page, 'CT');

    // 负例（先做，证明非法值不落库）：手机号不合公开格式规则应被 400/VALIDATION_ERROR 拒绝
    const bad = await apiCallExpectFail(page, 'POST', `/purchase/suppliers/${supplierId}/contacts`, {
      contact_name: '非法手机联系人',
      mobile_phone: '12345',
      is_primary: false,
    });
    expect(bad.status, '非法手机号应被 HTTP 拒绝').toBe(400);
    expect(failureCode(bad), '非法手机号机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const afterBad = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/suppliers/${supplierId}/contacts`),
      'bare',
      '非法值后的联系人列表'
    );
    expect(afterBad.length, '非法手机号被拒后不得写入任何联系人行').toBe(0);

    // 正向建：返回体带 id，回读列表逐字段核对提交真值
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', `/purchase/suppliers/${supplierId}/contacts`, { contact_name: '张主联', department: '采购部', position: '经理', mobile_phone: '13800138000', email: 'zhang@example.com', is_primary: true });
    const contactId = requireId(created, '联系人创建');
    const createdRows = pickListArray<Record<string, unknown>>(await apiCallRaw<unknown>(page, 'GET', `/purchase/suppliers/${supplierId}/contacts`), 'bare', '建后联系人列表');
    const readBack = rowById(createdRows, contactId, '联系人建后列表');
    expect(String(readBack.contact_name), '联系人姓名回读').toBe('张主联');
    expect(String(readBack.mobile_phone), '联系人手机号回读').toBe('13800138000');
    expect(String(readBack.department), '联系人部门回读').toBe('采购部');
    expect(readBack.is_primary, '主要联系人标记回读').toBe(true);

    // 改：只提交 position 与手机号，未提交的 department 必须原样保留
    const updRes = await apiCallRaw<Record<string, unknown>>(page, 'PUT', `/purchase/suppliers/${supplierId}/contacts/${contactId}`, { position: '副经理', mobile_phone: '13900139000' });
    expect(String(updRes.position), 'PUT 响应回显职位改写').toBe('副经理');
    const updRows = pickListArray<Record<string, unknown>>(await apiCallRaw<unknown>(page, 'GET', `/purchase/suppliers/${supplierId}/contacts`), 'bare', '改值后联系人列表');
    const updated = rowById(updRows, contactId, '联系人改值后列表');
    expect(String(updated.position), '联系人职位改写落库').toBe('副经理');
    expect(String(updated.mobile_phone), '联系人手机号改写落库').toBe('13900139000');
    expect(String(updated.department), '未提交字段不应被更新清空').toBe('采购部');
    expect(String(updated.contact_name), '未提交姓名不应漂移').toBe('张主联');

    // 删：删除链路断库里已消失，并断父供应商未被级联误伤（删前后 status/is_enabled 同源不变）
    const parentBefore = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/suppliers/${supplierId}`);
    const delRes = await apiCall(page, 'DELETE', `/purchase/suppliers/${supplierId}/contacts/${contactId}`);
    expect(delRes.code, '删除成功信封码').toBe(200);
    const delRaw = await apiCallRaw(page, 'GET', `/purchase/suppliers/${supplierId}/contacts`);
    const delRows = pickListArray<Record<string, unknown>>(delRaw, 'bare', '删除后联系人列表');
    expect(
      delRows.some(r => Number(r.id) === contactId),
      '删除后联系人不应再在列表里'
    ).toBe(false);
    const parentAfter = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/suppliers/${supplierId}`);
    expect(Number(parentAfter.id), '删除子资源后父供应商应仍存在').toBe(Number(parentBefore.id));
    expect(String(parentAfter.status), '子资源删除不得改写父供应商 status').toBe(String(parentBefore.status));
    expect(parentAfter.is_enabled, '子资源删除不得改写父供应商 is_enabled').toBe(parentBefore.is_enabled);
  });

  test('151-02 资质改值与删除：建后回读、改名改有效期只动提交列、删后库里消失', async ({ page }) => {
    const supplierId = await newSupplier(page, 'QL');

    const created = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/purchase/suppliers/${supplierId}/qualifications`,
      {
        qualification_name: '营业执照',
        qualification_type: 'business_license',
        qualification_no: `BL${genCode('Q')}`,
        issuing_authority: '市场监督管理局',
        issue_date: isoDate(-30),
        valid_until: isoDate(365),
        need_annual_check: false,
      }
    );
    const qualId = requireId(created, '资质创建');

    const listRaw = await apiCallRaw<unknown>(page, 'GET', `/purchase/suppliers/${supplierId}/qualifications`);
    const initial = pickListArray<Record<string, unknown>>(listRaw, 'bare', '资质初始列表');
    const row = initial.find(r => Number(r.id) === qualId);
    if (!row) throw new Error(`资质列表未找到 id=${qualId}，实际 ${JSON.stringify(initial)}`);
    expect(String(row.qualification_name), '资质名称回读').toBe('营业执照');
    expect(row.need_annual_check, '年检标记回读').toBe(false);
    expect(String(row.issue_date), '发证日期回读').toBe(isoDate(-30));

    // 改：提交新名称与新有效期与年检标记，发证机关未提交应原样保留
    const newUntil = isoDate(730);
    const updRes = await apiCallRaw<Record<string, unknown>>(page, 'PUT', `/purchase/suppliers/${supplierId}/qualifications/${qualId}`, { qualification_name: '营业执照（换证）', qualification_type: 'business_license', qualification_no: String(row.qualification_no), issuing_authority: '市场监督管理局', issue_date: isoDate(-30), valid_until: newUntil, need_annual_check: true });
    expect(String(updRes.qualification_name), 'PUT 响应回显资质改名').toBe('营业执照（换证）');
    const afterRaw = await apiCallRaw(page, 'GET', `/purchase/suppliers/${supplierId}/qualifications`);
    const after = pickListArray<Record<string, unknown>>(afterRaw, 'bare', '资质改值后列表');
    const upd = rowById(after, qualId, '改值后资质列表');
    expect(String(upd.qualification_name), '资质名称改写落库').toBe('营业执照（换证）');
    expect(String(upd.valid_until), '资质有效期改写落库').toBe(newUntil);
    expect(upd.need_annual_check, '年检标记改写落库').toBe(true);
    expect(String(upd.issuing_authority), '发证机关应随整单提交保持不变').toBe('市场监督管理局');

    // 删：删除后库里消失，并断父供应商未被级联误伤（删前后 status/is_enabled 同源不变）
    const qualParentBefore = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/suppliers/${supplierId}`);
    const qualDelRes = await apiCall(page, 'DELETE', `/purchase/suppliers/${supplierId}/qualifications/${qualId}`);
    expect(Number(qualDelRes.data && (qualDelRes.data as Record<string, unknown>).deleted_id), '删除信封回显被删资质 id').toBe(qualId);
    const goneRaw = await apiCallRaw(page, 'GET', `/purchase/suppliers/${supplierId}/qualifications`);
    const gone = pickListArray<Record<string, unknown>>(goneRaw, 'bare', '删除后的资质列表');
    expect(
      gone.some(r => Number(r.id) === qualId),
      '删除后资质不应再在列表里'
    ).toBe(false);
    const qualParentAfter = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/suppliers/${supplierId}`);
    expect(Number(qualParentAfter.id), '删除资质后父供应商应仍存在').toBe(Number(qualParentBefore.id));
    expect(String(qualParentAfter.status), '资质删除不得改写父供应商 status').toBe(String(qualParentBefore.status));
    expect(qualParentAfter.is_enabled, '资质删除不得改写父供应商 is_enabled').toBe(qualParentBefore.is_enabled);
  });

  test('151-03 供应商启停开关：停用同时落 is_enabled=false 与 status=inactive，再启用两列回到启用态', async ({
    page,
  }) => {
    const supplierId = await newSupplier(page, 'ST');

    const offRes = await apiCallRaw<Record<string, unknown>>(page, 'POST', `/purchase/suppliers/${supplierId}/status`, { enable: false });
    expect(offRes.is_enabled, '停用响应回显 is_enabled=false').toBe(false);
    const offRaw = await apiCallRaw(page, 'GET', `/purchase/suppliers/${supplierId}`);
    const off = offRaw as Record<string, unknown>;
    expect(off.is_enabled, '停用后 is_enabled 应为 false').toBe(false);
    expect(String(off.status), '停用后 status 文本列应为 inactive').toBe('inactive');

    const onRes = await apiCallRaw<Record<string, unknown>>(page, 'POST', `/purchase/suppliers/${supplierId}/status`, { enable: true });
    expect(String(onRes.status), '启用响应回显 status=active').toBe('active');
    const onRaw = await apiCallRaw(page, 'GET', `/purchase/suppliers/${supplierId}`);
    const on = onRaw as Record<string, unknown>;
    expect(on.is_enabled, '启用后 is_enabled 应为 true').toBe(true);
    expect(String(on.status), '启用后 status 文本列应回到 active').toBe('active');
  });
});
