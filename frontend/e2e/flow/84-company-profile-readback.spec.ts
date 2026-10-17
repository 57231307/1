import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  API_BASE,
  API_PREFIX,
  APP_ERROR_CODES,
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
} from './helpers';

/**
 * 84 公司档案服务端化：读回持久 + 敏感列不泄露（保存后读回持久化 / 列锁定家族）
 *
 * 后端契约（真实实现，e2e 按出参键与信封断言）：
 * - GET  /company-profile       登录即可读的公开投影，data 键恰为 {name, short_name, logo}，
 *                               由投影结构体在类型层只含这三列，敏感工商/税务/银行列不经此路出入。
 * - GET  /company-profile/full  权限键 company-profile:read（仅系统超管种子），返回整行 Model 的
 *                               snake_case 键；registered_capital 序列化为字符串、
 *                               establishment_date 为 YYYY-MM-DD 或 null、updated_by 为数字。
 * - PUT  /company-profile/full  权限键 company-profile:update（仅系统超管种子），全量替换语义：
 *                               未提交的可选列清空为 null；name trim 后非空否则 400；logo 非空时
 *                               须为合法 base64、解码后以 PNG 魔数开头且 ≤512KB，否则 400。
 *
 * 断言均为值判据（读回原样、键集合严格相等、负向体不含哨兵值），不用 `if visible` 分支，
 * 也不得只断状态码。负例断 status=400 且 code=VALIDATION_ERROR（真实信封，非猜文案）。
 *
 * 单例共享态安全：company_profile 是全库单例，任何成功 PUT 都会改全局。写档案的用例在 try/finally
 * 里先快照再还原，且整个 describe 用 serial 模式串行，避免同库并发读写互相看到对方的中间态。
 * 重要约束（不可假绿，故在此点名）：迁移播种基线是空名行，而后端 PUT 拒绝空名，故空名基线经
 * API 不可达——快照为空名时还原只能退化为中性占位名 + 清空其余列（清掉本规格写入的唯一哨兵），
 * 无法逐字节回到空名。残差交给编排方：若将来出现读取默认品牌的其它规格，需后端提供重置口或测后清库。
 */

type Row = Record<string, unknown>;

// 1x1 合法 PNG 的 base64 裸串（解码首字节即 PNG 魔数 89 50 4E 47），作为 logo 提交值。
const VALID_PNG_B64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=';
// 反例：纯文本 base64，解码为 "hello"（首字节 0x68，非 PNG 魔数），应触发写入口 400。
const NON_PNG_B64 = 'aGVsbG8=';

// 空名基线不可达时的中性占位名（仅用于清掉本规格写入的唯一哨兵列，非断言目标）。
const PLACEHOLDER_NAME = 'E2E-PLACEHOLDER-COMPANY';

// PUT /company-profile/full 的可编辑子集（不含 id / updated_at / updated_by）；全量替换下必须
// 逐列提交才能把快照原样还原，漏列会被后端清成 null。
const EDITABLE_KEYS = [
  'name',
  'short_name',
  'credit_code',
  'legal_representative',
  'tax_registration_number',
  'bank_name',
  'bank_account',
  'taxpayer_type',
  'phone',
  'fax',
  'email',
  'website',
  'address',
  'registered_capital',
  'establishment_date',
  'logo',
  'description',
] as const;

function uniq(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

/** 由快照整行取可编辑子集，逐列透传（值已是序列化形态，可直接回传给 PUT）。 */
function editableFrom(snap: Row): Record<string, unknown> {
  const body: Record<string, unknown> = {};
  for (const k of EDITABLE_KEYS) body[k] = snap[k] ?? null;
  return body;
}

/** 中性清空体：占位名 + 其余可编辑列全 null，用于无法回到空名基线时清掉唯一哨兵列。 */
function neutralClearBody(): Record<string, unknown> {
  const body = editableFrom({});
  body.name = PLACEHOLDER_NAME;
  return body;
}

/**
 * 还原快照：快照名非空 → 逐列原样 PUT 回；快照为空名基线 → PUT 不可达空名，退化为中性清空体。
 * 还原是 housekeeping，放在断言之后；若还原本身失败，大声打日志暴露真实原因，绝不静默。
 */
async function restoreProfile(page: Page, snap: Row): Promise<void> {
  const body = String(snap.name ?? '').trim() ? editableFrom(snap) : neutralClearBody();
  try {
    await apiCall(page, 'PUT', '/company-profile/full', body);
  } catch (e) {
    console.error(`[84][restore] 公司档案还原失败: ${(e as Error).message}`);
    throw e;
  }
}

test.describe.serial('84 公司档案读回持久 + 敏感列不泄露', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('写非敏感可编辑列后 GET full 原样读回持久', async ({ page }) => {
    const stamp = uniq();
    const name = `E2E-CO-${stamp}`;
    const shortName = `E2E-SHORT-${stamp}`;
    const phone = `010-${stamp}`;
    const email = `e2e-${stamp}@example.test`;
    const website = `https://e2e-${stamp}.example.test`;
    const address = `E2E Address ${stamp}`;
    const establishmentDate = '2016-07-15';
    const capital = 8888888;

    const snap = await apiCallRaw<Row>(page, 'GET', '/company-profile/full');
    try {
      await apiCallRaw(page, 'PUT', '/company-profile/full', {
        name,
        short_name: shortName,
        phone,
        email,
        website,
        address,
        registered_capital: capital,
        establishment_date: establishmentDate,
      });

      const full = await apiCallRaw<Row>(page, 'GET', '/company-profile/full');
      expect(full.name, `name 读回应为刚写入值，实际：${String(full.name)}`).toBe(name);
      expect(full.short_name).toBe(shortName);
      expect(full.phone).toBe(phone);
      expect(full.email).toBe(email);
      expect(full.website).toBe(website);
      expect(full.address).toBe(address);
      expect(full.establishment_date, `成立日期应为 YYYY-MM-DD，实际：${String(full.establishment_date)}`).toBe(
        establishmentDate
      );
      // 注册资本序列化为字符串（契约）：类型判 string、数值判相等，不锁死小数位以免脆断。
      expect(typeof full.registered_capital, `注册资本应序列化为字符串，实际类型：${typeof full.registered_capital}`).toBe(
        'string'
      );
      expect(Number(full.registered_capital)).toBe(capital);
    } finally {
      await restoreProfile(page, snap);
    }
  });

  test('公开投影键集合恰为 name/short_name/logo 且不泄露刚写入的敏感值', async ({ page }) => {
    const stamp = uniq();
    const name = `E2E-LEAK-${stamp}`;
    const creditCode = `CC${stamp}SENTINEL`;
    const bankAccount = `BA${stamp}SENTINEL`;

    const snap = await apiCallRaw<Row>(page, 'GET', '/company-profile/full');
    try {
      await apiCallRaw(page, 'PUT', '/company-profile/full', {
        name,
        credit_code: creditCode,
        bank_account: bankAccount,
      });

      // 前提正锁：敏感值确实写进整行（否则公开投影不含它，会因未写入而假通过）。
      const full = await apiCallRaw<Row>(page, 'GET', '/company-profile/full');
      expect(full.credit_code).toBe(creditCode);
      expect(full.bank_account).toBe(bankAccount);

      // 公开投影：键集合严格相等（排序后），多一个/少一个敏感列都会红。
      const pub = await apiCall<{ name: string; short_name: string | null; logo: string | null }>(
        page,
        'GET',
        '/company-profile'
      );
      const keys = Object.keys(pub.data).sort();
      expect(keys, `公开投影键集合应恰为 name/short_name/logo，实际：${keys.join(',')}`).toEqual([
        'logo',
        'name',
        'short_name',
      ]);

      // 负向锁：原始响应体文本不得出现刚设的敏感值（GET 无需 CSRF，沿用登录会话 cookie）。
      const raw = await page.request.fetch(`${API_BASE}${API_PREFIX}/company-profile`, {
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      });
      const bodyText = await raw.text();
      expect(bodyText, `公开投影响应体不应含 credit_code 哨兵值`).not.toContain(creditCode);
      expect(bodyText, `公开投影响应体不应含 bank_account 哨兵值`).not.toContain(bankAccount);
    } finally {
      await restoreProfile(page, snap);
    }
  });

  test('合法 PNG logo 写入后全量面与公开投影都读回同值', async ({ page }) => {
    const name = `E2E-LOGO-${uniq()}`;

    const snap = await apiCallRaw<Row>(page, 'GET', '/company-profile/full');
    try {
      await apiCallRaw(page, 'PUT', '/company-profile/full', { name, logo: VALID_PNG_B64 });

      const full = await apiCallRaw<Row>(page, 'GET', '/company-profile/full');
      expect(full.logo, 'logo 应原样读回').toBe(VALID_PNG_B64);

      const pub = await apiCall<{ logo: string | null }>(page, 'GET', '/company-profile');
      expect(pub.data.logo, '公开投影也应带 logo 且等于提交值').toBe(VALID_PNG_B64);
    } finally {
      await restoreProfile(page, snap);
    }
  });

  test('非 PNG 魔数的 logo base64 被拒 400', async ({ page }) => {
    // 该请求应在校验层被拒、不写库，故不需快照/还原（写失败不改变单例）。
    const fail = await apiCallExpectFail(page, 'PUT', '/company-profile/full', {
      name: `E2E-BADLOGO-${uniq()}`,
      logo: NON_PNG_B64,
    });
    expect(fail.status, `非 PNG 魔数应被拒 400，实际 status=${fail.status} code=${fail.code}`).toBe(400);
    expect(fail.code, `拒绝码应为 VALIDATION_ERROR，实际：${fail.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });

  test('空白 name 被拒 400', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'PUT', '/company-profile/full', {
      name: '   ',
      short_name: null,
    });
    expect(fail.status, `空白 name 应被拒 400，实际 status=${fail.status} code=${fail.code}`).toBe(400);
    expect(fail.code, `拒绝码应为 VALIDATION_ERROR，实际：${fail.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });
});
