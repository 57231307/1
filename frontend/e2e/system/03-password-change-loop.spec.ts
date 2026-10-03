// 系统管理 E2E 套件 — 03 用户改密闭环（改密 → 旧密码不可登录 → 新密码可登录）
//
// 覆盖真实功能面（逐端点对后端 routes 源码核实，均为真实注册端点）：
//   POST   /users                  routes/iam.rs:21 user_handler::create_user
//        —— admin 会话（require_admin_role, user_handler.rs:300）；
//           CreateUserRequest{username 3-50, password 强度策略, role_id?}（:75-87）；
//           响应 UserResponse{id,...}（:351 ApiResponse::success(user.into())）。
//   POST   /auth/login             routes/auth.rs:18 auth_handler::login
//        —— CSRF 豁免（middleware/csrf.rs:171 is_public_path 跳过校验）；
//           Set-Cookie 下发 access_token/refresh_token/csrf_token（httpOnly 会话链路，
//           flow/setup-wizard/00-setup-wizard.spec.ts:229-245 已实证同形状）。
//   POST   /users/change-password  routes/iam.rs:27-30 user_handler::change_password
//        —— 需认证会话 + 消费 CSRF；ChangePasswordRequest{old_password,new_password}
//           （user_handler.rs:686-691）；原密码错 → 401 UNAUTHORIZED（:719-723），
//           新旧相同 → 400 BAD_REQUEST（:730-733），密码含用户名片段 → 400（:734-736，
//           password_policy_service.rs:166-173 contains_username_fragment=小写子串包含）；
//           成功 → data={success:true}，且 update_password_and_revoke 吊销旧会话（:755）。
//   DELETE /users/{id}             routes/iam.rs:26 delete_user —— 清理载体（best-effort）。
//
// 密码策略真相（services/auth/password_policy_service.rs:156-163 is_common_password：
// 小写子串黑名单 admin/root/password/123456/qwerty…；强度需含大小写+数字+特殊字符，
// 参照 utils/password_validator.rs 与 system/01-users.spec.ts:37-44 已验证的构造法）：
// 本用例 P0/P1 均为随机后缀强密码，且必然不含用户名（用户名 e2e_pwd_*，密码 Xk9#/Zr7$ 模板）。
//
// 假绿防线：
// - 改密调用**不走 apiCall**：apiCall 在 CSRF 竞败时经 refreshCsrfToken 静默重登为
//   TEST_USERNAME 管理员（helpers.ts:1257-1263），会把「以新用户身份改密」偷换成
//   「管理员给自己改密」的假绿。此处用裸 page.request 携带当前会话 CSRF 直调，
//   CSRF cookie 缺失即前置判红，失败绝不换会话兜底。
// - 旧密码不可登录：断真实 HTTP 语义 401 + 机器码 UNAUTHORIZED，不是断 toast 文案。
// - 新密码可登录：断 200 + 信封 code=200 + data.user.username 等于本用例自建用户名
//   （防「登录了共享 admin 账号」冒充闭环成功）。
// - 用例自建自流转：一次性用户 + 一次性密码，finally 恢复管理员会话并清理用户。
import { test, expect, type Page } from '@playwright/test';
import {
  apiCall,
  ensureTestEntities,
  failureCode,
  genCode,
  loginViaUI,
  tryCleanup,
  API_BASE,
  API_PREFIX,
  type ApiFailureBody,
} from '../flow/helpers';

async function rawLogin(page: Page, username: string, password: string) {
  return page.request.post(`${API_BASE}${API_PREFIX}/auth/login`, {
    data: { username, password },
    headers: { 'Content-Type': 'application/json', 'X-Requested-With': 'XMLHttpRequest' },
  });
}

test.describe('系统管理 - 03 用户改密闭环', () => {
  test('改密成功 → 旧密码登录 401 → 新密码登录 200（真实端点全链路）', async ({ page }) => {
    // ---- 前置：admin 真实登录（建用户需要 admin 会话）----
    await loginViaUI(page);
    await ensureTestEntities(page);

    const username = `e2e_pwd_${genCode('U')}`.toLowerCase();
    const suffix = Math.floor(Math.random() * 1e6)
      .toString()
      .padStart(6, '0');
    // 强度模板对齐 system/01 已验证可过策略的构造：大写+小写+数字+特殊字符、长度≥8、
    // 不含常见弱密码子串、不含用户名
    const pwOld = `Xk9#oLd${suffix}vQ2$`;
    const pwNew = `Zr7$eWn${suffix}mP4!`;
    expect(pwOld.toLowerCase().includes(username), '前置：P0 不得含用户名').toBe(false);
    expect(pwNew.toLowerCase().includes(username), '前置：P1 不得含用户名').toBe(false);

    // 一次性用户**故意不带角色**建：#4671 本用例的真红是 `POST /users/change-password`
    // 返回 403 `FORBIDDEN`「没有关联角色，无法访问」（xr35/backend.log:38084，认证成功
    // user_id=42 之后被 permission.rs 的 extract_role_id 拦死）。改密只操作调用者自身凭据，
    // 属自助端点，不得要求任何业务角色/权限码；把"无角色"从偶然前提改为显式前提并回读校验，
    // 本用例才成为该豁免（middleware/public_routes.rs AUTH_ONLY_PATHS + permission.rs
    // 判定顺序前移）的活体锁——将来 setup 若给账号自动塞角色，用例会绿而回归检不出来。
    // 用户名同样保持分片/进程唯一（genCode 范式），不复用任何种子账号名。
    const created = await apiCall<{ id?: number; username?: string; role_id?: number | null }>(
      page,
      'POST',
      '/users',
      {
        username,
        password: pwOld,
      }
    );
    const userId = created.data?.id;
    expect(
      userId,
      `创建一次性用户应返回 id（POST /users 真实响应）：${JSON.stringify(created)}`
    ).toBeTruthy();
    expect(created.data?.username, '创建响应应回显用户名').toBe(username);
    expect(
      created.data?.role_id ?? null,
      `[system/03] 前置：一次性用户必须无角色（role_id=NULL），实际 ${JSON.stringify(created.data)}——带角色即本用例失去对"自助端点被 RBAC 拦死"回归的检出力`
    ).toBeNull();

    try {
      // ---- 1. 新用户以旧密码建立真实会话（login 免 CSRF，Set-Cookie 入本 context）----
      const login0 = await rawLogin(page, username, pwOld);
      expect(login0.status(), `前置：新用户旧密码应可登录，body=${await login0.text()}`).toBe(200);
      const login0Body = await login0.json();
      expect(login0Body.code, '登录成功信封 code 应为 200').toBe(200);
      expect(login0Body.data?.user?.username, '会话应属于自建用户').toBe(username);
      const csrfCookies = await page.context().cookies();
      const csrf = csrfCookies.find(c => c.name === 'csrf_token')?.value;
      expect(csrf, '登录后 csrf_token cookie 应下发（改密请求的 CSRF 前置）').toBeTruthy();

      // ---- 2. 会话内改密（裸调用，禁走 apiCall 的 admin 兜底重登）----
      const changeResp = await page.request.post(`${API_BASE}${API_PREFIX}/users/change-password`, {
        data: { old_password: pwOld, new_password: pwNew },
        headers: {
          'Content-Type': 'application/json',
          'X-Requested-With': 'XMLHttpRequest',
          'X-CSRF-Token': csrf as string,
        },
      });
      const changeText = await changeResp.text();
      expect(changeResp.status(), `改密应 200，实际 body=${changeText}`).toBe(200);
      const changeBody = JSON.parse(changeText) as {
        code?: number;
        data?: { success?: boolean };
      };
      expect(changeBody.code, `改密信封 code 应为 200：${changeText}`).toBe(200);
      expect(
        changeBody.data?.success,
        `改密应返回 data.success=true（ChangePasswordResponse 真相 user_handler.rs:696-699）：${changeText}`
      ).toBe(true);

      // ---- 3. 旧密码不可登录：401 + UNAUTHORIZED ----
      const oldAgain = await rawLogin(page, username, pwOld);
      expect(oldAgain.status(), '改密后旧密码不应再能登录').toBe(401);
      const oldBody = (await oldAgain.json().catch(() => null)) as ApiFailureBody | null;
      expect(failureCode(oldBody), '旧密码登录失败机器码应为 UNAUTHORIZED').toBe('UNAUTHORIZED');

      // ---- 4. 新密码可登录：200 且身份为自建用户 ----
      const newLogin = await rawLogin(page, username, pwNew);
      const newText = await newLogin.text();
      expect(newLogin.status(), `新密码应可登录，body=${newText}`).toBe(200);
      const newBody = JSON.parse(newText) as {
        code?: number;
        data?: { user?: { username?: string } };
      };
      expect(newBody.code, '新密码登录信封 code 应为 200').toBe(200);
      expect(newBody.data?.user?.username, '新密码登录应命中自建用户本身').toBe(username);
    } finally {
      // 当前 context 会话属于自建用户；清理需 admin → force 重登恢复，再删一次性用户。
      await loginViaUI(page, undefined, undefined, true);
      await tryCleanup(page, 'DELETE', `/users/${userId}`, '[system/03] one-time user');
    }
  });
});
