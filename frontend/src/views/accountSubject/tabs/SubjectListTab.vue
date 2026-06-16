<!--
  SubjectListTab.vue - 会计科目 Tab
  来源：原 accountSubject/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="subject-list-tab">
    <div class="page-header">
      <h2 class="page-title">会计科目</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>新建科目
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>导出
        </el-button>
      </div>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="科目编码">
          <el-input v-model="queryForm.code" placeholder="编码" clearable />
        </el-form-item>
        <el-form-item label="科目名称">
          <el-input v-model="queryForm.name" placeholder="名称" clearable />
        </el-form-item>
        <el-form-item label="科目类别">
          <el-select v-model="queryForm.category" placeholder="选择类别" clearable>
            <el-option label="资产" value="asset" />
            <el-option label="负债" value="liability" />
            <el-option label="权益" value="equity" />
            <el-option label="成本" value="cost" />
            <el-option label="损益" value="profit_loss" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="subjectList"
        stripe
        row-key="id"
        default-expand-all
        :tree-props="{ children: 'children' }"
      >
        <el-table-column prop="code" label="科目编码" width="120" />
        <el-table-column prop="name" label="科目名称" min-width="200" />
        <el-table-column prop="category" label="科目类别" width="100">
          <template #default="{ row }">
            <el-tag size="small">{{ getCategoryLabel(row.category) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="balance_type" label="余额方向" width="100">
          <template #default="{ row }">
            <el-tag :type="row.balance_type === 'debit' ? 'success' : 'danger'" size="small">
              {{ row.balance_type === 'debit' ? '借方' : '贷方' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="level" label="级次" width="80" align="center" />
        <el-table-column prop="is_enabled" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.is_enabled ? 'success' : 'info'" size="small">
              {{ row.is_enabled ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="160" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openDialog(row)">编辑</el-button>
            <el-button type="danger" link size="small" @click="deleteSubject(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑科目' : '新建科目'" width="500px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item label="科目编码" prop="code">
          <el-input v-model="form.code" :disabled="!!form.id" />
        </el-form-item>
        <el-form-item label="科目名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="科目类别" prop="category">
          <el-select v-model="form.category" placeholder="选择类别" style="width: 100%">
            <el-option label="资产" value="asset" />
            <el-option label="负债" value="liability" />
            <el-option label="权益" value="equity" />
            <el-option label="成本" value="cost" />
            <el-option label="损益" value="profit_loss" />
          </el-select>
        </el-form-item>
        <el-form-item label="余额方向" prop="balance_type">
          <el-radio-group v-model="form.balance_type">
            <el-radio value="debit">借方</el-radio>
            <el-radio value="credit">贷方</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="上级科目">
          <el-tree-select
            v-model="form.parent_id"
            :data="parentSubjectOptions"
            :props="{ label: 'name', value: 'id' }"
            placeholder="选择上级科目"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="form.is_enabled" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="form.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Download } from '@element-plus/icons-vue'
import {
  listAccountSubjects,
  createAccountSubject,
  updateAccountSubject,
  deleteAccountSubject,
  type AccountSubjectEntity,
} from '@/api/account-subject'
import { logger } from '@/utils/logger'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const subjectList = ref<AccountSubjectEntity[]>([])
const formRef = ref<FormInstance>()

const queryForm = reactive({
  code: '',
  name: '',
  category: '',
})

const form = reactive<Partial<AccountSubjectEntity>>({
  id: undefined,
  code: '',
  name: '',
  parent_id: undefined,
  level: 1,
  category: 'asset',
  type: 'detail',
  balance_type: 'debit',
  is_enabled: true,
  description: '',
})

const rules: FormRules = {
  code: [{ required: true, message: '请输入科目编码', trigger: 'blur' }],
  name: [{ required: true, message: '请输入科目名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择科目类别', trigger: 'change' }],
  balance_type: [{ required: true, message: '请选择余额方向', trigger: 'change' }],
}

const parentSubjectOptions = computed(() => subjectList.value)

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
  loading.value = true
  try {
    const res = await listAccountSubjects(queryForm)
    const d = (res as { data?: unknown }).data as
      | AccountSubjectEntity[]
      | {
          items?: AccountSubjectEntity[]
          data?: AccountSubjectEntity[]
          list?: AccountSubjectEntity[]
        }
    if (Array.isArray(d)) {
      subjectList.value = d
    } else {
      subjectList.value = d?.items || d?.data || d?.list || []
    }
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || '获取科目列表失败')
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  fetchSubjects()
}

const handleReset = () => {
  queryForm.code = ''
  queryForm.name = ''
  queryForm.category = ''
  fetchSubjects()
}

const openDialog = (row?: AccountSubjectEntity) => {
  formRef.value?.resetFields()
  if (row) {
    Object.assign(form, row)
  } else {
    form.id = undefined
    form.code = ''
    form.name = ''
    form.parent_id = undefined
    form.level = 1
    form.category = 'asset'
    form.type = 'detail'
    form.balance_type = 'debit'
    form.is_enabled = true
    form.description = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (form.id) {
        await updateAccountSubject(form.id, form)
        ElMessage.success('更新成功')
      } else {
        await createAccountSubject(form)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchSubjects()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const deleteSubject = async (row: AccountSubjectEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(`确定删除科目 "${row.name}" 吗？`, '删除确认', { type: 'warning' })
    await deleteAccountSubject(row.id)
    ElMessage.success('删除成功')
    fetchSubjects()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const handleExport = () => {
  type SubjectWithChildren = AccountSubjectEntity & { children?: SubjectWithChildren[] }
  const flatten = (items: SubjectWithChildren[]): AccountSubjectEntity[] => {
    return items.reduce<AccountSubjectEntity[]>((acc, item) => {
      const { children: _children, ...rest } = item as SubjectWithChildren
      void _children
      acc.push(rest as AccountSubjectEntity)
      if ((item as SubjectWithChildren).children) {
        acc.push(...flatten((item as SubjectWithChildren).children!))
      }
      return acc
    }, [])
  }
  const flat = flatten(subjectList.value as SubjectWithChildren[])
  const csvContent = [
    ['科目编码', '科目名称', '科目类别', '余额方向', '级次', '状态'],
    ...flat.map(s => [
      s.code,
      s.name,
      getCategoryLabel(s.category),
      s.balance_type === 'debit' ? '借方' : '贷方',
      `L${s.level}`,
      s.is_enabled ? '启用' : '禁用',
    ]),
  ]
    .map(row => row.map(cell => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `会计科目表_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
  logger.info('会计科目表已导出')
}

onMounted(() => {
  fetchSubjects()
})
</script>
