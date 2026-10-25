import { test, expect } from '../diagnose-fixture';
import type { Locator, Page } from '@playwright/test';
import { loginAsRole, apiCall, apiCallRaw, genCode, tryCleanup, API_PREFIX } from './helpers';
import {
  safeGoto,
  waitForDialog,
  fillFieldByLabel,
  submitDialog,
  findTableRow,
  formItemByExactLabel,
} from './ui-helpers';

/**
 * 74 客户域改接回归（客户列表/读写/详情均走标准客户域端点）。
 *
 * 钉死三条契约（对照后端真实 DTO/路由，非猜测）：
 * - D1 列表：GET /crm/customers → customer_handler::list_customers → PaginatedResponse
 *   （信封键 items/total），每行 = models/customer.rs::Model，含 customer_name/contact_person/
 *   contact_phone 三键。用例对目标行三列逐一取列断真值（columnCell 按列头定位到格，
 *   不放宽为整行 hasText），并以 admin 会话断言"三列非空且不含掩码星号"（admin 不脱敏）。
 * - D2 写入：POST/PUT/DELETE /crm/customers 落 customers 表。用例抓 UI 提交的真实请求体断言：
 *   落在 /crm/customers、不含 customer_code（服务端建档生成）、credit_limit 若携带必须为 string
 *   （后端 Option<String>，number 即 422）、不含线索域键 tax_number；并反向断言线索域列表
 *   （/crm/customers/enhanced = crm_lead）JSON 中查无该唯一名。
 * - D3 详情：viewDetail 用 row.id push /crm/detail/{id}，detail.vue 以该 id 打
 *   GET /crm/customers/{id}/360（cust.rs CustomerEntity::find_by_id，客户域）。
 *   row.id = customer.id，用例直接 GET 360 断言 data.customer.id 回显命中。
 *
 * 夹具合法性（对照后端校验，避免 CHECK/validate 假红）：
 * - customer_name 长度 1..200；contact_phone ≤20 且满足前端 ^1[3-9]\d{9}$ 手机号规则；
 *   customer_type ∈ ALLOWED 五值取 'retail'；
 * - credit_limit '5000.00'（Option<String> 十进制串）；contact_email 留空不发送
 *   （空串会触发 #[validate(email)] 422，前端 buildCreatePayload 同口径 omit）。
 */

type Row = Record<string, unknown>;

/**
 * 按**列头文案**定位目标行的那一格单元格（不是整行文本），用于真读列值。
 * 与 56-read-after-write 同判据：表头未出现该列名 / 行列数不覆盖该列，都按真实缺陷判红
 * （列缺失、表头文案漂移、行渲染不齐都会让"三列空白"重新变成看不见的缺陷）。
 */
async function columnCell(page: Page, row: Locator, columnLabel: string): Promise<Locator> {
  const headers = page.getByRole('columnheader');
  const headerCount = await headers.count();
  let idx = -1;
  for (let i = 0; i < headerCount; i++) {
    const label = (await headers.nth(i).innerText()).trim();
    if (label === columnLabel) {
      idx = i;
      break;
    }
  }
  // 定位循环以 label===columnLabel 逐字符相等取列，未命中时 idx 保持 -1：判负即列缺失或文案漂移
  expect(
    idx,
    `表格中未渲染出表头列「${columnLabel}」（当前可见表头 ${headerCount} 个，URL=${page.url()}）` +
      `——列缺失或表头文案漂移属真实缺陷，非用例问题，不放宽为整行 hasText`
  ).not.toBe(-1);

  const cells = row.getByRole('cell');
  const cellCount = await cells.count();
  expect(
    cellCount,
    `目标行单元格数 ${cellCount} 未覆盖到「${columnLabel}」所在的第 ${idx} 列——行/列渲染不齐属真实缺陷`
  ).toBeGreaterThan(idx);
  return cells.nth(idx);
}

function uniquePhone(): string {
  // 11 位、^1[3-9]\d{9}$：139 + Date.now 后 8 位
  return `139${Date.now().toString().slice(-8)}`;
}

test.describe('74 CRM 客户域改接（BUG-005/004 回归护栏）', () => {
  test('T1 API 面：POST /crm/customers 真实建档 → items 列表三列有值 → 360 命中 → 线索域查无此行', async ({
    page,
  }) => {
    await loginAsRole(page, 'admin');
    const tag = genCode('CUST74A');
    const contactPerson = `联系人${tag}`;
    const phone = uniquePhone();

    // 1) 建到客户域：apiCall 在信封 code≠200/0 时抛错，返回即已确认 2xx+业务成功
    const created = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_name: tag,
      contact_person: contactPerson,
      contact_phone: phone,
      customer_type: 'retail',
      status: 'active',
      credit_limit: '5000.00',
    });
    expect(created, 'POST /crm/customers 应返回创建行').not.toBeNull();
    expect(
      Number.isInteger(created.id),
      `创建响应 id 应为真实整数主键，实际=${String(created.id)}`
    ).toBe(true);
    expect(created.id as number).toBeGreaterThan(0);
    expect(
      String(created.customer_code ?? ''),
      'customer_code 应由服务端建档生成（非空）'
    ).not.toBe('');
    // admin 写响应不脱敏：回显原文手机号即"落的是 customer 表、键名 contact_phone"的实证
    expect(created.contact_phone).toBe(phone);

    // 2) 标准列表读回：信封键 items/total（PaginatedResponse，定义见 utils/response.rs）
    const list = await apiCall<{ items: Row[]; total: number; page: number; page_size: number }>(
      page,
      'GET',
      `/crm/customers?page=1&page_size=20&keyword=${encodeURIComponent(tag)}`
    );
    expect(Array.isArray(list.data.items), 'data.items 应为数组（listKey=items）').toBe(true);
    expect(typeof list.data.total, 'data.total 应为数字').toBe('number');
    const row = list.data.items.find(r => r.id === created.id);
    expect(row, `标准 /crm/customers 应能读到刚建客户 id=${String(created.id)}`).toBeDefined();
    // 三列真值断言：键存在且非空、值与提交一致
    expect(row!.customer_name, 'customer_name 列键应有值（BUG-005 空白列 1/3）').toBe(tag);
    expect(row!.contact_person, 'contact_person 列键应有值（BUG-005 空白列 2/3）').toBe(
      contactPerson
    );
    expect(row!.contact_phone, 'contact_phone 列键应有值（BUG-005 空白列 3/3）').toBe(phone);
    expect(String(row!.contact_phone)).not.toContain('*');

    // 3) 线索域查无护栏：该唯一名不得出现在线索域列表（/crm/customers/enhanced = crm_lead 整行）
    const enhanced = await apiCall<unknown>(
      page,
      'GET',
      `/crm/customers/enhanced?page=1&page_size=100&keyword=${encodeURIComponent(tag)}`
    );
    expect(
      JSON.stringify(enhanced.data),
      '线索域列表不得出现本客户唯一名（若出现=写入又落 crm_lead 表，假成功写入回归）'
    ).not.toContain(tag);

    // 4) 详情命中护栏：360 端点（cust.rs CustomerEntity::find_by_id）按客户 id 命中
    const detail = await apiCallRaw<Row>(page, 'GET', `/crm/customers/${String(created.id)}/360`);
    const customer = detail.customer as Row;
    expect(customer, '360 出参应含 customer 对象').toBeDefined();
    expect(customer.id, '360 的 customer.id 应等于建档 id（详情不再 404）').toBe(created.id);
    expect(customer.customer_name).toBe(tag);

    await tryCleanup(page, 'DELETE', `/crm/customers/${String(created.id)}`, '清理 API 建客户');
  });

  test('T2 UI 面：新建弹窗提交落 /crm/customers（请求体契约）→ 刷新后三列单元格显示真值', async ({
    page,
  }) => {
    await loginAsRole(page, 'admin');
    const tag = genCode('CUST74U');
    const contactPerson = `联系人${tag}`;
    const phone = uniquePhone();

    // 客户列表唯一入口 /customer（原 /crm 列表已合并迁入）
    await safeGoto(page, '/customer');
    await page.locator('.filter-card').first().waitFor({ state: 'visible', timeout: 15_000 });

    // 打开新建客户弹窗（按钮文案 = customer.index.button.create）
    await page.getByRole('button', { name: '新建客户' }).click();
    const dialog = await waitForDialog(page);
    // 弹窗字段文案取本页真实渲染的 i18n 键：views/customer/tabs/CustomerFormTab.vue 的
    // el-form-item label 绑 customer.form.label.*（customerName=客户名称、contactPerson=联系人、
    // contactPhone=**联系电话**）。fillFieldByLabel 走 formItemByExactLabel 的行首尾锚定精确匹配，
    // 少一个字都不命中，故弹窗内只能用「联系电话」。
    // 列表列头是另一套键（customer/index.vue 的 column.phone 绑 customer.index.table.column.phone=电话），
    // 下方按列头读单元格仍用「电话」——两套文案不可互换。
    await fillFieldByLabel(dialog, page, '客户名称', tag);
    await fillFieldByLabel(dialog, page, '联系人', contactPerson);
    await fillFieldByLabel(dialog, page, '联系电话', phone);
    // 硬断言「联系电话」这项真收到了值：弹窗项文案是 customer.form.label.contactPhone（联系电话），
    // 与列表列头 customer.index.table.column.phone（电话）是两套键，按列头文案在弹窗内锚定恒 0 命中。
    // 只断"点了保存、请求发了"不足以区分"填进去的是空串"，故按同一精确 label 回读控件值；
    // 该项控件是 el-input（真 <input>），值须与提交原文逐字符相等。
    await expect(
      formItemByExactLabel(dialog, '联系电话').locator('input').first(),
      '「联系电话」输入框必须收到用例填入的手机号原文，否则提交体里的 contact_phone 不可能等于它'
    ).toHaveValue(phone);

    // 抓前端真实请求：POST 且 pathname 精确等于 ${API_PREFIX}/crm/customers
    // （精确等式排除 /crm/customers/enhanced 与 /crm/customers/{id}/... 子路径——
    //   若页面回退接错端点，本等待将永不命中直接超时判红，不放行）
    // 跳过 CSRF 首发瞬态 403（admin 被权限中间件短路，该路由唯一 403 源是 CSRF 中间件、
    // 拦截器刷新令牌后静默重放成 200；持续真 403 会超时判红，同 56 口径）
    const respPromise = page.waitForResponse(
      res => {
        if (res.request().method() !== 'POST' || res.status() === 403) return false;
        try {
          return new URL(res.url()).pathname === `${API_PREFIX}/crm/customers`;
        } catch {
          return false;
        }
      },
      { timeout: 30_000 }
    );
    await submitDialog(dialog, /保存/);
    const resp = await respPromise;
    expect(resp.status(), 'UI 新建 POST /crm/customers 应返回 2xx').toBeLessThan(300);

    const payloadRaw = resp.request().postData();
    expect(payloadRaw, '未捕获到 UI 提交请求体').not.toBeNull();
    const payload = JSON.parse(String(payloadRaw)) as Row;
    // D2 请求体契约：客户域键集
    expect(payload.customer_name, '提交体应含 customer_name').toBe(tag);
    expect(payload.contact_person).toBe(contactPerson);
    expect(payload.contact_phone).toBe(phone);
    expect(
      Object.prototype.hasOwnProperty.call(payload, 'customer_code'),
      'customer_code 不得提交（创建 DTO 由服务端生成，编辑 DTO 无此键）'
    ).toBe(false);
    if (payload.credit_limit !== undefined) {
      expect(
        typeof payload.credit_limit,
        'credit_limit 后端为 Option<String>（number 即 422，C-03），前端必须发 string'
      ).toBe('string');
    }
    expect(payload.tax_number, 'tax_number 是线索域键，不得再出现在客户域提交体').toBeUndefined();

    const body = (await resp.json()) as { code?: number; data?: Row };
    expect(
      body.code === 200 || body.code === 0,
      `响应信封业务码应为成功，实际 code=${String(body.code)}`
    ).toBe(true);
    const createdId = Number(body.data?.id);
    expect(
      Number.isInteger(createdId) && createdId > 0,
      `响应 data.id 应为真实客户主键（假成功写入了 lead 则此处无 id），实际=${String(body.data?.id)}`
    ).toBe(true);

    // reload 强制重新拉取后端（模拟用户刷新，清空 SPA 内存态）→ 关键字过滤 → 逐列断真值
    // 关键字必须点「查询」按钮（customer.index.button.query）才生效：本页关键词输入框
    // （customer/index.vue 的关键词输入项）没绑 @keyup.enter/@change，回车不发带 keyword 的请求；
    // 后端 keyword 对 customer_name/customer_code contains 过滤
    // （services/customer_ops/query.rs 的关键词分支），过滤后目标行必落在第 1 页，不靠分页碰运气。
    await page.reload();
    await page.locator('.filter-card').first().waitFor({ state: 'visible', timeout: 15_000 });
    await fillFieldByLabel(page, page, '关键词', tag);
    const filteredList = page.waitForResponse(
      res => {
        if (res.request().method() !== 'GET' || !res.url().includes('keyword=')) return false;
        try {
          return new URL(res.url()).pathname === `${API_PREFIX}/crm/customers`;
        } catch {
          return false;
        }
      },
      { timeout: 30_000 }
    );
    await page.getByRole('button', { name: '查询' }).first().click();
    await filteredList;
    const row = await findTableRow(page, tag);
    expect(
      row,
      `刷新后列表应仍能按关键字 ${tag} 查到目标行（写入真实落库 + 列表同源）`
    ).not.toBeNull();

    const nameCell = await columnCell(page, row!, '客户名称');
    expect(
      (await nameCell.innerText()).trim(),
      '客户名称列应渲染真值（防"count>0 列空白"假绿）'
    ).toBe(tag);
    const personCell = await columnCell(page, row!, '联系人');
    expect((await personCell.innerText()).trim(), '联系人列应渲染真值').toBe(contactPerson);
    const phoneCell = await columnCell(page, row!, '电话');
    const phoneText = (await phoneCell.innerText()).trim();
    expect(phoneText, '电话列应渲染真值（admin 不脱敏，C-13）').toBe(phone);
    expect(phoneText, 'admin 会话电话列不得是掩码').not.toContain('*');

    await tryCleanup(page, 'DELETE', `/crm/customers/${String(createdId)}`, '清理 UI 建客户');
  });
});
