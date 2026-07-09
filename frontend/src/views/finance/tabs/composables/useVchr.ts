/**
 * useVchr.ts - 凭证管理核心 composable
 * 任务编号: P14 批 1 B3 I-2（拆分原 VoucherTab.vue）
 * 提供凭证列表查询、表单管理、科目加载等核心方法
 * 流程操作（提交/审核/过账/导出/打印）由 useVchrProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive, computed } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getSubjectTree,
  listVouchers,
  createVoucher,
  type AccountSubject,
  type Voucher,
} from '@/api/finance'
import { logger } from '@/utils/logger'
import { formatMoney, getVchrStatusLabel, getVchrStatusType } from './vchrFmts'

/**
 * 凭证管理 composable
 * 集中管理凭证列表、表单、科目等业务状态
 */
export function useVchr() {
  // 列表数据
  const vouchers = ref<Voucher[]>([])
  const subjects = ref<AccountSubject[]>([])
  const voucherLoading = ref(false)
  const voucherTotal = ref(0)

  // 列表查询条件
  const voucherQuery = reactive({
    voucher_no: '',
    date_range: [] as string[],
    status: '',
  })

  // 列表分页参数
  const voucherQueryParams = reactive({
    page: 1,
    page_size: 20,
  })

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

  // 凭证列表
  const fetchVouchers = async () => {
    voucherLoading.value = true
    try {
      const params = {
        ...voucherQuery,
        page: voucherQueryParams.page,
        page_size: voucherQueryParams.page_size,
      }
      const res = await listVouchers(params)
      const d = (res as { data?: unknown }).data as
        | Voucher[]
        | { items?: Voucher[]; data?: Voucher[]; list?: Voucher[] }
      if (Array.isArray(d)) {
        vouchers.value = d
      } else {
        vouchers.value = d?.items || d?.data || d?.list || []
      }
      const totalRaw = (res as { total?: number }).total
      voucherTotal.value = totalRaw || (Array.isArray(d) ? d.length : 0)
    } catch (error) {
      const err = error as Error
      ElMessage.error(err.message || '获取凭证列表失败')
    } finally {
      voucherLoading.value = false
    }
  }

  const resetVoucherQuery = () => {
    voucherQuery.voucher_no = ''
    voucherQuery.date_range = []
    voucherQuery.status = ''
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

  return {
    // 列表
    vouchers,
    voucherLoading,
    voucherTotal,
    voucherQuery,
    voucherQueryParams,
    fetchVouchers,
    resetVoucherQuery,
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
  }
}
