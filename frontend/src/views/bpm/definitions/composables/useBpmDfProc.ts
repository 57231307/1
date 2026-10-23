/**
 * useBpmDfProc.ts - BPM 流程定义流程操作 composable
 * 封装搜索 / 重置 / 创建 / 编辑 / 删除 / 版本 / 创建版本 / 激活 / 保存为模板等流程性方法
 *
 * 设计说明：通过 callbacks 接收 useBpmDf 的状态引用（Reactive 包装层）
 */
import { isDialogDismissal } from '@/utils/monitor';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import {
  deleteBpmDefinition,
  updateBpmDefinition,
  createBpmDefinition,
  getBpmDefinitionById,
  createBpmVersion,
  activateBpmVersion,
  saveBpmAsTemplate,
  type ProcessDefinition,
  type ProcessEdge,
  type ProcessNode,
  type ProcessVersion,
  type CreateProcessDefinitionPayload,
  type UpdateProcessDefinitionPayload,
} from '@/api/bpm-enhanced';
import { logger } from '@/utils/logger';

/**
 * 按节点数组顺序线性生成边（相邻两节点两两相连），并携带各节点出边的条件表达式。
 *
 * 界面节点列表顺序即审批顺序：引擎按"当前节点的出边"逐级推进，且解析首个任务时
 * 不校验 start 出边 target 的节点类型（后端 resolve_first_task_node 直接取边指向的目标），
 * 因此边的顺序必须与界面展示顺序显式一致——本函数按数组序生成，表格按数组序渲染，两者同源。
 * 当前表单没有"连线"交互，每个节点至多一条出边；条件表达式挂在**边**上（edge.condition），
 * 由节点行编辑态字段 edge_condition 带出：非空才写入 condition 键，空则省略该键。
 * 提交前由 validateLinearChain 钉住 start_event 位于首位、其唯一出边指向第一个 user_task。
 * 键名与引擎实际读取逐字符一致（source / target 见后端 resolve_first_task_node 与
 * try_advance_to_next_node 的取键；condition 见 try_advance_to_next_node 经 evaluate_bpm_condition）。
 * 无条件边刻意不携带 condition 键：表达式解析失败按 fail-closed 判不匹配，
 * 空串 condition 是 poison 值而非"无条件"；且带病 condition 的边匹配不到时，
 * 线性链会退化成"无匹配出边"——发起期 validate_forward_path 会当场拒绝该定义
 * （第三处 fail-open 已由裁定消灭），因此这里绝不能把空的 edge_condition 落成空串 condition。
 */
export function buildLinearEdges(nodes: ProcessNode[]): ProcessEdge[] {
  return nodes.slice(0, -1).map((node, index) => {
    const condition = node.edge_condition?.trim();
    const edge: ProcessEdge = { source: node.id, target: nodes[index + 1].id };
    if (condition) {
      edge.condition = condition;
    }
    return edge;
  });
}

/**
 * 提交前校验节点链是否满足引擎可执行不变量；通过返回 null，
 * 否则返回首个违规项对应的 i18n 错误键（由调用方显式报错、拒绝提交）。
 *
 * 不变量依据引擎解析逻辑逐条钉住：
 * - 类型只允许 start_event / user_task / end_event（后端 resolve_first_task_node 与
 *   try_advance_to_next_node：其余 token 无执行路径，推进命中未知类型会被 fail-closed 拒绝）；
 * - 恰一个 start_event 在首位、恰一个 end_event 在末位、中间全为 user_task，
 *   保证 start 的唯一出边指向第一个审批节点（否则边 target 不做类型校验，
 *   非审批节点会被当首任务建单，见 resolve_first_task_node 取首出边的逻辑）；
 * - 每个 user_task 的审批人类型必须是 user 或 dept_superior
 *   （后端 resolve_node_assignee：其余取值拒绝发起）；
 *   选 user 时 assignee_value 必须是数字用户 ID（须解析为正整数），
 *   选 dept_superior 时不需 assignee_value（按部门上级运行期解析）。
 */
export function validateLinearChain(nodes: ProcessNode[]): string | null {
  const engineTokens: string[] = ['start_event', 'user_task', 'end_event'];
  if (nodes.some(node => !engineTokens.includes(node.type))) {
    return 'bpm.definitions.formErrors.unknownNodeType';
  }
  const starts = nodes.filter(node => node.type === 'start_event');
  const ends = nodes.filter(node => node.type === 'end_event');
  const approvals = nodes.filter(node => node.type === 'user_task');
  if (
    starts.length !== 1 ||
    ends.length !== 1 ||
    approvals.length < 1 ||
    nodes[0].type !== 'start_event' ||
    nodes[nodes.length - 1].type !== 'end_event' ||
    nodes.slice(1, -1).some(node => node.type !== 'user_task')
  ) {
    return 'bpm.definitions.formErrors.chainOrder';
  }
  if (
    approvals.some(node => {
      const assigneeType = node.assignee_type ?? 'user';
      if (assigneeType === 'dept_superior') {
        return false;
      }
      if (assigneeType !== 'user') {
        return true;
      }
      return (
        typeof node.assignee_value !== 'number' ||
        !Number.isInteger(node.assignee_value) ||
        node.assignee_value <= 0
      );
    })
  ) {
    return 'bpm.definitions.formErrors.assigneeRequired';
  }
  return null;
}

/**
 * 提交用的持久化节点：剥离表单编辑态字段 edge_condition。
 * 引擎从不读取节点级条件（条件权威位置是 config.edges[].condition，由 buildLinearEdges 落），
 * 留在 config.nodes 里即是可被误当生效项的虚假能力，故按后端实际消费的键集裁剪。
 */
export function toPersistedNodes(nodes: ProcessNode[]): ProcessNode[] {
  return nodes.map(node => {
    const { edge_condition: _edgeCondition, ...rest } = node;
    return rest;
  });
}

/**
 * 编辑回填：把持久化的 config.edges[].condition 映回源节点的编辑态字段 edge_condition。
 * 表单按节点行编辑条件、提交时再写回边，因此回填必须以边为真相源还原到对应 source 节点，
 * 否则编辑存量带条件的定义会静默丢掉条件（第二件事 #82 的判据即 edge.condition）。
 */
export function attachEdgeConditions(nodes: ProcessNode[], edges?: ProcessEdge[]): ProcessNode[] {
  if (!edges || edges.length === 0) {
    return nodes.map(node => ({ ...node }));
  }
  const conditionBySource = new Map<string, string>();
  for (const edge of edges) {
    if (edge.condition) {
      conditionBySource.set(edge.source, edge.condition);
    }
  }
  return nodes.map(node => ({
    ...node,
    edge_condition: conditionBySource.get(node.id),
  }));
}

/**
 * 流程回调（接收 useBpmDf 返回的状态，自动解包后的值类型）
 * 适配 useTableApi：page 独立 ref，queryParams 不含 page/page_size
 */
interface BpmDfCallbacks {
  // 列表
  definitions: ProcessDefinition[];
  loading: boolean;
  total: number;
  // 分页（useTableApi 独立 ref）
  page: number;
  // 过滤（useTableApi queryParams 为 Record<string, unknown>）
  queryParams: Record<string, unknown>;
  // 表单
  dialogVisible: boolean;
  isEdit: boolean;
  submitLoading: boolean;
  formData: {
    id?: number;
    process_key: string;
    process_name: string;
    description: string;
    category: string;
    nodes: ProcessNode[];
  };
  // 版本
  versionDialogVisible: boolean;
  versionLoading: boolean;
  currentDefinition: ProcessDefinition | null;
  versions: ProcessVersion[];
  // 模板
  templateDialogVisible: boolean;
  templateLoading: boolean;
  templateForm: { template_name: string; category: string; description: string };
  // 方法
  fetchDefinitions: () => Promise<void>;
  fetchVersions: (definitionId: number) => Promise<void>;
}

/**
 * BPM 流程定义流程操作方法集合
 */
export function useBpmDfProc(cb: BpmDfCallbacks) {
  /** 搜索（page 独立 ref，refresh 别名 fetchDefinitions） */
  const handleSearch = () => {
    cb.page = 1;
    cb.fetchDefinitions();
  };

  /** 重置（page 独立 ref，queryParams 为 Record<string, unknown>） */
  const handleReset = () => {
    cb.queryParams.category = '';
    cb.page = 1;
    cb.fetchDefinitions();
  };

  /** 新建 */
  const handleCreate = () => {
    cb.isEdit = false;
    Object.assign(cb.formData, {
      id: undefined,
      process_key: '',
      process_name: '',
      description: '',
      category: 'finance',
      nodes: [],
    });
    cb.dialogVisible = true;
  };

  /** 编辑（按 ID 回源最新定义，失败保留行数据） */
  const handleEdit = async (row: ProcessDefinition) => {
    cb.isEdit = true;
    let source: ProcessDefinition = row;
    try {
      const res = await getBpmDefinitionById(row.id);
      if (res.data) source = res.data;
    } catch (e) {
      logger.warn('流程定义回源失败，使用行数据', e instanceof Error ? e.message : String(e));
    }
    Object.assign(cb.formData, {
      id: source.id,
      process_key: source.process_key,
      process_name: source.process_name,
      description: source.description || '',
      category: source.category || 'finance',
      // 条件真源在 config.edges[].condition，回填映到节点行编辑态（attachEdgeConditions），
      // 否则编辑存量带条件定义会静默丢条件
      nodes: attachEdgeConditions(source.nodes || [], source.config?.edges),
    });
    cb.dialogVisible = true;
  };

  /** 删除 */
  const handleDelete = async (row: ProcessDefinition) => {
    try {
      await ElMessageBox.confirm(`确定要删除流程定义「${row.process_name}」吗？`, '确认删除', {
        type: 'warning',
      });
      await deleteBpmDefinition(row.id);
      msg.success('deleteSuccess');
      await cb.fetchDefinitions();
    } catch (error) {
      if (!isDialogDismissal(error)) {
        const errMsg = error instanceof Error ? error.message : msg.translate('deleteFailed');
        logger.error(errMsg);
        ElMessage.error(errMsg);
      }
    }
  };

  /** 提交表单 */
  const handleSubmit = async () => {
    // 引擎可执行不变量不满足时拒绝提交并显式点名问题（validateLinearChain），
    // 不放行一个"保存成功但发起/推进时被引擎拒绝或绕过审批"的定义
    const chainErrorKey = validateLinearChain(cb.formData.nodes);
    if (chainErrorKey) {
      const detail = i18n.global.t(chainErrorKey);
      logger.error(`流程定义提交被拦截: ${detail}`);
      ElMessage.error(detail);
      return;
    }
    cb.submitLoading = true;
    try {
      // 后端契约：流程节点与边持久化在 config.nodes / config.edges（后端
      // resolve_first_task_node 与 try_advance_to_next_node 均只从 config 取 nodes/edges），
      // 顶层 nodes 非契约字段会被 serde 忽略 → 节点静默丢失；
      // 缺 edges 时审批推进匹配不到边，发起期 validate_forward_path 直接拒绝该定义，
      // 不建无法闭合的实例（第三处 fail-open 的完成收尾旁路已被消灭）。
      // 本表单无连线交互，edges 按节点顺序线性生成并携带节点行的条件（buildLinearEdges）。
      // config.nodes 只留引擎读取的键，edge_condition 属表单编辑态，提交前剥离（toPersistedNodes）。
      // 载荷键名按 DTO 真实字段名构造（name/code 为 CreateProcessDefinitionRequest/
      // UpdateProcessDefinitionRequest 的 Rust 字段名；serde alias process_name/process_key
      // 门禁不识别且视图内部键与 UI 表单解耦，统一按真实字段名提交）。
      const config = {
        nodes: toPersistedNodes(cb.formData.nodes),
        edges: buildLinearEdges(cb.formData.nodes),
      };
      if (cb.isEdit && cb.formData.id) {
        const payload: UpdateProcessDefinitionPayload = {
          name: cb.formData.process_name,
          description: cb.formData.description,
          category: cb.formData.category,
          config,
        };
        await updateBpmDefinition(cb.formData.id, payload);
        msg.success('updateSuccess');
      } else {
        const payload: CreateProcessDefinitionPayload = {
          code: cb.formData.process_key,
          name: cb.formData.process_name,
          description: cb.formData.description,
          category: cb.formData.category,
          config,
        };
        await createBpmDefinition(payload);
        // 建单成功文案为「新增成功」= common.message.createSuccess。
        // msg.success('createSuccess') 会解析到顶层 message.createSuccess（值「创建成功」），
        // 与本模块既定新建提示文案不一致，故直接取 common 命名空间的对应 key。
        ElMessage.success(i18n.global.t('common.message.createSuccess'));
      }
      cb.dialogVisible = false;
      await cb.fetchDefinitions();
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : msg.translate('operationFailed');
      logger.error(errMsg);
      ElMessage.error(errMsg);
    } finally {
      cb.submitLoading = false;
    }
  };

  /** 打开版本对话框 */
  const handleOpenVersions = async (row: ProcessDefinition) => {
    cb.currentDefinition = row;
    cb.versionDialogVisible = true;
    await cb.fetchVersions(row.id);
  };

  /** 创建新版本 */
  const handleCreateVersion = async () => {
    if (!cb.currentDefinition) return;
    try {
      await ElMessageBox.confirm('确定要创建新版本吗？', '提示', { type: 'info' });
      await createBpmVersion(cb.currentDefinition.id, { change_log: '新建版本' });
      msg.success('newVersionCreated');
      await cb.fetchVersions(cb.currentDefinition.id);
      await cb.fetchDefinitions();
    } catch (error) {
      if (!isDialogDismissal(error)) {
        const errMsg =
          error instanceof Error ? error.message : msg.translate('createVersionFailed');
        logger.error(errMsg);
        ElMessage.error(errMsg);
      }
    }
  };

  /** 激活版本 */
  const handleActivateVersion = async (version: ProcessVersion) => {
    try {
      await activateBpmVersion(version.id);
      msg.success('versionActivated');
      if (cb.currentDefinition) {
        await cb.fetchVersions(cb.currentDefinition.id);
      }
      await cb.fetchDefinitions();
    } catch (error) {
      const errMsg =
        error instanceof Error ? error.message : msg.translate('activateVersionFailed');
      logger.error(errMsg);
      ElMessage.error(errMsg);
    }
  };

  /** 打开保存为模板对话框 */
  const handleOpenSaveAsTemplate = (row: ProcessDefinition) => {
    cb.currentDefinition = row;
    Object.assign(cb.templateForm, {
      template_name: `${row.process_name}模板`,
      category: row.category || 'finance',
      description: '',
    });
    cb.templateDialogVisible = true;
  };

  /** 提交保存为模板 */
  const handleSaveAsTemplate = async () => {
    if (!cb.currentDefinition) return;
    cb.templateLoading = true;
    try {
      await saveBpmAsTemplate(
        cb.currentDefinition.id,
        cb.templateForm as {
          template_name: string;
          category: string;
          description?: string;
        }
      );
      msg.success('savedAsTemplate');
      cb.templateDialogVisible = false;
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : msg.translate('saveFailed');
      logger.error(errMsg);
      ElMessage.error(errMsg);
    } finally {
      cb.templateLoading = false;
    }
  };

  // 暴露格式化工具函数
  return {
    handleSearch,
    handleReset,
    handleCreate,
    handleEdit,
    handleDelete,
    handleSubmit,
    handleOpenVersions,
    handleCreateVersion,
    handleActivateVersion,
    handleOpenSaveAsTemplate,
    handleSaveAsTemplate,
  };
}
