/**
 * useVchr.ts - 凭证管理核心 composable
 * 任务编号: P14 批 1 B3 I-2（拆分原 VoucherTab.vue）
 * 提供凭证列表查询、表单管理、科目加载等核心方法
 * 流程操作（提交/审核/过账/导出/打印）由 useVchrProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 289：vouchers 接入 useTableApi，移除手写分页逻辑，返回 reactive 包装
 */
import { ref, reactive, computed } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getSubjectTree,
  createVoucher,
  type AccountSubject,
  type Voucher,
} from '@/api/finance'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'
import { formatMoney, getVchrStatusLabel, getVchrStatusType } from './vchrFmts'

/**
 * 凭证管理 composable
 * 集中管理凭证列表、表单、科目等业务状态
 */
export function useVchr() {
  // 列表数据接入 useTableApi
  // 凭证 API 返回 ApiResponse<Voucher[]>，data 为裸数组；useTableApi detectList 兼容裸数组
  // 分页参数使用 snake_case（page/page_size），匹配 useTableApi 默认配置
  const {
    data: vouchers,
    total: voucherTotal,
    loading: voucherLoading,
    page,
    pageSize,
    queryParams,
    refresh: fetchVouchers,
  } = useTableApi<Voucher>({
    url: '/vouchers',
    defaultPageSize: 20,
    defaultParams: {
      voucher_no: '',
      date_range: [] as string[],
      status: '',
    },
    onError: (err: unknown) => {
      logger.error('获取凭证列表失败', err)
      ElMessage.error('获取凭证列表失败')
    },
  })

  const subjects = ref<AccountSubject[]>([])

  // 表单相关
  const voucherFormRef = ref<FormInstance>()
  const voucherSubmitLoading = ref(false)
  const voucherForm = reactive({
    voucher_date: '',
    voucher_type: 'JZ',
    entries: [
      { subject_id: undefined as number | undefined, debit: 0, credit: 0, summary: '' },
      { subject_id: undefined as number | undefined, debit: 0, credit: 0, summary: '' },
    ],
  })

  const voucherRules: FormRules = {
    voucher_date: [{ required: true, message: '请选择凭证日期', trigger: 'change' }],
    voucher_type: [{ required: true, message: '请选择凭证类型', trigger: 'change' }],
  }

  // 详情相关
  const currentVoucher = ref<Voucher | null>(null)

  // 叶子科目（用于树形选择）
  const leafSubjects = computed(() => {
    const flatten = (list: AccountSubject[]): AccountSubject[] => {
      return list.reduce((acc, item) => {
        if (item.is_leaf) acc.push(item)
        if (item.children?.length) acc.push(...flatten(item.children))
        return acc
      }, [] as AccountSubject[])
    }
    return flatten(subjects.value)
  })

  // 借贷合计
  const totalDebit = computed(() => voucherForm.entries.reduce((sum, e) => sum + (e.debit || 0), 0))
  const totalCredit = computed(() => voucherForm.entries.reduce((sum, e) => sum + (e.credit || 0), 0))
  const isBalanced = computed(() => Math.abs(totalDebit.value - totalCredit.value) < 0.01)

  // 科目加载
  const fetchSubjects = async () => {
    try {
      const res = await getSubjectTree()
      const d = res.data as AccountSubject[] | { items?: AccountSubject[]; data?: AccountSubject[] }
      subjects.value = Array.isArray(d) ? d : d?.items || d?.data || []
    } catch (error) {
      const err = error as Error
      logger.warn('获取科目列表失败', err.message)
    }
  }

  /** 查询：重置页码，触发加载（筛选条件已由父组件同步到 queryParams） */
  const handleSearch = () => {
    page.value = 1
    fetchVouchers()
  }

  /** 重置过滤：清空筛选条件 + 重置页码，触发加载 */
  const handleReset = () => {
    queryParams.value = {
      ...queryParams.value,
      voucher_no: '',
      date_range: [],
      status: '',
    }
    page.value = 1
    fetchVouchers()
  }

  // 分录管理
  const addEntry = () => {
    voucherForm.entries.push({ subject_id: undefined, debit: 0, credit: 0, summary: '' })
  }

  const removeEntry = (index: number) => {
    if (voucherForm.entries.length > 2) {
      voucherForm.entries.splice(index, 1)
    } else {
      ElMessage.warning('至少保留两条分录')
    }
  }

  // 表单提交
  const submitVoucherForm = async () => {
    const valid = await voucherFormRef.value?.validate()
    if (!valid) return false

    if (!isBalanced.value) {
      ElMessage.warning('借贷不平衡，请检查分录金额')
      return false
    }

    voucherSubmitLoading.value = true
    try {
      await createVoucher({
        voucher_date: voucherForm.voucher_date,
        voucher_type: voucherForm.voucher_type,
        entries: voucherForm.entries
          .filter(e => e.subject_id)
          .map(e => ({
            subject_id: e.subject_id!,
            debit: e.debit || 0,
            credit: e.credit || 0,
            summary: e.summary,
          })),
      })
      ElMessage.success('创建成功')
      await fetchVouchers()
      return true
    } catch (error) {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
      return false
    } finally {
      voucherSubmitLoading.value = false
    }
  }

  // 详情查看
  const viewVoucher = (row: Voucher) => {
    currentVoucher.value = row
  }

  // 使用 reactive 包装所有 ref 字段，访问 reactive 字段时 Vue 自动解包 ref，
  // 父组件通过 vchr.vouchers 即可直接获得 Voucher[] 类型的值
  return reactive({
    // 列表
    vouchers,
    voucherLoading,
    voucherTotal,
    page,
    pageSize,
    queryParams,
    fetchVouchers,
    handleSearch,
    handleReset,
    // 科目
    subjects,
    leafSubjects,
    fetchSubjects,
    // 表单
    voucherFormRef,
    voucherForm,
    voucherSubmitLoading,
    voucherRules,
    submitVoucherForm,
    addEntry,
    removeEntry,
    // 详情
    currentVoucher,
    viewVoucher,
    // 工具
    formatMoney,
    getVchrStatusLabel,
    getVchrStatusType,
    totalDebit,
    totalCredit,
    isBalanced,
  })
}
