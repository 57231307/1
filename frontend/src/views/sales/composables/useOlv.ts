/**
 * useOlv.ts - 销售订单列表主业务 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales/views/OrderListView.vue）
 * 提供销售订单列表查询/分页/loading、辅助数据加载、统计计算、表单数据、列定义
 * 业务流程（审批/取消/发货/提交）由 useOlvProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive, watch, h } from 'vue'
import { ElTag } from 'element-plus'
import { useTableApi } from '@/composables/useTableApi'
import type { ColumnDef } from '@/components/V2Table/types'
import { type SalesOrder, type SalesOrderItem } from '@/api/sales'
import { request } from '@/api/request'
import type { Customer } from '@/api/customer'
import type { Product } from '@/api/product'
import { getStatusType, getStatusText, formatAmount } from './olvFmts'

/** 销售订单明细行表单类型 */
export interface OrderItemForm {
  id: number
  product_id: number | undefined
  product_name: string
  product_code: string
  quantity: number
  unit: string
  unit_price: number
  subtotal: number
}

/** 销售订单表单类型 */
export interface OrderForm {
  id?: number
  customer_id: number | undefined
  customer_name: string
  order_date: Date | string
  required_date: string
  contact_person: string
  contact_phone: string
  delivery_address: string
  remark: string
  items: OrderItemForm[]
  total_amount?: number
}

/**
 * 销售订单列表 composable
 * 集中管理列表数据、分页、辅助数据、过滤表单、统计、表单、列定义
 * 对话框可见性由父组件本地 ref 管理
 */
export function useOlv() {
  // 过滤表单（本地 UI 状态）
  const filterForm = reactive({
    order_no: '',
    customer_name: '',
    status: '',
    dateRange: [] as Date[] | null,
  })

  // 列表数据（由 useTableApi 接管分页/loading/重试）
  const tableApi = useTableApi<SalesOrder>('/sales/orders')
  const {
    data,
    loading,
    page,
    pageSize,
    total,
    refresh,
    reset: resetTable,
    setQueryParam,
  } = tableApi

  // 辅助数据（不走 useTableApi，保留原 request.get 写法）
  const customers = ref<Customer[]>([])
  const products = ref<Product[]>([])
  const warehouses = ref<{ id: number; warehouse_name?: string; name?: string }[]>([])

  // 统计
  const stats = reactive({
    totalCount: 0,
    pendingCount: 0,
    approvedCount: 0,
    totalAmount: 0,
  })

  // 订单表单对话框
  const formDialogTitle = ref('新建销售订单')
  const submitting = ref(false)
  const formData = reactive<OrderForm>({
    id: 0,
    customer_id: undefined,
    customer_name: '',
    order_date: new Date(),
    required_date: '',
    contact_person: '',
    contact_phone: '',
    delivery_address: '',
    remark: '',
    items: [
      {
        id: Date.now(),
        product_id: undefined,
        product_name: '',
        product_code: '',
        quantity: 1,
        unit: '米',
        unit_price: 0,
        subtotal: 0,
      },
    ],
    total_amount: 0,
  })

  // 详情对话框
  const viewDialogVisible = ref(false)
  const currentOrder = ref<SalesOrder | null>(null)

  // 发货对话框
  const deliveryDialogVisible = ref(false)
  const deliveryForm = reactive({
    order_id: 0,
    order_no: '',
    customer_name: '',
    delivery_date: '',
    warehouse_id: undefined as number | undefined,
    items: [] as {
      product_id: number
      product_name: string
      quantity: number
      delivered_quantity: number
      deliver_quantity: number
      unit_price: number
      remarks: string
    }[],
  })

  // 监听列表数据变化，重新计算统计
  watch(
    [data, total],
    ([orders, totalValue]) => {
      stats.totalCount = totalValue
      stats.pendingCount = orders.filter((o: SalesOrder) => o.status === 'pending').length
      stats.approvedCount = orders.filter((o: SalesOrder) => o.status === 'approved').length
      stats.totalAmount = orders.reduce(
        (sum: number, o: SalesOrder) => sum + (o.total_amount || 0),
        0
      )
    },
    { immediate: true }
  )

  /**
   * 列定义：使用 renderCell 自定义渲染
   * - 订单金额：¥ + toLocaleString
   * - 状态 el-tag：5 种 type 映射
   * 父组件在 OlvTbl 内部追加操作列（查看 / 审批 / 发货 / 取消）
   */
  const columns: ColumnDef<SalesOrder>[] = [
    { key: 'order_no', title: '订单号', width: 140, fixed: 'left' },
    { key: 'customer_name', title: '客户', minWidth: 150 },
    { key: 'order_date', title: '订单日期', width: 120 },
    { key: 'required_date', title: '交货日期', width: 120 },
    {
      key: 'total_amount',
      title: '订单金额',
      width: 120,
      align: 'right',
      renderCell: (row: SalesOrder) => h('span', formatAmount(row.total_amount || 0)),
    },
    {
      key: 'status',
      title: '状态',
      width: 100,
      align: 'center',
      renderCell: (row: SalesOrder) =>
        h(ElTag, { type: getStatusType(row.status), size: 'small' }, () =>
          getStatusText(row.status)
        ),
    },
    { key: 'creator_name', title: '创建人', width: 100 },
  ]

  /** 同步过滤表单到 queryParams 并刷新 */
  const syncQueryParams = () => {
    if (filterForm.order_no) setQueryParam('order_no', filterForm.order_no)
    if (filterForm.customer_name) setQueryParam('customer_name', filterForm.customer_name)
    if (filterForm.status) setQueryParam('status', filterForm.status)
    if (filterForm.dateRange && filterForm.dateRange.length === 2) {
      setQueryParam('start_date', filterForm.dateRange[0])
      setQueryParam('end_date', filterForm.dateRange[1])
    }
    page.value = 1
    refresh()
  }

  /** 查询 */
  const handleQuery = () => {
    syncQueryParams()
  }

  /** 重置 */
  const handleReset = () => {
    filterForm.order_no = ''
    filterForm.customer_name = ''
    filterForm.status = ''
    filterForm.dateRange = null
    resetTable()
    refresh()
  }

  /** 分页 - 当前页（useTableApi 自动 watch 重载） */
  const handlePageChange = (newPage: number) => {
    page.value = newPage
  }

  /** 分页 - 每页大小（useTableApi 自动 watch 重载） */
  const handleSizeChange = (newSize: number) => {
    pageSize.value = newSize
  }

  /** 加载客户 */
  const fetchCustomers = async () => {
    try {
      const res = await request.get<{ list?: Customer[] } | Customer[]>('/customers')
      const d = res
      if (Array.isArray(d)) {
        customers.value = d
      } else if (d && typeof d === 'object' && 'list' in d) {
        customers.value = d.list || []
      } else {
        customers.value = []
      }
    } catch (error) {
      customers.value = []
      void error
    }
  }

  /** 加载产品 */
  const fetchProducts = async () => {
    try {
      const res = await request.get<{ list?: Product[] } | Product[]>('/products')
      const d = res
      if (Array.isArray(d)) {
        products.value = d
      } else if (d && typeof d === 'object' && 'list' in d) {
        products.value = d.list || []
      } else {
        products.value = []
      }
    } catch (error) {
      products.value = []
      void error
    }
  }

  /** 加载仓库 */
  const fetchWarehouses = async () => {
    try {
      const res = await request.get<
        | { list?: { id: number; warehouse_name?: string; name?: string }[] }
        | { id: number; warehouse_name?: string; name?: string }[]
      >('/warehouses')
      const d = res
      if (Array.isArray(d)) {
        warehouses.value = d
      } else if (d && typeof d === 'object' && 'list' in d) {
        warehouses.value = d.list || []
      } else {
        warehouses.value = []
      }
    } catch (error) {
      warehouses.value = []
      void error
    }
  }

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    formDialogTitle.value = '新建销售订单'
    Object.assign(formData, {
      id: 0,
      customer_id: undefined,
      customer_name: '',
      order_date: new Date(),
      required_date: '',
      contact_person: '',
      contact_phone: '',
      delivery_address: '',
      remark: '',
      items: [
        {
          id: Date.now(),
          product_id: undefined,
          product_name: '',
          product_code: '',
          quantity: 1,
          unit: '米',
          unit_price: 0,
          subtotal: 0,
        },
      ],
      total_amount: 0,
    })
  }

  /** 准备编辑表单（父组件需自行打开对话框） */
  const prepareEdit = (row: SalesOrder) => {
    formDialogTitle.value = '编辑销售订单'
    Object.assign(formData, {
      id: row.id,
      customer_id: row.customer_id,
      customer_name: row.customer_name,
      order_date: row.order_date,
      required_date: row.required_date || '',
      contact_person: row.contact_person || '',
      contact_phone: row.contact_phone || '',
      delivery_address: row.delivery_address || '',
      remark: row.remark || '',
      items: row.items?.map((it: SalesOrderItem) => ({
        id: it.id || Date.now(),
        product_id: it.product_id,
        product_name: it.product_name,
        product_code: it.product_code || '',
        quantity: it.quantity,
        unit: it.unit || '',
        unit_price: it.unit_price,
        subtotal: it.subtotal,
      })) || [
        {
          id: Date.now(),
          product_id: undefined,
          product_name: '',
          product_code: '',
          quantity: 1,
          unit: '米',
          unit_price: 0,
          subtotal: 0,
        },
      ],
      total_amount: row.total_amount,
    })
  }

  /** 准备发货对话框（父组件需自行打开对话框） */
  const prepareDelivery = (row: SalesOrder) => {
    Object.assign(deliveryForm, {
      order_id: row.id,
      order_no: row.order_no,
      customer_name: row.customer_name,
      delivery_date: '',
      warehouse_id: undefined,
      items:
        row.items?.map(item => ({
          product_id: item.product_id,
          product_name: item.product_name,
          quantity: item.quantity,
          delivered_quantity: item.delivered_quantity || 0,
          deliver_quantity: 0,
          unit_price: item.unit_price,
          remarks: '',
        })) || [],
    })
  }

  /** 初始化加载 */
  const initLoad = () => {
    fetchCustomers()
    fetchProducts()
    fetchWarehouses()
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 列表与分页
    data,
    loading,
    page,
    pageSize,
    total,
    refresh,
    // 辅助数据
    customers,
    products,
    warehouses,
    // 过滤表单
    filterForm,
    // 统计
    stats,
    // 表单
    formDialogTitle,
    submitting,
    formData,
    // 详情对话框
    viewDialogVisible,
    currentOrder,
    // 发货对话框
    deliveryDialogVisible,
    deliveryForm,
    // 列定义
    columns,
    // 操作
    handleQuery,
    handleReset,
    handlePageChange,
    handleSizeChange,
    fetchCustomers,
    fetchProducts,
    fetchWarehouses,
    prepareCreate,
    prepareEdit,
    prepareDelivery,
    initLoad,
  })
}

export type { OrderForm, OrderItemForm }
