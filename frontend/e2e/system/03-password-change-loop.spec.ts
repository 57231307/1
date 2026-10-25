// 系统管理 E2E 套件 03 用户改密闭环，覆盖 改密、错误旧密码被拒、改后旧密码失效、新密码可登录
//
// 覆盖真实功能面，逐端点对后端 routes 源码核实，均为真实注册端点
//   users 创建端点（POST），admin 会话专用（require_admin_role），CreateUserRequest 含
//   username 长度 3 到 50、password 走强度策略校验、role_id 可选，响应回显 id 与 username。
//   auth 登录端点（POST），CSRF 豁免（中间件 is_public_path 跳过校验），
//   Set-Cookie 下发 access_token、refresh_token、csrf_token（httpOnly 会话链路，
//   同仓 setup 向导用例已实证同形状）。
//   users change-password 端点（POST），需认证会话并消费 CSRF，ChangePasswordRequest 含
//   old_password 与 new_password，原密码不匹配 401 且机器码 UNAUTHORIZED，
//   新旧相同 400，新密码含用户名片段 400（contains_username_fragment 为小写子串包含判定），
//   成功时 data 内 success 为 true，且 update_password_and_revoke 吊销旧会话。
//   users 删除端点（DELETE 按 id），清理载体（best-effort）。
//
// 密码策略真相（password_policy_service 的 is_common_password 为小写子串黑名单，收录
// admin、root、password、123456、qwerty 等，强度判据复用 utils 的 password_validator，
// 需大小写、数字、特殊字符齐备）。本用例 P0 与 P1 是确定性模板强密码，不拼随机后缀，
// 因此不存在随机命中键盘序列或常见弱密码子串的可能，且必然不含用户名（用户名以 e2e_pwd 开头）。
// 造数密码处附有专门的构造说明，本文件另有负向用例锁住键盘序列规则真的在生效。
//
// 假绿防线
// - 改密调用不走 apiCall，因为 apiCall 在 CSRF 竞败时经 refreshCsrfToken 静默重登为
//   TEST_USERNAME 管理员，会把 以新用户身份改密 偷换成 管理员给自己改密 的假绿。
//   此处用裸请求携带当前会话 CSRF 直调，CSRF cookie 缺失即前置判红，失败绝不换会话兜底。
// - 错误旧密码探针，正式改密前先用不匹配的原密码真实提交一次，断恰 401 且机器码
//   UNAUTHORIZED，钉住 服务端确实逐次核验原密码，而不是只看改密返回 200。
//   该校验通过后 CSRF token 已被消费并轮换，探针后重新读取最新 token 再执行真实改密。
// - 旧密码不可登录，断真实 HTTP 语义 401 且机器码 UNAUTHORIZED，不是断 toast 文案。
// - 新密码可登录，断 200 且信封 code 为 200 且 data 内 user 的 username 等于本用例
//   自建用户名（防 登录了共享 admin 账号 冒充闭环成功）。
// - 用例自建自流转，一次性用户加一次性密码，finally 恢复管理员会话并清理用户，
//   两条用例的用户名都由 genCode 生成、各自独立，不复用任何种子账号或固定密码夹具，
//   多 worker 并发各建各号，夹具可重入互不踩踏。
// - 负向锁，同文件另有一条用例故意用含键盘序列的密码建用户，断恰 400、业务机器码
//   VALIDATION_ERROR、失败文案非空且明确不是 5xx。没有这条负向锁，正例的 "密码必定合规"
//   只剩前端自我声明，服务端哪天把键盘序列校验悄悄关掉，正例仍会一路判绿。
import { test, expect, type Page } from '../diagnose-fixture';
import {
  apiCall,
  apiCallExpectFail,
  ensureTestEntities,
  failureCode,
  genCode,
  loginViaUI,
  tryCleanup,
  API_BASE,
  API_PREFIX,
  APP_ERROR_CODES,
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
    // ---- 前置 admin 真实登录，建用户需要 admin 会话 ----
    await loginViaUI(page);
    await ensureTestEntities(page);

    const username = `e2e_pwd_${genCode('U')}`.toLowerCase();
    // 造数密码用确定性模板，不再拼随机数字后缀，随机重试与放宽服务端校验都不做
    // 服务端键盘序列检测取连续 4 字符窗口，比对表里每个窗口都是单一字符类别，
    // 即字母行窗口全是字母、数字行窗口全是数字、符号行窗口全是符号，正序逆序都同样比对，
    // 随机 6 位数字后缀约有千分之四的概率命中 4 连号而把整条用例变成随机红
    // 本模板按 两字母一数字一符号 循环排布，任一连续 4 字符窗口都同时含字母与非字母字符，
    // 不可能与任一单类窗口相等，故必定不命中键盘序列，
    // 其余判据同时满足，大写小写数字符号四类齐备、长度 16 不短于最小 8、
    // 无 3 字符重复、无 3 字符递增或递减、不含用户名片段、不命中常见弱密码子串表
    const pwOld = 'Bk9#Vn4$Wm7!Qx2&';
    const pwNew = 'Tz6#Hd3$Rj8!Yp5&';
    expect(pwOld.toLowerCase().includes(username), '前置：P0 不得含用户名').toBe(false);
    expect(pwNew.toLowerCase().includes(username), '前置：P1 不得含用户名').toBe(false);

    // 一次性用户故意不带角色建，本用例的真红是 users 改密端点返回 403 FORBIDDEN
    // "没有关联角色，无法访问"，即认证成功之后被权限中间件的 extract_role_id 环节拦死。
    // 改密只操作调用者自身凭据，属自助端点，不得要求任何业务角色或权限码。
    // 把 无角色 从偶然前提改为显式前提并回读校验，本用例才成为该豁免
    // （自助端点白名单加权限判定顺序前移）的活体锁，将来 setup 若给账号自动塞角色，
    // 用例会绿而回归检不出来。用户名同样保持分片与进程唯一（genCode 范式），
    // 不复用任何种子账号名。
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
      // ---- 第 1 步 新用户以旧密码建立真实会话（login 免 CSRF，Set-Cookie 入本 context）----
      const login0 = await rawLogin(page, username, pwOld);
      expect(login0.status(), `前置：新用户旧密码应可登录，body=${await login0.text()}`).toBe(200);
      const login0Body = await login0.json();
      expect(login0Body.code, '登录成功信封 code 应为 200').toBe(200);
      expect(login0Body.data?.user?.username, '会话应属于自建用户').toBe(username);
      const csrfCookies = await page.context().cookies();
      const csrf = csrfCookies.find(c => c.name === 'csrf_token')?.value;
      expect(csrf, '登录后 csrf_token cookie 应下发（改密请求的 CSRF 前置）').toBeTruthy();

      // 错误旧密码探针，钉住 服务端确实逐次核验原密码，而不是只看改密入口返回 200
      const wrongOldResp = await page.request.post(
        `${API_BASE}${API_PREFIX}/users/change-password`,
        {
          data: { old_password: `${pwOld}X7!`, new_password: pwNew },
          headers: {
            'Content-Type': 'application/json',
            'X-Requested-With': 'XMLHttpRequest',
            'X-CSRF-Token': csrf as string,
          },
        }
      );
      const wrongOldStatus = wrongOldResp.status();
      const wrongOldBody = (await wrongOldResp.json().catch(() => null)) as ApiFailureBody | null;
      expect(
        wrongOldStatus,
        `错误旧密码改密应恰 401，实际 status=${wrongOldStatus} 响应=${JSON.stringify(wrongOldBody)}`
      ).toBe(401);
      expect(
        failureCode(wrongOldBody),
        `错误旧密码改密机器码应为 UNAUTHORIZED，实际响应=${JSON.stringify(wrongOldBody)}`
      ).toBe('UNAUTHORIZED');

      // 探针这次 CSRF 校验通过后旧 token 已被消费并轮换，真实改密前重新读取最新值
      const csrfFresh = (await page.context().cookies()).find(c => c.name === 'csrf_token')?.value;
      expect(csrfFresh, '探针后 csrf_token 应已轮换重新下发').toBeTruthy();

      // ---- 第 2 步 会话内改密（裸调用，禁走 apiCall 的 admin 兜底重登）----
      const changeResp = await page.request.post(`${API_BASE}${API_PREFIX}/users/change-password`, {
        data: { old_password: pwOld, new_password: pwNew },
        headers: {
          'Content-Type': 'application/json',
          'X-Requested-With': 'XMLHttpRequest',
          'X-CSRF-Token': csrfFresh as string,
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

      // ---- 第 3 步 旧密码不可登录，401 且 UNAUTHORIZED ----
      const oldAgain = await rawLogin(page, username, pwOld);
      expect(oldAgain.status(), '改密后旧密码不应再能登录').toBe(401);
      const oldBody = (await oldAgain.json().catch(() => null)) as ApiFailureBody | null;
      expect(failureCode(oldBody), '旧密码登录失败机器码应为 UNAUTHORIZED').toBe('UNAUTHORIZED');

      // ---- 第 4 步 新密码可登录，200 且身份为自建用户 ----
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
      // 当前 context 会话属于自建用户，清理需 admin，force 重登恢复后再删一次性用户。
      await loginViaUI(page, undefined, undefined, true);
      await tryCleanup(page, 'DELETE', `/users/${userId}`, '[system/03] one-time user');
    }
  });

  test('负向锁：含键盘序列的密码建用户应恰 400 且业务码与文案齐全', async ({ page }) => {
    // 本用例把 造数密码必定合规 这一前提钉回真实服务端行为，键盘序列规则一旦被关掉就判红。
    // 构造只在第 6 到第 9 个字符处放一条 4 连字母行序列，四类字符齐备、长度 15、
    // 无重复无递增递减、不含用户名片段、不命中常见弱密码子串表与精确黑名单，
    // 即除键盘序列外其余判据全部满足，所以被拒的原因只可能是键盘序列这一条。
    await loginViaUI(page);
    const username = `e2e_pwd_${genCode('U')}`.toLowerCase();
    const rejectedPassword = 'Kd3!Zqwer7#Vn9$';
    expect(rejectedPassword.toLowerCase().includes(username), '前置：负向密码不得含用户名').toBe(
      false
    );

    const res = await apiCallExpectFail(page, 'POST', '/users', {
      username,
      password: rejectedPassword,
    });
    // 恰 400，既不许退化成 5xx，也不许被降级成 401 或 403 之类的会话面失败
    expect(
      res.status,
      `含键盘序列的密码建用户应恰 400，实际 status=${res.status} 响应=${JSON.stringify(res)}`
    ).toBe(400);
    expect(res.status >= 500, `服务端不得把密码校验拒绝上抛成 5xx，实际 status=${res.status}`).toBe(
      false
    );
    expect(
      failureCode(res),
      `业务机器码应为 VALIDATION_ERROR，实际响应=${JSON.stringify(res)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      typeof res.message === 'string' && res.message.length > 0,
      `失败文案不得为空，实际响应=${JSON.stringify(res)}`
    ).toBe(true);
    // 恰 400 意味着用户未被创建，因此本用例不需要清理载体，若哪天变成 200 上面第一条先红
  });
});
