<!--
  SubjectListTab.vue - 会计科目 Tab
  来源：原 accountSubject/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="subject-list-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('accountSubject.title') }}</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ $t('accountSubject.create') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>{{ $t('accountSubject.export') }}
        </el-button>
      </div>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm" :aria-label="$t('accountSubject.filter.ariaLabel')">
        <el-form-item :label="$t('accountSubject.filter.code')">
          <el-input v-model="queryForm.code" :placeholder="$t('accountSubject.filter.codePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.filter.name')">
          <el-input v-model="queryForm.name" :placeholder="$t('accountSubject.filter.namePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.filter.category')">
          <el-select v-model="queryForm.category" :placeholder="$t('accountSubject.filter.categoryPlaceholder')" clearable>
            <el-option :label="$t('accountSubject.category.asset')" value="asset" />
            <el-option :label="$t('accountSubject.category.liability')" value="liability" />
            <el-option :label="$t('accountSubject.category.equity')" value="equity" />
            <el-option :label="$t('accountSubject.category.cost')" value="cost" />
            <el-option :label="$t('accountSubject.category.profitLoss')" value="profit_loss" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{ $t('accountSubject.filter.query') }}</el-button>
          <el-button @click="handleReset">{{ $t('accountSubject.filter.reset') }}</el-button>
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
        :aria-label="$t('accountSubject.table.ariaLabel')"
      >
        <el-table-column prop="code" :label="$t('accountSubject.table.code')" width="120" />
        <el-table-column prop="name" :label="$t('accountSubject.table.name')" min-width="200" />
        <el-table-column prop="category" :label="$t('accountSubject.table.category')" width="100">
          <template #default="{ row }">
            <el-tag size="small">{{ getCategoryLabel(row.category) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="balance_type" :label="$t('accountSubject.table.balanceType')" width="100">
          <template #default="{ row }">
            <el-tag :type="row.balance_type === 'debit' ? 'success' : 'danger'" size="small">
              {{ row.balance_type === 'debit' ? $t('accountSubject.balanceType.debit') : $t('accountSubject.balanceType.credit') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="level" :label="$t('accountSubject.table.level')" width="80" align="center" />
        <el-table-column prop="is_enabled" :label="$t('accountSubject.table.status')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.is_enabled ? 'success' : 'info'" size="small">
              {{ row.is_enabled ? $t('accountSubject.status.enabled') : $t('accountSubject.status.disabled') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('accountSubject.table.operation')" width="160" fixed="right">
          <template #default="{ row }">
            <el-button v-permission="'account_subject:update'" type="primary" link size="small" @click="openDialog(row)">{{ $t('accountSubject.table.edit') }}</el-button>
            <el-button v-permission="'account_subject:delete'" type="danger" link size="small" @click="deleteSubject(row)">{{ $t('accountSubject.table.delete') }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? $t('accountSubject.dialog.editTitle') : $t('accountSubject.dialog.createTitle')" width="500px" :aria-label="form.id ? $t('accountSubject.dialog.editAriaLabel') : $t('accountSubject.dialog.createAriaLabel')">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" :aria-label="$t('accountSubject.dialog.ariaLabel')">
        <el-form-item :label="$t('accountSubject.filter.code')" prop="code">
          <el-input v-model="form.code" :disabled="!!form.id" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.filter.name')" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.filter.category')" prop="category">
          <el-select v-model="form.category" :placeholder="$t('accountSubject.filter.categoryPlaceholder')" style="width: 100%">
            <el-option :label="$t('accountSubject.category.asset')" value="asset" />
            <el-option :label="$t('accountSubject.category.liability')" value="liability" />
            <el-option :label="$t('accountSubject.category.equity')" value="equity" />
            <el-option :label="$t('accountSubject.category.cost')" value="cost" />
            <el-option :label="$t('accountSubject.category.profitLoss')" value="profit_loss" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('accountSubject.table.balanceType')" prop="balance_type">
          <el-radio-group v-model="form.balance_type">
            <el-radio value="debit">{{ $t('accountSubject.balanceType.debit') }}</el-radio>
            <el-radio value="credit">{{ $t('accountSubject.balanceType.credit') }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="$t('accountSubject.dialog.parentSubject')">
          <el-tree-select
            v-model="form.parent_id"
            :data="parentSubjectOptions"
            :props="{ label: 'name', value: 'id' }"
            :placeholder="$t('accountSubject.dialog.parentPlaceholder')"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.dialog.enable')">
          <el-switch v-model="form.is_enabled" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.dialog.description')">
          <el-input v-model="form.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('accountSubject.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ $t('accountSubject.dialog.confirm') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Download } from '@element-plus/icons-vue'
import {
  getAccountSubjectList,
  createAccountSubject,
  updateAccountSubject,
  deleteAccountSubject,
  type AccountSubjectEntity,
} from '@/api/account-subject'
import { logger } from '@/utils/logger'
import { exportToExcel } from '@/utils/export'

const { t } = useI18n({ useScope: 'global' })

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

const rules = computed<FormRules>(() => ({
  code: [{ required: true, message: t('accountSubject.validation.codeRequired'), trigger: 'blur' }],
  name: [{ required: true, message: t('accountSubject.validation.nameRequired'), trigger: 'blur' }],
  category: [{ required: true, message: t('accountSubject.validation.categoryRequired'), trigger: 'change' }],
  balance_type: [{ required: true, message: t('accountSubject.validation.balanceTypeRequired'), trigger: 'change' }],
}))

const parentSubjectOptions = computed(() => subjectList.value)

const getCategoryLabel = (category: string) => {
  const map: Record<string, string> = {
    asset: t('accountSubject.category.asset'),
    liability: t('accountSubject.category.liability'),
    equity: t('accountSubject.category.equity'),
    cost: t('accountSubject.category.cost'),
    profit_loss: t('accountSubject.category.profitLoss'),
  }
  return map[category] || category
}

const getBalanceTypeLabel = (balanceType: string) => {
  return balanceType === 'debit'
    ? t('accountSubject.balanceType.debit')
    : t('accountSubject.balanceType.credit')
}

const getStatusLabel = (isEnabled: boolean) => {
  return isEnabled
    ? t('accountSubject.status.enabled')
    : t('accountSubject.status.disabled')
}

const fetchSubjects = async () => {
  loading.value = true
  try {
    const res = await getAccountSubjectList(queryForm)
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
    ElMessage.error(err.message || t('accountSubject.message.fetchListFailed'))
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
        ElMessage.success(t('accountSubject.message.updateSuccess'))
      } else {
        await createAccountSubject(form)
        ElMessage.success(t('accountSubject.message.createSuccess'))
      }
      dialogVisible.value = false
      fetchSubjects()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || t('accountSubject.message.operationFailed'))
    } finally {
      submitLoading.value = false
    }
  })
}

const deleteSubject = async (row: AccountSubjectEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(
      t('accountSubject.message.deleteConfirm', { name: row.name }),
      t('accountSubject.message.deleteConfirmTitle'),
      { type: 'warning' }
    )
    await deleteAccountSubject(row.id)
    ElMessage.success(t('accountSubject.message.deleteSuccess'))
    fetchSubjects()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('accountSubject.message.deleteFailed'))
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
  exportToExcel({
    filename: t('accountSubject.exportFile.filename'),
    format: 'excel',
    data: flat.map((s): Record<string, unknown> => ({ ...s })),
    columns: [
      { key: 'code', title: t('accountSubject.exportFile.code') },
      { key: 'name', title: t('accountSubject.exportFile.name') },
      {
        key: 'category',
        title: t('accountSubject.exportFile.category'),
        formatter: (value: unknown) => getCategoryLabel(String(value)),
      },
      {
        key: 'balance_type',
        title: t('accountSubject.exportFile.balanceType'),
        formatter: (value: unknown) => getBalanceTypeLabel(String(value)),
      },
      {
        key: 'level',
        title: t('accountSubject.exportFile.level'),
        formatter: (value: unknown) => `L${value}`,
      },
      {
        key: 'is_enabled',
        title: t('accountSubject.exportFile.status'),
        formatter: (value: unknown) => getStatusLabel(Boolean(value)),
      },
    ],
  })
  logger.info(t('accountSubject.exportedLog'))
}

onMounted(() => {
  fetchSubjects()
})
</script>
