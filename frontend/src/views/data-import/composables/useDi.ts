/**
 * useDi.ts - 数据导入核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 data-import/index.vue）
 * 提供导入模板 / 任务列表查询、分页、状态等核心方法
 * 业务流程（新建/编辑/删除/下载/上传/重试/取消）由 useDiProc 提供
 * 行为完全保持一致（仅结构重构）
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 * 子组件通过 :model-value/@update:model-value 模式传入；不会修改 prop
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { listImportTemplates, listImportTasks, type ImportTemplate, type ImportTask } from '@/api/data-import'
import { logger } from '@/utils/logger'

/**
 * 模板查询参数
 */
export interface TplQuery {
  page: number
  page_size: number
  keyword: string
  module: string
}

/**
 * 任务查询参数
 */
export interface TaskQuery {
  page: number
  page_size: number
  status: string
}

/**
 * 数据导入主业务 composable
 * 集中管理模板 + 任务列表、分页、查询条件
 */
export function useDi() {
  // 当前激活的 Tab
  const activeTab = ref('templates')

  // 模板
  const templates = ref<ImportTemplate[]>([])
  const templateTotal = ref(0)
  const templateLoading = ref(false)
  const templateQuery = reactive<TplQuery>({
    page: 1,
    page_size: 20,
    keyword: '',
    module: '',
  })

  // 任务
  const tasks = ref<ImportTask[]>([])
  const taskTotal = ref(0)
  const taskLoading = ref(false)
  const taskQuery = reactive<TaskQuery>({
    page: 1,
    page_size: 20,
    status: '',
  })

  /**
   * 加载模板列表
   */
  const fetchTemplates = async () => {
    templateLoading.value = true
    try {
      const res = await listImportTemplates(templateQuery as unknown as Record<string, unknown>)
      templates.value = res.data || []
      templateTotal.value = res.total || 0
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : '获取模板失败'
      logger.error(msg)
      ElMessage.error(msg)
    } finally {
      templateLoading.value = false
    }
  }

  /**
   * 加载任务列表
   */
  const fetchTasks = async () => {
    taskLoading.value = true
    try {
      const res = await listImportTasks(taskQuery as unknown as Record<string, unknown>)
      tasks.value = res.data || []
      taskTotal.value = res.total || 0
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : '获取任务失败'
      logger.error(msg)
      ElMessage.error(msg)
    } finally {
      taskLoading.value = false
    }
  }

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    activeTab,
    // 模板
    templates,
    templateTotal,
    templateLoading,
    templateQuery,
    fetchTemplates,
    // 任务
    tasks,
    taskTotal,
    taskLoading,
    taskQuery,
    fetchTasks,
  })
}
