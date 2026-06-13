<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElSelect,
  ElDatePicker,
  ElMessageBox,
  ElMessage,
  ElRow,
  ElCol,
  ElInputNumber,
  ElPagination,
} from 'element-plus'
import {
  Plus,
  Edit,
  Delete,
  View,
  Refresh,
  Check,
  Printer,
  Download,
} from '@element-plus/icons-vue'
import printJS from 'print-js'
import {
  listVouchers,
  getVoucher,
  createVoucher,
  updateVoucher,
  deleteVoucher,
  approveVoucher,
  postVoucher,
  unpostVoucher,
  getVoucherTypes,
  generateVoucherNo,
  type VoucherEntity,
} from '@/api/voucher'
import { getAccountSubjectTree } from '@/api/account-subject'

const tableData = ref<VoucherEntity[]>([])
const total = ref(0)
const loading = ref(false)
const searchForm = ref({
  voucher_no: '',
  voucher_date_start: '',
  voucher_date_end: '',
  type: '',
  status: '',
})
const pagination = ref({
  page: 1,
  pageSize: 20,
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增凭证')
const form = ref<Partial<VoucherEntity>>({
  voucher_no: '',
  voucher_date: new Date().toISOString().split('T')[0],
  type: 'general',
  status: 'draft',
  description: '',
  total_debit: 0,
  total_credit: 0,
  entries: [{ account_subject_id: 0, debit_amount: 0, credit_amount: 0, description: '' }],
})

const viewDialogVisible = ref(false)
const viewData = ref<VoucherEntity | null>(null)

const voucherTypes = ref<{ label: string; value: string }[]>([])
const accountSubjectOptions = ref<{ label: string; value: number }[]>([])

const statusOptions = [
  { label: '全部', value: '' },
  { label: '草稿', value: 'draft' },
  { label: '已审核', value: 'approved' },
  { label: '已记账', value: 'posted' },
]

const getStatusLabel = (value: string) => {
  return statusOptions.find(s => s.value === value)?.label || value
}

const getStatusClass = (value: string) => {
  switch (value) {
    case 'draft':
      return 'status-draft'
    case 'approved':
      return 'status-approved'
    case 'posted':
      return 'status-posted'
    default:
      return ''
  }
}

const handlePrint = () => {
  const printData = tableData.value.map((item: any, index: number) => ({
    序号: index + 1,
    凭证号: item.voucher_no,
    日期: item.voucher_date,
    类型: item.type === 'general' ? '通用' : '自定义',
    摘要: item.description || '-',
    借方金额: `¥${item.total_debit}`,
    贷方金额: `¥${item.total_credit}`,
    状态: getStatusLabel(item.status),
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}),
    type: 'table' as any,
    header: '会计凭证列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  })
}

const handleExport = () => {
  const csvContent = [
    ['凭证号', '日期', '类型', '摘要', '借方金额', '贷方金额', '状态'],
    ...tableData.value.map((item: any) => [
      item.voucher_no,
      item.voucher_date,
      item.type === 'general' ? '通用' : '自定义',
      item.description || '-',
      item.total_debit,
      item.total_credit,
      getStatusLabel(item.status),
    ]),
  ]
    .map(row => row.map(cell => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `会计凭证_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

const openAddDialog = async () => {
  dialogTitle.value = '新增凭证'
  try {
    const res: any = await generateVoucherNo()
    const voucherNo = res.data?.voucher_no || res.data || ''
    form.value = {
      voucher_no: voucherNo,
      voucher_date: new Date().toISOString().split('T')[0],
      type: 'general',
      status: 'draft',
      description: '',
      total_debit: 0,
      total_credit: 0,
      entries: [{ account_subject_id: 0, debit_amount: 0, credit_amount: 0, description: '' }],
    }
    dialogVisible.value = true
  } catch (error) {
    ElMessage.error('生成凭证号失败')
  }
}

const openEditDialog = async (row: VoucherEntity) => {
  dialogTitle.value = '编辑凭证'
  const res: any = await getVoucher(row.id!)
  form.value = { ...res.data }
  dialogVisible.value = true
}

const openViewDialog = async (row: VoucherEntity) => {
  try {
    const res: any = await getVoucher(row.id!)
    viewData.value = res.data!
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error('获取详情失败')
  }
}

const handleSubmit = async () => {
  if (!form.value.voucher_no || !form.value.voucher_date) {
    ElMessage.warning('请填写必填字段')
    return
  }
  const totalDebit = form.value.total_debit ?? 0
  const totalCredit = form.value.total_credit ?? 0
  if (Math.abs(totalDebit - totalCredit) > 0.01) {
    ElMessage.warning('借贷不平')
    return
  }
  const validEntries = (form.value.entries || []).filter(
    e => e.account_subject_id > 0 && (e.debit_amount > 0 || e.credit_amount > 0)
  )
  if (validEntries.length === 0) {
    ElMessage.warning('请至少添加一条有效的分录')
    return
  }
  try {
    const data = { ...form.value, entries: validEntries }
    if (form.value.id) {
      await updateVoucher(form.value.id, data)
      ElMessage.success('更新成功')
    } else {
      await createVoucher(data)
      ElMessage.success('新增成功')
    }
    dialogVisible.value = false
    loadData()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const handleDelete = async (row: VoucherEntity) => {
  if (row.status === 'posted') {
    ElMessage.warning('已记账的凭证不能删除')
    return
  }
  try {
    await ElMessageBox.confirm('确定要删除这个凭证吗？', '提示', {
      type: 'warning',
    })
    await deleteVoucher(row.id!)
    ElMessage.success('删除成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消删除')
  }
}

const handleApprove = async (row: VoucherEntity) => {
  try {
    await ElMessageBox.confirm('确定要审核这个凭证吗？', '提示', {
      type: 'warning',
    })
    await approveVoucher(row.id!)
    ElMessage.success('审核成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

const handlePost = async (row: VoucherEntity) => {
  try {
    await ElMessageBox.confirm('确定要记账这个凭证吗？', '提示', {
      type: 'warning',
    })
    await postVoucher(row.id!)
    ElMessage.success('记账成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

const handleUnpost = async (row: VoucherEntity) => {
  try {
    await ElMessageBox.confirm('确定要反记账这个凭证吗？', '提示', {
      type: 'warning',
    })
    await unpostVoucher(row.id!)
    ElMessage.success('反记账成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

const calculateTotals = () => {
  if (!form.value.entries) return
  let totalDebit = 0
  let totalCredit = 0
  form.value.entries.forEach(entry => {
    totalDebit += entry.debit_amount || 0
    totalCredit += entry.credit_amount || 0
  })
  form.value.total_debit = totalDebit
  form.value.total_credit = totalCredit
}

const loadData = async () => {
  loading.value = true
  try {
    const params = {
      ...searchForm.value,
      page: pagination.value.page,
      page_size: pagination.value.pageSize,
    }
    const res: any = await listVouchers(params)
    const d = res?.data || res
    tableData.value = Array.isArray(d) ? d : d?.list || d?.items || []
    total.value = res?.total || d?.total || 0
  } catch (error) {
    ElMessage.error('获取凭证列表失败')
  } finally {
    loading.value = false
  }
}

const loadVoucherTypes = async () => {
  try {
    const res: any = await getVoucherTypes()
    const d = res?.data || res
    voucherTypes.value = Array.isArray(d)
      ? d.map((t: any) => (typeof t === 'string' ? { label: t, value: t } : t))
      : []
  } catch (error) {
    console.error('获取凭证类型失败', error)
  }
}

const loadAccountSubjects = async () => {
  try {
    const res: any = await getAccountSubjectTree()
    const d = res?.data || res
    const items = Array.isArray(d) ? d : d?.items || d?.data || []
    const flattenOptions = (items: any[]): { label: string; value: number }[] => {
      const result: { label: string; value: number }[] = []
      const traverse = (nodes: any[]) => {
        nodes.forEach(node => {
          result.push({ label: node.name, value: node.id })
          if (node.children && node.children.length > 0) {
            traverse(node.children)
          }
        })
      }
      traverse(items)
      return result
    }
    accountSubjectOptions.value = flattenOptions(items)
  } catch (error) {
    console.error('获取科目列表失败', error)
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  loadData()
}

const handleReset = () => {
  searchForm.value = {
    voucher_no: '',
    voucher_date_start: '',
    voucher_date_end: '',
    type: '',
    status: '',
  }
  handleSearch()
}

const handlePageChange = (page: number) => {
  pagination.value.page = page
  loadData()
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.value.pageSize = pageSize
  pagination.value.page = 1
  loadData()
}

const getTypeLabel = (type: string) => {
  return voucherTypes.value.find(t => t.value === type)?.label || type
}

const addEntry = () => {
  if (!form.value.entries) {
    form.value.entries = []
  }
  form.value.entries.push({
    account_subject_id: 0,
    debit_amount: 0,
    credit_amount: 0,
    description: '',
  })
}

const removeEntry = (index: number) => {
  if (form.value.entries && form.value.entries.length > 1) {
    form.value.entries.splice(index, 1)
  }
}

watch(() => form.value.entries, calculateTotals, { deep: true })

loadData()
loadVoucherTypes()
loadAccountSubjects()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.voucher_no"
            placeholder="凭证号"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.voucher_date_start"
            type="date"
            placeholder="开始日期"
            class="filter-item"
          />
        </ElCol>
        <ElCol :span="6">
          <ElDatePicker
            v-model="searchForm.voucher_date_end"
            type="date"
            placeholder="结束日期"
            class="filter-item"
          />
        </ElCol>
        <ElCol :span="6">
          <ElSelect v-model="searchForm.status" placeholder="状态" class="filter-item">
            <ElOption v-for="s in statusOptions" :key="s.value" :label="s.label" :value="s.value" />
          </ElSelect>
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">查询</ElButton>
        <ElButton @click="handleReset">重置</ElButton>
        <ElButton type="success" @click="openAddDialog"> <Plus /> 新增凭证 </ElButton>
        <ElButton @click="handlePrint"> <Printer /> 打印 </ElButton>
        <ElButton @click="handleExport"> <Download /> 导出 </ElButton>
      </div>
    </div>

    <ElTable
      :data="tableData"
      :loading="loading"
      border
      fit
      highlight-current-row
      style="width: 100%"
    >
      <ElTableColumn prop="voucher_no" label="凭证号" width="120" />
      <ElTableColumn prop="voucher_date" label="凭证日期" width="120" />
      <ElTableColumn prop="type" label="凭证类型" width="100">
        <template #default="scope">
          {{ getTypeLabel(scope.row.type) }}
        </template>
      </ElTableColumn>
      <ElTableColumn prop="total_debit" label="借方金额" width="120" align="right">
        <template #default="scope">{{ (scope.row.total_debit ?? 0).toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="total_credit" label="贷方金额" width="120" align="right">
        <template #default="scope">{{ (scope.row.total_credit ?? 0).toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="status" label="状态" width="100">
        <template #default="scope">
          <span :class="['status-tag', getStatusClass(scope.row.status)]">
            {{ getStatusLabel(scope.row.status) }}
          </span>
        </template>
      </ElTableColumn>
      <ElTableColumn prop="created_by_name" label="制单人" width="100" />
      <ElTableColumn prop="approved_by_name" label="审核人" width="100" />
      <ElTableColumn prop="posted_by_name" label="记账人" width="100" />
      <ElTableColumn label="操作" width="300" align="center">
        <template #default="scope">
          <ElButton size="small" @click="openViewDialog(scope.row as any)">
            <View />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="primary"
            @click="openEditDialog(scope.row as any)"
          >
            <Edit />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="warning"
            @click="handleApprove(scope.row as any)"
          >
            <Check /> 审核
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'approved'"
            size="small"
            type="success"
            @click="handlePost(scope.row as any)"
          >
            <Check /> 记账
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'posted'"
            size="small"
            type="info"
            @click="handleUnpost(scope.row as any)"
          >
            <Refresh /> 反记账
          </ElButton>
          <ElButton
            v-if="scope.row.status !== 'posted'"
            size="small"
            type="danger"
            @click="handleDelete(scope.row as any)"
          >
            <Delete />
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div
      class="pagination-wrapper"
      style="margin-top: 16px; display: flex; justify-content: flex-end"
    >
      <ElPagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="handlePageSizeChange"
        @current-change="handlePageChange"
      />
    </div>

    <ElDialog v-model="dialogVisible" :title="dialogTitle" width="800px">
      <ElForm :model="form" label-width="100px">
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem label="凭证号" prop="voucher_no">
              <ElInput v-model="form.voucher_no" readonly />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="凭证日期" prop="voucher_date">
              <ElDatePicker v-model="form.voucher_date" type="date" />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem label="凭证类型" prop="type">
              <ElSelect v-model="form.type" placeholder="请选择凭证类型">
                <ElOption
                  v-for="t in voucherTypes"
                  :key="t.value"
                  :label="t.label"
                  :value="t.value"
                />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="摘要" prop="description">
              <ElInput v-model="form.description" placeholder="请输入摘要" />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElFormItem label="分录明细">
          <div class="entries-table">
            <div class="entries-header">
              <span class="col-subject">会计科目</span>
              <span class="col-debit">借方金额</span>
              <span class="col-credit">贷方金额</span>
              <span class="col-desc">摘要</span>
              <span class="col-action">操作</span>
            </div>
            <div v-for="(entry, index) in form.entries" :key="index" class="entries-row">
              <ElSelect
                v-model="entry.account_subject_id"
                placeholder="选择科目"
                class="col-subject"
              >
                <ElOption
                  v-for="subject in accountSubjectOptions"
                  :key="subject.value"
                  :label="subject.label"
                  :value="subject.value"
                />
              </ElSelect>
              <ElInputNumber v-model="entry.debit_amount" :precision="2" class="col-debit" />
              <ElInputNumber v-model="entry.credit_amount" :precision="2" class="col-credit" />
              <ElInput v-model="entry.description" placeholder="摘要" class="col-desc" />
              <ElButton
                v-if="(form.entries || []).length > 1"
                size="small"
                type="danger"
                @click="removeEntry(index)"
              >
                删除
              </ElButton>
            </div>
            <ElButton type="text" @click="addEntry">+ 添加分录</ElButton>
          </div>
        </ElFormItem>
        <ElRow :gutter="20" class="total-row">
          <ElCol :span="12" class="total-item">
            <span class="label">借方合计:</span>
            <span class="value debit">{{ (form.total_debit ?? 0).toFixed(2) }}</span>
          </ElCol>
          <ElCol :span="12" class="total-item">
            <span class="label">贷方合计:</span>
            <span class="value credit">{{ (form.total_credit ?? 0).toFixed(2) }}</span>
            <span
              v-if="Math.abs((form.total_debit ?? 0) - (form.total_credit ?? 0)) > 0.01"
              class="error"
            >
              借贷不平
            </span>
            <span v-else class="success">借贷平衡</span>
          </ElCol>
        </ElRow>
      </ElForm>
      <template #footer>
        <ElButton @click="dialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="handleSubmit">确定</ElButton>
      </template>
    </ElDialog>

    <ElDialog v-model="viewDialogVisible" title="凭证详情" width="800px">
      <div v-if="viewData" class="voucher-detail">
        <div class="voucher-header">
          <div class="header-left">
            <span class="voucher-no">{{ viewData.voucher_no }}</span>
            <span class="voucher-type">{{ getTypeLabel(viewData.type) }}</span>
          </div>
          <div class="header-right">
            <span>{{ viewData.voucher_date }}</span>
            <span :class="['status-tag', getStatusClass(viewData.status)]">
              {{ getStatusLabel(viewData.status) }}
            </span>
          </div>
        </div>
        <div v-if="viewData.description" class="voucher-desc">{{ viewData.description }}</div>
        <div class="entries-table">
          <div class="entries-header">
            <span class="col-subject">会计科目</span>
            <span class="col-debit">借方金额</span>
            <span class="col-credit">贷方金额</span>
            <span class="col-desc">摘要</span>
          </div>
          <div v-for="(entry, index) in viewData.entries" :key="index" class="entries-row">
            <span class="col-subject"
              >{{ entry.account_subject_code }} - {{ entry.account_subject_name }}</span
            >
            <span class="col-debit">{{ entry.debit_amount.toFixed(2) }}</span>
            <span class="col-credit">{{ entry.credit_amount.toFixed(2) }}</span>
            <span class="col-desc">{{ entry.description || '-' }}</span>
          </div>
        </div>
        <div class="total-row">
          <div class="total-item">
            <span class="label">借方合计:</span>
            <span class="value debit">{{ viewData.total_debit.toFixed(2) }}</span>
          </div>
          <div class="total-item">
            <span class="label">贷方合计:</span>
            <span class="value credit">{{ viewData.total_credit.toFixed(2) }}</span>
          </div>
        </div>
        <ElDescriptions :column="3" border class="voucher-meta">
          <ElDescriptionsItem label="制单人">{{
            viewData.created_by_name || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="审核人">{{
            viewData.approved_by_name || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="记账人">{{
            viewData.posted_by_name || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="审核时间">{{
            viewData.approved_at || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="记账时间">{{ viewData.posted_at || '-' }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建时间">{{ viewData.created_at || '-' }}</ElDescriptionsItem>
        </ElDescriptions>
      </div>
    </ElDialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.filter-container {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.filter-actions {
  margin-top: 10px;
}

.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}

.status-draft {
  background: #f5f7fa;
  color: #909399;
}

.status-approved {
  background: #e6f7ff;
  color: #1890ff;
}

.status-posted {
  background: #f0f9eb;
  color: #67c23a;
}

.entries-table {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}

.entries-header {
  display: flex;
  background: #f5f7fa;
  padding: 10px;
  font-weight: bold;
}

.entries-row {
  display: flex;
  padding: 10px;
  border-top: 1px solid #ebeef5;
}

.col-subject {
  flex: 2;
  margin-right: 10px;
}

.col-debit,
.col-credit {
  width: 120px;
  margin-right: 10px;
}

.col-desc {
  flex: 1;
  margin-right: 10px;
}

.col-action {
  width: 60px;
}

.total-row {
  display: flex;
  justify-content: flex-end;
  padding: 10px;
  background: #fafafa;
  margin-top: 10px;
}

.total-item {
  margin-left: 30px;
}

.total-item .label {
  margin-right: 10px;
  font-weight: bold;
}

.total-item .value {
  font-weight: bold;
  font-size: 16px;
}

.total-item .value.debit {
  color: #e74c3c;
}

.total-item .value.credit {
  color: #27ae60;
}

.total-item .error {
  color: #e74c3c;
  margin-left: 10px;
}

.total-item .success {
  color: #27ae60;
  margin-left: 10px;
}

.voucher-detail {
  padding: 20px;
}

.voucher-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.voucher-no {
  font-size: 20px;
  font-weight: bold;
}

.voucher-type {
  margin-left: 10px;
  color: #666;
}

.voucher-desc {
  padding: 10px;
  background: #f5f7fa;
  margin-bottom: 10px;
}

.voucher-meta {
  margin-top: 20px;
}
</style>
