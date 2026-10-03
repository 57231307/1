// 系统域全流程契约级 E2E — 24 API 网关密钥（描述/有效期真实回显 + 三态清空 + 非法格式拒绝）
//
// 靶心（本轮修复的真实缺陷，基准 backend/src/handlers/api_gateway_handler.rs +
// services/api_key_service.rs）：
//   ① 出参 description 曾被硬写成 ""——密钥描述"存进去却永远显示不出来"，编辑对话框也回显不出；
//   ② 出参 created_by_name 曾被硬写成 ""（前端密钥列表该列恒空白）——现在由读侧
//      column_as(users.username)+JoinType::LeftJoin+into_model::<ApiKeyWithCreator> 富化，
//      创建者用户行缺失时为 null（禁止空串/假名回退）；
//   ③ expires_at 曾用 unwrap_or_default()，列 NULL 时输出 ""（前端无法与脏值区分）——
//      现在 NULL → JSON null，列表显式渲染「永不过期」；
//   ④ 非法 expires_at 曾被 `.ok()` 吞成 Some(None) → 落库把密钥改成永不过期且无任何报错——
//      现在 400 VALIDATION_ERROR + 真实外显文案，且原 expires_at 一字不改（安全缺陷）；
//   ⑤ 入参三态（本仓 double_option 适配器）：键缺席=保持原值、显式 null=清空、有值=覆盖。
// 定位：与 smoke/api-gateway.smoke.spec.ts（只验页面挂载）互补，本文件走密钥契约级全流程。
// 取数口径：API 自建唯一密钥（key_name 用 genCode 锚定，UI 用密钥名称检索精确定位行）；
//   断言锚在后端生成/回传字段（id、expires_at、created_by_name=登录用户名）上；
//   afterEach 用 tryCleanup 撤销自建密钥；toast/列文案断言前强制 locale=zh-CN
//   （src/i18n/index.ts STORAGE_KEY='bingxi.locale'，否则 navigator.language 协商让文案随环境漂移）。
// 铁律：无 ?? 兜底掩盖 null / 无宽松状态码判绿 / 5xx 一律判红。
// ⚠ 交付说明：本轮环境禁止执行 Playwright，本文件**未能本机实跑**，按既有 fullflow 口径静态编写。
import { test, expect } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  TEST_USERNAME,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  failureCode,
  genCode,
  loginViaUI,
  tryCleanup,
} from '../flow/helpers';
import type { Locator, Page } from '@playwright/test';

const CLEANUP: Array<{ path: string; label: string }> = [];

/** locale 固定 zh-CN 后的界面文案（apiGateway.* 语言包字面量） */
const TXT = {
  tabKeys: 'API 密钥',
  search: '搜索',
  searchPlaceholder: '搜索密钥名称',
  edit: '编辑',
  editTitle: '编辑密钥',
  cancel: '取消',
  confirm: '确定',
  neverExpires: '永不过期',
  operationSuccess: '操作成功',
  expiresAtField: '过期时间',
} as const;

/** 后端 validation_displayable 的用户可见文案（与 handler EXPIRES_AT_FORMAT_ERROR 同源） */
const EXPIRES_AT_FORMAT_MSG = '有效期格式不正确，应为 ISO 8601（如 2026-12-31T23:59:59Z）';

test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
): void {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(obj[key], `${label}：键 "${key}" 期望=${JSON.stringify(expected)}`).toEqual(expected);
}

function requireId(obj: Record<string, unknown>, label: string): number {
  const id = Number(obj.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：无有效 id，实际键=${Object.keys(obj).join(',')}`);
  }
  return id;
}

/** API 自建唯一密钥（真实取数入口，与 UI 同一登录态），并登记 afterEach 清理 */
async function createKeyViaApi(
  page: Page,
  overrides: Record<string, unknown>
): Promise<Record<string, unknown>> {
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/api-gateway/keys', {
    key_name: genCode('E2E24KEY'),
    ...overrides,
  });
  const id = requireId(created, '建密钥');
  CLEANUP.push({ path: `/api-gateway/keys/${id}`, label: 'api_key' });
  return created;
}

async function readKey(page: Page, id: number): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `/api-gateway/keys/${id}`);
}

/** 进入 API 网关密钥 Tab 并按密钥名称检索（分页默认页大小下不依赖顺序） */
async function openKeysTabAndSearch(page: Page, keyName: string) {
  await page.goto('/api-gateway', { waitUntil: 'domcontentloaded' });
  await page.getByRole('tab', { name: TXT.tabKeys }).click();
  // 三个 tab 面板同页共存（非 active 的被隐藏），必须把定位器收敛到密钥面板，
  // 否则「搜索」按钮会命中多个（Endpoint/Log 面板也各有一个）触发 strict mode 报错
  const panel = page
    .locator('[role="tabpanel"]')
    .filter({ has: page.getByPlaceholder(TXT.searchPlaceholder) })
    .first();
  await panel.getByPlaceholder(TXT.searchPlaceholder).fill(keyName);
  await panel.getByRole('button', { name: TXT.search }).click();
  const row = panel.getByRole('row').filter({ hasText: keyName });
  await expect(row, `列表应出现自建密钥 ${keyName}`).toHaveCount(1);
  return { panel, row };
}

async function openEditDialog(page: Page, row: Locator): Promise<Locator> {
  await row.getByRole('button', { name: TXT.edit }).click();
  const dialog = page.getByRole('dialog', { name: TXT.editTitle });
  await expect(dialog).toBeVisible();
  return dialog;
}

test.describe('24 API 网关密钥：回显真值 + 三态清空 + 非法有效期拒绝', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('bingxi.locale', 'zh-CN');
    });
    await loginViaUI(page);
  });

  test('24-01 自建密钥→编辑清空描述→保存→重开编辑对话框断回显（缺陷①②⑤）', async ({ page }) => {
    const description = 'E2E24 对账只读密钥描述';
    const created = await createKeyViaApi(page, {
      description,
      expires_at: '2026-12-31T23:59:59Z',
    });
    const id = requireId(created, '建密钥');
    const keyName = String(created.key_name);

    // 出参真值（修复前 description/created_by_name 恒为 ""）
    expectKeyValue(created, 'description', description, '创建响应');
    expectKeyValue(created, 'created_by_name', TEST_USERNAME, '创建响应（LEFT JOIN users 真值）');
    expect(
      typeof created.expires_at === 'string' && created.expires_at.length > 0,
      `创建响应 expires_at 应为 ISO 字符串，实际=${JSON.stringify(created.expires_at)}`
    ).toBe(true);
    const expiryWire = String(created.expires_at);

    const detail = await readKey(page, id);
    expectKeyValue(detail, 'description', description, '详情回读');
    expectKeyValue(detail, 'created_by_name', TEST_USERNAME, '详情回读');

    // 列表列真值渲染（缺陷②的可见面：创建人列不再是空白）
    const { row } = await openKeysTabAndSearch(page, keyName);
    await expect(row.getByText(description, { exact: false })).toBeVisible();
    await expect(row.getByText(TEST_USERNAME, { exact: true })).toBeVisible();
    await expect(row.getByText('2026-12-31', { exact: false })).toBeVisible();

    // 编辑对话框回显原值（缺陷①的正解面）
    const dialog = await openEditDialog(page, row);
    const descTextarea = dialog.locator('textarea');
    await expect(descTextarea, '对话框必须回显后端真实描述').toHaveValue(description);
    await expect(dialog.getByRole('textbox', { name: TXT.expiresAtField })).not.toHaveValue('');

    // 清空描述 → 前端送显式 null（三态之清空态）→ 后端落 NULL
    await descTextarea.fill('');
    await dialog.getByRole('button', { name: TXT.confirm }).click();
    await expect(
      page.locator('.el-message--success', { hasText: TXT.operationSuccess })
    ).toBeVisible();
    await expect(dialog).toBeHidden();

    // 重开编辑对话框断回显：清空后确实为空（不是旧的空串假值，也不是残留原值）
    await openKeysTabAndSearch(page, keyName);
    const dialog2 = await openEditDialog(page, row);
    await expect(dialog2.locator('textarea'), '清空后重开对话框应回显空').toHaveValue('');
    await expect(
      dialog2.getByRole('textbox', { name: TXT.expiresAtField }),
      '只清描述不动有效期：过期时间仍须回显原值'
    ).not.toHaveValue('');
    await dialog2.getByRole('button', { name: TXT.cancel }).click();
    await expect(dialog2).toBeHidden();

    // 回查后端状态字面量：description 显式 null 键，expires_at 原值一字未改（三态之保持）
    const after = await readKey(page, id);
    expectKeyValue(after, 'description', null, '清空描述后详情');
    expectKeyValue(after, 'expires_at', expiryWire, '清空描述后详情（未提交即保持原值）');
    expectKeyValue(after, 'status', 'active', '清空描述后状态');
  });

  test('24-02 三态与非法有效期：键缺席保持→非法 400 且原值不动→显式 null 永不过期', async ({
    page,
  }) => {
    const description = 'E2E24 三态基线描述';
    const created = await createKeyViaApi(page, {
      description,
      expires_at: '2026-12-31T23:59:59Z',
    });
    const id = requireId(created, '三态基线密钥');
    const expiryWire = String(created.expires_at);
    const keyName = String(created.key_name);

    // 态一：键缺席=保持原值（前端「启用/停用」只送 status）
    await apiCall(page, 'PUT', `/api-gateway/keys/${id}`, { status: 'inactive' });
    const absentKept = await readKey(page, id);
    expectKeyValue(absentKept, 'description', description, '键缺席后详情');
    expectKeyValue(absentKept, 'expires_at', expiryWire, '键缺席后详情');
    expectKeyValue(absentKept, 'status', 'inactive', '键缺席后状态（status 本身被覆盖）');

    // 态二：有值但格式非法 → 400 VALIDATION_ERROR + 真实文案，且原 expires_at 未被改动
    //（缺陷④：旧实现把解析失败落成 NULL=永不过期，等于悄悄撤销有效期）
    const failed = await apiCallExpectFail(page, 'PUT', `/api-gateway/keys/${id}`, {
      expires_at: '2026-13-45',
    });
    expect(failed.status, `非法 expires_at 必须 400，实际体=${JSON.stringify(failed)}`).toBe(400);
    expect(
      failureCode(failed),
      `非法 expires_at 机器码应为 VALIDATION_ERROR，实际=${JSON.stringify(failed)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(failed.message, '文案必须原样外显，不得被脱敏').toBe(EXPIRES_AT_FORMAT_MSG);
    const afterInvalid = await readKey(page, id);
    expectKeyValue(afterInvalid, 'expires_at', expiryWire, '非法请求后原 expires_at 必须未被改动');
    expectKeyValue(afterInvalid, 'description', description, '非法请求不得顺带改描述');

    // 态三：显式 null=清空（永不过期），且密钥仍是启用态
    await apiCall(page, 'PUT', `/api-gateway/keys/${id}`, { status: 'active', expires_at: null });
    const cleared = await readKey(page, id);
    expectKeyValue(cleared, 'expires_at', null, '显式 null 清空后详情（NULL→JSON null）');
    expectKeyValue(cleared, 'status', 'active', '清空有效期不得顺手停用密钥');

    // 列表对 expires_at === null 显式渲染「永不过期」（缺陷③的可见面，不再是空白）
    const { row } = await openKeysTabAndSearch(page, keyName);
    await expect(row.getByText(TXT.neverExpires, { exact: true })).toBeVisible();
  });
});
