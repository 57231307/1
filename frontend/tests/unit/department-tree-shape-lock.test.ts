/**
 * 部门树出参形状锁（后端构造点 ↔ 前端声明 ↔ 消费视图三向互证）。
 * 功能：
 * 1) GET /departments/tree 的节点载体 = 后端 services/department_service.rs 的
 *    DepartmentTreeNode 结构体（Serialize、无 rename），出参键集恒为
 *    id/name/code/description/parent_id/manager_name/sort_order/is_active/children 九键，
 *    处理器出参为 ApiResponse<Vec<DepartmentTreeNode>>（data 裸数组，无分页信封）；
 *    根级与每一层同级的先后由后端显式定序（sort_order 升序、id 升序），
 *    父级缺席或成环而从任何根都走不到的节点由后端按根节点补回，出参覆盖全表每一行；
 * 2) 前端 DepartmentTreeNode 声明键集合与后端构造点键集合精确相等（差集双侧均为空），
 *    children 必须双侧同型递归（TS: DepartmentTreeNode[]；Rust: Vec<DepartmentTreeNode>），
 *    且不得声明后端树节点从不输出的 manager_id/created_at/updated_at；
 * 3) 列表/详情载体 department::Model 与树节点是两个不同类型：Department 接口键集合
 *    ⊆ Model 字段名集合（children 是树专属形态，不得回流到 Department）；
 * 4) 消费视图（views/departments/index.vue）：树出参按裸数组真源一次取全量并直渲，
 *    不得残留按分页信封凑全量的兜法（列表端点默认页宽会把页面静默削成前若干条），
 *    不得用 items/data 兜底链或 res.data as {} 断言掩盖信封形态漂移；表格数据源与上级
 *    tree-select 数据源同为一棵树出参，类型必须是 DepartmentTreeNode[]；表格列与行操作
 *    只绑树节点真实键；前端不得自行排序（定序是后端契约）；编辑初值唯一真源 =
 *    详情端点 getDepartment(row.id)，禁止直读行载体造成改动静默丢失。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内源码文本，不执行后端代码、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、CI 检出为 LF，
 * 本锁的字面量夹具与逐行正则对行尾形态敏感，不归一会让检测力自证被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const TREE_SERVICE_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/department_service.rs')
);
const TREE_HANDLER_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/handlers/department_handler.rs')
);
const DEPT_MODEL_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/models/department.rs'));
const DEPT_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/department.ts'));
const DEPT_PAGE_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/views/departments/index.vue'));

/** 树节点上后端从不输出、曾以前端假字段形态出现过的键：声明或树行绑定中出现即判红 */
const TREE_GHOST_KEYS = ['manager_id', 'created_at', 'updated_at'] as const;

/** 截取 TS interface 正文；接口被删除或改名即抛错（防判据静默失去覆盖） */
function tsInterfaceBody(src: string, name: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`TS 源码中未找到 interface ${name}——结构已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 提取 TS 接口块内的声明键 */
function tsInterfaceKeys(src: string, name: string): string[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}(?:readonly\s+)?([a-z][A-Za-z0-9_]*)\??\s*:/gm)].map(m => m[1]);
}

/** 截取 Rust struct 正文；结构体改名/删除即抛错（防判据静默失效） */
function rustStructBody(src: string, name: string): string {
  const start = src.indexOf(`pub struct ${name} {`);
  if (start === -1) {
    throw new Error(`后端源码中未找到 pub struct ${name}——载体已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`pub struct ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

/** 提取 Rust struct 的字段名（= serde snake_case 序列化键名，两载体均无 rename/rename_all） */
function rustStructFields(body: string): string[] {
  return [...body.matchAll(/pub\s+([a-z_][a-z0-9_]*)\s*:/g)].map(m => m[1]);
}

const treeBackendKeys = rustStructFields(rustStructBody(TREE_SERVICE_SRC, 'DepartmentTreeNode'));
const treeFrontendKeys = tsInterfaceKeys(DEPT_API_SRC, 'DepartmentTreeNode');
const modelBackendKeys = rustStructFields(rustStructBody(DEPT_MODEL_SRC, 'Model'));
const listFrontendKeys = tsInterfaceKeys(DEPT_API_SRC, 'Department');

/**
 * 提取模板中 el-table-column 标签的 prop 值（行数据字段绑定）。
 * 只认表格列标签：el-form-item 的 prop 是校验字段名（表单输入合法载体），不得混为一谈误判。
 */
function tableColumnProps(tplSrc: string): string[] {
  return [...tplSrc.matchAll(/<el-table-column\b[^>]*>/gs)]
    .map(tag => tag[0].match(/\sprop="([^"]+)"/)?.[1])
    .filter((p): p is string => Boolean(p));
}

describe('部门树出参形状锁（后端构造点 ↔ 前端声明 ↔ 消费视图）', () => {
  it('后端事实基准：树端点出参 ApiResponse<Vec<DepartmentTreeNode>>，节点九键、children 递归、键恒存在', () => {
    // 处理器签名逐字钉住：data 载荷就是裸数组 Vec<DepartmentTreeNode>
    expect(TREE_HANDLER_SRC).toContain('Result<Json<ApiResponse<Vec<DepartmentTreeNode>>>');
    // 树节点键集合精确等于九键（新增/删除后端字段将在此判红，须同步修订本锁与前端声明）
    expect(treeBackendKeys.sort()).toEqual(
      [
        'id',
        'name',
        'code',
        'description',
        'parent_id',
        'manager_name',
        'sort_order',
        'is_active',
        'children',
      ].sort()
    );
    // 键数地板：解析失效（空集/半集）判红
    expect(treeBackendKeys.length).toBeGreaterThanOrEqual(9);
    // children 递归自引用且无 skip_serializing_if：序列化后键恒存在（叶子为 []）
    expect(TREE_SERVICE_SRC).toMatch(/pub children: Vec<DepartmentTreeNode>/);
    expect(rustStructBody(TREE_SERVICE_SRC, 'DepartmentTreeNode')).not.toContain(
      'skip_serializing_if'
    );
    // manager_name 为展示瞬态字段：Model 上 #[sea_orm(ignore)]，由 service 查 users 回填，非 departments 列
    expect(DEPT_MODEL_SRC).toMatch(/#\[sea_orm\(ignore\)\]\s*pub manager_name: Option<String>/);
    expect(TREE_SERVICE_SRC).toContain('async fn fill_manager_names');
    // 幽灵键在后端树节点侧确实不存在（证明前端删绑的是"无此出参键"，不是解析失效）
    for (const key of TREE_GHOST_KEYS) {
      expect(treeBackendKeys, `DepartmentTreeNode 不应含键 ${key}`).not.toContain(key);
    }
    // 表格直渲树出参的前提：三列逐行取自列表/详情同一实体，不是为凑列而造的假值
    expect(TREE_SERVICE_SRC).toContain('code: dept.code,');
    expect(TREE_SERVICE_SRC).toContain('sort_order: dept.sort_order,');
    expect(TREE_SERVICE_SRC).toContain('is_active: dept.is_active,');
    // 显式定序：dept_map 是 HashMap，迭代序不保证，根级与每一层 children 都按 sort_order 升序、
    // id 升序的同一份全局顺序排列，并列 sort_order 时由 id 兜底成全序，前端直渲才不跳变
    expect(TREE_SERVICE_SRC).toContain(
      'order_keys.sort_unstable_by_key(|&(id, sort_order)| (sort_order, id));'
    );
    // 分页排序须构成全序：只按 name 排时同名行的先后不保证，offset 翻页会在页边界重复取与漏取
    expect(TREE_SERVICE_SRC).toMatch(
      /order_by\(department::Column::Name, Order::Asc\)\s*\.order_by\(\s*department::Column::Id, Order::Asc\s*\)/
    );
    // 出参覆盖全表每一行：从任何根都走不到的节点（父级缺席或成环）按根节点补回，不静默丢树
    expect(TREE_SERVICE_SRC).toContain('known_dept_ids');
  });

  it('前端声明：DepartmentTreeNode 键集合与后端精确相等，children 必填递归，getDepartmentTree 返回真型', () => {
    // 键数地板
    expect(treeFrontendKeys.length).toBeGreaterThanOrEqual(9);
    // 声明键集合 ⊆ 后端构造点键集合（本轮收紧：差集双侧均为空 = 精确相等）
    expect(treeFrontendKeys.filter(k => !treeBackendKeys.includes(k))).toEqual([]);
    expect(treeBackendKeys.filter(k => !treeFrontendKeys.includes(k))).toEqual([]);
    for (const key of TREE_GHOST_KEYS) {
      expect(treeFrontendKeys, `DepartmentTreeNode 不得声明后端不输出的键 ${key}`).not.toContain(
        key
      );
    }
    // children：后端键恒存在 ⇒ TS 必填（可选标记会授权消费点对必然存在的键写缺席分支）
    expect(tsInterfaceBody(DEPT_API_SRC, 'DepartmentTreeNode')).toMatch(
      /^ {2}children: DepartmentTreeNode\[\];$/m
    );
    expect(tsInterfaceBody(DEPT_API_SRC, 'DepartmentTreeNode')).not.toMatch(/children\?/);
    // 函数出参类型钉住真实信封形态：ApiResponse<Vec<...>> ⇒ data 裸数组
    expect(DEPT_API_SRC).toMatch(
      /function getDepartmentTree\(\): Promise<ApiResponse<DepartmentTreeNode\[\]>>/
    );
  });

  it('列表载体分型：Department 键集合 ⊆ department::Model 字段集，children 不得回流', () => {
    expect(modelBackendKeys.sort()).toEqual(
      [
        'id',
        'name',
        'code',
        'parent_id',
        'manager_id',
        'manager_name',
        'description',
        'sort_order',
        'is_active',
        'created_at',
        'updated_at',
      ].sort()
    );
    expect(listFrontendKeys.length).toBeGreaterThanOrEqual(11);
    expect(listFrontendKeys.filter(k => !modelBackendKeys.includes(k))).toEqual([]);
    // children 只在树载体存在：列表/详情/创建/更新出参（Model 直序列化）从不输出该键
    expect(listFrontendKeys).not.toContain('children');
  });

  it('消费视图（部门管理页）：一次取树零分页兜法、编辑初值走详情端点、表格列只绑树载体真实键', () => {
    // (a) 树出参按裸数组真源赋值：tree-select 的上级数据源来自 /departments/tree（data 恒裸数组），
    //     被改成 items 兜底链或 res.data as {} 断言即信封形态漂移被掩盖，判红
    expect(DEPT_PAGE_SRC).toContain('deptTreeData.value = treeRes.data;');
    expect(DEPT_PAGE_SRC).not.toMatch(/treeRes\.data as \{/);
    expect(DEPT_PAGE_SRC).not.toMatch(/deptTreeData\.value = .*\.items/);
    expect(DEPT_PAGE_SRC).not.toContain('.items ||');
    // (b) 上级 tree-select 数据源类型必须是树节点 DepartmentTreeNode[]，退化成列表型 Department[] 即载体漂移
    expect(DEPT_PAGE_SRC).toMatch(/deptTreeData\s*=\s*ref<DepartmentTreeNode\[\]>/);
    // (c) 编辑初值唯一真源 = 详情端点 getDepartment(row.id)（department::Model 全列出参）：
    //     直读列表/树行载体在字段缺项时会造成改动于提交时静默丢失，禁止以行字段凑表单
    expect(DEPT_PAGE_SRC).toContain('getDepartment(row.id)');
    // (d) 全量真相：数据源是一次取回的后端树出参（无分页信封，天然覆盖全表每一行）。
    //     列表端点默认页宽 10、clamp 上限 100，而本页没有任何翻页控件，
    //     任何"按列表端点凑全量"的兜法回流都会在部门数一多时静默丢功能，故这里钉的是兜法零残留。
    expect(DEPT_PAGE_SRC).toContain('const treeRes = await getDepartmentTree();');
    expect(DEPT_PAGE_SRC).not.toMatch(/\bgetDepartmentList\b/);
    expect(DEPT_PAGE_SRC).not.toMatch(/\bpage_size\b/);
    expect(DEPT_PAGE_SRC).not.toMatch(/\bloadAllDepartments\b/);
    expect(DEPT_PAGE_SRC).not.toMatch(/\bDEPARTMENT_PAGE_SIZE\b/);
    expect(DEPT_PAGE_SRC).not.toMatch(/\bdepartmentList\b/);
    expect(DEPT_PAGE_SRC).not.toMatch(/\.data\.total/);
    // 定序是后端契约：前端再排一次会覆盖后端 sort_order,id 全序，回流即判红
    expect(DEPT_PAGE_SRC).not.toMatch(/\.sort\(/);
    // (e) 层级渲染：表格数据源就是后端树出参本身（自带 children），并带 el-table 层级三属性
    expect(DEPT_PAGE_SRC).toMatch(/:data="deptTreeData"/);
    expect(DEPT_PAGE_SRC).toContain('row-key="id"');
    expect(DEPT_PAGE_SRC).toContain('default-expand-all');
    expect(DEPT_PAGE_SRC).toContain(`:tree-props="{ children: 'children' }"`);
    // 前端本地组装层级的旧形态不得回流：children 由后端给出，本地再按 parent_id 挂一次会挂双份
    expect(DEPT_PAGE_SRC).not.toMatch(/buildDepartmentRows/);
    // (f) 上级列真实语义：parent_id 为 null 显示"无上级"，上级在树出参里查不到时显示可区分文案，
    //     不得把"上级取不到"塌回"无上级"（那会把确有父级的部门谎报成顶级部门）。
    //     父级名必须取自递归扁平化出的全量映射：树形下父级可位于任意一层，只查当前层必查不到
    expect(DEPT_PAGE_SRC).toContain('departments.index.placeholderNoParent');
    expect(DEPT_PAGE_SRC).toMatch(/const parentName = deptNameById\.value\.get\(parentId\);/);
    expect(DEPT_PAGE_SRC).toMatch(
      /return parentName \?\? t\('departments\.index\.parentNotInArchive'\);/
    );
    expect(DEPT_PAGE_SRC).toContain('if (node.children.length > 0) walk(node.children);');
    // 表格列 prop 与行操作入参必须是树载体真实键：数据源已是 DepartmentTreeNode，
    // 绑定后端树出参从不输出的键会让该列恒空，按列表型收行载体则会在假字段上取值
    for (const p of tableColumnProps(DEPT_PAGE_SRC)) {
      expect(treeFrontendKeys, `表格列绑定键 ${p} 不在 DepartmentTreeNode 载体中`).toContain(p);
    }
    expect(DEPT_PAGE_SRC).toContain('handleEdit(row as DepartmentTreeNode)');
    expect(DEPT_PAGE_SRC).toContain('handleDelete(row as DepartmentTreeNode)');
  });

  it('检测力自证：复现历史假字段与降级形态的夹具必被上述判据抓到（防判据静默失效）', () => {
    // 夹具 A：把后端树节点从不输出的 manager_id 声明回 DepartmentTreeNode，
    // 差集判据与幽灵键名单两道锁都要抓到
    const mutatedIface = DEPT_API_SRC.replace(
      '  /** DB 可空列 description：后端出参键恒存在，真实空值为 null */',
      '  manager_id: number | null;\n  /** DB 可空列 description：后端出参键恒存在，真实空值为 null */'
    );
    expect(mutatedIface, '夹具 A 替换本身必须生效').not.toBe(DEPT_API_SRC);
    const mutatedKeys = tsInterfaceKeys(mutatedIface, 'DepartmentTreeNode');
    expect(mutatedKeys.filter(k => !treeBackendKeys.includes(k))).toContain('manager_id');
    expect(TREE_GHOST_KEYS, 'manager_id 须同时在幽灵键名单内').toContain('manager_id');
    // 夹具 A2：后端从树出参摘掉一列（此处 code）时，前端声明必须同步收缩，精确相等判据要抓到
    const backendWithoutCode = treeBackendKeys.filter(k => k !== 'code');
    expect(
      treeFrontendKeys.filter(k => !backendWithoutCode.includes(k)),
      '后端少出 code 而前端仍声明必须判红'
    ).toContain('code');
    // 夹具 B：children 改可选（纵容缺席分支），必填判据必须抓到
    const mutatedChildren = DEPT_API_SRC.replace(
      '  children: DepartmentTreeNode[];',
      '  children?: DepartmentTreeNode[];'
    );
    expect(mutatedChildren, '夹具替换本身必须生效').not.toBe(DEPT_API_SRC);
    expect(tsInterfaceBody(mutatedChildren, 'DepartmentTreeNode')).not.toMatch(
      /^ {2}children: DepartmentTreeNode\[\];$/m
    );
    // 夹具 C：把编辑初值改回直读列表行（去掉详情端点取数），(c) 判据必须抓到
    const mutatedEdit = DEPT_PAGE_SRC.replace('getDepartment(row.id)', 'row');
    expect(mutatedEdit, '夹具替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(mutatedEdit, '编辑初值回流成以行载体凑表单必须被 (c) 判据抓到').not.toContain(
      'getDepartment(row.id)'
    );
    // 夹具 C2：把表格列 prop 改成树载体从不输出的假键，表格列绑定判据必须抓到
    const mutatedCol = DEPT_PAGE_SRC.replace('prop="code"', 'prop="nickname"');
    expect(mutatedCol, '夹具替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(tableColumnProps(mutatedCol), '假键列回流必须被表格列绑定判据抓到').toContain(
      'nickname'
    );
    expect(treeFrontendKeys, '假键不得存在于 DepartmentTreeNode 载体').not.toContain('nickname');
    // 夹具 D：children 递归改指列表型（漂回 Department[]），递归同型判据必须抓到
    const mutatedRecur = DEPT_API_SRC.replace(
      '  children: DepartmentTreeNode[];',
      '  children: Department[];'
    );
    expect(mutatedRecur, '夹具替换本身必须生效').not.toBe(DEPT_API_SRC);
    expect(mutatedRecur).not.toMatch(/children: DepartmentTreeNode\[\]/);
    // 反向自证：后端结构体块改名时解析必须抛错而非静默空集
    expect(() =>
      rustStructFields(rustStructBody('pub struct Renamed {}\n', 'DepartmentTreeNode'))
    ).toThrow();
    // 夹具 E：数据源退回按列表端点凑全量（分页兜法回流，页面没有翻页控件时部门一多就静默丢），
    // (d) 的兜法零残留判据必须抓到
    const mutatedPaging = DEPT_PAGE_SRC.replace(
      'const treeRes = await getDepartmentTree();',
      'const treeRes = await getDepartmentList({ page: 1, page_size: DEPARTMENT_PAGE_SIZE });'
    );
    expect(mutatedPaging, '夹具 E 替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(mutatedPaging, '分页兜法回流必须被 (d) 抓到').toMatch(/\bpage_size\b/);
    // 夹具 E2：页面自行排序（覆盖后端 sort_order,id 全序），(d) 的前端不排序判据必须抓到
    const mutatedSort = DEPT_PAGE_SRC.replace(
      'deptTreeData.value = treeRes.data;',
      'deptTreeData.value = [...treeRes.data].sort((a, b) => a.sort_order - b.sort_order);'
    );
    expect(mutatedSort, '夹具 E2 替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(mutatedSort, '前端自行排序必须被 (d) 抓到').toMatch(/\.sort\(/);
    // 夹具 F：去掉层级展开属性，(e) 的 default-expand-all 判据必须失去命中
    const mutatedNoExpand = DEPT_PAGE_SRC.replace('default-expand-all', '');
    expect(mutatedNoExpand, '夹具 F 替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(mutatedNoExpand, '丢失层级展开属性必须被 (e) 抓到').not.toContain('default-expand-all');
    // 夹具 G：上级列查不到时塌回"无上级"，(f) 的可区分文案判据必须失去命中
    const mutatedParentFallback = DEPT_PAGE_SRC.replace(
      "return parentName ?? t('departments.index.parentNotInArchive');",
      "return parentName ?? t('departments.index.placeholderNoParent');"
    );
    expect(mutatedParentFallback, '夹具 G 替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(
      mutatedParentFallback.match(
        /return parentName \?\? t\('departments\.index\.parentNotInArchive'\);/
      ),
      '上级列塌回无上级必须被 (f) 抓到'
    ).toBeNull();
    // 夹具 H：父级名映射不再递归 children（树形下非根父级必查不到），(f) 的扁平化判据失去命中
    const mutatedNoWalk = DEPT_PAGE_SRC.replace(
      'if (node.children.length > 0) walk(node.children);',
      ''
    );
    expect(mutatedNoWalk, '夹具 H 替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(mutatedNoWalk, '扁平化丢失递归必须被 (f) 抓到').not.toContain(
      'if (node.children.length > 0) walk(node.children);'
    );
    // 夹具 I：表格行操作改回按列表载体收行（会在树节点不输出的假字段上取值），(f) 的载体判据失去命中
    const mutatedRowType = DEPT_PAGE_SRC.replace(
      'handleEdit(row as DepartmentTreeNode)',
      'handleEdit(row as Department)'
    );
    expect(mutatedRowType, '夹具 I 替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(mutatedRowType, '行操作载体漂回列表型必须被 (f) 抓到').not.toContain(
      'handleEdit(row as DepartmentTreeNode)'
    );
    // 夹具 J：树出参改用 items 兜底链（把信封形态漂移掩盖成"取不到就空"），(a) 的 not 判据必须抓到
    const mutatedItems = DEPT_PAGE_SRC.replace(
      'deptTreeData.value = treeRes.data;',
      'deptTreeData.value = treeRes.data.items || [];'
    );
    expect(mutatedItems, '夹具 J 替换本身必须生效').not.toBe(DEPT_PAGE_SRC);
    expect(mutatedItems, 'items 兜底链回流必须被 (a) 抓到').toContain('.items ||');
  });
});
