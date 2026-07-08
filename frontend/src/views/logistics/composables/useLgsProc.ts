/**
 * useLgsProc.ts - 物流管理流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 logistics/index.vue）
 * 封装创建 / 编辑 / 查看 / 发货 / 更新状态 / 删除等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 useLgs 的状态引用（Reactive 包装层）；
 * 内部访问 cb.isEdit.value 等即可修改实际 ref 的 value
 */
import { reactive, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { logisticsApi, type LogisticsWaybill, type WaybillStatus } from '@/api/logistics'
import { logger } from '@/utils/logger'

/**
 * 订单表单字段类型
 */
interface LgsFormData {
  id?: number | undefined
  order_id?: number | undefined
  logistics_company?: string
  tracking_number?: string
  driver_name?: string
  driver_phone?: string
  freight_fee?: number
  expected_arrival?: string
  notes?: string
}

/**
 * 状态表单字段类型
 */
interface LgsStatusForm {
  id: number
  currentStatus: WaybillStatus | ''
  newStatus: WaybillStatus | ''
}

/**
 * 流程回调（接收 useLgs 返回的状态）
 * 由于 useLgs 返回 reactive({...})，父组件传入 lgs.dialogVisible 等会自动解包为值；
 * 因此回调使用 plain 类型，useLgsProc 内部通过 cb.isEdit = newValue 修改（reactive 会更新底层 ref）
 */
interface LgsCallbacks {
  // 详情对话框
  detailDialogVisible: boolean
  // 详情数据
  detailData: LogisticsWaybill
  // 表单
  isEdit: boolean
  formData: LgsFormData
  submitLoading: boolean
  dialogVisible: boolean
  // 状态表单
  statusForm: LgsStatusForm
  statusDialogVisible: boolean
  // 列表刷新
  fetchData: () => Promise<void>
}

/**
 * 物流管理流程操作方法集合
 */
export function useLgsProc(cb: LgsCallbacks) {
  /** 打开新建对话框 */
  const handleCreate = () => {
    cb.isEdit = false
    Object.assign(cb.formData, {
      id: undefined,
      order_id: undefined,
      logistics_company: '',
      tracking_number: '',
      driver_name: '',
      driver_phone: '',
      freight_fee: 0,
      expected_arrival: '',
      notes: '',
    })
    cb.dialogVisible = true
  }

  /** 打开编辑对话框 */
  const handleEdit = (row: LogisticsWaybill) => {
    cb.isEdit = true
    Object.assign(cb.formData, {
      id: row.id,
      order_id: row.order_id,
      logistics_company: row.logistics_company,
      tracking_number: row.tracking_number,
      driver_name: row.driver_name,
      driver_phone: row.driver_phone,
      freight_fee: row.freight_fee,
      expected_arrival: row.expected_arrival,
      notes: row.notes,
    })
    cb.dialogVisible = true
  }

  /** 查看详情 */
  const handleView = async (row: LogisticsWaybill) => {
    try {
      const res = await logisticsApi.getById(row.id!)
      cb.detailData = res.data
      cb.detailDialogVisible = true
    } catch (error) {
      logger.error('获取详情失败:', error)
    }
  }

  /**
   * 提交表单（仅 API 调用，校验已由 LgsForm 内部完成）
   */
  const handleSubmit = async () => {
    cb.submitLoading = true
    try {
      if (cb.isEdit && cb.formData.id) {
        await logisticsApi.update(cb.formData.id, cb.formData)
        ElMessage.success('更新成功')
      } else {
        await logisticsApi.create(cb.formData)
        ElMessage.success('创建成功')
      }
      cb.dialogVisible = false
      await cb.fetchData()
    } catch (error) {
      logger.error('提交失败:', error)
    } finally {
      cb.submitLoading = false
    }
  }

  /** 发货 */
  const handleShip = async (row: LogisticsWaybill) => {
    try {
      await ElMessageBox.confirm('确定要发货吗？', '提示', { type: 'warning' })
      await logisticsApi.update(row.id!, { status: 'shipped' })
      ElMessage.success('发货成功')
      await cb.fetchData()
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('发货失败:', error)
      }
    }
  }

  /** 打开更新状态对话框 */
  const handleUpdateStatus = (row: LogisticsWaybill) => {
    cb.statusForm.id = row.id!
    cb.statusForm.currentStatus = row.status
    cb.statusForm.newStatus = ''
    cb.statusDialogVisible = true
  }

  /** 提交状态更新 */
  const handleStatusSubmit = async () => {
    try {
      await logisticsApi.update(cb.statusForm.id, { status: cb.statusForm.newStatus as WaybillStatus })
      ElMessage.success('状态更新成功')
      cb.statusDialogVisible = false
      await cb.fetchData()
    } catch (error) {
      logger.error('状态更新失败:', error)
    }
  }

  /** 删除 */
  const handleDelete = async (row: LogisticsWaybill) => {
    try {
      await ElMessageBox.confirm('确定要删除该运单吗？', '提示', { type: 'warning' })
      await logisticsApi.delete(row.id!)
      ElMessage.success('删除成功')
      await cb.fetchData()
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('删除失败:', error)
      }
    }
  }

  /** 可选新状态映射（根据当前状态） */
  const availableStatuses = computed(() => {
    const map: Record<string, { label: string; value: string }[]> = {
      shipped: [
        { label: '运输中', value: 'in_transit' },
        { label: '已签收', value: 'delivered' },
      ],
      in_transit: [{ label: '已签收', value: 'delivered' }],
    }
    return map[cb.statusForm.currentStatus] || []
  })

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    handleCreate,
    handleEdit,
    handleView,
    handleSubmit,
    handleShip,
    handleUpdateStatus,
    handleStatusSubmit,
    handleDelete,
    availableStatuses,
  })
}
