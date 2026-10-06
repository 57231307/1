/**
 * useBpmDfProc.ts - BPM 流程定义流程操作 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 bpm/definitions.vue）
 * 封装搜索 / 重置 / 创建 / 编辑 / 删除 / 版本 / 创建版本 / 激活 / 保存为模板等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 useBpmDf 的状态引用（Reactive 包装层）
 */
import { isDialogDismissal } from '@/utils/monitor';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
// D14 Batch 5b：原 bpmEnhancedApi 对象已转风格 B 函数
import {
  deleteBpmDefinition,
  updateBpmDefinition,
  createBpmDefinition,
  getBpmDefinitionById,
  createBpmVersion,
  activateBpmVersion,
  saveBpmAsTemplate,
  type ProcessDefinition,
  type ProcessNode,
  type ProcessVersion,
  type CreateProcessDefinitionPayload,
  type UpdateProcessDefinitionPayload,
} from '@/api/bpm-enhanced';
import { logger } from '@/utils/logger';

/**
 * 流程回调（接收 useBpmDf 返回的状态，自动解包后的值类型）
 * 批次 282：适配 useTableApi（page 独立 ref，queryParams 不含 page/page_size）
 */
interface BpmDfCallbacks {
  // 列表
  definitions: ProcessDefinition[];
  loading: boolean;
  total: number;
  // 分页（useTableApi 独立 ref）
  page: number;
  // 过滤（批次 282：useTableApi queryParams 为 Record<string, unknown>）
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
 * 节点类型映射
 */
const NODE_TYPE_MAP: Record<string, string> = {
  start: '开始',
  end: '结束',
  approval: '审批',
  condition: '条件',
  notify: '通知',
};

/**
 * 节点 assignee_type 映射
 */
const ASSIGNEE_TYPE_MAP: Record<string, string> = {
  user: '指定用户',
  role: '指定角色',
  department: '指定部门',
  dynamic: '动态计算',
};

/**
 * 审批人类型映射
 */
function getNodeTypeName(type: string): string {
  return NODE_TYPE_MAP[type] || type;
}

function getAssigneeTypeName(type?: string): string {
  if (!type) return '';
  return ASSIGNEE_TYPE_MAP[type] || type;
}

/**
 * BPM 流程定义流程操作方法集合
 */
export function useBpmDfProc(cb: BpmDfCallbacks) {
  /** 搜索（批次 282：page 独立 ref，refresh 别名 fetchDefinitions） */
  const handleSearch = () => {
    cb.page = 1;
    cb.fetchDefinitions();
  };

  /** 重置（批次 282：page 独立 ref，queryParams 为 Record<string, unknown>） */
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
      nodes: source.nodes || [],
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
    cb.submitLoading = true;
    try {
      // 后端契约：流程节点持久化在 config.nodes（bpm_service.rs:141），顶层 nodes 非契约字段
      // 会被 serde 忽略 → 节点静默丢失。提交时把 nodes 包进 config，并按 DTO 真实键名构造载荷
      // （name/code 为 CreateProcessDefinitionRequest/UpdateProcessDefinitionRequest 的 Rust
      // 字段名；serde alias process_name/process_key 门禁不识别且视图内部键与 UI 表单解耦，
      // 统一按真实字段名提交）。
      if (cb.isEdit && cb.formData.id) {
        const payload: UpdateProcessDefinitionPayload = {
          name: cb.formData.process_name,
          description: cb.formData.description,
          category: cb.formData.category,
          config: { nodes: cb.formData.nodes },
        };
        await updateBpmDefinition(cb.formData.id, payload);
        msg.success('updateSuccess');
      } else {
        const payload: CreateProcessDefinitionPayload = {
          code: cb.formData.process_key,
          name: cb.formData.process_name,
          description: cb.formData.description,
          category: cb.formData.category,
          config: { nodes: cb.formData.nodes },
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
    getNodeTypeName,
    getAssigneeTypeName,
  };
}
