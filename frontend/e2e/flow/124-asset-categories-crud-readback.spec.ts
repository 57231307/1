import { test, expect } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  deferCleanup,
  flushDeferredCleanups,
  failureCode,
  type DeferredCleanup,
} from './helpers';

/**
 * 124 资产分类端点族的增删改查读回（finance 域，asset-categories 族）
 *
 * 功能：守住资产分类配置面（表 asset_categories）确实把提交值落库并能逐键读回，
 * 覆盖以下五个真实端点，均由 asset_category_handler 产出：
 * - GET /asset-categories 分页列表（页码基数 1，入参 page/page_size/is_active），
 *   出参 ApiResponse<PaginatedResponse<Model>>，data.items 为分类对象数组、data.total 在分页信封顶层。
 *   默认 page=1 page_size=20；page_size 经 clamp(1,100) 限制上限 100。
 * - POST /asset-categories 建分类，category_code/category_name 必填；
 *   可选 parent_id(i32) default_useful_life(i32) default_depreciation_method(String)
 *   default_salvage_rate(Decimal(5,4)) description(String)。
 *   is_active 由 service 硬编码为 true，不接受外部传入。
 * - GET /asset-categories/{id} 取单条，id 不存在返回 404 且码为 NOT_FOUND。
 * - PUT /asset-categories/{id} 部分更新，只覆盖请求体里出现的列，未出现的列保持原值；
 *   可空列三态（对齐 RFC 7386）：键缺席=保持原值、显式 null=清空为 NULL、有值=覆盖。
 *   id 不存在返回 404 且码为 NOT_FOUND。
 * - DELETE /asset-categories/{id} 软删除（service 设 is_active=false），
 *   删后按 id 再查仍返回 200 但 is_active 为 false。
 *
 * 调用方：routes/finance.rs:1179-1192 注册，handler 在 handlers/asset_category_handler.rs。
 * 入参：CreateCategoryDto 必填 category_code(String) category_name(String)；
 *   可选 parent_id(i32) default_useful_life(i32) default_depreciation_method(String)
 *   default_salvage_rate(Decimal) description(String)。
 * 传给谁：AssetCategoryService → sea_orm Entity → PostgreSQL。
 * 存什么：id category_code category_name parent_id default_useful_life
 *   default_depreciation_method default_salvage_rate description is_active created_at updated_at。
 * 存哪里：表 asset_categories，路由前缀 /api/v1/erp/asset-categories。
 *
 * 断言均为值判据（写什么读回什么、更新后旧值变新值、软删后 is_active 变 false），
 * 不用恒真、不只判 ok()。
 */

type CategoryRow = {
  id: number;
  category_code: string;
  category_name: string;
  parent_id: number | null;
  default_useful_life: number | null;
  default_depreciation_method: string | null;
  default_salvage_rate: string | null;
  description: string | null;
  is_active: boolean;
};

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

test.describe.serial('124 资产分类增删改查读回', () => {
  const cleanup: DeferredCleanup[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    // 资产分类 delete 是软删（is_active=false），flushDeferredCleanups 走 DELETE 端点即可停用。
    await flushDeferredCleanups(page, cleanup);
  });

  test('POST 建分类后由单条读回逐键持久，未提交列落后端默认 null', async ({ page }) => {
    const tag = uniqTag();
    const code = `e2e-ac-${tag}`;
    const name = `e2e资产分类-${tag}`;

    const created = await apiCall<CategoryRow>(page, 'POST', '/asset-categories', {
      category_code: code,
      category_name: name,
    });
    const id = created.data.id;
    expect(Number.isInteger(id) && id > 0, `创建应返回正整数 id，实际：${String(id)}`).toBe(true);
    deferCleanup(cleanup, 'DELETE', `/asset-categories/${id}`, `ac-${tag}`);

    expect(created.data.category_code, '建接口应回显 category_code').toBe(code);
    expect(created.data.category_name).toBe(name);
    expect(created.data.parent_id, '未提交 parent_id 应为 null').toBeNull();
    expect(created.data.default_useful_life, '未提交使用年限应为 null').toBeNull();
    expect(created.data.default_depreciation_method, '未提交折旧方法应为 null').toBeNull();
    expect(created.data.default_salvage_rate, '未提交残值率应为 null').toBeNull();
    expect(created.data.description, '未提交描述应为 null').toBeNull();
    expect(created.data.is_active, '新建分类 is_active 应为 true').toBe(true);

    // 从数据库经单条读回（证明落库，不只是 POST 回显）。
    const fetched = await apiCall<CategoryRow>(page, 'GET', `/asset-categories/${id}`);
    expect(fetched.data.id).toBe(id);
    expect(fetched.data.category_code).toBe(code);
    expect(fetched.data.category_name).toBe(name);
    expect(fetched.data.is_active).toBe(true);
  });

  test('GET 列表分页读回包含刚建分类', async ({ page }) => {
    const tag = uniqTag();
    const code = `e2e-ac-list-${tag}`;

    const created = await apiCall<CategoryRow>(page, 'POST', '/asset-categories', {
      category_code: code,
      category_name: `e2e列表分类-${tag}`,
      default_useful_life: 120,
      default_depreciation_method: 'straight_line',
      default_salvage_rate: '0.0500',
      description: `e2e-desc-${tag}`,
    });
    const id = created.data.id;
    deferCleanup(cleanup, 'DELETE', `/asset-categories/${id}`, `ac-list-${tag}`);

    // 本链页码基数 1（handler:113 unwrap_or(1).max(1)），page=1 取首页。
    const listed = await apiCall<{
      items: CategoryRow[];
      total: number;
      page: number;
      page_size: number;
    }>(page, 'GET', '/asset-categories?page=1&page_size=100');

    const hit = listed.data.items.find(r => r.id === id);
    expect(hit, `列表应包含刚建分类 id=${id}`).toBeDefined();
    expect(hit!.category_code).toBe(code);
    expect(hit!.default_useful_life).toBe(120);
    expect(hit!.default_depreciation_method).toBe('straight_line');
    expect(hit!.description).toBe(`e2e-desc-${tag}`);
    expect(Number(hit!.default_salvage_rate), '残值率应持久 0.05').toBeCloseTo(0.05, 4);
  });

  test('PUT 部分更新后旧值变新值，缺席列保持原值', async ({ page }) => {
    const tag = uniqTag();
    const code = `e2e-ac-upd-${tag}`;

    const created = await apiCall<CategoryRow>(page, 'POST', '/asset-categories', {
      category_code: code,
      category_name: `原始名称-${tag}`,
      default_useful_life: 60,
      default_depreciation_method: 'double_declining',
      description: '原始描述',
    });
    const id = created.data.id;
    deferCleanup(cleanup, 'DELETE', `/asset-categories/${id}`, `ac-upd-${tag}`);

    const updated = await apiCall<CategoryRow>(page, 'PUT', `/asset-categories/${id}`, {
      category_name: `更新名称-${tag}`,
      default_useful_life: 240,
    });
    expect(updated.data.category_name).toBe(`更新名称-${tag}`);
    expect(updated.data.default_useful_life).toBe(240);
    // 缺席列保持原值
    expect(
      updated.data.default_depreciation_method,
      '缺席的折旧方法应保持原值 double_declining'
    ).toBe('double_declining');
    expect(updated.data.description, '缺席的描述应保持原值').toBe('原始描述');

    // 从数据库读回确认
    const fetched = await apiCall<CategoryRow>(page, 'GET', `/asset-categories/${id}`);
    expect(fetched.data.category_name).toBe(`更新名称-${tag}`);
    expect(fetched.data.default_useful_life).toBe(240);
  });

  test('DELETE 软删后 is_active 变 false', async ({ page }) => {
    const tag = uniqTag();
    const code = `e2e-ac-del-${tag}`;

    const created = await apiCall<CategoryRow>(page, 'POST', '/asset-categories', {
      category_code: code,
      category_name: `待删分类-${tag}`,
    });
    const id = created.data.id;

    const delRes = await apiCall<string>(page, 'DELETE', `/asset-categories/${id}`);
    expect(delRes.code, '删除应返回 200').toBe(200);
    expect(delRes.data, '删除返回应含"已停用"').toContain('已停用');

    // 软删后按 id 读回仍 200 但 is_active=false
    const afterDel = await apiCall<CategoryRow>(page, 'GET', `/asset-categories/${id}`);
    expect(afterDel.code, '软删后 GET 仍 200').toBe(200);
    expect(afterDel.data.is_active, '软删后 is_active 应为 false').toBe(false);
    // 不再需要清理——已软删
  });

  test('POST 缺必填 category_code 返回 400 且码为 VALIDATION_ERROR', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/asset-categories', {
      category_name: 'x',
    });
    expect(fail.status, '缺必填应 400').toBe(400);
    expect(failureCode(fail), '码应为 VALIDATION_ERROR').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('GET 不存在的 id 返回 404 且码为 NOT_FOUND', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'GET', '/asset-categories/999999999');
    expect(fail.status).toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });
});
