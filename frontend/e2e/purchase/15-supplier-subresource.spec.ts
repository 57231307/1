// 采购管理 E2E 套件 — 15 供应商子资源 API 契约（select 检索 + 联系人 CRUD + 启停状态）
// 覆盖 purchase A 档零覆盖端点（page.request 直连契约覆盖，CI shard 实跑）：
//   GET    /purchase/suppliers/select                     list_suppliers（下拉检索通道）
//   POST   /purchase/suppliers/{id}/contacts              create_supplier_contact
//   PUT    /purchase/suppliers/{id}/contacts/{cid}        update_supplier_contact
//   DELETE /purchase/suppliers/{id}/contacts/{cid}        delete_supplier_contact
//   POST   /purchase/suppliers/{id}/status                toggle_supplier_status（enable=true/false）
// 契约来源：backend/src/routes/purchase/369/373/378、supplier_handler/181/237/266/290、
//   supplier_service.rs（create_supplier 仅需唯一 supplier_name、toggle 落 is_enabled + status=
//   master_data::ACTIVE('active')/INACTIVE('inactive')）。
// 脱敏事实：list_suppliers（含 /suppliers/select）对非 admin 掩 mobile_phone/email（field_mask.rs）；
//   故 select 与列表断言只用未脱敏字段（id / supplier_name / is_enabled / status）；
//   contacts 子端点不经掩码，可断 mobile_phone/contact_name/department。
// 清理与隔离：每条用例自建专属供应商（唯一名 E2E-SUP-*），停用/启用只作用于该专属供应商，
//   **绝不切换 ensureTestEntities 产出的共享 supplierId**（共享夹具被其它用例并发消费，切换会互相污染）；
//   供应商有 DELETE /suppliers/{id} 终止腿、联系人有 DELETE 终止腿，故自建即可清，
//   未向后端新增任何 DELETE/取消端点。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, apiCallExpectFail, tryCleanup } from '../flow/helpers';

/** 供应商出参（键对齐 models/supplier.rs；select/detail 中 phone/email 可能被脱敏，本套件不断言它们） */
interface SupplierRow {
  id: number;
  supplier_code: string;
  supplier_name: string;
  status: string | null;
  is_enabled: boolean | null;
}

/** 联系人出参（键对齐 models/supplier_contact.rs；contacts 子端点未脱敏，可断 phone 等） */
interface SupplierContact {
  id: number;
  supplier_id: number;
  contact_name: string;
  department: string | null;
  position: string | null;
  mobile_phone: string;
  is_primary: boolean;
}

/** 分页信封 PaginatedResponse{items,total,page,page_size}（utils/response.rs 唯一形状） */
interface Paginated<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 建一张专属供应商（唯一名，其余字段走后端默认）。仅 supplier_name 必填且需全库唯一，
 * 故用时间戳标记；id 缺失即抛（apiCall 非 200 已抛）。登记 DELETE 清理腿。
 */
async function createDedicatedSupplier(
  page: import('@playwright/test').Page,
  tag: string
): Promise<SupplierRow> {
  const s = await apiCallRaw<SupplierRow>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `E2E-SUP-${tag}`,
  });
  expect(s.id, '建专属供应商应返回 id').toBeTruthy();
  expect(s.supplier_name, `建单回显名称应为提交值，实际 ${s.supplier_name}`).toBe(`E2E-SUP-${tag}`);
  CLEANUP.push({ path: `/purchase/suppliers/${s.id}`, label: 'supplier' });
  return s;
}

/** 读取指定供应商的联系人裸数组（GET /suppliers/{id}/contacts 出参为 Vec） */
async function listContacts(
  page: import('@playwright/test').Page,
  supplierId: number
): Promise<SupplierContact[]> {
  const res = await apiCall<SupplierContact[]>(
    page,
    'GET',
    `/purchase/suppliers/${supplierId}/contacts`
  );
  return res.data;
}

test.describe('15 供应商子资源 API 契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('15-01 select 检索命中自建供应商并按 id/名称对齐', async ({ page }) => {
    const stamp = Date.now();
    const s = await createDedicatedSupplier(page, `SEL-${stamp}`);

    // /suppliers/select 与 /suppliers 同一 handler，但为独立快照路径，须逐路径探到。
    const list = await apiCallRaw<Paginated<SupplierRow>>(
      page,
      'GET',
      `/purchase/suppliers/select?keyword=${encodeURIComponent(s.supplier_name)}&page=1&page_size=10`
    );
    // 分页信封形状断言（非"有响应即通过"）：items 为数组且命中本次自建供应商
    expect(Array.isArray(list.items), 'select 应返回 items 数组').toBe(true);
    const hit = list.items.find(r => r.id === s.id);
    expect(hit, `select 检索应命中 id=${s.id} 的自建供应商`).toBeTruthy();
    expect(hit!.supplier_name, '命中行名称应与建单一致').toBe(s.supplier_name);
  });

  test('15-02 select 关键字过滤真实下推：命中目标、排除干扰', async ({ page }) => {
    const stamp = Date.now();
    const target = await createDedicatedSupplier(page, `SEL-TGT-${stamp}`);
    const distractor = await createDedicatedSupplier(page, `SEL-DIS-${stamp}`);

    const list = await apiCallRaw<Paginated<SupplierRow>>(
      page,
      'GET',
      `/purchase/suppliers/select?keyword=${encodeURIComponent(target.supplier_name)}&page=1&page_size=10`
    );
    expect(
      list.items.some(r => r.id === target.id),
      '结果应含目标供应商'
    ).toBe(true);
    // 干扰项名称唯一且不同串，若命中即证明 keyword 过滤根本没下推到 SQL
    expect(
      list.items.some(r => r.id === distractor.id),
      '结果不应含未匹配关键字的干扰供应商'
    ).toBe(false);
  });

  test('15-03 创建联系人并落库回读全部业务字段', async ({ page }) => {
    const stamp = Date.now();
    const s = await createDedicatedSupplier(page, `CTC-${stamp}`);

    const c = await apiCallRaw<SupplierContact>(
      page,
      'POST',
      `/purchase/suppliers/${s.id}/contacts`,
      {
        contact_name: '张三',
        department: '采购部',
        position: '经理',
        mobile_phone: '13900139000',
        is_primary: true,
      }
    );
    expect(c.id, '创建联系人应返回 id').toBeTruthy();
    expect(c.contact_name, '回显联系人姓名').toBe('张三');
    expect(c.is_primary, '回显主联系人标记').toBe(true);

    // 落库回读：contacts 子端点不经脱敏，逐字段核对持久化（源码若丢字段/未持久化即红）
    const rows = await listContacts(page, s.id);
    const saved = rows.find(r => r.id === c.id);
    expect(saved, '回读应命中新建联系人').toBeTruthy();
    expect(saved!.contact_name, '回读姓名').toBe('张三');
    expect(saved!.department, '回读部门应持久化').toBe('采购部');
    expect(saved!.position, '回读职位应持久化').toBe('经理');
    expect(saved!.mobile_phone, '回读手机号（子端点未脱敏）').toBe('13900139000');
    expect(saved!.supplier_id, '联系人应归属自建供应商').toBe(s.id);
  });

  test('15-04 非法手机号被 validate 拒绝且不落库', async ({ page }) => {
    // CreateContactRequest.mobile_phone 经 validate_mobile_phone（^1[3-9]\d{9}$）；
    // '12345' 不合法 → 校验拒绝（4xx），联系人不应被创建。
    const stamp = Date.now();
    const s = await createDedicatedSupplier(page, `CTC-BAD-${stamp}`);

    const before = (await listContacts(page, s.id)).length;
    const fail = await apiCallExpectFail(page, 'POST', `/purchase/suppliers/${s.id}/contacts`, {
      contact_name: '李四',
      mobile_phone: '12345',
      is_primary: false,
    });
    expect(fail.status, `非法手机号应 4xx，实际 ${fail.status}`).toBeGreaterThanOrEqual(400);
    expect(fail.status, '校验拒绝应为 4xx 非 5xx').toBeLessThan(500);

    const after = (await listContacts(page, s.id)).length;
    expect(after, '非法请求不应新增联系人').toBe(before);
  });

  test('15-05 更新联系人字段并落库回读', async ({ page }) => {
    const stamp = Date.now();
    const s = await createDedicatedSupplier(page, `CTC-UPD-${stamp}`);
    const c = await apiCallRaw<SupplierContact>(
      page,
      'POST',
      `/purchase/suppliers/${s.id}/contacts`,
      { contact_name: '王五', department: '质检部', mobile_phone: '13800138000', is_primary: true }
    );

    const upd = await apiCallRaw<SupplierContact>(
      page,
      'PUT',
      `/purchase/suppliers/${s.id}/contacts/${c.id}`,
      { contact_name: '王五改', department: '供应链部', is_primary: false }
    );
    expect(upd.contact_name, 'PUT 出参姓名应回显新值').toBe('王五改');
    expect(upd.is_primary, 'PUT 出参主联系人标记应变 false').toBe(false);

    // 回读：三值一致；未提交的手机号应保持原值（源码若把更新写成整行覆盖/清空即红）
    const rows = await listContacts(page, s.id);
    const saved = rows.find(r => r.id === c.id);
    expect(saved!.contact_name, '回读姓名应为新值').toBe('王五改');
    expect(saved!.department, '回读部门应为新值').toBe('供应链部');
    expect(saved!.is_primary, '回读主标记应为 false').toBe(false);
    expect(saved!.mobile_phone, '未提交的手机号应保持不变').toBe('13800138000');
  });

  test('15-06 删除联系人后 by-supplier 回读消失且同户其它联系人保留', async ({ page }) => {
    const stamp = Date.now();
    const s = await createDedicatedSupplier(page, `CTC-DEL-${stamp}`);
    const kept = await apiCallRaw<SupplierContact>(
      page,
      'POST',
      `/purchase/suppliers/${s.id}/contacts`,
      { contact_name: '保留人', mobile_phone: '13700137000', is_primary: false }
    );
    const doomed = await apiCallRaw<SupplierContact>(
      page,
      'POST',
      `/purchase/suppliers/${s.id}/contacts`,
      { contact_name: '待删人', mobile_phone: '13600136000', is_primary: true }
    );

    const del = await apiCall(page, 'DELETE', `/purchase/suppliers/${s.id}/contacts/${doomed.id}`);
    expect(del.code, 'DELETE 联系人应返回成功信封码').toBe(200);

    const rows = await listContacts(page, s.id);
    expect(
      rows.some(r => r.id === doomed.id),
      '被删联系人不应再出现'
    ).toBe(false);
    expect(
      rows.some(r => r.id === kept.id),
      '同供应商其它联系人应保留（源码若误删整户即红）'
    ).toBe(true);
  });

  test('15-07 停用/启用自建供应商双向状态跃迁并回读', async ({ page }) => {
    // 仅作用于专属自建供应商，绝不切换共享夹具 supplierId（并发其它用例消费中）。
    const stamp = Date.now();
    const s = await createDedicatedSupplier(page, `STA-${stamp}`);

    // 停用：enable=false → is_enabled=false, status=inactive
    const off = await apiCallRaw<SupplierRow>(page, 'POST', `/purchase/suppliers/${s.id}/status`, {
      enable: false,
    });
    expect(off.is_enabled, '停用后 is_enabled 应为 false').toBe(false);
    expect(off.status, `停用后 status 应为 inactive，实际 ${off.status}`).toBe('inactive');
    const offRead = await apiCallRaw<SupplierRow>(page, 'GET', `/purchase/suppliers/${s.id}`);
    expect(offRead.is_enabled, '回读 is_enabled 应为 false').toBe(false);
    expect(offRead.status, '回读 status 应为 inactive').toBe('inactive');

    // 启用：enable=true → is_enabled=true, status=active
    const on = await apiCallRaw<SupplierRow>(page, 'POST', `/purchase/suppliers/${s.id}/status`, {
      enable: true,
    });
    expect(on.is_enabled, '启用后 is_enabled 应为 true').toBe(true);
    expect(on.status, `启用后 status 应为 active，实际 ${on.status}`).toBe('active');
    const onRead = await apiCallRaw<SupplierRow>(page, 'GET', `/purchase/suppliers/${s.id}`);
    expect(onRead.is_enabled, '回读 is_enabled 应为 true').toBe(true);
    expect(onRead.status, '回读 status 应为 active').toBe('active');
  });

  test('15-08 状态切换缺 enable 键被反序列化拒绝', async ({ page }) => {
    // ToggleStatusRequest{ enable: bool } 无默认，缺该键 serde 反序列化失败 → 4xx。
    const stamp = Date.now();
    const s = await createDedicatedSupplier(page, `STA-BAD-${stamp}`);

    const fail = await apiCallExpectFail(page, 'POST', `/purchase/suppliers/${s.id}/status`, {
      not_enable_field: true,
    });
    expect(fail.status, `缺 enable 键应 4xx，实际 ${fail.status}`).toBeGreaterThanOrEqual(400);
    expect(fail.status, '反序列化拒绝应为 4xx 非 5xx').toBeLessThan(500);

    // 状态未被非法请求改动：回读仍为自建默认启用态非 inactive
    const after = await apiCallRaw<SupplierRow>(page, 'GET', `/purchase/suppliers/${s.id}`);
    expect(after.status, '非法切换不应把状态置为 inactive').not.toBe('inactive');
  });
});
