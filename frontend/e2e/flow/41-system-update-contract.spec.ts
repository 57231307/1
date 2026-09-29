// 系统更新「契约级」e2e —— 锁住本轮功能改造不回退（任务 #122）
//
// 背景（已改源码、须被 e2e 真实锁住）：
//   1) GET /system-update/tasks、GET /system-update/backups 原「错绑」返回单对象 / Vec<String>，
//      现改为「真实列表」，返回标准分页信封 ApiResponse{ data: PaginatedResponse{ items,total,page,page_size } }
//      （backend/src/handlers/system_update_handler.rs 的 list_update_tasks / list_backup_tasks）。
//   2) GET /system-update/current-version → get_version 返回 { version, release_date, changelog }；
//      前端契约已由 build_date 对齐为 release_date（commit e3bcedf4）。
//   3) GET /system-update/versions 列表端点已删除（版本 tab 移除，前端不再渲染、不再调用）。
//
// 「假阳性防线」（吸取教训：e2e 全绿 ≠ 功能真实）：
//   - 分页信封断言用「data 不是裸数组 / 不是裸单对象，且含 items 数组键 + total/page/page_size」这一
//     结构不变式，空表（items=[] total=0）也成立，能锁住「不回退成 Vec<String>（裸数组）或单对象」。
//   - 非空时进一步断首个元素是对象且含各自契约键（tasks: task_code/status；backups: backup_code），
//     彻底锁死「对象数组」而非「字符串数组」。
//   - versions 已删：以真实 HTTP 状态断言其不再返回 2xx 列表语义（期望 404，路由未注册），
//     并在页面级断言 /system-update 只剩 2 个 tab（版本 tab 消失，结构锁不依赖 i18n 文案）。
//
// 断言全部基于真实响应结构，无放宽、无 mock。诚实：空表就断空，非空就断对象契约。
import { test, expect } from '../diagnose-fixture';
import { API_BASE, API_PREFIX, BASE_URL, loginViaUI, apiCallRaw } from './helpers';

/** 后端分页信封（utils/response.rs 的 PaginatedResponse 序列化形状，键名与 serde 输出一致） */
interface PaginatedEnvelope<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

test.describe('系统更新 列表端点正名 + 版本契约（真实 API 级）', () => {
  // 复用真实 admin 会话（与 40-system-update-authz 一致：读端点受权限中间件保护，admin 才放行）。
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('GET /system-update/tasks 返回分页信封（不回退成单对象/裸数组），空表则 items=[] total=0', async ({
    page,
  }) => {
    const env = await apiCallRaw<PaginatedEnvelope<Record<string, unknown>>>(
      page,
      'GET',
      '/system-update/tasks?page=1&page_size=20'
    );

    // 1) data 必须是「分页信封对象」：非裸数组（回退 Vec 的迹象）、非 undefined。
    expect(env, 'tasks 响应 data 应存在（分页信封对象）').toBeTruthy();
    expect(
      Array.isArray(env),
      `tasks.data 不应是裸数组（回退成 Vec 的迹象）：实际=${JSON.stringify(env)}`
    ).toBe(false);

    // 2) 信封四要素契约级断言（空表也成立）。
    expect(
      Array.isArray(env.items),
      `tasks.data.items 应为数组：实际=${JSON.stringify(env.items)}`
    ).toBe(true);
    expect(typeof env.total, 'tasks.data.total 应为 number').toBe('number');
    expect(env.total, 'tasks.data.total 应非负').toBeGreaterThanOrEqual(0);
    // 后端 list_update_tasks 不读分页参数、恒返回第 1 页（全表），请求页即第 1 页。
    expect(env.page, '后端全表返回，page 应恒为第 1 页（请求页）').toBe(1);
    expect(typeof env.page_size, 'tasks.data.page_size 应为 number').toBe('number');
    // 全表不变式：total 恒等于 items 长度（handler 以 len(items) 作 total）。
    expect(env.total, `全表分页：total(${env.total}) 应等于 items 长度(${env.items.length})`).toBe(
      env.items.length
    );

    // 3) 空表诚实断言；非空则锁「元素是对象且含 UpdateTask 契约键」→ 正名不回退成字符串数组。
    if (env.items.length === 0) {
      expect(env.total, '空表时 total 必须为 0').toBe(0);
    } else {
      const first = env.items[0] as Record<string, unknown>;
      expect(typeof first, 'tasks item 应为对象（非字符串）').toBe('object');
      expect(Array.isArray(first), 'tasks item 不应是数组').toBe(false);
      expect(first, 'tasks item 应含 task_code 键').toHaveProperty('task_code');
      expect(first, 'tasks item 应含 status 键').toHaveProperty('status');
    }
  });

  test('GET /system-update/backups 返回分页信封（SystemBackup 对象数组，非 Vec<String>），空表则 items=[] total=0', async ({
    page,
  }) => {
    const env = await apiCallRaw<PaginatedEnvelope<Record<string, unknown>>>(
      page,
      'GET',
      '/system-update/backups?page=1&page_size=20'
    );

    // 1) data 是分页信封对象，非裸数组（backups 原错绑正是裸 Vec<String>）。
    expect(env, 'backups 响应 data 应存在（分页信封对象）').toBeTruthy();
    expect(
      Array.isArray(env),
      `backups.data 不应是裸数组（原错绑 Vec<String> 的回退迹象）：实际=${JSON.stringify(env)}`
    ).toBe(false);

    // 2) 信封四要素。
    expect(Array.isArray(env.items), 'backups.data.items 应为数组').toBe(true);
    expect(typeof env.total, 'backups.data.total 应为 number').toBe('number');
    expect(env.total, 'backups.data.total 应非负').toBeGreaterThanOrEqual(0);
    expect(env.page, '后端全表返回，page 应恒为第 1 页（请求页）').toBe(1);
    expect(typeof env.page_size, 'backups.data.page_size 应为 number').toBe('number');
    expect(env.total, `全表分页：total(${env.total}) 应等于 items 长度(${env.items.length})`).toBe(
      env.items.length
    );

    // 3) 关键正名锁：非空时 items 元素必须是「SystemBackup 对象」，绝非字符串。
    if (env.items.length === 0) {
      expect(env.total, '空表时 total 必须为 0').toBe(0);
    } else {
      const first = env.items[0] as Record<string, unknown>;
      expect(
        typeof first,
        `backups item 应为对象而非字符串（锁正名，防回退 Vec<String>）：实际=${JSON.stringify(first)}`
      ).toBe('object');
      expect(Array.isArray(first), 'backups item 不应是数组').toBe(false);
      expect(first, 'backups item 应含 backup_code 键').toHaveProperty('backup_code');
      expect(first, 'backups item 应含 status 键').toHaveProperty('status');
    }
  });

  test('GET /system-update/current-version 契约对齐 release_date（version 非空、含 release_date 键）', async ({
    page,
  }) => {
    // 后端 get_version（system_update_handler.rs:169-178）返回 ApiResponse<VersionResponse>，
    // VersionResponse { version: String, release_date: String, changelog: Option<String> }。
    // 前端契约已由 build_date 对齐为 release_date（commit e3bcedf4），故此处只断 release_date，不再断 build_date。
    const v = await apiCallRaw<{ version?: string; release_date?: string }>(
      page,
      'GET',
      '/system-update/current-version'
    );

    expect(typeof v.version, 'version 应为字符串').toBe('string');
    expect(v.version!.length, 'version 应为非空字符串').toBeGreaterThan(0);
    // 锁契约对齐：响应载荷含 release_date 键（serde 恒输出，非 Option skip）。
    expect(
      v,
      `current-version data 应含 release_date 键（契约已由 build_date 对齐）：实际=${JSON.stringify(v)}`
    ).toHaveProperty('release_date');
  });

  test('GET /system-update/versions 已删除：不再返回 2xx 列表语义（路由未注册，期望 404）', async ({
    page,
  }) => {
    // 该列表 GET 已从 routes 移除（仅剩 /system-update/versions/{versionId} 详情等）。
    // admin 会话下，auth/权限中间件放行后由 axum 路由匹配，未知路径应 404，
    // 绝不返回 ApiResponse<Vec<String>>（2xx）——锁「版本列表端点正被移除」不回退。
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/system-update/versions`, {
      headers: { 'X-Requested-With': 'XMLHttpRequest' },
    });
    const status = resp.status();
    expect(
      resp.ok(),
      `已删的 versions GET 不应为 2xx 成功（不得再提供列表语义）：HTTP ${status}`
    ).toBe(false);
    expect(
      status,
      `已删的 versions GET 应 404（路由未注册）：HTTP ${status}，若变 200 说明列表端点被回退加回`
    ).toBe(404);
  });

  test('访问 /system-update：版本 tab 不再渲染（页面仅剩 2 个 tab，无「版本」整词 tab）', async ({
    page,
  }) => {
    // 版本 tab 移除属前端结构变更（index.vue 仅剩 tasks / backups 两个 el-tab-pane）。
    // 用「tab 数量 === 2」这一不依赖 i18n 文案的结构不变式作主锁；
    // 再叠加「无 name 恰为『版本』整词的 tab」防文案级回退（精确整词，不误伤含『版本』子串的其它 tab）。
    await page.goto(`${BASE_URL}/system-update`);
    await expect(page.locator('.el-tabs__item').first(), '系统更新页应渲染 tabs').toBeVisible({
      timeout: 30_000,
    });
    await expect(
      page.locator('.el-tabs__item'),
      '版本 tab 移除后应仅剩 2 个 tab（任务、备份）'
    ).toHaveCount(2);
    await expect(
      page.getByRole('tab', { name: /^版本$/ }),
      '不应再存在名为『版本』的 tab'
    ).toHaveCount(0);
  });
});

// ────────────────────────────────────────────────────────────────────────────
// 以下属「CI 无法验证的真实项」：涉及设备权限 / 外网 release 资产 / 下载链路 / 二进制替换。
// 依红线不为这些写「永远通过」或断言不存在行为的假用例，显式 test.skip 并写明中文原因，
// 交回编排方（部署/人工验证）。绝不 skip 掉本该可在 CI 真测的项。
// ────────────────────────────────────────────────────────────────────────────
test.describe('系统更新：CI 不可测项（显式 skip，不写假断言）', () => {
  test.skip('GET /system-update/check 的 release_notes 内容核验', async () => {
    // skip 原因（真实、非掩盖）：/system-update/check → check_for_updates 依赖 api.github.com 拉取真实
    // release 资产与 changelog。CI 环境网络不可达 / 不稳定，且 release_notes / current_release_notes
    // 的具体内容取决于真实发布，属网络与部署前提，无法在 CI 稳定验证内容正确性。
    // 结构层面（如 has_update 为布尔）已由后端单元/契约测试覆盖；此处不强断言 release_notes 非空，
    // 以免写出「依赖外网却恒真」的假绿。若要覆盖，需在可达外网且存在真实 release 资产的部署环境人工验证。
    expect(true).toBe(false); // 永不执行（上方 skip），占位以防误去掉 skip 后静默假绿。
  });

  test.skip('更新下载多镜像 + SHA-256/MD5 完整性强校验', async () => {
    // skip 原因：download_and_update 的多镜像加速与 SHA-256/MD5 校验依赖真实 release 资产下载链路
    // 与二进制文件，属真实下载/部署前提，CI 无法安全执行（会真实替换二进制，高危且需专用环境）。
    expect(true).toBe(false);
  });

  test.skip('摄像头扫码（getUserMedia，二维码 / 条形码）', async () => {
    // skip 原因：getUserMedia 需 HTTPS 安全上下文 + 真实摄像头设备 + 用户授权弹窗。
    // 本地 CI（headless、HTTP baseURL、无物理摄像头、无权限授予）无法满足，属设备与部署前提。
    // 组件代码已核对正确（限定二维码/条形码类型并调用 getUserMedia），此处不在 CI 造假断言。
    expect(true).toBe(false);
  });
});
