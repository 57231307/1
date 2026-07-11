/**
 * useDi.ts - 数据导入核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 data-import/index.vue）
 * 提供导入模板 / 任务列表查询、分页、状态等核心方法
 * 业务流程（新建/编辑/删除/下载/上传/重试/取消）由 useDiProc 提供
 * 行为完全保持一致（仅结构重构）
 *
 * 批次 289：templates 和 tasks 分别接入 useTableApi（两个实例），
 *   移除手写分页逻辑，返回 reactive 包装
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import type { ImportTemplate, ImportTask } from '@/api/data-import'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

/**
 * 数据导入主业务 composable
 * 集中管理模板 + 任务列表、分页、查询条件
 * 两个表格各自使用独立的 useTableApi 实例
 */
export function useDi() {
  // 当前激活的 Tab
  const activeTab = ref('templates')

  // 模板列表接入 useTableApi
  // API 返回 ApiResponse<ImportTemplate[]>，data 为裸数组；useTableApi detectList 兼容裸数组
  const {
    data: templates,
    total: templateTotal,
    loading: templateLoading,
    page: templatePage,
    pageSize: templatePageSize,
    queryParams: templateQueryParams,
    refresh: fetchTemplates,
  } = useTableApi<ImportTemplate>({
    url: '/data-import/templates',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      module: '',
    },
    onError: (err: unknown) => {
      logger.error('获取模板列表失败', err)
      ElMessage.error('获取模板列表失败')
    },
  })

  // 任务列表接入 useTableApi
  const {
    data: tasks,
    total: taskTotal,
    loading: taskLoading,
    page: taskPage,
    pageSize: taskPageSize,
    queryParams: taskQueryParams,
    refresh: fetchTasks,
  } = useTableApi<ImportTask>({
    url: '/data-import/tasks',
    defaultPageSize: 20,
    defaultParams: {
      status: '',
    },
    onError: (err: unknown) => {
      logger.error('获取任务列表失败', err)
      ElMessage.error('获取任务列表失败')
    },
  })

  /** 模板查询：重置页码，触发加载（筛选条件已由父组件同步到 queryParams） */
  const handleTemplateSearch = () => {
    templatePage.value = 1
    fetchTemplates()
  }

  /** 任务查询：重置页码，触发加载（筛选条件已由父组件同步到 queryParams） */
  const handleTaskSearch = () => {
    taskPage.value = 1
    fetchTasks()
  }

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    activeTab,
    // 模板
    templates,
    templateTotal,
    templateLoading,
    templatePage,
    templatePageSize,
    templateQueryParams,
    fetchTemplates,
    handleTemplateSearch,
    // 任务
    tasks,
    taskTotal,
    taskLoading,
    taskPage,
    taskPageSize,
    taskQueryParams,
    fetchTasks,
    handleTaskSearch,
  })
}
