<template>
  <div class="finance-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="科目管理" name="subject">
        <div class="page-header">
          <h2 class="page-title">会计科目</h2>
          <div class="header-actions">
            <el-button type="primary" @click="openSubjectDialog()">
              <el-icon><Plus /></el-icon>
              新建科目
            </el-button>
            <el-button @click="handlePrintSubjects">
              <el-icon><Printer /></el-icon>
              打印
            </el-button>
            <el-button @click="handleExportSubjects">
              <el-icon><Download /></el-icon>
              导出
            </el-button>
          </div>
        </div>

        <el-card shadow="hover">
          <el-table
            v-loading="subjectLoading"
            :data="subjects"
            stripe
            row-key="id"
            default-expand-all
          >
            <el-table-column prop="code" label="科目编码" width="120" />
            <el-table-column prop="name" label="科目名称" min-width="200" />
            <el-table-column prop="category" label="科目类别" width="100">
              <template #default="{ row }">
                <el-tag size="small">{{ getCategoryLabel(row.category) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="direction" label="余额方向" width="100">
              <template #default="{ row }">
                <el-tag :type="row.direction === 'debit' ? 'success' : 'danger'" size="small">
                  {{ row.direction === 'debit' ? '借方' : '贷方' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="level" label="级次" width="80" align="center" />
            <el-table-column prop="is_leaf" label="末级" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.is_leaf ? 'success' : 'info'" size="small">
                  {{ row.is_leaf ? '是' : '否' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
                  {{ row.status === 1 ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openSubjectDialog(row)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="deleteSubject(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="凭证管理" name="voucher">
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
                <el-button type="primary" link size="small" @click="viewVoucher(row)"
                  >查看</el-button
                >
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
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="subjectDialogVisible"
      :title="subjectForm.id ? '编辑科目' : '新建科目'"
      width="500px"
    >
      <el-form ref="subjectFormRef" :model="subjectForm" :rules="subjectRules" label-width="80px">
        <el-form-item label="科目编码" prop="code">
          <el-input v-model="subjectForm.code" placeholder="请输入科目编码" />
        </el-form-item>
        <el-form-item label="科目名称" prop="name">
          <el-input v-model="subjectForm.name" placeholder="请输入科目名称" />
        </el-form-item>
        <el-form-item label="上级科目">
          <el-tree-select
            v-model="subjectForm.parent_id"
            :data="subjectTreeData"
            :props="{ label: 'name', value: 'id' }"
            placeholder="选择上级科目"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item label="科目类别" prop="category">
          <el-select v-model="subjectForm.category" placeholder="选择科目类别">
            <el-option label="资产" value="asset" />
            <el-option label="负债" value="liability" />
            <el-option label="所有者权益" value="equity" />
            <el-option label="成本" value="cost" />
            <el-option label="损益" value="profit_loss" />
          </el-select>
        </el-form-item>
        <el-form-item label="余额方向" prop="direction">
          <el-radio-group v-model="subjectForm.direction">
            <el-radio value="debit">借方</el-radio>
            <el-radio value="credit">贷方</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="状态">
          <el-switch v-model="subjectForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="subjectDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="subjectSubmitLoading" @click="submitSubject"
          >确定</el-button
        >
      </template>
    </el-dialog>

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
import printJS from 'print-js'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getSubjectTree,
  createSubject,
  updateSubject,
  deleteSubject as deleteSubjectApi,
  type AccountSubject,
} from '@/api/finance'
import {
  listVouchers,
  createVoucher,
  submitVoucher as submitVoucherApi,
  reviewVoucher as reviewVoucherApi,
  postVoucher as postVoucherApi,
  type Voucher,
} from '@/api/finance'

const activeTab = ref('subject')

const subjects = ref<AccountSubject[]>([])
const vouchers = ref<Voucher[]>([])
const subjectLoading = ref(false)
const voucherLoading = ref(false)

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

const fetchSubjects = async () => {
  subjectLoading.value = true
  try {
    const res = await getSubjectTree()
    const d = res.data as any
    subjects.value = d?.items || d?.data || d || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取科目列表失败')
  } finally {
    subjectLoading.value = false
  }
}

const fetchVouchers = async () => {
  voucherLoading.value = true
  try {
    const res = await listVouchers(voucherQuery)
    const d = res.data as any
    vouchers.value = d?.items || d?.data || d || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取凭证列表失败')
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

const subjectTreeData = computed(() => subjects.value)
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

const getCategoryLabel = (category: string) => {
  const map: Record<string, string> = {
    asset: '资产',
    liability: '负债',
    equity: '权益',
    cost: '成本',
    profit_loss: '损益',
  }
  return map[category] || category
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

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const subjectDialogVisible = ref(false)
const subjectFormRef = ref<FormInstance>()
const subjectSubmitLoading = ref(false)
const subjectForm = reactive({
  id: 0,
  code: '',
  name: '',
  parent_id: undefined as number | undefined,
  category: '',
  direction: 'debit',
  status: 1,
})

const subjectRules: FormRules = {
  code: [{ required: true, message: '请输入科目编码', trigger: 'blur' }],
  name: [{ required: true, message: '请输入科目名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择科目类别', trigger: 'change' }],
  direction: [{ required: true, message: '请选择余额方向', trigger: 'change' }],
}

const openSubjectDialog = (row?: AccountSubject) => {
  subjectFormRef.value?.resetFields()
  if (row) {
    subjectForm.id = row.id
    subjectForm.code = row.code
    subjectForm.name = row.name
    subjectForm.parent_id = row.parent_id
    subjectForm.category = row.category
    subjectForm.direction = row.direction
    subjectForm.status = row.status
  } else {
    subjectForm.id = 0
    subjectForm.code = ''
    subjectForm.name = ''
    subjectForm.parent_id = undefined
    subjectForm.category = ''
    subjectForm.direction = 'debit'
    subjectForm.status = 1
  }
  subjectDialogVisible.value = true
}

const submitSubject = async () => {
  const valid = await subjectFormRef.value?.validate()
  if (!valid) return

  subjectSubmitLoading.value = true
  try {
    if (subjectForm.id) {
      await updateSubject(subjectForm.id, { name: subjectForm.name, status: subjectForm.status })
      ElMessage.success('更新成功')
    } else {
      await createSubject({
        code: subjectForm.code,
        name: subjectForm.name,
        parent_id: subjectForm.parent_id,
        category: subjectForm.category,
        direction: subjectForm.direction,
      })
      ElMessage.success('创建成功')
    }
    subjectDialogVisible.value = false
    fetchSubjects()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    subjectSubmitLoading.value = false
  }
}

const deleteSubject = async (row: AccountSubject) => {
  try {
    await ElMessageBox.confirm(`确定删除科目 "${row.name}" 吗？`, '删除确认', { type: 'warning' })
    await deleteSubjectApi(row.id)
    ElMessage.success('删除成功')
    fetchSubjects()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

const voucherDialogVisible = ref(false)
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

const totalDebit = computed(() => voucherForm.entries.reduce((sum, e) => sum + (e.debit || 0), 0))
const totalCredit = computed(() => voucherForm.entries.reduce((sum, e) => sum + (e.credit || 0), 0))
const isBalanced = computed(() => Math.abs(totalDebit.value - totalCredit.value) < 0.01)

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
        .filter((e) => e.subject_id)
        .map((e) => ({
          subject_id: e.subject_id!,
          debit: e.debit || 0,
          credit: e.credit || 0,
          summary: e.summary,
        })),
    })
    ElMessage.success('创建成功')
    voucherDialogVisible.value = false
    fetchVouchers()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    voucherSubmitLoading.value = false
  }
}

const voucherViewVisible = ref(false)
const currentVoucher = ref<Voucher | null>(null)

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
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const reviewVoucher = async (row: Voucher) => {
  try {
    await ElMessageBox.confirm('确定审核该凭证吗？', '审核确认', { type: 'info' })
    await reviewVoucherApi(row.id)
    ElMessage.success('审核成功')
    fetchVouchers()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
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
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const handlePrintSubjects = () => {
  const printData = subjects.value.map((item: any, index: number) => ({
    序号: index + 1,
    科目编码: item.code,
    科目名称: item.name,
    科目类别: getCategoryLabel(item.category),
    余额方向: item.direction === 'debit' ? '借方' : '贷方',
    级次: `L${item.level}`,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}),
    type: 'table' as any,
    header: '会计科目表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  })
}

const handleExportSubjects = () => {
  const csvContent = [
    ['科目编码', '科目名称', '科目类别', '余额方向', '级次'],
    ...subjects.value.map((item: any) => [
      item.code,
      item.name,
      getCategoryLabel(item.category),
      item.direction === 'debit' ? '借方' : '贷方',
      `L${item.level}`,
    ]),
  ]
    .map((row) => row.map((cell) => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `会计科目表_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

const handleExportVouchers = () => {
  const csvContent = [
    ['凭证号', '凭证日期', '凭证类型', '借方金额', '贷方金额', '状态', '制单人', '创建时间'],
    ...vouchers.value.map((item: any) => [
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
    .map((row) => row.map((cell) => `"${cell ?? ''}"`).join(','))
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
      (item: any) => `
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
.finance-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.filter-card {
  margin-bottom: 20px;
}
.entry-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 16px;
}
.entry-summary {
  display: flex;
  gap: 24px;
  font-weight: 500;
}
.text-red {
  color: #f56c6c;
}
</style>
