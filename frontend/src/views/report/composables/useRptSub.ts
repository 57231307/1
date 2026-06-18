/**
 * useRptSub - 报表订阅管理 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue）
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listSubscriptions,
  createSubscription,
  updateSubscription,
  deleteSubscription,
  toggleSubscription,
  sendSubscriptionNow,
  type ReportSubscription,
  type ReportTemplate,
} from '@/api/report-enhanced'
import { logger } from '@/utils/logger'

/**
 * 订阅表单数据结构
 */
export interface SubFormData {
  id: number
  template_id: number
  template_name: string
  schedule: 'daily' | 'weekly' | 'monthly'
  schedule_time: string
  recipients: string
  format: 'pdf' | 'excel' | 'both'
  active: boolean
}

/**
 * 报表订阅管理 composable
 */
export function useRptSub() {
  const subscriptionDialogVisible = ref(false)
  const subscriptions = ref<ReportSubscription[]>([])
  const subscriptionTotal = ref(0)
  const subFormVisible = ref(false)

  /**
   * 订阅表单初始默认值
   */
  const defaultSubForm = (templateId: number = 0, templateName: string = ''): SubFormData => ({
    id: 0,
    template_id: templateId,
    template_name: templateName,
    schedule: 'weekly',
    schedule_time: '09:00',
    recipients: '',
    format: 'excel',
    active: true,
  })

  const subForm = ref<SubFormData>(defaultSubForm())

  /**
   * 加载订阅列表
   */
  const handleSubscriptions = async (row: ReportTemplate) => {
    subForm.value.template_id = row.id
    subForm.value.template_name = row.name
    try {
      const res: any = await listSubscriptions({ template_id: row.id })
      subscriptions.value = res.data?.list || []
      subscriptionTotal.value = res.data?.total || 0
    } catch {
      logger.warn('加载订阅列表失败')
    }
    subscriptionDialogVisible.value = true
  }

  /**
   * 打开订阅表单（新建/编辑）
   */
  const openSubForm = (row?: ReportSubscription) => {
    if (row) {
      subForm.value = {
        id: row.id,
        template_id: row.template_id,
        template_name: row.template_name,
        schedule: row.schedule,
        schedule_time: row.schedule_time,
        recipients: row.recipients.join(', '),
        format: row.format,
        active: row.active,
      }
    } else {
      subForm.value = defaultSubForm(subForm.value.template_id, subForm.value.template_name)
    }
    subFormVisible.value = true
  }

  /**
   * 提交订阅表单（创建/更新）
   */
  const handleSubmitSubscription = async () => {
    if (!subForm.value.recipients) {
      ElMessage.warning('请填写接收人邮箱')
      return
    }
    const recipients = subForm.value.recipients
      .split(',')
      .map(r => r.trim())
      .filter(Boolean)
    const data = {
      template_id: subForm.value.template_id,
      schedule: subForm.value.schedule,
      schedule_time: subForm.value.schedule_time,
      recipients,
      format: subForm.value.format,
    }

    try {
      if (subForm.value.id) {
        await updateSubscription(subForm.value.id, {
          schedule: subForm.value.schedule,
          schedule_time: subForm.value.schedule_time,
          recipients,
          format: subForm.value.format,
          active: subForm.value.active,
        })
        ElMessage.success('更新成功')
      } else {
        await createSubscription(data)
        ElMessage.success('创建成功')
      }
      subFormVisible.value = false
      handleSubscriptions({
        id: subForm.value.template_id,
        name: subForm.value.template_name,
      } as ReportTemplate)
    } catch {
      ElMessage.error('操作失败')
    }
  }

  /**
   * 切换订阅启用状态
   */
  const handleToggleSubscription = async (row: ReportSubscription) => {
    try {
      await toggleSubscription(row.id)
      ElMessage.success('状态已切换')
      handleSubscriptions({ id: row.template_id, name: '' } as ReportTemplate)
    } catch {
      ElMessage.error('操作失败')
    }
  }

  /**
   * 删除订阅
   */
  const handleDeleteSubscription = async (row: ReportSubscription) => {
    try {
      await ElMessageBox.confirm('确定要删除这个订阅吗？', '提示', { type: 'warning' })
      await deleteSubscription(row.id)
      ElMessage.success('删除成功')
      handleSubscriptions({ id: row.template_id, name: '' } as ReportTemplate)
    } catch (error: any) {
      if (error !== 'cancel') {
        ElMessage.error('删除失败')
      }
    }
  }

  /**
   * 立即发送订阅
   */
  const handleSendNow = async (row: ReportSubscription) => {
    try {
      await sendSubscriptionNow(row.id)
      ElMessage.success('发送成功')
    } catch {
      ElMessage.error('发送失败')
    }
  }

  /**
   * 调度频率显示文本
   */
  const getScheduleLabel = (schedule: string) => {
    const map: Record<string, string> = { daily: '每天', weekly: '每周', monthly: '每月' }
    return map[schedule] || schedule
  }

  /**
   * 导出格式显示文本
   */
  const getFormatLabel = (format: string) => {
    const map: Record<string, string> = { pdf: 'PDF', excel: 'Excel', both: 'PDF + Excel' }
    return map[format] || format
  }

  return {
    subscriptionDialogVisible,
    subscriptions,
    subscriptionTotal,
    subFormVisible,
    subForm,
    handleSubscriptions,
    openSubForm,
    handleSubmitSubscription,
    handleToggleSubscription,
    handleDeleteSubscription,
    handleSendNow,
    getScheduleLabel,
    getFormatLabel,
  }
}
