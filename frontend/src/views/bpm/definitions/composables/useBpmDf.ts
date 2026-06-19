/**
 * useBpmDf.ts - BPM 流程定义核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 bpm/definitions.vue）
 * 提供流程定义列表 / 分页 / 过滤 / 节点列表等核心方法
 * 业务流程（创建/编辑/删除/版本/创建版本/激活/保存为模板）由 useBpmDfProc 提供
 * 行为完全保持一致（仅结构重构）
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

/**
 * BPM 流程定义主业务 composable
 * 集中管理列表、分页、过滤、节点、版本
 */
export function useBpmDf() {
  // 列表
  const definitions = ref<ProcessDefinition[]>([])
  const loading = ref(false)
  const total = ref(0)

  // 过滤
  const queryParams = reactive({
    page: 1,
    page_size: 20,
    keyword: '',
    category: '',
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

  /**
   * 加载列表
   */
  const fetchDefinitions = async () => {
    loading.value = true
    try {
      const res = await bpmEnhancedApi.listDefinitions(queryParams as never)
      definitions.value = (res.data?.list || []) as ProcessDefinition[]
      total.value = (res.data?.total || 0) as number
    } catch (error) {
      const msg = error instanceof Error ? error.message : '获取流程定义列表失败'
      logger.error(msg)
      ElMessage.error(msg)
    } finally {
      loading.value = false
    }
  }

  /**
   * 加载版本
   */
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
    // 列表
    definitions,
    loading,
    total,
    // 过滤
    queryParams,
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
    fetchDefinitions,
    fetchVersions,
  })
}
