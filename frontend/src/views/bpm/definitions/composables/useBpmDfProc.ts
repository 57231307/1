/**
 * useBpmDfProc.ts - BPM 流程定义流程操作 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 bpm/definitions.vue）
 * 封装搜索 / 重置 / 创建 / 编辑 / 删除 / 版本 / 创建版本 / 激活 / 保存为模板等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 useBpmDf 的状态引用（Reactive 包装层）
 */
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  bpmEnhancedApi,
  type ProcessDefinition,
  type ProcessNode,
  type ProcessVersion,
} from '@/api/bpm-enhanced'
import { logger } from '@/utils/logger'

/**
 * 流程回调（接收 useBpmDf 返回的状态，自动解包后的值类型）
 * 批次 282：适配 useTableApi（page 独立 ref，queryParams 不含 page/page_size）
 */
interface BpmDfCallbacks {
  // 列表
  definitions: ProcessDefinition[]
  loading: boolean
  total: number
  // 分页（useTableApi 独立 ref）
  page: number
  // 过滤（useTableApi queryParams，不含分页参数）
  queryParams: { keyword: string; category: string }
  // 表单
  dialogVisible: boolean
  isEdit: boolean
  submitLoading: boolean
  formData: {
    id?: number
    process_key: string
    process_name: string
    description: string
    category: string
    nodes: ProcessNode[]
  }
  // 版本
  versionDialogVisible: boolean
  versionLoading: boolean
  currentDefinition: ProcessDefinition | null
  versions: ProcessVersion[]
  // 模板
  templateDialogVisible: boolean
  templateLoading: boolean
  templateForm: { template_name: string; category: string; description: string }
  // 方法
  fetchDefinitions: () => Promise<void>
  fetchVersions: (definitionId: number) => Promise<void>
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
}

/**
 * 节点 assignee_type 映射
 */
const ASSIGNEE_TYPE_MAP: Record<string, string> = {
  user: '指定用户',
  role: '指定角色',
  department: '指定部门',
  dynamic: '动态计算',
}

/**
 * 审批人类型映射
 */
function getNodeTypeName(type: string): string {
  return NODE_TYPE_MAP[type] || type
}

function getAssigneeTypeName(type?: string): string {
  if (!type) return ''
  return ASSIGNEE_TYPE_MAP[type] || type
}

/**
 * BPM 流程定义流程操作方法集合
 */
export function useBpmDfProc(cb: BpmDfCallbacks) {
  /** 搜索（批次 282：page 独立 ref，refresh 别名 fetchDefinitions） */
  const handleSearch = () => {
    cb.page = 1
    cb.fetchDefinitions()
  }

  /** 重置（批次 282：page 独立 ref） */
  const handleReset = () => {
    cb.queryParams.keyword = ''
    cb.queryParams.category = ''
    cb.page = 1
    cb.fetchDefinitions()
  }

  /** 新建 */
  const handleCreate = () => {
    cb.isEdit = false
    Object.assign(cb.formData, {
      id: undefined,
      process_key: '',
      process_name: '',
      description: '',
      category: 'finance',
      nodes: [],
    })
    cb.dialogVisible = true
  }

  /** 编辑 */
  const handleEdit = (row: ProcessDefinition) => {
    cb.isEdit = true
    Object.assign(cb.formData, {
      id: row.id,
      process_key: row.process_key,
      process_name: row.process_name,
      description: row.description || '',
      category: row.category || 'finance',
      nodes: row.nodes || [],
    })
    cb.dialogVisible = true
  }

  /** 删除 */
  const handleDelete = async (row: ProcessDefinition) => {
    try {
      await ElMessageBox.confirm(
        `确定要删除流程定义「${row.process_name}」吗？`,
        '确认删除',
        { type: 'warning' }
      )
      await bpmEnhancedApi.deleteDefinition(row.id)
      ElMessage.success('删除成功')
      await cb.fetchDefinitions()
    } catch (error) {
      if (error !== 'cancel') {
        const msg = error instanceof Error ? error.message : '删除失败'
        logger.error(msg)
        ElMessage.error(msg)
      }
    }
  }

  /** 提交表单 */
  const handleSubmit = async () => {
    cb.submitLoading = true
    try {
      if (cb.isEdit && cb.formData.id) {
        await bpmEnhancedApi.updateDefinition(
          cb.formData.id,
          cb.formData as unknown as Partial<ProcessDefinition>
        )
        ElMessage.success('更新成功')
      } else {
        await bpmEnhancedApi.createDefinition(cb.formData as unknown as Partial<ProcessDefinition>)
        ElMessage.success('创建成功')
      }
      cb.dialogVisible = false
      await cb.fetchDefinitions()
    } catch (error) {
      const msg = error instanceof Error ? error.message : '操作失败'
      logger.error(msg)
      ElMessage.error(msg)
    } finally {
      cb.submitLoading = false
    }
  }

  /** 打开版本对话框 */
  const handleOpenVersions = async (row: ProcessDefinition) => {
    cb.currentDefinition = row
    cb.versionDialogVisible = true
    await cb.fetchVersions(row.id)
  }

  /** 创建新版本 */
  const handleCreateVersion = async () => {
    if (!cb.currentDefinition) return
    try {
      await ElMessageBox.confirm('确定要创建新版本吗？', '提示', { type: 'info' })
      await bpmEnhancedApi.createVersion(cb.currentDefinition.id, { change_log: '新建版本' })
      ElMessage.success('新版本已创建')
      await cb.fetchVersions(cb.currentDefinition.id)
      await cb.fetchDefinitions()
    } catch (error) {
      if (error !== 'cancel') {
        const msg = error instanceof Error ? error.message : '创建版本失败'
        logger.error(msg)
        ElMessage.error(msg)
      }
    }
  }

  /** 激活版本 */
  const handleActivateVersion = async (version: ProcessVersion) => {
    try {
      await bpmEnhancedApi.activateVersion(version.id)
      ElMessage.success('版本已激活')
      if (cb.currentDefinition) {
        await cb.fetchVersions(cb.currentDefinition.id)
      }
      await cb.fetchDefinitions()
    } catch (error) {
      const msg = error instanceof Error ? error.message : '激活版本失败'
      logger.error(msg)
      ElMessage.error(msg)
    }
  }

  /** 打开保存为模板对话框 */
  const handleOpenSaveAsTemplate = (row: ProcessDefinition) => {
    cb.currentDefinition = row
    Object.assign(cb.templateForm, {
      template_name: `${row.process_name}模板`,
      category: row.category || 'finance',
      description: '',
    })
    cb.templateDialogVisible = true
  }

  /** 提交保存为模板 */
  const handleSaveAsTemplate = async () => {
    if (!cb.currentDefinition) return
    cb.templateLoading = true
    try {
      await bpmEnhancedApi.saveAsTemplate(
        cb.currentDefinition.id,
        cb.templateForm as {
          template_name: string
          category: string
          description?: string
        }
      )
      ElMessage.success('已保存为模板')
      cb.templateDialogVisible = false
    } catch (error) {
      const msg = error instanceof Error ? error.message : '保存失败'
      logger.error(msg)
      ElMessage.error(msg)
    } finally {
      cb.templateLoading = false
    }
  }

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
  }
}
