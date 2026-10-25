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
 * 103 产品类别的增改删与树挂载读回（catalog 域，categories 族）
 * 覆盖六个真实端点（handlers/product_category_handler.rs，基础增删改查由 define_crud_handlers 宏生成）：
 * - POST /categories 建类别，name 必填（validator 限长 1 到 100），code 可选（未提交时服务层按
 *   CAT 加纳秒时间戳生成，DB 列 code 全表唯一），parent_id 可选（指向不存在父级返回 404 且码为
 *   NOT_FOUND），description 可选，返回落库后的整行。
 * - GET /categories/{id} 取单条，id 不存在返回 404 且码为 NOT_FOUND。
 * - PUT /categories/{id} 部分更新，各列可选只覆盖提交的列；改 name 时服务层查重（排除自身），
 *   与他行同名返回 400 且码为 BUSINESS_ERROR。
 * - DELETE /categories/{id} 物理删（经审计删除），有子类别时返回 400 且码为 BUSINESS_ERROR 拒绝删。
 * - GET /categories 列表，入参含 page、page_size 与 search（按 name 做 LIKE 包含匹配），
 *   出参 data 是分页信封 items 与 total。
 * - GET /categories/tree 树接口，只返回 parent_id 为空的根节点数组，子类别挂在对应节点的
 *   children 里，节点键为 id、name、parent_id、description、children。
 * 删除是物理删且删前有子守卫，故删后 GET 该 id 变 404 是删除生效的直接判据；
 * 类别名不加全局唯一约束（仅 update 查重），但每行 code 唯一且断言一律按自建行 id 命中，
 * 并发分片的他人在途数据不会进入本规格的读回集合。
 * 断言均为值判据（写什么读回什么、树上父子挂载位置、更新后旧值变新值、负例钉到具体 HTTP 码与机器码），
 * 不只判元素可见、不以小于 500 当健康。
 */

// 类别整行（product_category Model 的 snake_case 键）。
type Category = {
  id: number;
  name: string;
  code: string;
  parent_id: number | null;
  description: string | null;
  created_at: string;
  updated_at: string;
};

// 列表出参（ApiResponse.data 即分页信封）。
type CategoryPage = { items: Category[]; total: number; page: number; page_size: number };

// 树节点（services/product_category_service.rs CategoryTreeNode 序列化键）。
type TreeNode = {
  id: number;
  name: string;
  parent_id: number | null;
  description: string | null;
  children: TreeNode[];
};

function uniq(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

/** 建一个根类别（parent_id 缺省为空），name 与 code 均带唯一后缀避开 update 查重与 code 唯一约束。 */
async function createRoot(page: import('@playwright/test').Page, tag: string): Promise<Category> {
  return apiCallRaw<Category>(page, 'POST', '/categories', {
    name: `E2E-CAT-${tag}`,
    code: `E2E-CODE-${tag}`.slice(0, 50),
    description: `根类别 ${tag}`,
  });
}

test.describe.serial('103 产品类别增改删与树读回', () => {
  const cleanup: DeferredCleanup[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    // 逆序 flush 即子先于父删除，规避父有子时的删除守卫 400。
    await flushDeferredCleanups(page, cleanup);
  });

  test('建父挂子后逐键读回，列表按 search 命中、树按父子挂载', async ({ page }) => {
    const tag = uniq();
    const parent = await createRoot(page, tag);
    deferCleanup(cleanup, 'DELETE', `/categories/${parent.id}`, `cat-parent-${tag}`);
    expect(Number.isInteger(parent.id) && parent.id > 0, `建父类应返回正整数 id`).toBe(true);
    expect(parent.parent_id, `根类别 parent_id 应为空`).toBeNull();

    const child = await apiCallRaw<Category>(page, 'POST', '/categories', {
      name: `E2E-CAT-CHILD-${tag}`,
      code: `E2E-CODE-CHILD-${tag}`.slice(0, 50),
      parent_id: parent.id,
    });
    deferCleanup(cleanup, 'DELETE', `/categories/${child.id}`, `cat-child-${tag}`);

    // 单条逐键读回（写什么落什么）。
    const gotChild = await apiCallRaw<Category>(page, 'GET', `/categories/${child.id}`);
    expect(gotChild.name).toBe(`E2E-CAT-CHILD-${tag}`);
    expect(
      gotChild.parent_id,
      `子类 parent_id 应落父 id=${parent.id}，实际：${String(gotChild.parent_id)}`
    ).toBe(parent.id);
    expect(
      gotChild.description,
      `未提交 description 应落 NULL（表示未填，不写空串兜底），实际：${String(gotChild.description)}`
    ).toBeNull();

    // 列表 search 按名称包含命中（分页信封 items）。
    const listed = await apiCall<CategoryPage>(
      page,
      'GET',
      `/categories?search=E2E-CAT-CHILD-${tag}&page=1&page_size=20`
    );
    const hit = listed.data.items.find(r => r.id === child.id);
    expect(hit, `列表 search 应含刚建子类 id=${child.id}`).toBeTruthy();
    expect(hit!.name).toBe(`E2E-CAT-CHILD-${tag}`);

    // 树读回：子节点必须挂在父节点的 children 里，且父节点出现在根层。
    const tree = await apiCallRaw<TreeNode[]>(page, 'GET', '/categories/tree');
    const treeNode = tree.find(n => n.id === parent.id);
    expect(treeNode, `树根层应含刚建父类 id=${parent.id}`).toBeTruthy();
    expect(
      treeNode!.children.some(n => n.id === child.id),
      `父类 children 应含子类 id=${child.id}，实际：${JSON.stringify(treeNode!.children.map(n => n.id))}`
    ).toBe(true);
  });

  test('PUT 部分更新只改提交列，改 name 与他行同名被拒 400', async ({ page }) => {
    const tag = uniq();
    const a = await createRoot(page, `${tag}A`);
    deferCleanup(cleanup, 'DELETE', `/categories/${a.id}`, `cat-a-${tag}`);
    const b = await createRoot(page, `${tag}B`);
    deferCleanup(cleanup, 'DELETE', `/categories/${b.id}`, `cat-b-${tag}`);
    const bCode = b.code;

    await apiCall(page, 'PUT', `/categories/${b.id}`, {
      name: `E2E-CAT-RENAMED-${tag}`,
      description: `改后描述 ${tag}`,
    });
    const after = await apiCallRaw<Category>(page, 'GET', `/categories/${b.id}`);
    expect(after.name, `名称应更新为新值，实际：${after.name}`).toBe(`E2E-CAT-RENAMED-${tag}`);
    expect(after.description, `描述应更新，实际：${String(after.description)}`).toBe(
      `改后描述 ${tag}`
    );
    expect(after.code, `未提交的 code 不应被清空，实际：${after.code}`).toBe(bCode);

    // 改名校验（排除自身）：把 a 改成 b 现有名应被拒 400 业务码。
    const dup = await apiCallExpectFail(page, 'PUT', `/categories/${a.id}`, {
      name: `E2E-CAT-RENAMED-${tag}`,
    });
    expect(dup.status, `重名更新应 400，实际 status=${dup.status} code=${dup.code}`).toBe(400);
    expect(dup.code, `重名更新的码应为 BUSINESS_ERROR，实际：${String(dup.code)}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    // 被拒后 a 的名称未变（拒绝不落库）。
    const aAfter = await apiCallRaw<Category>(page, 'GET', `/categories/${a.id}`);
    expect(aAfter.name).toBe(`E2E-CAT-${tag}A`);
  });

  test('有子类别的父类删除被守卫拒 400，先删子再删父后单条 404', async ({ page }) => {
    const tag = uniq();
    const parent = await createRoot(page, tag);
    const child = await apiCallRaw<Category>(page, 'POST', '/categories', {
      name: `E2E-CAT-GUARD-${tag}`,
      code: `E2E-CODE-GUARD-${tag}`.slice(0, 50),
      parent_id: parent.id,
    });

    // 守卫：父有子时删除应 400 业务码，且父行仍在。
    const blocked = await apiCallExpectFail(page, 'DELETE', `/categories/${parent.id}`);
    expect(
      blocked.status,
      `有子的父类删除应 400，实际 status=${blocked.status} code=${blocked.code}`
    ).toBe(400);
    expect(blocked.code, `守卫拒绝的码应为 BUSINESS_ERROR，实际：${String(blocked.code)}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const stillThere = await apiCallRaw<Category>(page, 'GET', `/categories/${parent.id}`);
    expect(stillThere.id, `守卫拒绝后父类应仍可查`).toBe(parent.id);

    // 先删子、再删父，物理删后单条读回 404。
    await apiCall(page, 'DELETE', `/categories/${child.id}`);
    await apiCall(page, 'DELETE', `/categories/${parent.id}`);
    const gone = await apiCallExpectFail(page, 'GET', `/categories/${parent.id}`);
    expect(
      gone.status,
      `物理删后按 id 取应 404，实际 status=${gone.status} code=${gone.code}`
    ).toBe(404);
    expect(gone.code, `删除后读回的码应为 NOT_FOUND，实际：${String(gone.code)}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });

  test('挂不存在父级被拒 404，取未知 id 详情 404', async ({ page }) => {
    const tag = uniq();
    const badParent = await apiCallExpectFail(page, 'POST', '/categories', {
      name: `E2E-CAT-BADP-${tag}`,
      code: `E2E-CODE-BADP-${tag}`.slice(0, 50),
      parent_id: 990001,
    });
    expect(
      badParent.status,
      `指向不存在父级应 404，实际 status=${badParent.status} code=${badParent.code}`
    ).toBe(404);
    expect(badParent.code, `不存在父级的码应为 NOT_FOUND，实际：${String(badParent.code)}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );

    const missing = await apiCallExpectFail(page, 'GET', '/categories/990002');
    expect(
      missing.status,
      `取未知类别应 404，实际 status=${missing.status} code=${missing.code}`
    ).toBe(404);
    expect(missing.code, `取未知类别的码应为 NOT_FOUND，实际：${String(missing.code)}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });

  test('缺 name 建类别被拒 400 且码为 VALIDATION_ERROR', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/categories', {
      code: `E2E-CODE-NO-NAME-${uniq()}`.slice(0, 50),
    });
    expect(fail.status, `缺 name 应 400，实际 status=${fail.status} code=${fail.code}`).toBe(400);
    expect(fail.code, `缺 name 的码应为 VALIDATION_ERROR，实际：${String(fail.code)}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });
});
