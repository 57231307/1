/**
 * BPM 流程定义表单产出的 config 载荷锁（节点类型词表 + edges 形状）
 *
 * 背景缺陷：表单曾提交 start/approval/condition/notify/end 等引擎不识别的节点类型，
 * 且 config 只带 nodes 不带 edges——审批引擎判定值逐字符为
 * start_event / user_task / end_event（后端 resolve_first_task_node 与
 * try_advance_to_next_node 只认这三个值），推进时从 config.edges 找匹配边
 * （try_advance_to_next_node 按当前节点 id 匹配 source），缺 edges 则首个节点批准后
 * 流程直接完成 = 运行时绕过审批。
 *
 * 本用例对 composable 真实产出的 payload 做结构断言：
 *   正向：节点类型序列与引擎 token 严格相等；edges 非空、相邻线性配对、
 *         source/target 引用真实节点 id、无条件边不携带 condition 键、
 *         条件只挂边不挂节点（节点级条件键被 toPersistedNodes 剥离）；
 *         start_event 恰一个且其首条出边 target 为 user_task（引擎不校验边
 *         target 类型，见 resolve_first_task_node 直接取首出边目标，首位错位会把非审批节点当首任务）；
 *         user_task 的审批人类型限引擎词表 user / dept_superior：user 须 assignee_value
 *         为数字用户 ID，dept_superior 运行期按部门上级解析、不需选人（后端 resolve_node_assignee）。
 *   反向：payload 中不得出现引擎不认的旧裸词（approval/condition/notify/start/end），
 *         节点上不得出现引擎从不读取的 condition / next_nodes 键——
 *         任何一处把代码改回旧词表或删掉 edges，对应断言立即失败（检测力由
 *         严格相等 + 枚举禁止自证，不依赖子串巧合）。
 * validateLinearChain 纯函数与 handleSubmit 拦截各有独立用例：
 * 旧词/乱序/缺审批人的节点集必须被拒绝提交（引擎 fail-closed 前移为前端显式报错，
 * 界面顺序即审批顺序，多审批节点线性依次推进）。
 * 另对表单源码做静态扫描：节点类型下拉的提交值必须是且仅是三个引擎 token；
 * 审批人列必须是可搜索用户选择（el-select），不得回到手填 ID 的 el-input。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { createBpmDefinition, updateBpmDefinition, type ProcessNode } from '@/api/bpm-enhanced';
import { getUserList } from '@/api/user';
import {
  fetchUserCandidates,
  ASSIGNEE_SEARCH_PAGE_SIZE,
} from '@/views/bpm/definitions/composables/useAssigneeSearch';
import {
  useBpmDfProc,
  buildLinearEdges,
  validateLinearChain,
  toPersistedNodes,
} from '@/views/bpm/definitions/composables/useBpmDfProc';

vi.mock('@/api/bpm-enhanced', () => ({
  createBpmDefinition: vi.fn().mockResolvedValue({ data: {} }),
  updateBpmDefinition: vi.fn().mockResolvedValue({ data: {} }),
  deleteBpmDefinition: vi.fn().mockResolvedValue({ data: null }),
  getBpmDefinitionById: vi.fn().mockResolvedValue({ data: null }),
  createBpmVersion: vi.fn().mockResolvedValue({ data: {} }),
  activateBpmVersion: vi.fn().mockResolvedValue({ data: null }),
  saveBpmAsTemplate: vi.fn().mockResolvedValue({ data: {} }),
}));

vi.mock('@/api/user', () => ({
  getUserList: vi.fn().mockResolvedValue({
    data: {
      users: [{ id: 3, username: 'zhang3', real_name: '张三' }],
      total: 1,
      page: 1,
      page_size: 50,
    },
  }),
}));

// 以启动目录定位 frontend 根：CI 与 npm test 都在 frontend/ 下运行
// （不能用 import.meta.url —— jsdom 下非 file: 协议会抛 ERR_INVALID_URL_SCHEME）。
const FRONTEND = process.cwd();
const SRC = join(FRONTEND, 'src');
if (!existsSync(join(SRC, 'views'))) {
  throw new Error(
    `该用例需在 frontend 目录下运行以扫描 src/**/*.vue，当前目录 ${FRONTEND} 内没有 src/views`
  );
}

/** 引擎逐字符识别的节点类型（写入端真源见文件头注释指向的后端函数） */
const ENGINE_NODE_TYPES = ['start_event', 'user_task', 'end_event'];
/** 本用例构造的节点序列对应的类型链（两个审批节点夹在 start/end 之间） */
const EXPECTED_TYPE_CHAIN = ['start_event', 'user_task', 'user_task', 'end_event'];
/** 引擎无任何执行路径的旧裸词：出现在 payload 即为运行时绕过审批级缺陷 */
const ENGINE_UNKNOWN_TOKENS = ['start', 'end', 'approval', 'condition', 'notify'];

function makeNodes(): ProcessNode[] {
  return [
    { id: 'n_start', type: 'start_event', name: 'start-node' },
    {
      id: 'n_appr1',
      type: 'user_task',
      name: 'approval-1',
      assignee_type: 'user',
      assignee_value: 3,
    },
    {
      id: 'n_appr2',
      type: 'user_task',
      name: 'approval-2',
      assignee_type: 'user',
      assignee_value: 5,
    },
    { id: 'n_end', type: 'end_event', name: 'end-node' },
  ];
}

/** 构造与 useBpmDf(reactive 包装层) 同构的回调状态（结构由 BpmDfCallbacks 上下文推断） */
function makeCb(isEdit: boolean, nodes: ProcessNode[] = makeNodes()) {
  return {
    definitions: [],
    loading: false,
    total: 0,
    page: 1,
    queryParams: {},
    dialogVisible: false,
    isEdit,
    submitLoading: false,
    formData: {
      id: isEdit ? 7 : undefined,
      process_key: 'leave_flow',
      process_name: 'leave-approval-flow',
      description: '',
      category: 'hr',
      nodes,
    },
    versionDialogVisible: false,
    versionLoading: false,
    currentDefinition: null,
    versions: [],
    templateDialogVisible: false,
    templateLoading: false,
    templateForm: { template_name: '', category: 'hr', description: '' },
    fetchDefinitions: vi.fn().mockResolvedValue(undefined),
    fetchVersions: vi.fn().mockResolvedValue(undefined),
  };
}

describe('buildLinearEdges：按节点顺序线性生成边', () => {
  it('相邻两两配对，键只有 source/target（无条件边省略 condition 键）', () => {
    expect(buildLinearEdges(makeNodes())).toEqual([
      { source: 'n_start', target: 'n_appr1' },
      { source: 'n_appr1', target: 'n_appr2' },
      { source: 'n_appr2', target: 'n_end' },
    ]);
  });

  it('空数组与单节点不产生边', () => {
    expect(buildLinearEdges([])).toEqual([]);
    expect(buildLinearEdges([makeNodes()[0]])).toEqual([]);
  });
});

describe('handleSubmit：提交 payload 的 config 词表与 edges 形状锁定', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('新建：createBpmDefinition 收到 config.nodes（引擎词表）+ 非空线性 edges', async () => {
    const cb = makeCb(false);
    const proc = useBpmDfProc(cb);
    await proc.handleSubmit();

    expect(createBpmDefinition).toHaveBeenCalledTimes(1);
    const payload = vi.mocked(createBpmDefinition).mock.calls[0][0];
    const config = payload.config;
    expect(config).toBeDefined();

    // 正向：节点类型序列与引擎 token 严格相等（改回旧词表此处即失败）
    expect(config!.nodes!.map(n => n.type)).toEqual(EXPECTED_TYPE_CHAIN);
    // 正向：edges 非空且相邻线性配对（漏提交 edges 此处即失败）
    expect(config!.edges).toEqual([
      { source: 'n_start', target: 'n_appr1' },
      { source: 'n_appr1', target: 'n_appr2' },
      { source: 'n_appr2', target: 'n_end' },
    ]);
    // 边引用的 source/target 必须是真实存在的节点 id（否则引擎匹配不到推进边）
    const ids = new Set(config!.nodes!.map(n => n.id));
    for (const edge of config!.edges!) {
      expect(Object.keys(edge).sort()).toEqual(['source', 'target']);
      expect(ids.has(edge.source)).toBe(true);
      expect(ids.has(edge.target)).toBe(true);
    }

    // 反向：不得出现引擎不认的旧裸词
    for (const node of config!.nodes!) {
      // 正向白名单：每个类型都必须是引擎识别的 token
      expect(ENGINE_NODE_TYPES).toContain(node.type);
      for (const banned of ENGINE_UNKNOWN_TOKENS) {
        expect(node.type).not.toBe(banned);
      }
      // 反向：引擎从不读取节点级 condition/next_nodes（条件挂在 edge.condition），
      // 表单编辑态的 edge_condition 也必须被 toPersistedNodes 剥离后才进 config.nodes
      expect(node).not.toHaveProperty('condition');
      expect(node).not.toHaveProperty('next_nodes');
      expect(node).not.toHaveProperty('edge_condition');
      // 反向：审批人类型只允许引擎词表 user / dept_superior（后端 resolve_node_assignee）
      if (node.assignee_type !== undefined) {
        expect(['user', 'dept_superior']).toContain(node.assignee_type);
      }
    }

    // 正向（引擎首任务解析不变量，后端 resolve_first_task_node 不校验边 target 类型）：
    // start_event 恰一个，其首条出边的 target 必须是 user_task 节点
    const starts = config!.nodes!.filter(n => n.type === 'start_event');
    expect(starts).toHaveLength(1);
    const firstOutEdge = config!.edges!.find(e => e.source === starts[0].id);
    expect(firstOutEdge).toBeDefined();
    const firstTask = config!.nodes!.find(n => n.id === firstOutEdge!.target);
    expect(firstTask).toBeDefined();
    expect(firstTask!.type).toBe('user_task');
    // 正向：每个审批节点的审批人是数字用户 ID（后端 resolve_node_assignee 须解析为正整数）
    for (const node of config!.nodes!.filter(n => n.type === 'user_task')) {
      expect(node.assignee_type).toBe('user');
      expect(typeof node.assignee_value).toBe('number');
      expect(Number.isInteger(node.assignee_value)).toBe(true);
      expect(node.assignee_value! > 0).toBe(true);
    }
    // 反向：序列化 payload 的任何键值里也不得出现旧裸词充当类型（引号闭合避免
    // 与 start_event/end_event 子串误判）
    expect(JSON.stringify(config)).not.toMatch(/"type":"(?:start|end|approval|condition|notify)"/);
  });

  it('编辑：updateBpmDefinition 同样带 config.edges', async () => {
    const cb = makeCb(true);
    const proc = useBpmDfProc(cb);
    await proc.handleSubmit();

    expect(updateBpmDefinition).toHaveBeenCalledTimes(1);
    const [id, payload] = vi.mocked(updateBpmDefinition).mock.calls[0];
    expect(id).toBe(7);
    expect(payload.config!.edges).toHaveLength(3);
    expect(payload.config!.nodes!.map(n => n.type)).toEqual(EXPECTED_TYPE_CHAIN);
  });
});

describe('审批人候选远程检索（keyword 收窄，不冒充全量）', () => {
  it('调用参数带 keyword、page=1、page_size=50（后端 clamp(1,100) 之下的如实取法）', async () => {
    vi.clearAllMocks();
    const users = await fetchUserCandidates('zhang');
    expect(getUserList).toHaveBeenCalledTimes(1);
    expect(vi.mocked(getUserList).mock.calls[0][0]).toEqual({
      keyword: 'zhang',
      page: 1,
      page_size: 50,
    });
    expect(ASSIGNEE_SEARCH_PAGE_SIZE).toBe(50);
    expect(users).toEqual([{ id: 3, username: 'zhang3', real_name: '张三' }]);
  });

  it('候选检索失败原样上抛：不吞错、不以空列表兜底放行提交', async () => {
    vi.clearAllMocks();
    const boom = new Error('403 列出用户列表仅限管理员');
    vi.mocked(getUserList).mockRejectedValueOnce(boom);
    await expect(fetchUserCandidates('x')).rejects.toThrow('403 列出用户列表仅限管理员');
  });
});

describe('validateLinearChain：引擎可执行不变量的提交前拦截', () => {
  it('合法线性链通过校验（返回 null）', () => {
    expect(validateLinearChain(makeNodes())).toBeNull();
    // 单审批节点最小链同样合法
    expect(
      validateLinearChain([
        { id: 's', type: 'start_event', name: 's' },
        { id: 'a', type: 'user_task', name: 'a', assignee_type: 'user', assignee_value: 1 },
        { id: 'e', type: 'end_event', name: 'e' },
      ])
    ).toBeNull();
  });

  it('旧裸词节点类型被拦截（引擎无执行路径）', () => {
    expect(
      validateLinearChain([
        { id: 's', type: 'start', name: 's' },
        { id: 'a', type: 'approval', name: 'a' },
        { id: 'e', type: 'end', name: 'e' },
      ] as unknown as ProcessNode[])
    ).toBe('bpm.definitions.formErrors.unknownNodeType');
  });

  it('start 不在首位 / 首出边将指向非审批节点时被拦截', () => {
    // start_event 不在首位：线性生成后 start 的首条出边 target 不再是第一个审批节点
    expect(
      validateLinearChain([
        { id: 'a', type: 'user_task', name: 'a', assignee_type: 'user', assignee_value: 1 },
        { id: 's', type: 'start_event', name: 's' },
        { id: 'e', type: 'end_event', name: 'e' },
      ])
    ).toBe('bpm.definitions.formErrors.chainOrder');
    // start 之后直接是 end：首个任务将落在 end_event 上
    expect(
      validateLinearChain([
        { id: 's', type: 'start_event', name: 's' },
        { id: 'e', type: 'end_event', name: 'e' },
      ])
    ).toBe('bpm.definitions.formErrors.chainOrder');
    // 无 start
    expect(
      validateLinearChain([
        { id: 'a', type: 'user_task', name: 'a', assignee_type: 'user', assignee_value: 1 },
      ])
    ).toBe('bpm.definitions.formErrors.chainOrder');
  });

  it('审批节点缺数字审批人被拦截', () => {
    expect(
      validateLinearChain([
        { id: 's', type: 'start_event', name: 's' },
        { id: 'a', type: 'user_task', name: 'a' },
        { id: 'e', type: 'end_event', name: 'e' },
      ])
    ).toBe('bpm.definitions.formErrors.assigneeRequired');
    // 字符串 ID（手填残留/存量数据）未重选同样拦截——ID 必须由可搜索选择带出为数字
    expect(
      validateLinearChain([
        { id: 's', type: 'start_event', name: 's' },
        {
          id: 'a',
          type: 'user_task',
          name: 'a',
          assignee_type: 'user',
          assignee_value: '3',
        } as unknown as ProcessNode,
        { id: 'e', type: 'end_event', name: 'e' },
      ])
    ).toBe('bpm.definitions.formErrors.assigneeRequired');
  });

  it('handleSubmit 对违规节点集拒绝提交：不发起任何 API 写请求', async () => {
    vi.clearAllMocks();
    const legacyNodes = makeNodes().map((n, i) =>
      i === 1 ? { ...n, type: 'approval' as unknown as ProcessNode['type'] } : n
    );
    const cb = makeCb(false, legacyNodes);
    const proc = useBpmDfProc(cb);
    await proc.handleSubmit();
    expect(createBpmDefinition).not.toHaveBeenCalled();
    expect(updateBpmDefinition).not.toHaveBeenCalled();
  });
});

describe('审批人按部门上级解析（dept_superior）契约', () => {
  it('validateLinearChain 接受 dept_superior 节点无 assignee_value（运行期取人）', () => {
    expect(
      validateLinearChain([
        { id: 's', type: 'start_event', name: 's' },
        { id: 'a', type: 'user_task', name: 'a', assignee_type: 'dept_superior' },
        { id: 'e', type: 'end_event', name: 'e' },
      ])
    ).toBeNull();
  });

  it('validateLinearChain 仍拦截 user 类型缺数字审批人（词表两值各有约束）', () => {
    expect(
      validateLinearChain([
        { id: 's', type: 'start_event', name: 's' },
        { id: 'a', type: 'user_task', name: 'a', assignee_type: 'user' },
        { id: 'e', type: 'end_event', name: 'e' },
      ])
    ).toBe('bpm.definitions.formErrors.assigneeRequired');
  });

  it('sensitive 标记随节点持久化（引擎读取节点级 sensitive 判定再上一级）', () => {
    const persisted = toPersistedNodes([
      {
        id: 'a',
        type: 'user_task',
        name: 'a',
        assignee_type: 'dept_superior',
        sensitive: true,
        edge_condition: '${amount} > 1',
      },
    ]);
    expect(persisted[0]).toEqual({
      id: 'a',
      type: 'user_task',
      name: 'a',
      assignee_type: 'dept_superior',
      sensitive: true,
    });
    // edge_condition 属编辑态，剥离后才进 config.nodes（节点级条件非引擎读取键）
    expect(persisted[0]).not.toHaveProperty('edge_condition');
  });
});

describe('buildLinearEdges 条件分支（挂在边、非节点）', () => {
  it('节点带非空 edge_condition 时该出边携带 condition 键', () => {
    const edges = buildLinearEdges([
      { id: 's', type: 'start_event', name: 's' },
      {
        id: 'a',
        type: 'user_task',
        name: 'a',
        assignee_type: 'dept_superior',
        edge_condition: '${amount} > 10000',
      },
      { id: 'e', type: 'end_event', name: 'e' },
    ]);
    expect(edges).toEqual([
      { source: 's', target: 'a' },
      { source: 'a', target: 'e', condition: '${amount} > 10000' },
    ]);
  });

  it('空/空白 edge_condition 省略 condition 键（空串是 poison 值而非无条件）', () => {
    const edges = buildLinearEdges([
      {
        id: 'a',
        type: 'user_task',
        name: 'a',
        assignee_type: 'dept_superior',
        edge_condition: '   ',
      },
      { id: 'e', type: 'end_event', name: 'e' },
    ]);
    expect(Object.keys(edges[0]).sort()).toEqual(['source', 'target']);
    expect(edges[0]).not.toHaveProperty('condition');
  });
});

describe('表单源码静态锁：节点类型选项与审批人输入形态', () => {
  const formPath = join(SRC, 'views/bpm/definitions/components/BpmDefinitionForm.vue');
  const formSrc = readFileSync(formPath, 'utf-8');

  it('节点类型下拉的提交值必须且仅是三个引擎 token', () => {
    const block = formSrc.match(/<el-select v-model="row\.type"[\s\S]*?<\/el-select>/);
    expect(block).not.toBeNull();
    const optionValues = [...block![0].matchAll(/value="([^"]+)"/g)].map(m => m[1]);
    expect(optionValues).toEqual(['start_event', 'user_task', 'end_event']);
  });

  it('源码中不得残留引擎不认的旧审批人类型 option value（role/department/dynamic 无执行路径）', () => {
    expect(formSrc).not.toMatch(/value="(role|department|dynamic)"/);
    expect(formSrc).not.toMatch(/value="(start|end|approval|condition|notify)"/);
  });

  it('审批人类型下拉的提交值必须且仅是引擎词表 dept_superior 与 user', () => {
    const block = formSrc.match(/<el-select\s+v-model="row\.assignee_type"[\s\S]*?<\/el-select>/);
    expect(block).not.toBeNull();
    const optionValues = [...block![0].matchAll(/value="([^"]+)"/g)].map(m => m[1]);
    expect(optionValues).toEqual(['dept_superior', 'user']);
  });

  it('条件分支与敏感操作列折叠在高级区内且默认收起（用户裁定：列保留但折叠为高级）', () => {
    // 检测力：把 advancedOpen 默认改 true 或把两列的 v-if 摘掉（改回常显），本锁红
    expect(formSrc).toMatch(/const advancedOpen = ref\(false\)/);
    const conditionCol = formSrc.match(
      /<el-table-column\s+v-if="advancedOpen"[\s\S]*?form\.condition[\s\S]*?<\/el-table-column>/
    );
    expect(conditionCol).not.toBeNull();
    const sensitiveCol = formSrc.match(
      /<el-table-column\s+v-if="advancedOpen"[\s\S]*?form\.sensitive[\s\S]*?<\/el-table-column>/
    );
    expect(sensitiveCol).not.toBeNull();
  });

  it('审批人列是按 keyword 远程检索的用户选择，不是手填 ID 的输入框', () => {
    const block = formSrc.match(
      /t\('bpm\.definitions\.form\.assignee'\)[\s\S]*?<\/el-table-column>/
    );
    expect(block).not.toBeNull();
    expect(block![0]).toContain('filterable');
    expect(block![0]).toContain('el-select');
    expect(block![0]).toContain('remote');
    expect(block![0]).toContain(':remote-method');
    expect(block![0]).not.toContain('el-input');
    // 候选必须按行独立持有（共享单一数组会让多行下拉互相串数据）
    expect(formSrc).toContain('rowCandidates');
    expect(formSrc).toContain('rowLoading');
  });

  it('反向锁：definitions 域不得再用大 page_size 凑全量（后端 clamp(1,100) 静默截断）', () => {
    const files = [
      join(SRC, 'views/bpm/definitions.vue'),
      formPath,
      join(SRC, 'views/bpm/definitions/composables/useBpmDf.ts'),
      join(SRC, 'views/bpm/definitions/composables/useBpmDfProc.ts'),
      join(SRC, 'views/bpm/definitions/composables/useAssigneeSearch.ts'),
    ];
    for (const f of files) {
      const src = readFileSync(f, 'utf-8');
      expect({ file: f, hits200: /page_size:\s*200/.test(src) }).toEqual({
        file: f,
        hits200: false,
      });
    }
  });

  it('反向锁：全仓 getUserList 不得以 page_size>100 凑全量，存量挂账冻结新增', () => {
    // GET /users 的 page_size 被后端 list_users 的 clamp(1,100) 静默削顶，
    // 传 >100 当全量=第 101 个之后的用户永远取不到；如实取法是 keyword 收窄
    // （fetchUserCandidates 的调用参数 pin 见上方用例）。
    // 其余域的同族写法登记为挂账、不在本用例管辖内——新增一处、或挂账被清掉
    // 却不作为（清单没同步收缩），都判红。沿用 translated-value-select 的
    // KNOWN_PENDING 挂账形态。
    const KNOWN_PENDING_OVERCLAMP: Record<string, number> = {
      'src/views/bpm/index.vue': 1,
      'src/views/bpm/approval/composables/useBpmApProc.ts': 1,
      'src/views/system/oa-announcements/index.vue': 1,
      'src/views/system/tabs/NotificationTab.vue': 1,
      'src/views/quality/composables/useQualityLookups.ts': 1,
    };
    const walkVueTs = (dir: string, out: string[] = []): string[] => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const p = join(dir, entry.name);
        if (entry.isDirectory()) walkVueTs(p, out);
        else if (entry.name.endsWith('.vue') || entry.name.endsWith('.ts')) out.push(p);
      }
      return out;
    };
    const found: Record<string, number> = {};
    for (const file of walkVueTs(SRC)) {
      const calls =
        readFileSync(file, 'utf-8').match(/getUserList\s*\(\s*\{[^}]*?page_size:\s*(\d+)/g) ?? [];
      const over = calls.filter(c => Number(/page_size:\s*(\d+)/.exec(c)![1]) > 100);
      if (over.length > 0) {
        found[relative(FRONTEND, file).split(sep).join('/')] = over.length;
      }
    }
    expect(found).toEqual(KNOWN_PENDING_OVERCLAMP);
  });
});
