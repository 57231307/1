import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  APP_ERROR_CODES,
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  deferCleanup,
  flushDeferredCleanups,
  type DeferredCleanup,
} from './helpers';

/**
 * 98 报表增强订阅的建改查与启停读回（analytics 域，reports enhanced subscriptions 族）
 *
 * 覆盖六个真实端点（前端 api 层 report-enhanced.ts 逐一封装、advanced/index.vue 挂 SubscriptionPanel、
 * 面板内实调用这些函数，权限键按 URL 段取 reports/enhanced）：
 * - POST /reports/enhanced/subscriptions 建订阅，入参 name、template_id、frequency、recipients 必填，
 *   export_format 缺省 pdf、is_enabled 缺省 true，落库 status 为 ACTIVE、run_count 为 0、
 *   retry_count 为 0、next_run_at 由频率算出（DAILY 为次日、WEEKLY 为次周、MONTHLY 为三十天后），
 *   created_by 取当前登录用户。校验都在服务层：收件人为空、邮箱格式非法、频率不在
 *   DAILY、WEEKLY、MONTHLY 三者之一，一律返回 400 且码为 VALIDATION_ERROR；template_id 指向不存在的模板
 *   返回 404 且码为 NOT_FOUND；模板非公开且非本人创建则 403。
 * - GET /reports/enhanced/subscriptions 列表，出参是分页信封，data 下含 items 数组与顶层 total，
 *   且只列 status 为 ACTIVE 的行（软删后的行自动从列表消失），支持 template_id 查询参数过滤。
 * - GET /reports/enhanced/subscriptions/{id} 取单条逐键读回。
 * - PUT /reports/enhanced/subscriptions/{id} 部分更新，入参各列皆可选，只覆盖提交的列。
 * - POST /reports/enhanced/subscriptions/{id}/toggle 启停，入参 enabled 必填布尔，返回更新后的整行。
 * - DELETE /reports/enhanced/subscriptions/{id} 是软删，把 status 置 INACTIVE 且 is_enabled 置 false，
 *   并不物理删行，故删除是否生效只靠列表（按 status 为 ACTIVE 过滤）不再含该 id 反证，不直断删接口码。
 *
 * 外键前置自给自足：订阅必须引用一个真实且公开的报表模板。测试库无预置公开模板，且模板 create 禁用
 * 自定义 SQL、编码唯一，故每个用例先用 POST /reports/enhanced/templates 建一个私有 code 唯一的公开
 * 模板再订阅，模板与订阅都在清理队列里删除，不碰他人在途数据、也不删任何种子行。
 *
 * 断言均为值判据（写什么读回什么、启停后读回 enabled、软删后列表不再含该 id），
 * 列表按 data.items 判、不把 data 当裸数组，负例钉到具体 HTTP 码与机器码。
 */

// 订阅整行（report_subscription Model 的 snake_case 键，序列化后 recipients 为字符串数组）。
type Sub = {
  id: number;
  name: string;
  template_id: number;
  frequency: string;
  recipients: string[];
  export_format: string;
  is_enabled: boolean;
  status: string;
  next_run_at: string | null;
  last_run_at: string | null;
  run_count: number;
  created_by: number;
};

// 订阅列表出参（ApiResponse.data 即分页信封）。
type SubPage = { items: Sub[]; total: number; page: number; page_size: number };

function uniq(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

/**
 * 建一个仅供本用例订阅引用的公开报表模板并返回其 id。
 * code 用唯一后缀避开全表唯一约束；不提交 data_source_sql（后端禁用自定义 SQL 报表入口）；
 * is_public 置 true 以便同会话直接订阅、并绕开私有模板归属判定。
 */
async function provisionTemplate(page: Page, tag: string): Promise<number> {
  const tpl = await apiCallRaw<{ id: number }>(page, 'POST', '/reports/enhanced/templates', {
    name: `E2E TPL ${tag}`,
    code: `e2e-tpl-${tag}`.slice(0, 50),
    report_type: 'e2e_probe',
    columns: [],
    is_public: true,
  });
  return tpl.id;
}

test.describe.serial('98 报表增强订阅建改查与启停读回', () => {
  const cleanup: DeferredCleanup[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    await flushDeferredCleanups(page, cleanup);
  });

  test('建订阅后单条与列表逐键读回，缺省列落后端默认', async ({ page }) => {
    const tag = uniq();
    const templateId = await provisionTemplate(page, tag);
    deferCleanup(cleanup, 'DELETE', `/reports/enhanced/templates/${templateId}`, `tpl-${tag}`);

    const recipient = `e2e-${tag}@example.test`;
    const created = await apiCallRaw<Sub>(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `E2E-SUB-${tag}`,
      template_id: templateId,
      frequency: 'DAILY',
      recipients: [recipient],
    });
    const id = created.id;
    expect(Number.isInteger(id) && id > 0, `建订阅应返回正整数 id，实际：${String(id)}`).toBe(true);
    deferCleanup(cleanup, 'DELETE', `/reports/enhanced/subscriptions/${id}`, `sub-${tag}`);

    // 建接口回显即落库默认。
    expect(created.name).toBe(`E2E-SUB-${tag}`);
    expect(created.template_id).toBe(templateId);
    expect(created.frequency).toBe('DAILY');
    expect(
      created.recipients,
      `收件人应原样存为字符串数组，实际：${JSON.stringify(created.recipients)}`
    ).toEqual([recipient]);
    expect(
      created.export_format,
      `未提交 export_format 应落默认 pdf，实际：${String(created.export_format)}`
    ).toBe('pdf');
    expect(
      created.is_enabled,
      `未提交 is_enabled 应落默认 true，实际：${String(created.is_enabled)}`
    ).toBe(true);
    expect(created.status, `新建订阅状态应为 ACTIVE，实际：${String(created.status)}`).toBe(
      'ACTIVE'
    );
    expect(created.run_count).toBe(0);
    expect(
      created.next_run_at,
      `DAILY 频率应算出非空 next_run_at，实际：${String(created.next_run_at)}`
    ).not.toBeNull();
    expect(
      created.last_run_at,
      `新建订阅 last_run_at 应为空，实际：${String(created.last_run_at)}`
    ).toBeNull();
    expect(Number.isInteger(created.created_by), `created_by 应为登录用户 id 整数`).toBe(true);

    // 从数据库经单条接口逐键读回。
    const single = await apiCallRaw<Sub>(page, 'GET', `/reports/enhanced/subscriptions/${id}`);
    expect(single.id).toBe(id);
    expect(single.name).toBe(`E2E-SUB-${tag}`);
    expect(single.frequency).toBe('DAILY');
    expect(single.recipients).toEqual([recipient]);

    // 从数据库经列表按模板过滤读回（证明落库且列表可检索到）。
    const listed = await apiCall<SubPage>(
      page,
      'GET',
      `/reports/enhanced/subscriptions?template_id=${templateId}&page=1&page_size=50`
    );
    const hit = listed.data.items.find(r => r.id === id);
    expect(hit, `列表按 template_id=${templateId} 应含刚建订阅 id=${id}`).toBeTruthy();
    expect(hit!.name).toBe(`E2E-SUB-${tag}`);
  });

  test('toggle 停用后读回 enabled 为 false 且状态仍 ACTIVE 不出列表', async ({ page }) => {
    const tag = uniq();
    const templateId = await provisionTemplate(page, tag);
    deferCleanup(cleanup, 'DELETE', `/reports/enhanced/templates/${templateId}`, `tpl-${tag}`);

    const created = await apiCallRaw<Sub>(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `E2E-SUB-TOG-${tag}`,
      template_id: templateId,
      frequency: 'WEEKLY',
      recipients: [`e2e-${tag}@example.test`],
    });
    const id = created.id;
    expect(created.is_enabled).toBe(true);
    deferCleanup(cleanup, 'DELETE', `/reports/enhanced/subscriptions/${id}`, `sub-tog-${tag}`);

    const toggled = await apiCallRaw<Sub>(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${id}/toggle`,
      {
        enabled: false,
      }
    );
    expect(
      toggled.is_enabled,
      `停用后 is_enabled 应为 false，实际：${String(toggled.is_enabled)}`
    ).toBe(false);

    // 停用不改 status（仍 ACTIVE），故按 ACTIVE 过滤的列表仍应含它、且带 enabled false。
    const listed = await apiCall<SubPage>(
      page,
      'GET',
      `/reports/enhanced/subscriptions?template_id=${templateId}&is_enabled=false&page=1&page_size=50`
    );
    const hit = listed.data.items.find(r => r.id === id);
    expect(hit, `停用订阅应仍出现在列表（status 仍 ACTIVE）`).toBeTruthy();
    expect(hit!.is_enabled).toBe(false);
    expect(hit!.status, `停用订阅的 status 仍应为 ACTIVE，实际：${String(hit!.status)}`).toBe(
      'ACTIVE'
    );

    // 再启用回来。
    const reEnabled = await apiCallRaw<Sub>(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${id}/toggle`,
      {
        enabled: true,
      }
    );
    expect(reEnabled.is_enabled, `启用后 is_enabled 应回到 true`).toBe(true);
  });

  test('PUT 部分更新只改提交的列并读回新值', async ({ page }) => {
    const tag = uniq();
    const templateId = await provisionTemplate(page, tag);
    deferCleanup(cleanup, 'DELETE', `/reports/enhanced/templates/${templateId}`, `tpl-${tag}`);

    const created = await apiCallRaw<Sub>(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `E2E-SUB-OLD-${tag}`,
      template_id: templateId,
      frequency: 'DAILY',
      recipients: [`old-${tag}@example.test`],
      export_format: 'excel',
    });
    const id = created.id;
    deferCleanup(cleanup, 'DELETE', `/reports/enhanced/subscriptions/${id}`, `sub-upd-${tag}`);

    const newRecipient = `new-${tag}@example.test`;
    await apiCall(page, 'PUT', `/reports/enhanced/subscriptions/${id}`, {
      name: `E2E-SUB-NEW-${tag}`,
      recipients: [newRecipient],
    });

    const after = await apiCallRaw<Sub>(page, 'GET', `/reports/enhanced/subscriptions/${id}`);
    expect(after.name, `名称应更新为新值，实际：${String(after.name)}`).toBe(`E2E-SUB-NEW-${tag}`);
    expect(
      after.recipients,
      `收件人应更新为新邮箱，实际：${JSON.stringify(after.recipients)}`
    ).toEqual([newRecipient]);
    // 未提交的列保持原值（证明部分更新，不是全量覆盖清空）。
    expect(
      after.frequency,
      `未提交的 frequency 应保持 DAILY，实际：${String(after.frequency)}`
    ).toBe('DAILY');
    expect(
      after.export_format,
      `未提交的 export_format 应保持 excel，实际：${String(after.export_format)}`
    ).toBe('excel');
    expect(
      after.template_id,
      `未提交的 template_id 不应被改，实际：${String(after.template_id)}`
    ).toBe(templateId);
  });

  test('收件人为空、邮箱非法、频率越界被拒 400 且码为 VALIDATION_ERROR', async ({ page }) => {
    const tag = uniq();
    const templateId = await provisionTemplate(page, tag);
    deferCleanup(cleanup, 'DELETE', `/reports/enhanced/templates/${templateId}`, `tpl-${tag}`);

    const emptyRec = await apiCallExpectFail(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `E2E-SUB-EMPTY-${tag}`,
      template_id: templateId,
      frequency: 'DAILY',
      recipients: [],
    });
    expect(
      emptyRec.status,
      `空收件人应 400，实际 status=${emptyRec.status} code=${emptyRec.code}`
    ).toBe(400);
    expect(emptyRec.code, `空收件人的码应为 VALIDATION_ERROR，实际：${emptyRec.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    const badEmail = await apiCallExpectFail(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `E2E-SUB-BADEMAIL-${tag}`,
      template_id: templateId,
      frequency: 'DAILY',
      recipients: ['not-an-email'],
    });
    expect(
      badEmail.status,
      `非法邮箱应 400，实际 status=${badEmail.status} code=${badEmail.code}`
    ).toBe(400);
    expect(badEmail.code, `非法邮箱的码应为 VALIDATION_ERROR，实际：${badEmail.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    const badFreq = await apiCallExpectFail(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `E2E-SUB-BADFREQ-${tag}`,
      template_id: templateId,
      frequency: 'HOURLY',
      recipients: [`e2e-${tag}@example.test`],
    });
    expect(
      badFreq.status,
      `非法频率应 400，实际 status=${badFreq.status} code=${badFreq.code}`
    ).toBe(400);
    expect(badFreq.code, `非法频率的码应为 VALIDATION_ERROR，实际：${badFreq.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });

  test('订阅不存在模板被拒 404 且码为 NOT_FOUND，取未知 id 详情 404', async ({ page }) => {
    const tag = uniq();

    const missingTpl = await apiCallExpectFail(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `E2E-SUB-NOTPL-${tag}`,
      template_id: 999999999,
      frequency: 'DAILY',
      recipients: [`e2e-${tag}@example.test`],
    });
    expect(
      missingTpl.status,
      `引用不存在模板应 404，实际 status=${missingTpl.status} code=${missingTpl.code}`
    ).toBe(404);
    expect(missingTpl.code, `引用不存在模板的码应为 NOT_FOUND，实际：${missingTpl.code}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );

    const getMissing = await apiCallExpectFail(
      page,
      'GET',
      '/reports/enhanced/subscriptions/999999999'
    );
    expect(
      getMissing.status,
      `取未知订阅应 404，实际 status=${getMissing.status} code=${getMissing.code}`
    ).toBe(404);
    expect(getMissing.code, `取未知订阅的码应为 NOT_FOUND，实际：${getMissing.code}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });

  test('软删后按 ACTIVE 过滤的列表不再含该订阅，反证删除生效', async ({ page }) => {
    const tag = uniq();
    const templateId = await provisionTemplate(page, tag);
    // 模板本身不在本用例结束前删（列表过滤要读它关联的行），交 finally 统一删。
    deferCleanup(cleanup, 'DELETE', `/reports/enhanced/templates/${templateId}`, `tpl-${tag}`);

    const created = await apiCallRaw<Sub>(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `E2E-SUB-DEL-${tag}`,
      template_id: templateId,
      frequency: 'MONTHLY',
      recipients: [`e2e-${tag}@example.test`],
    });
    const id = created.id;

    // 删前先确认在按 ACTIVE 过滤的列表里（否则删后消失无法归因于删除）。
    const before = await apiCall<SubPage>(
      page,
      'GET',
      `/reports/enhanced/subscriptions?template_id=${templateId}&page=1&page_size=50`
    );
    expect(
      before.data.items.some(r => r.id === id),
      `删前列表应含 id=${id}`
    ).toBe(true);

    await apiCall(page, 'DELETE', `/reports/enhanced/subscriptions/${id}`);

    const after = await apiCall<SubPage>(
      page,
      'GET',
      `/reports/enhanced/subscriptions?template_id=${templateId}&page=1&page_size=50`
    );
    expect(
      after.data.items.some(r => r.id === id),
      `软删（status 置 INACTIVE）后按 ACTIVE 过滤的列表不应再含 id=${id}`
    ).toBe(false);
  });
});
