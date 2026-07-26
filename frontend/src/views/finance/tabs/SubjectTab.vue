<!--
  SubjectTab.vue - 会计科目 Tab
  来源：原 finance/index.vue 中 科目管理 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="subject-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('finance.subjectTab.pageTitle') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openSubjectDialog()">
          <el-icon><Plus /></el-icon>
          {{ t('finance.subjectTab.buttonNewSubject') }}
        </el-button>
        <el-button @click="handlePrintSubjects">
          <el-icon><Printer /></el-icon>
          {{ t('finance.subjectTab.buttonPrint') }}
        </el-button>
        <el-button @click="handleExportSubjects">
          <el-icon><Download /></el-icon>
          {{ t('finance.subjectTab.buttonExport') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="subjectLoading" :data="subjects" stripe row-key="id" default-expand-all :aria-label="t('finance.subjectTab.ariaLabel')">
        <el-table-column prop="code" :label="t('finance.subjectTab.columnCode')" width="120" />
        <el-table-column prop="name" :label="t('finance.subjectTab.columnName')" min-width="200" />
        <el-table-column prop="category" :label="t('finance.subjectTab.columnCategory')" width="100">
          <template #default="{ row }">
            <el-tag size="small">{{ getCategoryLabel(row.category) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="direction" :label="t('finance.subjectTab.columnDirection')" width="100">
          <template #default="{ row }">
            <el-tag :type="row.direction === 'debit' ? 'success' : 'danger'" size="small">
              {{ row.direction === 'debit' ? t('finance.subjectTab.directionDebit') : t('finance.subjectTab.directionCredit') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="level" :label="t('finance.subjectTab.columnLevel')" width="80" align="center" />
        <el-table-column prop="is_leaf" :label="t('finance.subjectTab.columnIsLeaf')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.is_leaf ? 'success' : 'info'" size="small">
              {{ row.is_leaf ? t('finance.subjectTab.yes') : t('finance.subjectTab.no') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="t('finance.subjectTab.columnStatus')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
              {{ row.status === 1 ? t('finance.subjectTab.statusActive') : t('finance.subjectTab.statusInactive') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('finance.subjectTab.columnAction')" width="150" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button v-permission="'finance_subject:update'" type="primary" link size="small" @click="openSubjectDialog(row)"
              >{{ t('finance.subjectTab.buttonEdit') }}</el-button
            >
            <el-button v-permission="'finance_subject:delete'" type="danger" link size="small" @click="deleteSubject(row)">{{ t('finance.subjectTab.buttonDelete') }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="subjectDialogVisible"
      :title="subjectForm.id ? t('finance.subjectTab.dialogTitleEdit') : t('finance.subjectTab.dialogTitleNew')"
      width="500px"
      :aria-label="subjectForm.id ? t('finance.subjectTab.dialogAriaLabelEdit') : t('finance.subjectTab.dialogAriaLabelNew')"
    >
      <el-form ref="subjectFormRef" :model="subjectForm" :rules="subjectRules" label-width="80px" :aria-label="t('finance.subjectTab.formAriaLabel')">
        <el-form-item :label="t('finance.subjectTab.labelCode')" prop="code">
          <el-input v-model="subjectForm.code" :placeholder="t('finance.subjectTab.placeholderCode')" />
        </el-form-item>
        <el-form-item :label="t('finance.subjectTab.labelName')" prop="name">
          <el-input v-model="subjectForm.name" :placeholder="t('finance.subjectTab.placeholderName')" />
        </el-form-item>
        <el-form-item :label="t('finance.subjectTab.labelParent')">
          <el-tree-select
            v-model="subjectForm.parent_id"
            :data="subjectTreeData"
            :props="{ label: 'name', value: 'id' }"
            :placeholder="t('finance.subjectTab.placeholderParent')"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item :label="t('finance.subjectTab.labelCategory')" prop="category">
          <el-select v-model="subjectForm.category" :placeholder="t('finance.subjectTab.placeholderCategory')">
            <el-option :label="t('finance.subjectTab.optionAsset')" value="asset" />
            <el-option :label="t('finance.subjectTab.optionLiability')" value="liability" />
            <el-option :label="t('finance.subjectTab.optionEquity')" value="equity" />
            <el-option :label="t('finance.subjectTab.optionCost')" value="cost" />
            <el-option :label="t('finance.subjectTab.optionProfitLoss')" value="profit_loss" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('finance.subjectTab.labelDirection')" prop="direction">
          <el-radio-group v-model="subjectForm.direction">
            <el-radio value="debit">{{ t('finance.subjectTab.directionDebit') }}</el-radio>
            <el-radio value="credit">{{ t('finance.subjectTab.directionCredit') }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="t('finance.subjectTab.labelStatus')">
          <el-switch v-model="subjectForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="subjectDialogVisible = false">{{ t('finance.subjectTab.buttonCancel') }}</el-button>
        <el-button type="primary" :loading="subjectSubmitLoading" @click="submitSubject"
          >{{ t('finance.subjectTab.buttonConfirm') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
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
import { exportToExcel } from '@/utils/export'

const { t } = useI18n({ useScope: 'global' })

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

const subjectRules = computed<FormRules>(() => ({
  code: [{ required: true, message: t('finance.subjectTab.ruleCodeRequired'), trigger: 'blur' }],
  name: [{ required: true, message: t('finance.subjectTab.ruleNameRequired'), trigger: 'blur' }],
  category: [{ required: true, message: t('finance.subjectTab.ruleCategoryRequired'), trigger: 'change' }],
  direction: [{ required: true, message: t('finance.subjectTab.ruleDirectionRequired'), trigger: 'change' }],
}))

const subjectTreeData = computed(() => subjects.value)

const getCategoryLabel = (category: string) => {
  const map: Record<string, string> = {
    asset: t('finance.subjectTab.categoryAsset'),
    liability: t('finance.subjectTab.categoryLiability'),
    equity: t('finance.subjectTab.categoryEquity'),
    cost: t('finance.subjectTab.categoryCost'),
    profit_loss: t('finance.subjectTab.categoryProfitLoss'),
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
    ElMessage.error(err.message || t('finance.subjectTab.messageFetchFailed'))
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
      ElMessage.success(t('finance.subjectTab.messageUpdateSuccess'))
    } else {
      await createSubject({
        code: subjectForm.code,
        name: subjectForm.name,
        parent_id: subjectForm.parent_id,
        category: subjectForm.category,
        direction: subjectForm.direction,
      })
      ElMessage.success(t('finance.subjectTab.messageCreateSuccess'))
    }
    subjectDialogVisible.value = false
    fetchSubjects()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('finance.subjectTab.messageOperationFailed'))
  } finally {
    subjectSubmitLoading.value = false
  }
}

const deleteSubject = async (row: AccountSubject) => {
  try {
    await ElMessageBox.confirm(
      t('finance.subjectTab.messageDeleteConfirm', { name: row.name }),
      t('finance.subjectTab.messageDeleteTitle'),
      { type: 'warning' },
    )
    await deleteSubjectApi(row.id)
    ElMessage.success(t('finance.subjectTab.messageDeleteSuccess'))
    fetchSubjects()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('finance.subjectTab.messageDeleteFailed'))
    }
  }
}

const handlePrintSubjects = () => {
  const printData = subjects.value.map((item, index) => ({
    [t('finance.subjectTab.exportColIndex')]: index + 1,
    [t('finance.subjectTab.exportColCode')]: item.code,
    [t('finance.subjectTab.exportColName')]: item.name,
    [t('finance.subjectTab.exportColCategory')]: getCategoryLabel(item.category),
    [t('finance.subjectTab.exportColDirection')]: item.direction === 'debit' ? t('finance.subjectTab.directionDebit') : t('finance.subjectTab.directionCredit'),
    [t('finance.subjectTab.exportColLevel')]: `L${item.level}`,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}) as string[],
    type: 'json',
    header: t('finance.subjectTab.printHeader'),
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  } as never)
}

const handleExportSubjects = () => {
  exportToExcel({
    filename: t('finance.subjectTab.exportFilename'),
    format: 'excel',
    data: subjects.value.map((item): Record<string, unknown> => ({ ...item })),
    columns: [
      { key: 'code', title: t('finance.subjectTab.exportColCode') },
      { key: 'name', title: t('finance.subjectTab.exportColName') },
      {
        key: 'category',
        title: t('finance.subjectTab.exportColCategory'),
        formatter: (value: unknown) => getCategoryLabel(String(value)),
      },
      {
        key: 'direction',
        title: t('finance.subjectTab.exportColDirection'),
        formatter: (value: unknown) => (value === 'debit' ? t('finance.subjectTab.directionDebit') : t('finance.subjectTab.directionCredit')),
      },
      {
        key: 'level',
        title: t('finance.subjectTab.exportColLevel'),
        formatter: (value: unknown) => `L${value}`,
      },
    ],
  })
}

onMounted(() => {
  fetchSubjects()
})
</script>
