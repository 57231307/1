/**
 * useDiProc.ts - 数据导入流程操作 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 data-import/index.vue）
 * 封装新建/编辑/删除模板、上传/重试/取消任务、下载模板/错误日志等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 useDi 的状态引用（Reactive 包装层）；
 * 由于 useDi 返回 reactive({...})，父组件传入 di.fetchTemplates 等会自动解包为值
 */
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import {
  createImportTemplate,
  updateImportTemplate,
  deleteImportTemplate,
  downloadImportTemplate,
  uploadImportFile,
  cancelImportTask,
  retryImportTask,
  downloadErrorLog,
  type ImportTemplate,
  type ImportTask,
} from '@/api/data-import'
import { logger } from '@/utils/logger'

/**
 * 模板表单字段类型（所有字段可选，兼容 Partial<ImportTemplate>）
 */
export interface DataImportTemplateFormData {
  id?: number
  template_code?: string
  template_name?: string
  description?: string
  module?: string
  file_format?: string
  columns?: unknown[]
  sample_data?: unknown[]
  status?: string
  [key: string]: unknown
}

/**
 * 流程回调（接收 useDi 返回的状态，自动解包后的值类型）
 * 批次 289：简化为仅包含实际使用的字段（fetchTemplates/fetchTasks/activeTab）
 */
interface DiCallbacks {
  // 模板列表刷新
  fetchTemplates: () => Promise<void>
  // 任务列表刷新
  fetchTasks: () => Promise<void>
  // 当前激活 Tab
  activeTab: string
}

/**
 * 数据导入流程操作方法集合
 */
export function useDiProc(cb: DiCallbacks) {
  // 模板对话框
  const templateDialogVisible = ref(false)
  const templateFormRef = ref<FormInstance>()
  const templateSubmitLoading = ref(false)
  const columnsText = ref('')
  const templateForm = reactive<DataImportTemplateFormData>({
    id: undefined,
    template_code: '',
    template_name: '',
    description: '',
    module: 'customer',
    file_format: 'xlsx',
    columns: [],
    sample_data: [],
    status: 'active',
  })

  const templateRules: FormRules = {
    template_code: [{ required: true, message: '请输入模板编号', trigger: 'blur' }],
    template_name: [{ required: true, message: '请输入模板名称', trigger: 'blur' }],
    module: [{ required: true, message: '请选择模块', trigger: 'change' }],
    file_format: [{ required: true, message: '请选择文件格式', trigger: 'change' }],
  }

  // 上传对话框
  const uploadDialogVisible = ref(false)
  const uploadLoading = ref(false)
  const uploadRef = ref<{ clearFiles: () => void } | null>(null)
  const currentTemplate = ref<ImportTemplate | null>(null)
  const selectedFile = ref<File | null>(null)

  /**
   * 打开模板新建/编辑对话框
   */
  const openTemplateDialog = (row?: ImportTemplate) => {
    if (row) {
      Object.assign(templateForm, row)
      columnsText.value = JSON.stringify(row.columns || [], null, 2)
    } else {
      Object.assign(templateForm, {
        id: undefined,
        template_code: '',
        template_name: '',
        description: '',
        module: 'customer',
        file_format: 'xlsx',
        columns: [],
        sample_data: [],
        status: 'active',
      })
      columnsText.value = ''
    }
    templateDialogVisible.value = true
  }

  /**
   * 提交模板表单
   */
  const handleTemplateSubmit = async () => {
    if (!templateFormRef.value) return
    await templateFormRef.value.validate(async (valid: boolean) => {
      if (!valid) return

      templateSubmitLoading.value = true
      try {
        if (columnsText.value) {
          try {
            templateForm.columns = JSON.parse(columnsText.value)
          } catch {
            ElMessage.error('列配置格式错误，请检查JSON格式')
            return
          }
        }
        if (templateForm.id) {
          await updateImportTemplate(templateForm.id, templateForm as unknown as Partial<ImportTemplate>)
        } else {
          await createImportTemplate(templateForm as unknown as Partial<ImportTemplate>)
        }
        ElMessage.success('操作成功')
        templateDialogVisible.value = false
        await cb.fetchTemplates()
      } catch (error: unknown) {
        const msg = error instanceof Error ? error.message : '操作失败'
        logger.error(msg)
        ElMessage.error(msg)
      } finally {
        templateSubmitLoading.value = false
      }
    })
  }

  /**
   * 删除模板
   */
  const handleDeleteTemplate = async (row: ImportTemplate) => {
    try {
      await ElMessageBox.confirm('确定要删除此模板吗？', '确认删除', { type: 'warning' })
      await deleteImportTemplate(row.id)
      ElMessage.success('删除成功')
      await cb.fetchTemplates()
    } catch (error: unknown) {
      if (error !== 'cancel') {
        const msg = error instanceof Error ? error.message : '删除失败'
        logger.error(msg)
        ElMessage.error(msg)
      }
    }
  }

  /**
   * 下载导入模板
   */
  const handleDownloadTemplate = async (row: ImportTemplate) => {
    try {
      const blob = await downloadImportTemplate(row.id)
      const link = document.createElement('a')
      link.href = URL.createObjectURL(blob)
      link.download = `${row.template_name}_模板.${row.file_format}`
      link.click()
      ElMessage.success('模板下载成功')
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : '下载失败'
      logger.error(msg)
      ElMessage.error(msg)
    }
  }

  /**
   * 打开上传对话框
   */
  const openUploadDialog = (row: ImportTemplate) => {
    currentTemplate.value = row
    selectedFile.value = null
    uploadDialogVisible.value = true
  }

  /**
   * 文件超出限制
   */
  const handleExceed = () => {
    ElMessage.warning('只能上传一个文件')
  }

  /**
   * 文件变化
   */
  const handleFileChange = (file: { raw?: File }) => {
    selectedFile.value = file.raw || null
  }

  /**
   * 提交上传
   */
  const handleUpload = async () => {
    if (!selectedFile.value || !currentTemplate.value) {
      ElMessage.warning('请选择文件')
      return
    }

    uploadLoading.value = true
    try {
      await uploadImportFile(currentTemplate.value.id, selectedFile.value)
      ElMessage.success('导入任务已创建')
      uploadDialogVisible.value = false
      await cb.fetchTasks()
      cb.activeTab = 'tasks'
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : '导入失败'
      logger.error(msg)
      ElMessage.error(msg)
    } finally {
      uploadLoading.value = false
    }
  }

  /**
   * 取消任务
   */
  const handleCancelTask = async (row: ImportTask) => {
    try {
      await ElMessageBox.confirm('确定要取消此任务吗？', '确认取消', { type: 'warning' })
      await cancelImportTask(row.id)
      ElMessage.success('任务已取消')
      await cb.fetchTasks()
    } catch (error: unknown) {
      if (error !== 'cancel') {
        const msg = error instanceof Error ? error.message : '取消失败'
        logger.error(msg)
        ElMessage.error(msg)
      }
    }
  }

  /**
   * 重试任务
   */
  const handleRetryTask = async (row: ImportTask) => {
    try {
      await retryImportTask(row.id)
      ElMessage.success('任务已重新开始')
      await cb.fetchTasks()
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : '重试失败'
      logger.error(msg)
      ElMessage.error(msg)
    }
  }

  /**
   * 下载错误日志
   */
  const handleDownloadErrorLog = async (row: ImportTask) => {
    try {
      const blob = await downloadErrorLog(row.id)
      const link = document.createElement('a')
      link.href = URL.createObjectURL(blob)
      link.download = `错误日志_${row.task_code}.txt`
      link.click()
      ElMessage.success('错误日志下载成功')
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : '下载失败'
      logger.error(msg)
      ElMessage.error(msg)
    }
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 模板对话框
    templateDialogVisible,
    templateFormRef,
    templateSubmitLoading,
    columnsText,
    templateForm,
    templateRules,
    openTemplateDialog,
    handleTemplateSubmit,
    handleDeleteTemplate,
    handleDownloadTemplate,
    // 上传对话框
    uploadDialogVisible,
    uploadLoading,
    uploadRef,
    currentTemplate,
    selectedFile,
    openUploadDialog,
    handleExceed,
    handleFileChange,
    handleUpload,
    // 任务操作
    handleCancelTask,
    handleRetryTask,
    handleDownloadErrorLog,
  })
}
