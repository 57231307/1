/**
 * useVchrLst.ts - 凭证列表核心 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 VoucherListTab.vue）
 * 提供凭证列表查询、表单管理、科目加载、详情等核心方法
 * 业务流程（打印/导出/审核/记账）由 useVchrLstProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 287：tableData 接入 useTableApi，移除手写分页逻辑
 *
 * 设计说明：使用 reactive 而非 ref 包装返回值，便于父组件
 *   const vchr = useVchrLst() 后直接以 plain value 形式访问 vchr.xxx
 *   避免子组件 prop 期望 boolean/array 等基础类型时类型不匹配
 */
import { ref, watch, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getVoucher,
  createVoucher,
  updateVoucher,
  getVoucherTypes,
  generateVoucherNo,
  type VoucherEntity,
} from '@/api/voucher'
import { getAccountSubjectTree } from '@/api/account-subject'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

/** 凭证选项 */
interface SubjectOption {
  label: string
  value: number
}

/**
 * 凭证列表 composable
 * 集中管理列表、表单、详情等业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useVchrLst() {
  // 列表数据接入 useTableApi
  // 凭证 API 返回 ApiResponse<VoucherEntity[]>，data 为裸数组；useTableApi detectList 兼容裸数组
  // 分页参数使用 snake_case（page/page_size），匹配 useTableApi 默认配置
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: loadData,
  } = useTableApi<VoucherEntity>({
    url: '/vouchers',
    defaultPageSize: 20,
    defaultParams: {
      voucher_no: '',
      voucher_date_start: '',
      voucher_date_end: '',
      type: '',
      status: '',
    },
    onError: (err: unknown) => {
      logger.error('获取凭证列表失败', err)
      ElMessage.error('获取凭证列表失败')
    },
  })

  // 新建/编辑对话框表单（reactive 包装以便子组件双向同步字段）
  const dialogTitle = ref('新增凭证')
  const form = reactive<Partial<VoucherEntity>>({
    voucher_no: '',
    voucher_date: new Date().toISOString().split('T')[0],
    type: 'general',
    status: 'draft',
    description: '',
    total_debit: 0,
    total_credit: 0,
    entries: [{ account_subject_id: 0, debit_amount: 0, credit_amount: 0, description: '' }],
  })

  // 详情对话框数据
  const viewData = ref<VoucherEntity | null>(null)

  // 凭证类型选项
  const voucherTypes = ref<{ label: string; value: string }[]>([])

  // 科目选项（扁平化）
  const accountSubjectOptions = ref<SubjectOption[]>([])

  /** 加载凭证类型 */
  const loadVoucherTypes = async () => {
    try {
      const res = await getVoucherTypes()
      const d = (res as { data?: unknown }).data
      if (Array.isArray(d)) {
        voucherTypes.value = (d as string[]).map(t => ({ label: t, value: t }))
      } else {
        voucherTypes.value = []
      }
    } catch (error) {
      logger.error('获取凭证类型失败', error)
    }
  }

  /** 加载科目（扁平化为下拉选项） */
  const loadAccountSubjects = async () => {
    try {
      const res = await getAccountSubjectTree()
      const d = (res as { data?: unknown }).data
      const items = (Array.isArray(d) ? d : []) as {
        id: number
        name: string
        children?: unknown[]
      }[]
      const flattenOptions = (): SubjectOption[] => {
        const result: SubjectOption[] = []
        const traverse = (ns: { id: number; name: string; children?: unknown[] }[]) => {
          ns.forEach(node => {
            result.push({ label: node.name, value: node.id })
            if (
              node.children &&
              (node.children as { id: number; name: string; children?: unknown[] }[]).length > 0
            ) {
              traverse(node.children as { id: number; name: string; children?: unknown[] }[])
            }
          })
        }
        traverse(items)
        return result
      }
      accountSubjectOptions.value = flattenOptions()
    } catch (error) {
      logger.error('获取科目列表失败', error)
    }
  }

  /** 计算借贷合计 */
  const calculateTotals = () => {
    if (!form.entries) return
    let totalDebit = 0
    let totalCredit = 0
    form.entries.forEach(entry => {
      totalDebit += entry.debit_amount || 0
      totalCredit += entry.credit_amount || 0
    })
    form.total_debit = totalDebit
    form.total_credit = totalCredit
  }

  /** 添加分录 */
  const addEntry = () => {
    if (!form.entries) {
      form.entries = []
    }
    form.entries.push({
      account_subject_id: 0,
      debit_amount: 0,
      credit_amount: 0,
      description: '',
    })
  }

  /** 删除分录 */
  const removeEntry = (index: number) => {
    if (form.entries && form.entries.length > 1) {
      form.entries.splice(index, 1)
    }
  }

  /** 准备新增对话框数据（父组件调用后需自行打开对话框） */
  const openAddDialog = async () => {
    dialogTitle.value = '新增凭证'
    try {
      const res = await generateVoucherNo()
      const data = (res as { data?: { voucher_no?: string } | string }).data
      const voucherNo =
        typeof data === 'string' ? data : (data as { voucher_no?: string })?.voucher_no || ''
      form.voucher_no = voucherNo
      form.voucher_date = new Date().toISOString().split('T')[0]
      form.type = 'general'
      form.status = 'draft'
      form.description = ''
      form.total_debit = 0
      form.total_credit = 0
      form.entries = [
        { account_subject_id: 0, debit_amount: 0, credit_amount: 0, description: '' },
      ]
    } catch (error) {
      logger.error('生成凭证号失败', error)
      ElMessage.error('生成凭证号失败')
    }
  }

  /** 准备编辑对话框数据（父组件调用后需自行打开对话框） */
  const openEditDialog = async (row: VoucherEntity) => {
    dialogTitle.value = '编辑凭证'
    const res = await getVoucher(row.id!)
    Object.assign(form, res.data)
  }

  /** 准备查看详情数据（父组件调用后需自行打开对话框） */
  const openViewDialog = async (row: VoucherEntity) => {
    try {
      const res = await getVoucher(row.id!)
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) viewData.value = res.data
    } catch (error) {
      logger.error('获取详情失败', error)
      ElMessage.error('获取详情失败')
    }
  }

  /** 提交表单（新增/编辑） */
  const handleSubmit = async () => {
    if (!form.voucher_no || !form.voucher_date) {
      ElMessage.warning('请填写必填字段')
      return false
    }
    const totalDebit = form.total_debit ?? 0
    const totalCredit = form.total_credit ?? 0
    if (Math.abs(totalDebit - totalCredit) > 0.01) {
      ElMessage.warning('借贷不平')
      return false
    }
    const validEntries = (form.entries || []).filter(
      e => e.account_subject_id > 0 && (e.debit_amount > 0 || e.credit_amount > 0)
    )
    if (validEntries.length === 0) {
      ElMessage.warning('请至少添加一条有效的分录')
      return false
    }
    try {
      const data = { ...form, entries: validEntries }
      if (form.id) {
        await updateVoucher(form.id, data)
        ElMessage.success('更新成功')
      } else {
        await createVoucher(data)
        ElMessage.success('新增成功')
      }
      await loadData()
      return true
    } catch (error) {
      logger.error('操作失败', error)
      ElMessage.error('操作失败')
      return false
    }
  }

  /** 查询：重置页码，触发加载（筛选条件已由父组件同步到 queryParams） */
  const handleSearch = () => {
    page.value = 1
    loadData()
  }

  /** 重置过滤：清空筛选条件 + 重置页码，触发加载 */
  const handleReset = () => {
    queryParams.value = {
      ...queryParams.value,
      voucher_no: '',
      voucher_date_start: '',
      voucher_date_end: '',
      type: '',
      status: '',
    }
    page.value = 1
    loadData()
  }

  /** 监听 entries 变化自动重算合计 */
  watch(() => form.entries, calculateTotals, { deep: true })

  // 使用 reactive 包装所有 ref 字段，访问 reactive 字段时 Vue 自动解包 ref，
  // 父组件通过 vchr.tableData 即可直接获得 VoucherEntity[] 类型的值
  return reactive({
    tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    dialogTitle,
    form,
    viewData,
    voucherTypes,
    accountSubjectOptions,
    loadData,
    handleSearch,
    handleReset,
    openAddDialog,
    openEditDialog,
    handleSubmit,
    addEntry,
    removeEntry,
    calculateTotals,
    openViewDialog,
    loadVoucherTypes,
    loadAccountSubjects,
  })
}
