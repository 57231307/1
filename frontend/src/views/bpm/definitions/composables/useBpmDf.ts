/**
 * useBpmDf.ts - BPM 流程定义核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 bpm/definitions.vue）
 * 提供流程定义列表 / 分页 / 过滤 / 节点列表等核心方法
 * 业务流程（创建/编辑/删除/版本/创建版本/激活/保存为模板）由 useBpmDfProc 提供
 * 批次 282：definitions 接入 useTableApi，移除手写分页/加载逻辑
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  bpmEnhancedApi,
  type ProcessDefinition,
  type ProcessNode,
  type ProcessVersion,
} from '@/api/bpm-enhanced'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

/** BPM 流程定义主业务 composable（集中管理列表、分页、过滤、节点、版本） */
export function useBpmDf() {
  // 列表 - 接入 useTableApi（批次 282）
  const {
    data: definitions,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: fetchDefinitions,
  } = useTableApi<ProcessDefinition>({
    url: '/bpm/definitions',
    defaultParams: { keyword: '', category: '' },
    onError: (err: unknown) => {
      const msg = err instanceof Error ? err.message : '获取流程定义列表失败'
      logger.error(msg)
      ElMessage.error(msg)
    },
  })

  // 表单对话框
  const dialogVisible = ref(false)
  const isEdit = ref(false)
  const submitLoading = ref(false)
  const formData = reactive<{
    id?: number
    process_key: string
    process_name: string
    description: string
    category: string
    nodes: ProcessNode[]
  }>({
    id: undefined,
    process_key: '',
    process_name: '',
    description: '',
    category: 'finance',
    nodes: [],
  })

  // 版本对话框
  const versionDialogVisible = ref(false)
  const versionLoading = ref(false)
  const currentDefinition = ref<ProcessDefinition | null>(null)
  const versions = ref<ProcessVersion[]>([])

  // 模板对话框
  const templateDialogVisible = ref(false)
  const templateLoading = ref(false)
  const templateForm = reactive<{
    template_name: string
    category: string
    description: string
  }>({
    template_name: '',
    category: 'finance',
    description: '',
  })

  /** 加载版本 */
  const fetchVersions = async (definitionId: number) => {
    versionLoading.value = true
    try {
      const res = await bpmEnhancedApi.listVersions(definitionId)
      versions.value = (res.data || []) as ProcessVersion[]
    } catch (error) {
      const msg = error instanceof Error ? error.message : '获取版本列表失败'
      logger.error(msg)
      ElMessage.error(msg)
    } finally {
      versionLoading.value = false
    }
  }

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    // 列表（useTableApi 管理）
    definitions,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    fetchDefinitions,
    // 表单对话框
    dialogVisible,
    isEdit,
    submitLoading,
    formData,
    // 版本对话框
    versionDialogVisible,
    versionLoading,
    currentDefinition,
    versions,
    // 模板对话框
    templateDialogVisible,
    templateLoading,
    templateForm,
    // 方法
    fetchVersions,
  })
}
