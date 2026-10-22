import { test, expect } from '../diagnose-fixture';
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
 * 87 API 网关端点元数据的增删改查读回（analytics 域，api-gateway 族）
 *
 * 守的是什么：网关端点登记面（表 api_endpoints）确实把提交值落库并能逐键读回，
 * 而不是只在写接口里回显一遍。覆盖以下五个真实端点，均由 api_gateway_handler 出：
 * - POST /api-gateway/endpoints 建端点，path 与 method 必填，缺任一返回 400 且码为 VALIDATION_ERROR；
 *   rate_limit 后端校验区间 0 到 10000，越界同样返回 400 且码为 VALIDATION_ERROR；
 *   同一 path 加 method 已存在时返回 400 且码为 BUSINESS_ERROR（先查后插，并由唯一约束兜底并发）。
 *   未提交的列取后端默认：status 为小写 active、rate_limit 为 0、timeout 为 30000、
 *   authentication 为 true、version 为 v1；description 与 module 为空串。
 * - GET /api-gateway/endpoints 列表，入参为查询串 page 与 page_size 及 keyword，
 *   keyword 命中 path 或 description 的子串即入选；出参是 ApiResponse，data 为端点对象数组、
 *   total 在顶层，不是分页信封的 items 结构，故此处按裸数组判 data。
 * - GET /api-gateway/endpoints/{id} 取单条，id 不存在返回 404 且码为 NOT_FOUND；
 *   若该端点带 deprecated_at 或 sunset_at，额外携带 is_deprecated 为 true。
 * - PUT /api-gateway/endpoints/{id} 部分更新，只覆盖请求体里出现的列，未出现的列保持原值；
 *   id 不存在返回 404 且码为 NOT_FOUND。
 * - DELETE /api-gateway/endpoints/{id} 删除，按主键删除不校验存在性（删不存在的 id 不报错），
 *   故删除是否真生效只能靠删后 GET 该 id 变 404 反证，不直断删除接口的返回码。
 *
 * 为什么用合成路径而不是复用真实路径：网关的按请求动态门控中间件只会拿当前真实请求的
 * method 与 uri 路径去匹配 api_endpoints 表，命中 inactive 会对该请求返回 503。本规格建的每条
 * 端点 path 都取一个绝不作为真实请求发出的合成值，动态门控查不到它、也就不会影响任何并发分片，
 * 这些行只被本规格的列表与详情读接口消费。
 *
 * 断言均为值判据（写什么读回什么、列表按关键字双向命中、更新后旧值变新值、负例钉到具体 HTTP
 * 码与机器码），不用只判元素可见、不只判状态字面量、不以小于 500 当健康。
 */

// 列表与详情接口出参中被本规格断言的列（snake_case，与 api_gateway_handler 投影一致）。
type GwRow = {
  id: number;
  path: string;
  method: string;
  description: string;
  module: string;
  status: string;
  rate_limit: number;
  timeout: number;
  authentication: boolean;
  version: string;
};

// 合成端点路径前缀：绝不作为真实请求发出，确保动态门控中间件查不到本规格建的行。
function gwProbePath(tag: string): string {
  return `/api/v1/erp/__e2e_gw_${tag}__`;
}

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

// 从成功信封顶层取 total（helpers 的 ApiResponse 类型未声明 total，显式收窄后可类型自检）。
function totalOf(env: unknown): number | undefined {
  return (env as { total?: number }).total;
}

test.describe.serial('87 API 网关端点元数据增删改查读回', () => {
  const cleanup: DeferredCleanup[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    await flushDeferredCleanups(page, cleanup);
  });

  test('POST 建端点后由列表与单条读回逐键持久，未提交列落后端默认', async ({ page }) => {
    const tag = uniqTag();
    const path = gwProbePath(tag);
    const desc = `e2e-gw-desc-${tag}`;
    const module = `e2e-gw-mod-${tag}`;

    const created = await apiCall<GwRow>(page, 'POST', '/api-gateway/endpoints', {
      path,
      method: 'POST',
      description: desc,
      module,
      rate_limit: 137,
      timeout: 8192,
      authentication: true,
      version: 'v7',
    });
    const id = created.data.id;
    expect(Number.isInteger(id) && id > 0, `创建应返回正整数 id，实际：${String(id)}`).toBe(true);
    deferCleanup(cleanup, 'DELETE', `/api-gateway/endpoints/${id}`, `api-gw-${tag}`);

    // 建接口自身回读。
    expect(created.data.path, '建接口应回显提交路径').toBe(path);
    expect(created.data.method).toBe('POST');
    expect(created.data.description).toBe(desc);
    expect(created.data.module).toBe(module);
    expect(
      created.data.rate_limit,
      `限流应等于提交值 137，实际：${String(created.data.rate_limit)}`
    ).toBe(137);
    expect(created.data.timeout).toBe(8192);
    expect(created.data.authentication).toBe(true);
    expect(created.data.version).toBe('v7');

    // 从数据库经列表接口读回（证明落库，不只是 POST 回显）。
    const listed = await apiCall<GwRow[]>(
      page,
      'GET',
      `/api-gateway/endpoints?keyword=${encodeURIComponent(tag)}&page=1&page_size=50`
    );
    const hit = listed.data.find(r => r.id === id);
    expect(hit, `列表按关键字 ${tag} 应能读到刚建端点 id=${id}`).toBeTruthy();
    expect(hit!.path).toBe(path);
    expect(hit!.method).toBe('POST');
    expect(hit!.description).toBe(desc);
    expect(hit!.module).toBe(module);

    // 从数据库经单条接口逐键读回。
    const single = await apiCallRaw<GwRow>(page, 'GET', `/api-gateway/endpoints/${id}`);
    expect(single.id).toBe(id);
    expect(single.path).toBe(path);
    expect(
      single.status,
      `未提交 status 应落后端默认小写 active，实际：${String(single.status)}`
    ).toBe('active');

    // 只提交 path 与 method、其余留空的建法应落全部后端默认。
    const bareTag = uniqTag();
    const bare = await apiCallRaw<GwRow>(page, 'POST', '/api-gateway/endpoints', {
      path: gwProbePath(bareTag),
      method: 'GET',
    });
    deferCleanup(cleanup, 'DELETE', `/api-gateway/endpoints/${bare.id}`, `api-gw-bare-${bareTag}`);
    expect(bare.rate_limit, `未提交 rate_limit 应落默认 0，实际：${String(bare.rate_limit)}`).toBe(
      0
    );
    expect(bare.timeout, `未提交 timeout 应落默认 30000，实际：${String(bare.timeout)}`).toBe(
      30000
    );
    expect(
      bare.authentication,
      `未提交 authentication 应落默认 true，实际：${String(bare.authentication)}`
    ).toBe(true);
    expect(bare.version, `未提交 version 应落默认 v1，实际：${String(bare.version)}`).toBe('v1');
    expect(bare.status, `未提交 status 应落默认 active，实际：${String(bare.status)}`).toBe(
      'active'
    );
    expect(
      bare.description,
      `未提交 description 应投影为空串，实际：${String(bare.description)}`
    ).toBe('');
  });

  test('列表关键字命中双向可分：命中含新行、不命中为空且顶层 total 随过滤', async ({ page }) => {
    const tag = uniqTag();
    const path = gwProbePath(tag);
    const created = await apiCallRaw<GwRow>(page, 'POST', '/api-gateway/endpoints', {
      path,
      method: 'GET',
      description: `e2e-gw-kw-${tag}`,
    });
    deferCleanup(cleanup, 'DELETE', `/api-gateway/endpoints/${created.id}`, `api-gw-kw-${tag}`);

    // 命中侧：过滤后至少含这一条，且顶层 total 不小于 1，杜绝空库上的 0 等于 0 空洞通过。
    const match = await apiCall<GwRow[]>(
      page,
      'GET',
      `/api-gateway/endpoints?keyword=${encodeURIComponent(tag)}&page=1&page_size=50`
    );
    expect(Array.isArray(match.data), `列表出参 data 应为数组，实际：${typeof match.data}`).toBe(
      true
    );
    expect(
      match.data.some(r => r.id === created.id),
      `命中关键字 ${tag} 的列表应包含 id=${created.id}`
    ).toBe(true);
    const matchTotal = totalOf(match);
    expect(
      typeof matchTotal === 'number' && matchTotal >= 1,
      `命中过滤的顶层 total 应为不小于 1 的数字，实际：${String(matchTotal)}`
    ).toBe(true);

    // 未命中侧：换一个绝不出现的关键字，data 应为空数组且 total 为 0，证明 keyword 真的下推过滤。
    const miss = await apiCall<GwRow[]>(
      page,
      'GET',
      `/api-gateway/endpoints?keyword=${encodeURIComponent(`nomatch_${tag}_zz`)}&page=1&page_size=50`
    );
    expect(miss.data, '未命中关键字应返回空数组').toEqual([]);
    expect(totalOf(miss), `未命中过滤的顶层 total 应为 0，实际：${String(totalOf(miss))}`).toBe(0);
  });

  test('PUT 部分更新只改提交的列并读回新值，未提交列保持原值', async ({ page }) => {
    const tag = uniqTag();
    const path = gwProbePath(tag);
    const created = await apiCallRaw<GwRow>(page, 'POST', '/api-gateway/endpoints', {
      path,
      method: 'GET',
      description: `e2e-gw-old-${tag}`,
      module: `e2e-gw-keep-${tag}`,
      rate_limit: 200,
      version: 'v3',
    });
    const id = created.id;
    deferCleanup(cleanup, 'DELETE', `/api-gateway/endpoints/${id}`, `api-gw-upd-${tag}`);

    await apiCall(page, 'PUT', `/api-gateway/endpoints/${id}`, {
      description: `e2e-gw-new-${tag}`,
      status: 'inactive',
      rate_limit: 5,
    });

    const after = await apiCallRaw<GwRow>(page, 'GET', `/api-gateway/endpoints/${id}`);
    // 被提交的列应变为新值。
    expect(after.description, `描述应更新为新值，实际：${String(after.description)}`).toBe(
      `e2e-gw-new-${tag}`
    );
    expect(after.status, `状态应更新为 inactive，实际：${String(after.status)}`).toBe('inactive');
    expect(after.rate_limit, `限流应更新为 5，实际：${String(after.rate_limit)}`).toBe(5);
    // 未提交的列应保持不变（证明部分更新语义，不是全量清空）。
    expect(after.module, `未提交的 module 应保持原值，实际：${String(after.module)}`).toBe(
      `e2e-gw-keep-${tag}`
    );
    expect(after.version, `未提交的 version 应保持原值 v3，实际：${String(after.version)}`).toBe(
      'v3'
    );
    expect(after.timeout, `未提交的 timeout 应保持默认 30000，实际：${String(after.timeout)}`).toBe(
      30000
    );
  });

  test('缺必填与非法限流被拒 400 且码为 VALIDATION_ERROR', async ({ page }) => {
    // 缺 method。
    const noMethod = await apiCallExpectFail(page, 'POST', '/api-gateway/endpoints', {
      path: gwProbePath(uniqTag()),
    });
    expect(
      noMethod.status,
      `缺 method 应 400，实际 status=${noMethod.status} code=${noMethod.code}`
    ).toBe(400);
    expect(noMethod.code, `缺 method 的码应为 VALIDATION_ERROR，实际：${noMethod.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // 缺 path。
    const noPath = await apiCallExpectFail(page, 'POST', '/api-gateway/endpoints', {
      method: 'GET',
    });
    expect(noPath.status, `缺 path 应 400，实际 status=${noPath.status} code=${noPath.code}`).toBe(
      400
    );
    expect(noPath.code, `缺 path 的码应为 VALIDATION_ERROR，实际：${noPath.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // 限流越界（大于 10000），限流校验先于必填，仍应 400 校验码。
    const badRate = await apiCallExpectFail(page, 'POST', '/api-gateway/endpoints', {
      path: gwProbePath(uniqTag()),
      method: 'GET',
      rate_limit: 99999,
    });
    expect(
      badRate.status,
      `限流越界应 400，实际 status=${badRate.status} code=${badRate.code}`
    ).toBe(400);
    expect(badRate.code, `限流越界的码应为 VALIDATION_ERROR，实际：${badRate.code}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });

  test('重复 path 加 method 被拒 400 且码为 BUSINESS_ERROR，未知 id 的读与更新 404', async ({
    page,
  }) => {
    const tag = uniqTag();
    const path = gwProbePath(tag);
    const first = await apiCallRaw<GwRow>(page, 'POST', '/api-gateway/endpoints', {
      path,
      method: 'DELETE',
    });
    deferCleanup(cleanup, 'DELETE', `/api-gateway/endpoints/${first.id}`, `api-gw-dup-${tag}`);

    // 同 path 同 method 再建应被判重拒绝。
    const dup = await apiCallExpectFail(page, 'POST', '/api-gateway/endpoints', {
      path,
      method: 'DELETE',
    });
    expect(
      dup.status,
      `重复 path 加 method 应 400，实际 status=${dup.status} code=${dup.code}`
    ).toBe(400);
    expect(dup.code, `重复建的码应为 BUSINESS_ERROR，实际：${dup.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );

    // 未知 id 的单条读应 404。
    const unknownId = 999999999;
    const getMissing = await apiCallExpectFail(page, 'GET', `/api-gateway/endpoints/${unknownId}`);
    expect(
      getMissing.status,
      `读未知 id 应 404，实际 status=${getMissing.status} code=${getMissing.code}`
    ).toBe(404);
    expect(getMissing.code, `读未知 id 的码应为 NOT_FOUND，实际：${getMissing.code}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );

    // 未知 id 的更新应 404。
    const putMissing = await apiCallExpectFail(page, 'PUT', `/api-gateway/endpoints/${unknownId}`, {
      description: 'nope',
    });
    expect(
      putMissing.status,
      `改未知 id 应 404，实际 status=${putMissing.status} code=${putMissing.code}`
    ).toBe(404);
    expect(putMissing.code, `改未知 id 的码应为 NOT_FOUND，实际：${putMissing.code}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });

  test('DELETE 删行后 GET 该 id 变 404，反证删除真生效', async ({ page }) => {
    const tag = uniqTag();
    const created = await apiCallRaw<GwRow>(page, 'POST', '/api-gateway/endpoints', {
      path: gwProbePath(tag),
      method: 'PATCH',
      description: `e2e-gw-del-${tag}`,
    });
    // 先确认删前可读（否则删后 404 无法归因于删除本身）。
    await apiCallRaw<GwRow>(page, 'GET', `/api-gateway/endpoints/${created.id}`);

    await apiCall(page, 'DELETE', `/api-gateway/endpoints/${created.id}`);

    const afterDelete = await apiCallExpectFail(
      page,
      'GET',
      `/api-gateway/endpoints/${created.id}`
    );
    expect(
      afterDelete.status,
      `删除后读该 id 应 404，实际 status=${afterDelete.status} code=${afterDelete.code}`
    ).toBe(404);
    expect(afterDelete.code, `删除后读的码应为 NOT_FOUND，实际：${afterDelete.code}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });
});
