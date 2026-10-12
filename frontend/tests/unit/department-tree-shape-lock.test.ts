/**
 * 部门树出参形状锁（后端构造点 ↔ 前端声明 ↔ 消费视图三向互证）。
 * 功能：
 * 1) GET /departments/tree 的节点载体 = 后端 services/department_service.rs 的
 *    DepartmentTreeNode 结构体（Serialize、无 rename），出参键集恒为
 *    id/name/description/parent_id/manager_name/children 六键，处理器出参为
 *    ApiResponse<Vec<DepartmentTreeNode>>（data 裸数组，无分页信封）；
 * 2) 前端 DepartmentTreeNode 声明键集合 ⊆ 后端构造点键集合（本锁收紧为精确相等），
 *    children 必须双侧同型递归（TS: DepartmentTreeNode[]；Rust: Vec<DepartmentTreeNode>），
 *    且不得声明后端树节点从不输出的 code/sort_order/is_active/manager_id/created_at/updated_at；
 * 3) 列表/详情载体 department::Model 与树节点是两个不同类型：Department 接口键集合
 *    ⊆ Model 字段名集合（children 是树专属形态，不得回流到 Department）；
 * 4) 消费视图不得在树行上绑定无载体字段，树响应不得用 items/data 兜底链掩盖信封形态漂移。
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
const DEPT_TAB_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/system/tabs/DepartmentTab.vue')
);

/** 树节点上后端从不输出、曾以前端假字段形态出现过的键：声明或树行绑定中出现即判红 */
const TREE_GHOST_KEYS = [
  'code',
  'sort_order',
  'is_active',
  'manager_id',
  'created_at',
  'updated_at',
] as const;

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
  it('后端事实基准：树端点出参 ApiResponse<Vec<DepartmentTreeNode>>，节点六键、children 递归、键恒存在', () => {
    // 处理器签名逐字钉住：data 载荷就是裸数组 Vec<DepartmentTreeNode>
    expect(TREE_HANDLER_SRC).toContain('Result<Json<ApiResponse<Vec<DepartmentTreeNode>>>');
    // 树节点键集合精确等于六键（新增/删除后端字段将在此判红，须同步修订本锁与前端声明）
    expect(treeBackendKeys.sort()).toEqual(
      ['id', 'name', 'description', 'parent_id', 'manager_name', 'children'].sort()
    );
    // 键数地板：解析失效（空集/半集）判红
    expect(treeBackendKeys.length).toBeGreaterThanOrEqual(6);
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
  });

  it('前端声明：DepartmentTreeNode 键集合与后端精确相等，children 必填递归，getDepartmentTree 返回真型', () => {
    // 键数地板
    expect(treeFrontendKeys.length).toBeGreaterThanOrEqual(6);
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

  it('消费视图：树响应零兜底链、树行不绑无载体字段、编辑初值走详情端点真源', () => {
    // DepartmentTab：整表数据源是树裸数组，直读 res.data；items/data 兜底链即信封形态掩盖，判红
    expect(DEPT_TAB_SRC).not.toContain('.items ||');
    expect(DEPT_TAB_SRC).not.toMatch(/res\.data as \{/);
    expect(DEPT_TAB_SRC).toContain('departments.value = res.data;');
    expect(DEPT_TAB_SRC).toContain('ref<DepartmentTreeNode[]>');
    // 树行上无载体 ⇒ 列已删（el-table-column 的 prop 行绑定出现即假展示回流；
    // 编辑对话框的 el-form-item prop="code" 是校验字段名，走详情端点真源填充，属合法绑定不在禁列）
    const tabColumnProps = tableColumnProps(DEPT_TAB_SRC);
    for (const key of ['code', 'sort_order', 'is_active']) {
      expect(tabColumnProps, `树表格列不得再绑定无载体字段 ${key}`).not.toContain(key);
      expect(DEPT_TAB_SRC, `树行单元格不得再读取 row.${key}`).not.toContain(`row.${key}`);
    }
    // 列数定形：name + manager_name + 操作列 = 3
    const tabColumns = (DEPT_TAB_SRC.match(/<el-table-column/g) || []).length;
    expect(tabColumns).toBe(3);
    // 编辑初值唯一真源 = 详情端点（Model 全列出参），不得从树行读缺失字段凑表单
    expect(DEPT_TAB_SRC).toContain('getDepartment(row.id)');
    // departments/index.vue：树数据仅作上级 tree-select 源，类型必须是树节点；列表列数据源为 list API 不受影响
    expect(DEPT_PAGE_SRC).toContain('ref<DepartmentTreeNode[]>');
    expect(DEPT_PAGE_SRC).toContain('deptTreeData.value = treeRes.data;');
  });

  it('检测力自证：复现历史假字段形态的夹具必被上述判据抓到（防判据静默失效）', () => {
    // 夹具 A：把后端不输出的 code 键加回 DepartmentTreeNode，子集判据必须抓到
    const mutatedIface = DEPT_API_SRC.replace(
      '  name: string;\n  /** DB 可空列 description：后端出参键恒存在，真实空值为 null */',
      '  name: string;\n  code: string;\n  /** DB 可空列 description：后端出参键恒存在，真实空值为 null */'
    );
    expect(mutatedIface, '夹具替换本身必须生效').not.toBe(DEPT_API_SRC);
    const mutatedKeys = tsInterfaceKeys(mutatedIface, 'DepartmentTreeNode');
    expect(mutatedKeys.filter(k => !treeBackendKeys.includes(k))).toContain('code');
    // 夹具 B：children 改可选（纵容缺席分支），必填判据必须抓到
    const mutatedChildren = DEPT_API_SRC.replace(
      '  children: DepartmentTreeNode[];',
      '  children?: DepartmentTreeNode[];'
    );
    expect(mutatedChildren, '夹具替换本身必须生效').not.toBe(DEPT_API_SRC);
    expect(tsInterfaceBody(mutatedChildren, 'DepartmentTreeNode')).not.toMatch(
      /^ {2}children: DepartmentTreeNode\[\];$/m
    );
    // 夹具 C：把无载体列写回 DepartmentTab 树行，绑定判据必须抓到
    const mutatedView = DEPT_TAB_SRC.replace(
      '<el-table-column prop="name"',
      '<el-table-column prop="code" />\n        <el-table-column prop="name"'
    );
    expect(mutatedView, '夹具替换本身必须生效').not.toBe(DEPT_TAB_SRC);
    expect(tableColumnProps(mutatedView), '无载体列回流必须被表格列绑定判据抓到').toContain('code');
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
  });
});
