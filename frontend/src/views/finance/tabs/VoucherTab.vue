<!--
  VoucherTab.vue - 凭证管理 Tab
  来源：原 finance/index.vue 中 凭证管理 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="voucher-tab">
    <div class="page-header">
      <h2 class="page-title">凭证管理</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openVoucherDialog()">
          <el-icon><Plus /></el-icon>
          新建凭证
        </el-button>
        <el-button @click="handlePrintVouchers">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExportVouchers">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="voucherQuery">
        <el-form-item label="凭证号">
          <el-input v-model="voucherQuery.voucher_no" placeholder="凭证号" clearable />
        </el-form-item>
        <el-form-item label="日期范围">
          <el-date-picker
            v-model="voucherQuery.date_range"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            value-format="YYYY-MM-DD"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="voucherQuery.status" placeholder="选择状态" clearable>
            <el-option label="草稿" value="draft" />
            <el-option label="已提交" value="submitted" />
            <el-option label="已审核" value="reviewed" />
            <el-option label="已过账" value="posted" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchVouchers">查询</el-button>
          <el-button @click="resetVoucherQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="voucherLoading" :data="vouchers" stripe>
        <el-table-column prop="voucher_no" label="凭证号" width="120" />
        <el-table-column prop="voucher_date" label="凭证日期" width="120" />
        <el-table-column prop="voucher_type" label="凭证类型" width="100" />
        <el-table-column label="借方金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_debit) }}
          </template>
        </el-table-column>
        <el-table-column label="贷方金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_credit) }}
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getVoucherStatusType(row.status)" size="small">
              {{ getVoucherStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" label="制单人" width="100" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewVoucher(row)">查看</el-button>
            <el-button
              v-if="row.status === 'draft'"
              type="primary"
              link
              size="small"
              @click="submitVoucher(row)"
              >提交</el-button
            >
            <el-button
              v-if="row.status === 'submitted'"
              type="success"
              link
              size="small"
              @click="reviewVoucher(row)"
              >审核</el-button
            >
            <el-button
              v-if="row.status === 'reviewed'"
              type="warning"
              link
              size="small"
              @click="postVoucher(row)"
              >过账</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="voucherQueryParams.page"
          v-model:page-size="voucherQueryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="voucherTotal"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchVouchers"
          @current-change="fetchVouchers"
        />
      </div>
    </el-card>

    <el-dialog v-model="voucherDialogVisible" title="新建凭证" width="800px">
      <el-form ref="voucherFormRef" :model="voucherForm" :rules="voucherRules" label-width="80px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="凭证日期" prop="voucher_date">
              <el-date-picker
                v-model="voucherForm.voucher_date"
                type="date"
                placeholder="选择日期"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="凭证类型" prop="voucher_type">
              <el-select
                v-model="voucherForm.voucher_type"
                placeholder="选择类型"
                style="width: 100%"
              >
                <el-option label="记" value="JZ" />
                <el-option label="收" value="SK" />
                <el-option label="付" value="FK" />
                <el-option label="转" value="ZZ" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider>分录明细</el-divider>
        <el-table :data="voucherForm.entries" stripe>
          <el-table-column label="摘要" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.summary" placeholder="摘要" />
            </template>
          </el-table-column>
          <el-table-column label="科目" min-width="200">
            <template #default="{ row }">
              <el-tree-select
                v-model="row.subject_id"
                :data="leafSubjects"
                :props="{ label: 'name', value: 'id' }"
                placeholder="选择科目"
                check-strictly
              />
            </template>
          </el-table-column>
          <el-table-column label="借方金额" width="130">
            <template #default="{ row }">
              <el-input-number
                v-model="row.debit"
                :min="0"
                :precision="2"
                :controls="false"
                style="width: 100%"
              />
            </template>
          </el-table-column>
          <el-table-column label="贷方金额" width="130">
            <template #default="{ row }">
              <el-input-number
                v-model="row.credit"
                :min="0"
                :precision="2"
                :controls="false"
                style="width: 100%"
              />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="{ $index }">
              <el-button type="danger" link size="small" @click="removeEntry($index)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <div class="entry-footer">
          <el-button type="primary" link @click="addEntry">添加分录</el-button>
          <div class="entry-summary">
            <span>借方合计: {{ formatMoney(totalDebit) }}</span>
            <span>贷方合计: {{ formatMoney(totalCredit) }}</span>
            <span :class="{ 'text-red': !isBalanced }">{{ isBalanced ? '已平衡' : '未平衡' }}</span>
          </div>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="voucherDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="voucherSubmitLoading" @click="submitVoucherForm"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="voucherViewVisible" title="凭证详情" width="800px">
      <el-descriptions :column="3" border>
        <el-descriptions-item label="凭证号">{{ currentVoucher?.voucher_no }}</el-descriptions-item>
        <el-descriptions-item label="凭证日期">{{
          currentVoucher?.voucher_date
        }}</el-descriptions-item>
        <el-descriptions-item label="凭证类型">{{
          currentVoucher?.voucher_type
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getVoucherStatusType(currentVoucher?.status)">
            {{ getVoucherStatusLabel(currentVoucher?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="制单人">{{
          currentVoucher?.created_by_name
        }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{
          currentVoucher?.created_at
        }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>分录明细</el-divider>
      <el-table :data="currentVoucher?.entries" stripe>
        <el-table-column prop="summary" label="摘要" min-width="150" />
        <el-table-column prop="subject_code" label="科目编码" width="100" />
        <el-table-column prop="subject_name" label="科目名称" min-width="150" />
        <el-table-column label="借方" width="120" align="right">
          <template #default="{ row }">
            {{ row.debit ? formatMoney(row.debit) : '' }}
          </template>
        </el-table-column>
        <el-table-column label="贷方" width="120" align="right">
          <template #default="{ row }">
            {{ row.credit ? formatMoney(row.credit) : '' }}
          </template>
        </el-table-column>
      </el-table>
      <div class="entry-footer">
        <span>借方合计: {{ formatMoney(currentVoucher?.total_debit || 0) }}</span>
        <span>贷方合计: {{ formatMoney(currentVoucher?.total_credit || 0) }}</span>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getSubjectTree,
  listVouchers,
  createVoucher,
  submitVoucher as submitVoucherApi,
  reviewVoucher as reviewVoucherApi,
  postVoucher as postVoucherApi,
  type AccountSubject,
  type Voucher,
} from '@/api/finance'
import { logger } from '@/utils/logger'

const vouchers = ref<Voucher[]>([])
const subjects = ref<AccountSubject[]>([])
const voucherLoading = ref(false)
const voucherSubmitLoading = ref(false)
const voucherDialogVisible = ref(false)
const voucherViewVisible = ref(false)
const voucherFormRef = ref<FormInstance>()
const currentVoucher = ref<Voucher | null>(null)

const voucherQuery = reactive({
  voucher_no: '',
  date_range: [] as string[],
  status: '',
})

const voucherQueryParams = reactive({
  page: 1,
  page_size: 20,
})

const voucherTotal = ref(0)

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

const totalDebit = computed(() => voucherForm.entries.reduce((sum, e) => sum + (e.debit || 0), 0))
const totalCredit = computed(() => voucherForm.entries.reduce((sum, e) => sum + (e.credit || 0), 0))
const isBalanced = computed(() => Math.abs(totalDebit.value - totalCredit.value) < 0.01)

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getVoucherStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    submitted: '已提交',
    reviewed: '已审核',
    posted: '已过账',
  }
  return map[status || ''] || status || ''
}

const getVoucherStatusType = (status?: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    submitted: 'warning',
    reviewed: 'success',
    posted: 'primary',
  }
  return map[status || ''] || 'info'
}

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

const openVoucherDialog = () => {
  voucherFormRef.value?.resetFields()
  voucherForm.voucher_date = new Date().toISOString().split('T')[0]
  voucherForm.voucher_type = 'JZ'
  voucherForm.entries = [
    { subject_id: undefined, debit: 0, credit: 0, summary: '' },
    { subject_id: undefined, debit: 0, credit: 0, summary: '' },
  ]
  voucherDialogVisible.value = true
}

const submitVoucherForm = async () => {
  const valid = await voucherFormRef.value?.validate()
  if (!valid) return

  if (!isBalanced.value) {
    ElMessage.warning('借贷不平衡，请检查分录金额')
    return
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
    voucherDialogVisible.value = false
    fetchVouchers()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
  } finally {
    voucherSubmitLoading.value = false
  }
}

const viewVoucher = (row: Voucher) => {
  currentVoucher.value = row
  voucherViewVisible.value = true
}

const submitVoucher = async (row: Voucher) => {
  try {
    await ElMessageBox.confirm('确定提交该凭证吗？', '提交确认', { type: 'info' })
    await submitVoucherApi(row.id)
    ElMessage.success('提交成功')
    fetchVouchers()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const reviewVoucher = async (row: Voucher) => {
  try {
    await ElMessageBox.confirm('确定审核该凭证吗？', '审核确认', { type: 'info' })
    await reviewVoucherApi(row.id)
    ElMessage.success('审核成功')
    fetchVouchers()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const postVoucher = async (row: Voucher) => {
  try {
    await ElMessageBox.confirm('确定过账该凭证吗？过账后不可修改。', '过账确认', {
      type: 'warning',
    })
    await postVoucherApi(row.id)
    ElMessage.success('过账成功')
    fetchVouchers()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const handleExportVouchers = () => {
  const csvContent = [
    ['凭证号', '凭证日期', '凭证类型', '借方金额', '贷方金额', '状态', '制单人', '创建时间'],
    ...vouchers.value.map(item => [
      item.voucher_no,
      item.voucher_date,
      item.voucher_type,
      item.total_debit,
      item.total_credit,
      getVoucherStatusLabel(item.status),
      item.created_by_name,
      item.created_at,
    ]),
  ]
    .map(row => row.map(cell => `"${cell ?? ''}"`).join(','))
    .join('\n')
  const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `凭证列表_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

const handlePrintVouchers = () => {
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error('无法打开打印窗口')
    return
  }
  const rows = vouchers.value
    .map(
      item => `
    <tr>
      <td>${item.voucher_no}</td><td>${item.voucher_date}</td><td>${item.voucher_type}</td>
      <td style="text-align:right">${formatMoney(item.total_debit)}</td>
      <td style="text-align:right">${formatMoney(item.total_credit)}</td>
      <td>${getVoucherStatusLabel(item.status)}</td><td>${item.created_by_name || '-'}</td>
    </tr>
  `
    )
    .join('')
  printWindow.document.write(`<html><head><meta charset="utf-8"><title>凭证列表</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>凭证列表</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${vouchers.value.length} 条</div>
    <table><thead><tr><th>凭证号</th><th>凭证日期</th><th>凭证类型</th><th>借方金额</th><th>贷方金额</th><th>状态</th><th>制单人</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
}

onMounted(() => {
  fetchSubjects()
  fetchVouchers()
})
</script>

<style scoped>
.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
