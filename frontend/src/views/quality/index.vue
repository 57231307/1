<template>
  <div class="quality-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="质量标准" name="standard">
        <StandardTab
          @open-history="viewVersionHistory"
          @open-approve="approveStandard"
        />
      </el-tab-pane>

      <el-tab-pane label="检验记录" name="record">
        <RecordTab />
      </el-tab-pane>

      <el-tab-pane label="缺陷管理" name="defect">
        <DefectTab />
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="standardDialogVisible"
      :title="standardForm.id ? '编辑标准' : '新建标准'"
      width="700px"
    >
      <el-form
        ref="standardFormRef"
        :model="standardForm"
        :rules="standardFormRules"
        label-width="100px"
      >
        <el-form-item label="标准编号" prop="standard_code">
          <el-input
            v-model="standardForm.standard_code"
            :disabled="!!standardForm.id"
            placeholder="请输入标准编号"
          />
        </el-form-item>
        <el-form-item label="标准名称" prop="standard_name">
          <el-input v-model="standardForm.standard_name" placeholder="请输入标准名称" />
        </el-form-item>
        <el-form-item label="类型" prop="type">
          <el-select v-model="standardForm.type" placeholder="请选择类型" style="width: 100%">
            <el-option label="产品标准" value="product" />
            <el-option label="工艺标准" value="process" />
          </el-select>
        </el-form-item>
        <el-form-item label="版本" prop="version">
          <el-input v-model="standardForm.version" placeholder="例如：1.0" />
        </el-form-item>
        <el-form-item label="标准内容" prop="content">
          <el-input
            v-model="standardForm.content"
            type="textarea"
            :rows="6"
            placeholder="请输入标准内容"
          />
        </el-form-item>
        <el-form-item label="附件" prop="attachments">
          <el-input
            v-model="attachmentsText"
            type="textarea"
            placeholder='JSON格式数组，例如：["附件1.pdf", "附件2.docx"]'
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="standardDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="standardSubmitLoading" @click="submitStandard"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="approveDialogVisible" title="审批质量标准" width="500px">
      <el-form
        ref="approveFormRef"
        :model="approveForm"
        :rules="approveFormRules"
        label-width="80px"
      >
        <el-form-item label="标准编号">
          <el-input :model-value="approveStandardItem?.standard_code" disabled />
        </el-form-item>
        <el-form-item label="标准名称">
          <el-input :model-value="approveStandardItem?.standard_name" disabled />
        </el-form-item>
        <el-form-item label="当前版本">
          <el-input :model-value="approveStandardItem?.version" disabled />
        </el-form-item>
        <el-form-item label="审批意见" prop="approval_comment">
          <el-input
            v-model="approveForm.approval_comment"
            type="textarea"
            :rows="4"
            placeholder="请输入审批意见"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="approveDialogVisible = false">取消</el-button>
        <el-button type="warning" :loading="approveSubmitLoading" @click="rejectStandard"
          >驳回</el-button
        >
        <el-button type="primary" :loading="approveSubmitLoading" @click="confirmApprove"
          >通过</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="versionHistoryVisible" title="版本历史" width="800px">
      <el-table v-loading="versionHistoryLoading" :data="versionHistoryList" stripe>
        <el-table-column prop="version" label="版本" width="100" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStandardStatusType(row.status)" size="small">
              {{ getStandardStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" label="创建人" width="100" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column prop="approved_by_name" label="审批人" width="100">
          <template #default="{ row }">
            {{ row.approved_by_name || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="approved_at" label="审批时间" width="160">
          <template #default="{ row }">
            {{ row.approved_at || '-' }}
          </template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="versionHistoryVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="recordDialogVisible"
      :title="recordForm.id ? '编辑检验' : '新建检验'"
      width="700px"
    >
      <el-form ref="recordFormRef" :model="recordForm" label-width="100px">
        <el-form-item label="记录编号" prop="record_no">
          <el-input v-model="recordForm.record_no" :disabled="!!recordForm.id" />
        </el-form-item>
        <el-form-item label="检验类型" prop="inspection_type">
          <el-select v-model="recordForm.inspection_type" style="width: 100%">
            <el-option label="进货检验" value="incoming" />
            <el-option label="过程检验" value="process" />
            <el-option label="成品检验" value="finished" />
            <el-option label="出厂检验" value="outgoing" />
          </el-select>
        </el-form-item>
        <el-form-item label="产品" prop="product_name">
          <el-input v-model="recordForm.product_name" placeholder="产品名称" />
        </el-form-item>
        <el-form-item label="批次号" prop="batch_no">
          <el-input v-model="recordForm.batch_no" />
        </el-form-item>
        <el-form-item label="检验日期" prop="inspection_date">
          <el-date-picker
            v-model="recordForm.inspection_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="检验员" prop="inspector">
          <el-input v-model="recordForm.inspector" />
        </el-form-item>
        <el-form-item label="检验结果" prop="result">
          <el-radio-group v-model="recordForm.result">
            <el-radio label="pass">合格</el-radio>
            <el-radio label="fail">不合格</el-radio>
            <el-radio label="pending">待检</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="recordForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="recordDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="recordSubmitLoading" @click="submitRecord"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, provide } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import StandardTab from './tabs/StandardTab.vue'
import RecordTab from './tabs/RecordTab.vue'
import DefectTab from './tabs/DefectTab.vue'
import {
  listQualityStandards,
  getQualityStandard,
  createQualityStandard,
  updateQualityStandard,
  approveQualityStandard,
  listQualityRecords,
  createQualityRecord,
  listDefects,
  getQualityStandardVersions,
  type QualityStandard,
  type QualityRecord,
  type Defect,
} from '@/api/quality'

// 检验记录列定义已下线（V2Table 改为 el-table，未使用此 columns）
// TODO(tech-debt): 后端 listQualityRecords 暂未支持分页字段；待后端分页参数就绪后接入 page/size。
// handlePageChange / handleSizeChange 已下线（V2Table 移除）

const activeTab = ref('standard')

// 为 StandardTab/RecordTab 提供 actions（inject('qualityActions')）
// 注意：provide 移到所有函数定义之后，避免 hoisting 问题（vue-tsc 报 used before declaration）
const standards = ref<QualityStandard[]>([])
const records = ref<QualityRecord[]>([])
const defects = ref<Defect[]>([])
const standardLoading = ref(false)
const recordLoading = ref(false)
const defectLoading = ref(false)

const fetchStandards = async () => {
  standardLoading.value = true
  try {
    const res: any = await listQualityStandards()
    standards.value = res.data! || []
  } finally {
    standardLoading.value = false
  }
}

const fetchRecords = async () => {
  recordLoading.value = true
  try {
    const res: any = await listQualityRecords()
    records.value = res.data! || []
  } finally {
    recordLoading.value = false
  }
}

const fetchDefects = async () => {
  defectLoading.value = true
  try {
    const res: any = await listDefects()
    defects.value = res.data! || []
  } finally {
    defectLoading.value = false
  }
}

const getStandardStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    approved: '已审批',
    published: '已发布',
    rejected: '已驳回',
  }
  return map[status] || status
}

const getStandardStatusType = (status: string) => {
  const map: Record<string, any> = {
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
  standard_code: [{ required: true, message: '请输入标准编号', trigger: 'blur' }],
  standard_name: [{ required: true, message: '请输入标准名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }],
  version: [{ required: true, message: '请输入版本号', trigger: 'blur' }],
  content: [{ required: true, message: '请输入标准内容', trigger: 'blur' }],
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
          ElMessage.error('附件格式错误，请检查JSON格式')
          return
        }
      }
      if (standardForm.id) {
        await updateQualityStandard(standardForm.id, standardForm as Partial<QualityStandard>)
      } else {
        await createQualityStandard(standardForm as Partial<QualityStandard>)
      }
      ElMessage.success('操作成功')
      standardDialogVisible.value = false
      fetchStandards()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
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
  approval_comment: [{ required: true, message: '请输入审批意见', trigger: 'blur' }],
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
      ElMessage.success('审批成功')
      approveDialogVisible.value = false
      fetchStandards()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
    } finally {
      approveSubmitLoading.value = false
    }
  })
}

const rejectStandard = async () => {
  if (!approveStandardItem.value!) return
  try {
    await ElMessageBox.confirm('确定要驳回此标准吗？', '确认驳回', { type: 'warning' })
    ElMessage.info('驳回功能待后端实现')
    approveDialogVisible.value = false
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '操作失败')
  }
}

const versionHistoryVisible = ref(false)
const versionHistoryLoading = ref(false)
const versionHistoryList = ref<QualityStandard[]>([])

const viewVersionHistory = async (row: QualityStandard) => {
  versionHistoryLoading.value = true
  try {
    const res: any = await getQualityStandardVersions(row.id)
    versionHistoryList.value = res.data! || []
    versionHistoryVisible.value = true
  } catch (e: any) {
    ElMessage.error(e.message || '获取版本历史失败')
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
      ElMessage.info('更新功能待实现')
    } else {
      await createQualityRecord(recordForm as Partial<QualityRecord>)
    }
    ElMessage.success('操作成功')
    recordDialogVisible.value = false
    fetchRecords()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
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
