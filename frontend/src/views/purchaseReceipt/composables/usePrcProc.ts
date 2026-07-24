/**
 * usePrcProc.ts - 采购入库流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 purchaseReceipt/index.vue）
 * 封装搜索 / 翻页 / 打开对话框 / 增删明细 / 提交 / 删除 / 审核等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 usePrc 的状态引用（Reactive 包装层）；
 * 由于 usePrc 返回 reactive({...})，父组件传入 prc.searchForm 等会自动解包为值
 */
import { reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  getPurchaseReceipt,
  getReceiptItems,
  createPurchaseReceipt,
  updatePurchaseReceipt,
  deletePurchaseReceipt,
  approvePurchaseReceipt,
  type PurchaseReceiptEntity,
  type ReceiptItem,
} from '@/api/purchaseReceipt'
import type { PrcForm } from './usePrc'

/**
 * 流程回调（接收 usePrc 返回的状态，自动解包后的值类型）
 * 批次 285：queryParams 放宽为 Record<string, unknown>，page 改为独立字段
 */
interface PrcCallbacks {
  // 查询参数（放宽为 Record<string, unknown>，兼容 useTableApi）
  queryParams: Record<string, unknown>
  // 当前页（独立字段，不在 queryParams 内）
  page: number
  // 表单
  dialogVisible: boolean
  dialogTitle: string
  form: PrcForm
  // 详情
  viewDialogVisible: boolean
  viewData: PurchaseReceiptEntity | null
  detailData: ReceiptItem[]
  // 列表刷新
  loadData: () => Promise<void>
}

/**
 * 采购入库流程操作方法集合
 */
export function usePrcProc(cb: PrcCallbacks) {
  /** 查询 */
  const handleSearch = () => {
    cb.page = 1
    cb.loadData()
  }

  /** 重置 */
  const handleReset = () => {
    cb.queryParams = {
      receipt_no: '',
      supplier_id: '',
      warehouse_id: '',
      status: '',
    }
    cb.page = 1
    cb.loadData()
  }

  /** 打开新增对话框 */
  const openAddDialog = () => {
    cb.dialogTitle = '新增入库'
    cb.form = {
      receipt_no: '',
      receipt_date: new Date().toISOString().split('T')[0],
      supplier_id: undefined,
      warehouse_id: undefined,
      status: 'draft',
      items: [{ product_id: 0, quantity: 0, price: 0, amount: 0 }],
    }
    cb.dialogVisible = true
  }

  /** 打开编辑对话框 */
  const openEditDialog = async (row: PurchaseReceiptEntity) => {
    cb.dialogTitle = '编辑入库'
    const res = await getPurchaseReceipt(row.id!)
    const itemsRes = await getReceiptItems(row.id!)
    cb.form = { ...(res.data as unknown as PrcForm), items: itemsRes.data?.items || [] }
    cb.dialogVisible = true
  }

  /** 打开详情对话框 */
  const openViewDialog = async (row: PurchaseReceiptEntity) => {
    try {
      const res = await getPurchaseReceipt(row.id!)
      cb.viewData = res.data || null
      const itemsRes = await getReceiptItems(row.id!)
      cb.detailData = itemsRes.data?.items || []
      cb.viewDialogVisible = true
    } catch (error) {
      ElMessage.error('获取详情失败')
    }
  }

  /** 添加明细 */
  const addItem = () => {
    if (!cb.form.items) cb.form.items = []
    cb.form.items.push({ product_id: 0, quantity: 0, price: 0, amount: 0 })
  }

  /** 删除明细 */
  const removeItem = (index: number) => {
    if ((cb.form.items || []).length > 1) {
      cb.form.items!.splice(index, 1)
    }
  }

  /** 计算明细金额 */
  const calculateItemAmount = (item: ReceiptItem) => {
    item.amount = (item.quantity || 0) * (item.price || 0)
  }

  /**
   * 提交表单（仅 API 调用 + 明细校验，表单规则校验已由 PurchaseReceiptForm 内部完成）
   */
  const handleSubmit = async () => {
    const validItems = (cb.form.items || []).filter(
      e => e.product_id > 0 && e.quantity !== 0
    )
    if (validItems.length === 0) {
      ElMessage.warning('请至少添加一条有效的入库明细')
      return
    }

    try {
      const data = { ...cb.form, items: validItems }
      if (cb.form.id) {
        await updatePurchaseReceipt(cb.form.id, data as PurchaseReceiptEntity)
        ElMessage.success('更新成功')
      } else {
        await createPurchaseReceipt(data as PurchaseReceiptEntity)
        ElMessage.success('新增成功')
      }
      cb.dialogVisible = false
      await cb.loadData()
    } catch (error) {
      ElMessage.error('操作失败')
    }
  }

  /** 删除入库单 */
  const handleDelete = async (row: PurchaseReceiptEntity) => {
    if (row.status === 'approved') {
      ElMessage.warning('已审核的入库单不能删除')
      return
    }
    try {
      await ElMessageBox.confirm('确定要删除这个入库单吗？', '提示', { type: 'warning' })
      await deletePurchaseReceipt(row.id!)
      ElMessage.success('删除成功')
      await cb.loadData()
    } catch (error) {
      ElMessage.info('取消删除')
    }
  }

  /** 审核入库单 */
  const handleApprove = async (row: PurchaseReceiptEntity) => {
    try {
      await ElMessageBox.confirm('确定要审核这个入库单吗？', '提示', { type: 'warning' })
      await approvePurchaseReceipt(row.id!)
      ElMessage.success('审核成功')
      await cb.loadData()
    } catch (error) {
      ElMessage.info('取消操作')
    }
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    handleSearch,
    handleReset,
    openAddDialog,
    openEditDialog,
    openViewDialog,
    addItem,
    removeItem,
    calculateItemAmount,
    handleSubmit,
    handleDelete,
    handleApprove,
  })
}
