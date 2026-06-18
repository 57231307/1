/**
 * useRptEdit - 报表模板创建/编辑表单 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue）
 */
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getReportTemplate,
  createReportTemplate,
  updateReportTemplate,
  getAvailableFields,
  type ReportTemplate,
  type ReportTemplateField,
  type ReportField,
} from '@/api/report-enhanced'

/**
 * 报表模板创建/编辑表单 composable
 */
export function useRptEdit() {
  // 模板编辑对话框
  const dialogVisible = ref(false)
  const dialogTitle = ref('创建模板')
  const isEdit = ref(false)
  const form = ref<Partial<ReportTemplate>>({
    name: '',
    description: '',
    type: '',
    category: '',
    fields: [],
    filters: [],
    group_by: [],
    sort_by: [],
    chart_type: 'none',
  })

  const availableFields = ref<ReportField[]>([])
  const selectedFieldKeys = ref<string[]>([])
  const fieldConfigs = ref<Record<string, Partial<ReportTemplateField>>>({})

  // 图表类型选项
  const chartTypeOptions = [
    { label: '无图表', value: 'none' },
    { label: '柱状图', value: 'bar' },
    { label: '折线图', value: 'line' },
    { label: '饼图', value: 'pie' },
    { label: '面积图', value: 'area' },
  ]

  /**
   * 打开创建模板对话框
   */
  const openCreateDialog = async () => {
    dialogTitle.value = '创建模板'
    isEdit.value = false
    form.value = {
      name: '',
      description: '',
      type: '',
      category: '',
      fields: [],
      filters: [],
      group_by: [],
      sort_by: [],
      chart_type: 'none',
    }
    selectedFieldKeys.value = []
    fieldConfigs.value = {}
    dialogVisible.value = true
  }

  /**
   * 打开编辑模板对话框
   */
  const openEditDialog = async (row: ReportTemplate) => {
    dialogTitle.value = '编辑模板'
    isEdit.value = true
    try {
      const res: any = await getReportTemplate(row.id)
      const data = res.data
      form.value = { ...data }
      selectedFieldKeys.value = data.fields?.map((f: ReportTemplateField) => f.field_key) || []
      fieldConfigs.value = {}
      data.fields?.forEach((f: ReportTemplateField) => {
        fieldConfigs.value[f.field_key] = {
          display_label: f.display_label,
          width: f.width,
          format: f.format,
        }
      })
      dialogVisible.value = true
    } catch {
      ElMessage.error('获取模板详情失败')
    }
  }

  /**
   * 报表类型变化时加载可用字段
   */
  const handleTypeChange = async () => {
    if (!form.value.type) return
    try {
      const res: any = await getAvailableFields(form.value.type)
      availableFields.value = res.data || []
    } catch {
      ElMessage.error('获取可用字段失败')
    }
  }

  /**
   * 已选字段
   */
  const selectedFields = computed(() => {
    return availableFields.value.filter(f => selectedFieldKeys.value.includes(f.key))
  })

  /**
   * 提交模板表单（创建/更新）
   */
  const handleSubmit = async (filterConditions: any[], onSuccess: () => void) => {
    if (!form.value.name || !form.value.type) {
      ElMessage.warning('请填写必填字段')
      return
    }
    if (selectedFieldKeys.value.length === 0) {
      ElMessage.warning('请至少选择一个字段')
      return
    }

    const fields: ReportTemplateField[] = selectedFieldKeys.value.map(key => {
      const field = availableFields.value.find(f => f.key === key)
      const config = fieldConfigs.value[key] || {}
      return {
        field_key: key,
        display_label: config.display_label || field?.label || key,
        visible: true,
        width: config.width,
        format: config.format,
      }
    })

    const data = {
      name: form.value.name!,
      description: form.value.description,
      type: form.value.type!,
      category: form.value.category || 'operation',
      fields,
      filters: filterConditions,
      group_by: form.value.group_by || [],
      sort_by: form.value.sort_by || [],
      chart_type: form.value.chart_type || 'none',
    }

    try {
      if (isEdit.value && form.value.id) {
        await updateReportTemplate(form.value.id, data)
        ElMessage.success('更新成功')
      } else {
        await createReportTemplate(data)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      onSuccess()
    } catch {
      ElMessage.error('操作失败')
    }
  }

  return {
    dialogVisible,
    dialogTitle,
    isEdit,
    form,
    availableFields,
    selectedFieldKeys,
    fieldConfigs,
    chartTypeOptions,
    openCreateDialog,
    openEditDialog,
    handleTypeChange,
    selectedFields,
    handleSubmit,
  }
}
