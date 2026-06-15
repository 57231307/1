<!--
  SubjectTab.vue - 会计科目 Tab
  来源：原 finance/index.vue 中 科目管理 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="subject-tab">
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
      <el-table v-loading="subjectLoading" :data="subjects" stripe row-key="id" default-expand-all>
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
            <el-button type="danger" link size="small" @click="deleteSubject(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

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

const subjects = ref<AccountSubject[]>([])
const subjectLoading = ref(false)
const subjectSubmitLoading = ref(false)
const subjectDialogVisible = ref(false)
const subjectFormRef = ref<FormInstance>()

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

const subjectTreeData = computed(() => subjects.value)

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

const fetchSubjects = async () => {
  subjectLoading.value = true
  try {
    const res = await getSubjectTree()
    const d = res.data as AccountSubject[] | { items?: AccountSubject[]; data?: AccountSubject[] }
    subjects.value = Array.isArray(d) ? d : d?.items || d?.data || []
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取科目列表失败')
  } finally {
    subjectLoading.value = false
  }
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
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
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
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const handlePrintSubjects = () => {
  const printData = subjects.value.map((item, index) => ({
    序号: index + 1,
    科目编码: item.code,
    科目名称: item.name,
    科目类别: getCategoryLabel(item.category),
    余额方向: item.direction === 'debit' ? '借方' : '贷方',
    级次: `L${item.level}`,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}) as string[],
    type: 'table',
    header: '会计科目表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  } as never)
}

const handleExportSubjects = () => {
  const csvContent = [
    ['科目编码', '科目名称', '科目类别', '余额方向', '级次'],
    ...subjects.value.map(item => [
      item.code,
      item.name,
      getCategoryLabel(item.category),
      item.direction === 'debit' ? '借方' : '贷方',
      `L${item.level}`,
    ]),
  ]
    .map(row => row.map(cell => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `会计科目表_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

onMounted(() => {
  fetchSubjects()
})
</script>
