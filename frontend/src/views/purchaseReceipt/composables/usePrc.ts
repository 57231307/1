/**
 * usePrc.ts - 采购入库核心 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 purchaseReceipt/index.vue）
 * 提供列表查询 / 表单 / 选项加载 / 详情等核心方法
 * 业务流程（提交 / 删除 / 审核）由 usePrcProc 提供
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 * 子组件通过 :model-value/@update:model-value 模式传入；不会修改 prop
 * 批次 285：tableData 接入 useTableApi，移除手写分页/加载逻辑
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { request } from '@/api/request'
import {
  type PurchaseReceiptEntity,
  type ReceiptItem,
} from '@/api/purchaseReceipt'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

/**
 * 入库表单字段类型（所有字段可选，兼容 Partial<PurchaseReceiptEntity>）
 */
export interface PrcForm {
  id?: number
  receipt_no?: string
  receipt_date?: string
  supplier_id?: number
  warehouse_id?: number
  status?: string
  items?: ReceiptItem[]
  [key: string]: unknown
}

/**
 * 采购入库主业务 composable
 * 集中管理列表、分页、搜索表单、入库表单、选项加载、详情
 */
export function usePrc() {
  // 列表 - 接入 useTableApi（批次 285）
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: loadData,
  } = useTableApi<PurchaseReceiptEntity>({
    url: '/purchase/receipts',
    defaultPageSize: 20,
    defaultParams: {
      receipt_no: '',
      supplier_id: '' as string | undefined,
      warehouse_id: '' as string | undefined,
      status: '',
    },
    onError: (err: unknown) => {
      // 使用类型守卫安全提取错误信息
      const errMsg = err instanceof Error ? err.message : ''
      ElMessage.error(errMsg || '加载失败')
    },
  })

  // 入库表单对话框
  const dialogVisible = ref(false)
  const dialogTitle = ref('新增入库')
  const form = ref<PrcForm>({
    receipt_no: '',
    receipt_date: new Date().toISOString().split('T')[0],
    supplier_id: undefined,
    warehouse_id: undefined,
    status: 'draft',
    items: [],
  })

  // 表单验证规则
  const formRules = {
    supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
    warehouse_id: [{ required: true, message: '请选择仓库', trigger: 'change' }],
    receipt_date: [{ required: true, message: '请选择入库日期', trigger: 'change' }],
  }

  // 详情对话框
  const viewDialogVisible = ref(false)
  const viewData = ref<PurchaseReceiptEntity | null>(null)
  const detailData = ref<ReceiptItem[]>([])

  // 选项
  const supplierOptions = ref<{ label: string; value: number }[]>([])
  const warehouseOptions = ref<{ label: string; value: number }[]>([])
  const productOptions = ref<{ label: string; value: number }[]>([])

  /** 加载供应商 */
  const loadSuppliers = async () => {
    try {
      const res: { data: { label: string; value: number }[] | null } = (await request.get(
        '/suppliers/select'
      )) as { data: { label: string; value: number }[] | null }
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) supplierOptions.value = res.data
    } catch (error) {
      logger.warn('加载供应商失败:', error)
    }
  }

  /** 加载仓库 */
  const loadWarehouses = async () => {
    try {
      const res: { data: { label: string; value: number }[] | null } = (await request.get(
        '/warehouses/select'
      )) as { data: { label: string; value: number }[] | null }
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) warehouseOptions.value = res.data
    } catch (error) {
      logger.warn('加载仓库失败:', error)
    }
  }

  /** 加载产品 */
  const loadProducts = async () => {
    try {
      const res: { data: { label: string; value: number }[] | null } = (await request.get(
        '/products/select'
      )) as { data: { label: string; value: number }[] | null }
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) productOptions.value = res.data
    } catch (error) {
      logger.warn('加载产品失败:', error)
    }
  }

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    // 列表（useTableApi 管理）
    tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    loadData,
    // 入库表单
    dialogVisible,
    dialogTitle,
    form,
    formRules,
    // 详情
    viewDialogVisible,
    viewData,
    detailData,
    // 选项
    supplierOptions,
    warehouseOptions,
    productOptions,
    // 加载方法
    loadSuppliers,
    loadWarehouses,
    loadProducts,
  })
}
