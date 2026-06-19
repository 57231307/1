/**
 * usePiProc.ts - 采购验货流程操作 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 purchase-inspection/index.vue）
 * 封装查询 / 重置 / 创建 / 编辑 / 查看 / 提交 / 完成等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 usePi 的状态引用（Reactive 包装层）
 */
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  purchaseInspectionApi,
  type PurchaseInspection,
  type PurchaseInspectionItem,
} from '@/api/purchase-inspection'
import { logger } from '@/utils/logger'

/**
 * 流程回调（接收 usePi 返回的状态，自动解包后的值类型）
 */
interface PiCallbacks {
  // 列表
  tableData: PurchaseInspection[]
  loading: boolean
  total: number
  dateRange: [Date, Date] | null
  // 查询参数
  queryParams: {
    page: number
    page_size: number
    keyword: string
    supplier_id?: number
    status: string
    result: string
  }
  // 选项
  suppliers: { id: number; name: string }[]
  receipts: { id: number; receipt_no: string }[]
  // 表单
  dialogVisible: boolean
  isEdit: boolean
  submitLoading: boolean
  formData: {
    id?: number
    receipt_id?: number
    inspection_date: string
    remark: string
    items: Partial<PurchaseInspectionItem>[]
  }
  // 详情
  detailDialogVisible: boolean
  detailData: PurchaseInspection
  // 方法
  fetchData: () => Promise<void>
  handleReceiptChange: (receiptId: number) => Promise<void>
}

/**
 * 采购验货流程操作方法集合
 */
export function usePiProc(cb: PiCallbacks) {
  /** 查询 */
  const handleQuery = () => {
    cb.queryParams.page = 1
    cb.fetchData()
  }

  /** 重置 */
  const handleReset = () => {
    cb.queryParams.keyword = ''
    cb.queryParams.supplier_id = undefined
    cb.queryParams.status = ''
    cb.queryParams.result = ''
    cb.dateRange = null
    cb.queryParams.page = 1
    cb.fetchData()
  }

  /** 创建检验单 */
  const handleCreate = () => {
    cb.isEdit = false
    Object.assign(cb.formData, {
      id: undefined,
      receipt_id: undefined,
      inspection_date: '',
      remark: '',
      items: [],
    })
    cb.dialogVisible = true
  }

  /** 编辑检验单 */
  const handleEdit = (row: PurchaseInspection) => {
    cb.isEdit = true
    Object.assign(cb.formData, {
      id: row.id,
      receipt_id: row.receipt_id,
      inspection_date: row.inspection_date,
      remark: row.remark || '',
      items: row.items || [],
    })
    cb.dialogVisible = true
  }

  /** 查看详情 */
  const handleView = async (row: PurchaseInspection) => {
    try {
      const res = await purchaseInspectionApi.getById(row.id!)
      cb.detailData = res.data
      cb.detailDialogVisible = true
    } catch (error) {
      logger.error('获取详情失败:', error)
    }
  }

  /** 提交表单（创建/更新） */
  const handleSubmit = async () => {
    try {
      if (cb.isEdit && cb.formData.id) {
        await purchaseInspectionApi.update(cb.formData.id, cb.formData as never)
        ElMessage.success('更新成功')
      } else {
        await purchaseInspectionApi.create(cb.formData as never)
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

  /** 提交前的加载状态开启（父组件调用） */
  const handleBeforeSubmit = () => {
    cb.submitLoading = true
  }

  /** 完成检验 */
  const handleComplete = async (row: PurchaseInspection) => {
    try {
      await ElMessageBox.confirm('确定要完成该检验单吗？', '提示', { type: 'warning' })
      await purchaseInspectionApi.complete(row.id!)
      ElMessage.success('操作成功')
      await cb.fetchData()
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('操作失败:', error)
      }
    }
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return {
    handleQuery,
    handleReset,
    handleCreate,
    handleEdit,
    handleView,
    handleSubmit,
    handleBeforeSubmit,
    handleComplete,
  }
}
