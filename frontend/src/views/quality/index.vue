<!--
  quality/index.vue - 质量管理
  D05 Batch 3：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="quality-page">
    <el-tabs v-model="activeTab" :aria-label="$t('quality.tabAriaLabel')">
      <el-tab-pane :label="$t('quality.tab.standard')" name="standard">
        <StandardTab
          @open-history="viewVersionHistory"
          @open-approve="approveStandard"
        />
      </el-tab-pane>

      <el-tab-pane :label="$t('quality.tab.record')" name="record">
        <RecordTab />
      </el-tab-pane>

      <el-tab-pane :label="$t('quality.tab.defect')" name="defect">
        <DefectTab />
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="standardDialogVisible"
      :title="standardForm.id ? $t('quality.standardDialog.editTitle') : $t('quality.standardDialog.createTitle')"
      width="700px"
      :aria-label="$t('quality.standardDialog.ariaLabel')"
    >
      <el-form
        ref="standardFormRef"
        :model="standardForm"
        :rules="standardFormRules"
        label-width="100px"
        :aria-label="$t('quality.standardDialog.formAriaLabel')"
      >
        <el-form-item :label="$t('quality.standardDialog.standardCode')" prop="standard_code">
          <el-input
            v-model="standardForm.standard_code"
            :disabled="!!standardForm.id"
            :placeholder="$t('quality.standardDialog.standardCodePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.standardName')" prop="standard_name">
          <el-input v-model="standardForm.standard_name" :placeholder="$t('quality.standardDialog.standardNamePlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.type')" prop="type">
          <el-select v-model="standardForm.type" :placeholder="$t('quality.standardDialog.typePlaceholder')" style="width: 100%">
            <el-option :label="$t('quality.standardDialog.typeProduct')" value="product" />
            <el-option :label="$t('quality.standardDialog.typeProcess')" value="process" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.version')" prop="version">
          <el-input v-model="standardForm.version" :placeholder="$t('quality.standardDialog.versionPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.content')" prop="content">
          <el-input
            v-model="standardForm.content"
            type="textarea"
            :rows="6"
            :placeholder="$t('quality.standardDialog.contentPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.standardDialog.attachments')" prop="attachments">
          <el-input
            v-model="attachmentsText"
            type="textarea"
            :placeholder="$t('quality.standardDialog.attachmentsPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="standardDialogVisible = false">{{ $t('quality.standardDialog.cancel') }}</el-button>
        <el-button type="primary" :loading="standardSubmitLoading" @click="submitStandard"
          >{{ $t('quality.standardDialog.confirm') }}</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="approveDialogVisible" :title="$t('quality.approveDialog.title')" width="500px" :aria-label="$t('quality.approveDialog.ariaLabel')">
      <el-form
        ref="approveFormRef"
        :model="approveForm"
        :rules="approveFormRules"
        label-width="80px"
        :aria-label="$t('quality.approveDialog.formAriaLabel')"
      >
        <el-form-item :label="$t('quality.approveDialog.standardCode')">
          <el-input :model-value="approveStandardItem?.standard_code" disabled />
        </el-form-item>
        <el-form-item :label="$t('quality.approveDialog.standardName')">
          <el-input :model-value="approveStandardItem?.standard_name" disabled />
        </el-form-item>
        <el-form-item :label="$t('quality.approveDialog.currentVersion')">
          <el-input :model-value="approveStandardItem?.version" disabled />
        </el-form-item>
        <el-form-item :label="$t('quality.approveDialog.approvalComment')" prop="approval_comment">
          <el-input
            v-model="approveForm.approval_comment"
            type="textarea"
            :rows="4"
            :placeholder="$t('quality.approveDialog.approvalCommentPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="approveDialogVisible = false">{{ $t('quality.approveDialog.cancel') }}</el-button>
        <el-button type="warning" :loading="approveSubmitLoading" @click="rejectStandard"
          >{{ $t('quality.approveDialog.reject') }}</el-button
        >
        <el-button type="primary" :loading="approveSubmitLoading" @click="confirmApprove"
          >{{ $t('quality.approveDialog.pass') }}</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="versionHistoryVisible" :title="$t('quality.versionHistory.title')" width="800px" :aria-label="$t('quality.versionHistory.ariaLabel')">
      <el-table v-loading="versionHistoryLoading" :data="versionHistoryList" stripe :aria-label="$t('quality.versionHistory.tableAriaLabel')">
        <el-table-column prop="version" :label="$t('quality.versionHistory.version')" width="100" />
        <el-table-column prop="status" :label="$t('quality.versionHistory.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="getStandardStatusType(row.status)" size="small">
              {{ getStandardStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" :label="$t('quality.versionHistory.createdBy')" width="100" />
        <el-table-column prop="created_at" :label="$t('quality.versionHistory.createdAt')" width="160" />
        <el-table-column prop="approved_by_name" :label="$t('quality.versionHistory.approvedBy')" width="100">
          <template #default="{ row }">
            {{ row.approved_by_name || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="approved_at" :label="$t('quality.versionHistory.approvedAt')" width="160">
          <template #default="{ row }">
            {{ row.approved_at || '-' }}
          </template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="versionHistoryVisible = false">{{ $t('quality.versionHistory.close') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="recordDialogVisible"
      :title="recordForm.id ? $t('quality.recordDialog.editTitle') : $t('quality.recordDialog.createTitle')"
      width="700px"
      :aria-label="$t('quality.recordDialog.ariaLabel')"
    >
      <el-form ref="recordFormRef" :model="recordForm" label-width="100px" :aria-label="$t('quality.recordDialog.formAriaLabel')">
        <el-form-item :label="$t('quality.recordDialog.recordNo')" prop="record_no">
          <el-input v-model="recordForm.record_no" :disabled="!!recordForm.id" />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.inspectionType')" prop="inspection_type">
          <el-select v-model="recordForm.inspection_type" style="width: 100%">
            <el-option :label="$t('quality.recordDialog.inspectionTypeOptions.incoming')" value="incoming" />
            <el-option :label="$t('quality.recordDialog.inspectionTypeOptions.process')" value="process" />
            <el-option :label="$t('quality.recordDialog.inspectionTypeOptions.finished')" value="finished" />
            <el-option :label="$t('quality.recordDialog.inspectionTypeOptions.outgoing')" value="outgoing" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.product')" prop="product_name">
          <el-input v-model="recordForm.product_name" :placeholder="$t('quality.recordDialog.productPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.batchNo')" prop="batch_no">
          <el-input v-model="recordForm.batch_no" />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.inspectionDate')" prop="inspection_date">
          <el-date-picker
            v-model="recordForm.inspection_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.inspector')" prop="inspector">
          <el-input v-model="recordForm.inspector" />
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.result')" prop="result">
          <el-radio-group v-model="recordForm.result">
            <el-radio label="pass">{{ $t('quality.recordDialog.resultOptions.pass') }}</el-radio>
            <el-radio label="fail">{{ $t('quality.recordDialog.resultOptions.fail') }}</el-radio>
            <el-radio label="pending">{{ $t('quality.recordDialog.resultOptions.pending') }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="$t('quality.recordDialog.remark')" prop="remark">
          <el-input v-model="recordForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="recordDialogVisible = false">{{ $t('quality.recordDialog.cancel') }}</el-button>
        <el-button type="primary" :loading="recordSubmitLoading" @click="submitRecord"
          >{{ $t('quality.recordDialog.confirm') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, provide } from 'vue'
import { useI18n } from 'vue-i18n'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import StandardTab from './tabs/StandardTab.vue'
import RecordTab from './tabs/RecordTab.vue'
import DefectTab from './tabs/DefectTab.vue'
import {
  getQualityStandardList,
  createQualityStandard,
  updateQualityStandard,
  approveQualityStandard,
  rejectQualityStandard,
  getQualityRecordList,
  createQualityRecord,
  // 批次 94 P2-12 修复：补全 updateQualityRecord 用于实现更新功能
  updateQualityRecord,
  getDefectList,
  getQualityStandardVersions,
  type QualityStandard,
  type QualityRecord,
  type Defect,
} from '@/api/quality'

// v11 批次 173 P2-1 修复：el-tag type 类型
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger'

// v11 批次 161 P2-5 修复：后端已支持分页（返回 PaginatedResponse 含 items+total），
// RecordTab 通过 useTableApi 自动消费分页元数据，无需在此维护分页状态

const { t } = useI18n({ useScope: 'global' })

// 为 StandardTab/RecordTab 提供 actions（inject('qualityActions')）
// 注意：provide 移到所有函数定义之后，避免 hoisting 问题（vue-tsc 报 used before declaration）
const activeTab = ref('standard')
const standards = ref<QualityStandard[]>([])
const records = ref<QualityRecord[]>([])
const defects = ref<Defect[]>([])
const standardLoading = ref(false)
const recordLoading = ref(false)
const defectLoading = ref(false)

const fetchStandards = async () => {
  standardLoading.value = true
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为直接使用 API 返回类型
    const res = await getQualityStandardList()
    // 安全检查：防止后端返回 data 为 null 时崩溃
    standards.value = res.data || []
  } finally {
    standardLoading.value = false
  }
}

const fetchRecords = async () => {
  recordLoading.value = true
  try {
    // v11 批次 161 P2-5 修复：后端返回 PaginatedResponse（items + total）
    const res = await getQualityRecordList({ page: 1, page_size: 10 })
    const data = res.data
    records.value = data?.items || []
  } finally {
    recordLoading.value = false
  }
}

const fetchDefects = async () => {
  defectLoading.value = true
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为直接使用 API 返回类型
    const res = await getDefectList()
    // 安全检查：防止后端返回 data 为 null 时崩溃
    defects.value = res.data || []
  } finally {
    defectLoading.value = false
  }
}

const getStandardStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: t('quality.standardStatus.draft'),
    approved: t('quality.standardStatus.approved'),
    published: t('quality.standardStatus.published'),
    rejected: t('quality.standardStatus.rejected'),
  }
  return map[status] || status
}

const getStandardStatusType = (status: string): TagType => {
  // v11 批次 173 P2-1 修复：Record<string, any> 改为 Record<string, TagType>
  const map: Record<string, TagType> = {
    draft: 'info',
    approved: 'warning',
    published: 'success',
    rejected: 'danger',
  }
  return map[status] || 'info'
}

const standardDialogVisible = ref(false)
const standardFormRef = ref<FormInstance>()
const standardSubmitLoading = ref(false)
const attachmentsText = ref('')
const standardForm = reactive({
  id: 0,
  standard_code: '',
  standard_name: '',
  version: '1.0',
  type: 'product' as const,
  status: 'draft' as const,
  content: '',
  attachments: [] as string[],
})
const standardFormRules: FormRules = {
  standard_code: [{ required: true, message: t('quality.validation.standardCodeRequired'), trigger: 'blur' }],
  standard_name: [{ required: true, message: t('quality.validation.standardNameRequired'), trigger: 'blur' }],
  type: [{ required: true, message: t('quality.validation.typeRequired'), trigger: 'change' }],
  version: [{ required: true, message: t('quality.validation.versionRequired'), trigger: 'blur' }],
  content: [{ required: true, message: t('quality.validation.contentRequired'), trigger: 'blur' }],
}

const openStandardDialog = (row?: QualityStandard) => {
  if (row) {
    Object.assign(standardForm, row)
    attachmentsText.value = JSON.stringify(row.attachments || [], null, 2)
  } else {
    Object.assign(standardForm, {
      id: 0,
      standard_code: '',
      standard_name: '',
      version: '1.0',
      type: 'product',
      status: 'draft',
      content: '',
      attachments: [],
    })
    attachmentsText.value = ''
  }
  standardDialogVisible.value = true
}

const submitStandard = async () => {
  if (!standardFormRef.value) return
  await standardFormRef.value.validate(async valid => {
    if (!valid) return

    standardSubmitLoading.value = true
    try {
      if (attachmentsText.value) {
        try {
          standardForm.attachments = JSON.parse(attachmentsText.value)
        } catch (e) {
          ElMessage.error(t('quality.message.attachmentsFormatError'))
          return
        }
      }
      if (standardForm.id) {
        await updateQualityStandard(standardForm.id, standardForm as Partial<QualityStandard>)
      } else {
        await createQualityStandard(standardForm as Partial<QualityStandard>)
      }
      ElMessage.success(t('quality.message.operationSuccess'))
      standardDialogVisible.value = false
      fetchStandards()
    } catch (e: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error((e instanceof Error ? e.message : String(e)) || t('quality.message.operationFailed'))
    } finally {
      standardSubmitLoading.value = false
    }
  })
}

const approveDialogVisible = ref(false)
const approveFormRef = ref<FormInstance>()
const approveSubmitLoading = ref(false)
const approveStandardItem = ref<QualityStandard | null>(null)
const approveForm = reactive({ approval_comment: '' })
const approveFormRules: FormRules = {
  approval_comment: [{ required: true, message: t('quality.validation.approvalCommentRequired'), trigger: 'blur' }],
}

const approveStandard = async (row: QualityStandard) => {
  approveStandardItem.value! = row
  approveForm.approval_comment = ''
  approveDialogVisible.value = true
}

const confirmApprove = async () => {
  if (!approveFormRef.value || !approveStandardItem.value!) return
  await approveFormRef.value.validate(async valid => {
    if (!valid) return

    approveSubmitLoading.value = true
    try {
      await approveQualityStandard(approveStandardItem.value!.id)
      ElMessage.success(t('quality.message.approveSuccess'))
      approveDialogVisible.value = false
      fetchStandards()
    } catch (e: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error((e instanceof Error ? e.message : String(e)) || t('quality.message.operationFailed'))
    } finally {
      approveSubmitLoading.value = false
    }
  })
}

const rejectStandard = async () => {
  if (!approveStandardItem.value!) return
  try {
    const reason = await ElMessageBox.prompt(t('quality.message.rejectPrompt'), t('quality.message.rejectTitle'), {
      type: 'warning',
      confirmButtonText: t('quality.message.rejectConfirmButton'),
      cancelButtonText: t('quality.message.rejectCancelButton'),
      inputPlaceholder: t('quality.message.rejectPlaceholder'),
      inputType: 'textarea',
    })
    // 批次 157d-2 修复：接入 rejectQualityStandard API
    await rejectQualityStandard(approveStandardItem.value!.id, {
      reject_reason: reason.value || undefined,
    })
    ElMessage.success(t('quality.message.rejectSuccess'))
    approveDialogVisible.value = false
    fetchStandards()
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel') ElMessage.error((e instanceof Error ? e.message : String(e)) || t('quality.message.operationFailed'))
  }
}

const versionHistoryVisible = ref(false)
const versionHistoryLoading = ref(false)
const versionHistoryList = ref<QualityStandard[]>([])

const viewVersionHistory = async (row: QualityStandard) => {
  versionHistoryLoading.value = true
  try {
    // v11 批次 173 P2-1 修复：const res: any 改为直接使用 API 返回类型
    const res = await getQualityStandardVersions(row.id)
    // 安全检查：防止后端返回 data 为 null 时崩溃
    versionHistoryList.value = res.data || []
    versionHistoryVisible.value = true
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || t('quality.message.fetchVersionHistoryFailed'))
  } finally {
    versionHistoryLoading.value = false
  }
}

const recordDialogVisible = ref(false)
const recordFormRef = ref<FormInstance>()
const recordSubmitLoading = ref(false)
const recordForm = reactive({
  id: 0,
  record_no: '',
  inspection_type: '',
  product_id: undefined as number | undefined,
  product_name: '',
  batch_no: '',
  inspection_date: '',
  inspector: '',
  result: 'pending' as const,
  defects: [] as Defect[],
  remark: '',
})

const openRecordDialog = (row?: QualityRecord) => {
  if (row) {
    Object.assign(recordForm, row)
  } else {
    Object.assign(recordForm, {
      id: 0,
      record_no: '',
      inspection_type: '',
      product_id: undefined,
      product_name: '',
      batch_no: '',
      inspection_date: '',
      inspector: '',
      result: 'pending',
      defects: [],
      remark: '',
    })
  }
  recordDialogVisible.value = true
}

const submitRecord = async () => {
  recordSubmitLoading.value = true
  try {
    if (recordForm.id) {
      // 批次 94 P2-12 修复：原占位"更新功能待实现"，现接入真实更新 API
      await updateQualityRecord(recordForm.id, recordForm as Partial<QualityRecord>)
    } else {
      await createQualityRecord(recordForm as Partial<QualityRecord>)
    }
    ElMessage.success(t('quality.message.operationSuccess'))
    recordDialogVisible.value = false
    fetchRecords()
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || t('quality.message.operationFailed'))
  } finally {
    recordSubmitLoading.value = false
  }
}

const hasLoaded = createLazyLoader()

onMounted(() => {
  fetchStandards()
  loadIfNot('records', fetchRecords, hasLoaded)
  loadIfNot('defects', fetchDefects, hasLoaded)
})

// provide 必须在所有函数定义之后，避免 hoisting 问题
provide('qualityActions', {
  openStandardDialog,
  openRecordDialog,
})
</script>

<style scoped>
.quality-page {
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
</style>
