/**
 * usePrc.ts - 采购入库核心 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 purchaseReceipt/index.vue）
 * 提供列表查询 / 表单 / 选项加载 / 详情等核心方法
 * 业务流程（提交 / 删除 / 审核）由 usePrcProc 提供
 * 行为完全保持一致（仅结构重构）
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 * 子组件通过 :model-value/@update:model-value 模式传入；不会修改 prop
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { request } from '@/api/request'
import {
  listPurchaseReceipts,
  type PurchaseReceiptEntity,
  type ReceiptItem,
} from '@/api/purchaseReceipt'
import { logger } from '@/utils/logger'

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
  // 列表
  const tableData = ref<PurchaseReceiptEntity[]>([])
  const total = ref(0)
  const loading = ref(false)

  // 搜索表单
  const searchForm = ref({
    receipt_no: '',
    supplier_id: '',
    warehouse_id: '',
    status: '',
  })

  // 分页
  const pagination = ref({
    page: 1,
    pageSize: 20,
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

  /**
   * 加载列表数据
   */
  const loadData = async () => {
    loading.value = true
    try {
      const res: { data: { list: PurchaseReceiptEntity[]; total: number } | null } =
        (await listPurchaseReceipts({
          page: pagination.value.page,
          pageSize: pagination.value.pageSize,
          receipt_no: searchForm.value.receipt_no,
          supplier_id: searchForm.value.supplier_id
            ? Number(searchForm.value.supplier_id)
            : undefined,
          warehouse_id: searchForm.value.warehouse_id
            ? Number(searchForm.value.warehouse_id)
            : undefined,
          status: searchForm.value.status,
        } as unknown as Record<string, unknown>)) as {
          data: { list: PurchaseReceiptEntity[]; total: number } | null
        }
      tableData.value = res.data!.list
      total.value = res.data!.total
    } catch (error) {
      logger.error('加载失败:', error)
      ElMessage.error('加载失败')
    } finally {
      loading.value = false
    }
  }

  /** 加载供应商 */
  const loadSuppliers = async () => {
    try {
      const res: { data: { label: string; value: number }[] | null } = (await request.get(
        '/suppliers/select'
      )) as { data: { label: string; value: number }[] | null }
      supplierOptions.value = res.data!
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
      warehouseOptions.value = res.data!
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
      productOptions.value = res.data!
    } catch (error) {
      logger.warn('加载产品失败:', error)
    }
  }

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    // 列表
    tableData,
    total,
    loading,
    // 搜索表单
    searchForm,
    // 分页
    pagination,
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
    loadData,
    loadSuppliers,
    loadWarehouses,
    loadProducts,
  })
}
