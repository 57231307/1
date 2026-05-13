<template>
  <div class="quality-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="质量标准" name="standard">
        <div class="page-header">
          <h2 class="page-title">质量标准管理</h2>
          <el-button type="primary" @click="openStandardDialog">
            <el-icon><Plus /></el-icon>
            新建标准
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="qualityStandards" v-loading="standardLoading" stripe>
            <el-table-column prop="standard_code" label="标准编号" width="140" />
            <el-table-column prop="standard_name" label="标准名称" min-width="180" />
            <el-table-column prop="version" label="版本" width="80" />
            <el-table-column prop="type" label="类型" width="100">
              <template #default="{ row }">
                <el-tag size="small">{{ row.type === 'product' ? '产品' : '工艺' }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getStandardStatusType(row.status)" size="small">
                  {{ getStandardStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_by_name" label="创建人" width="120" />
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="240" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openStandardDialog(row)">编辑</el-button>
                <el-button v-if="row.status === 'draft'" type="success" link size="small" @click="approveStandard(row)">审批</el-button>
                <el-button v-if="row.status === 'approved'" type="warning" link size="small" @click="publishStandard(row)">发布</el-button>
                <el-button type="danger" link size="small" @click="deleteStandard(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="检验记录" name="record">
        <div class="page-header">
          <h2 class="page-title">检验记录管理</h2>
          <el-button type="primary" @click="openRecordDialog">
            <el-icon><Plus /></el-icon>
            新建记录
          </el-button>
        </div>

        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="recordQuery">
            <el-form-item label="检验类型">
              <el-input v-model="recordQuery.inspection_type" placeholder="检验类型" clearable />
            </el-form-item>
            <el-form-item label="产品">
              <el-input v-model="recordQuery.product_name" placeholder="产品名称" clearable />
            </el-form-item>
            <el-form-item label="结果">
              <el-select v-model="recordQuery.result" placeholder="选择结果" clearable>
                <el-option label="通过" value="pass" />
                <el-option label="不通过" value="fail" />
                <el-option label="待处理" value="pending" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchQualityRecords">查询</el-button>
              <el-button @click="resetRecordQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card shadow="hover">
          <el-table :data="qualityRecords" v-loading="recordLoading" stripe>
            <el-table-column prop="record_no" label="记录编号" width="140" />
            <el-table-column prop="inspection_type" label="检验类型" width="120" />
            <el-table-column prop="product_name" label="产品名称" min-width="150" />
            <el-table-column prop="batch_no" label="批次号" width="120" />
            <el-table-column prop="inspection_date" label="检验日期" width="120" />
            <el-table-column prop="inspector" label="检验员" width="100" />
            <el-table-column prop="result" label="结果" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getResultType(row.result)" size="small">
                  {{ getResultLabel(row.result) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewRecord(row)">查看</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="standardDialogVisible" :title="standardForm.id ? '编辑质量标准' : '新建质量标准'" width="700px">
      <el-form ref="standardFormRef" :model="standardForm" :rules="standardRules" label-width="100px">
        <el-form-item label="标准编号" prop="standard_code">
          <el-input v-model="standardForm.standard_code" :disabled="!!standardForm.id" />
        </el-form-item>
        <el-form-item label="标准名称" prop="standard_name">
          <el-input v-model="standardForm.standard_name" />
        </el-form-item>
        <el-form-item label="版本" prop="version">
          <el-input v-model="standardForm.version" />
        </el-form-item>
        <el-form-item label="类型" prop="type">
          <el-select v-model="standardForm.type" placeholder="选择类型" style="width: 100%">
            <el-option label="产品" value="product" />
            <el-option label="工艺" value="process" />
          </el-select>
        </el-form-item>
        <el-form-item label="内容" prop="content">
          <el-input v-model="standardForm.content" type="textarea" :rows="6" />
        </el-form-item>
        <el-form-item label="状态">
          <el-tag :type="getStandardStatusType(standardForm.status)">
            {{ getStandardStatusLabel(standardForm.status) }}
          </el-tag>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="standardDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="standardSubmitLoading" @click="submitStandard">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="recordDialogVisible" :title="recordForm.id ? '编辑检验记录' : '新建检验记录'" width="700px">
      <el-form ref="recordFormRef" :model="recordForm" :rules="recordRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="记录编号" prop="record_no">
              <el-input v-model="recordForm.record_no" :disabled="!!recordForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="检验类型" prop="inspection_type">
              <el-input v-model="recordForm.inspection_type" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="产品" prop="product_name">
              <el-input v-model="recordForm.product_name" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="批次号" prop="batch_no">
              <el-input v-model="recordForm.batch_no" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="检验日期" prop="inspection_date">
              <el-date-picker v-model="recordForm.inspection_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="检验员" prop="inspector">
              <el-input v-model="recordForm.inspector" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="结果" prop="result">
          <el-radio-group v-model="recordForm.result">
            <el-radio value="pass">通过</el-radio>
            <el-radio value="fail">不通过</el-radio>
            <el-radio value="pending">待处理</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="缺陷明细">
          <el-table :data="recordForm.defects" border style="width: 100%">
            <el-table-column prop="defect_type" label="缺陷类型" width="120">
              <template #default="{ row }">
                <el-input v-model="row.defect_type" />
              </template>
            </el-table-column>
            <el-table-column prop="defect_description" label="描述" min-width="150">
              <template #default="{ row }">
                <el-input v-model="row.defect_description" />
              </template>
            </el-table-column>
            <el-table-column prop="severity" label="严重程度" width="120">
              <template #default="{ row }">
                <el-select v-model="row.severity" placeholder="选择">
                  <el-option label="轻微" value="minor" />
                  <el-option label="主要" value="major" />
                  <el-option label="严重" value="critical" />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column prop="quantity" label="数量" width="80">
              <template #default="{ row }">
                <el-input-number v-model="row.quantity" style="width: 100%" :min="0" />
              </template>
            </el-table-column>
            <el-table-column label="操作" width="80">
              <template #default="{ $index }">
                <el-button type="danger" link size="small" @click="removeDefect($index)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-button type="primary" link style="margin-top: 8px" @click="addDefect">添加缺陷</el-button>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="recordForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="recordDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="recordSubmitLoading" @click="submitRecord">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="recordViewVisible" title="检验记录详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="记录编号">{{ currentRecord?.record_no }}</el-descriptions-item>
        <el-descriptions-item label="检验类型">{{ currentRecord?.inspection_type }}</el-descriptions-item>
        <el-descriptions-item label="产品名称">{{ currentRecord?.product_name }}</el-descriptions-item>
        <el-descriptions-item label="批次号">{{ currentRecord?.batch_no }}</el-descriptions-item>
        <el-descriptions-item label="检验日期">{{ currentRecord?.inspection_date }}</el-descriptions-item>
        <el-descriptions-item label="检验员">{{ currentRecord?.inspector }}</el-descriptions-item>
        <el-descriptions-item label="结果">
          <el-tag :type="getResultType(currentRecord?.result)">
            {{ getResultLabel(currentRecord?.result) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ currentRecord?.created_at }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>缺陷明细</el-divider>
      <el-table :data="currentRecord?.defects || []" stripe>
        <el-table-column prop="defect_type" label="缺陷类型" width="120" />
        <el-table-column prop="defect_description" label="描述" min-width="150" />
        <el-table-column prop="severity" label="严重程度" width="100">
          <template #default="{ row }">
            <el-tag :type="getSeverityType(row.severity)">
              {{ getSeverityLabel(row.severity) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="quantity" label="数量" width="80" />
        <el-table-column prop="processed" label="已处理" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.processed ? 'success' : 'info'">
              {{ row.processed ? '是' : '否' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="120" />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listQualityStandards,
  getQualityStandard,
  createQualityStandard,
  updateQualityStandard,
  deleteQualityStandard,
  approveQualityStandard,
  publishQualityStandard,
  listQualityRecords,
  createQualityRecord,
  type QualityStandard,
  type QualityRecord,
  type Defect
} from '@/api/quality'

const activeTab = ref('standard')

const qualityStandards = ref<QualityStandard[]>([])
const qualityRecords = ref<QualityRecord[]>([])
const standardLoading = ref(false)
const recordLoading = ref(false)

const recordQuery = reactive({
  inspection_type: '',
  product_name: '',
  result: ''
})

const fetchQualityStandards = async () => {
  standardLoading.value = true
  try {
    const res = await listQualityStandards()
    qualityStandards.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取质量标准失败')
  } finally {
    standardLoading.value = false
  }
}

const fetchQualityRecords = async () => {
  recordLoading.value = true
  try {
    const res = await listQualityRecords(recordQuery)
    qualityRecords.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取检验记录失败')
  } finally {
    recordLoading.value = false
  }
}

const resetRecordQuery = () => {
  recordQuery.inspection_type = ''
  recordQuery.product_name = ''
  recordQuery.result = ''
  fetchQualityRecords()
}

const getStandardStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    approved: '已审批',
    published: '已发布'
  }
  return map[status] || status
}

const getStandardStatusType = (status: string) => {
  const map: Record<string, any> = {
    draft: 'info',
    approved: 'warning',
    published: 'success'
  }
  return map[status] || 'info'
}

const getResultLabel = (result?: string) => {
  const map: Record<string, string> = {
    pass: '通过',
    fail: '不通过',
    pending: '待处理'
  }
  return map[result || ''] || result || ''
}

const getResultType = (result?: string) => {
  const map: Record<string, any> = {
    pass: 'success',
    fail: 'danger',
    pending: 'warning'
  }
  return map[result || ''] || 'info'
}

const getSeverityLabel = (severity: string) => {
  const map: Record<string, string> = {
    minor: '轻微',
    major: '主要',
    critical: '严重'
  }
  return map[severity] || severity
}

const getSeverityType = (severity: string) => {
  const map: Record<string, any> = {
    minor: 'info',
    major: 'warning',
    critical: 'danger'
  }
  return map[severity] || 'info'
}

const standardDialogVisible = ref(false)
const standardFormRef = ref<FormInstance>()
const standardSubmitLoading = ref(false)
const standardForm = reactive({
  id: 0,
  standard_code: '',
  standard_name: '',
  version: '1.0',
  type: 'product' as 'product' | 'process',
  status: 'draft' as 'draft' | 'approved' | 'published',
  content: '',
  attachments: [] as string[]
})

const standardRules: FormRules = {
  standard_code: [{ required: true, message: '请输入标准编号', trigger: 'blur' }],
  standard_name: [{ required: true, message: '请输入标准名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }]
}

const openStandardDialog = async (row?: QualityStandard) => {
  if (row) {
    const res = await getQualityStandard(row.id)
    Object.assign(standardForm, res.data)
  } else {
    Object.assign(standardForm, {
      id: 0,
      standard_code: '',
      standard_name: '',
      version: '1.0',
      type: 'product',
      status: 'draft',
      content: '',
      attachments: []
    })
  }
  standardDialogVisible.value = true
}

const submitStandard = async () => {
  const valid = await standardFormRef.value?.validate()
  if (!valid) return

  standardSubmitLoading.value = true
  try {
    if (standardForm.id) {
      await updateQualityStandard(standardForm.id, standardForm)
      ElMessage.success('更新成功')
    } else {
      await createQualityStandard(standardForm)
      ElMessage.success('创建成功')
    }
    standardDialogVisible.value = false
    fetchQualityStandards()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    standardSubmitLoading.value = false
  }
}

const approveStandard = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm('确定审批此标准吗？', '确认', { type: 'info' })
    await approveQualityStandard(row.id)
    ElMessage.success('审批成功')
    fetchQualityStandards()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '操作失败')
  }
}

const publishStandard = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm('确定发布此标准吗？', '确认', { type: 'info' })
    await publishQualityStandard(row.id)
    ElMessage.success('发布成功')
    fetchQualityStandards()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '操作失败')
  }
}

const deleteStandard = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm(`确定删除标准 "${row.standard_name}" 吗？`, '删除确认', { type: 'warning' })
    await deleteQualityStandard(row.id)
    ElMessage.success('删除成功')
    fetchQualityStandards()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

const recordDialogVisible = ref(false)
const recordFormRef = ref<FormInstance>()
const recordSubmitLoading = ref(false)
const recordForm = reactive({
  id: 0,
  record_no: '',
  inspection_type: '',
  product_id: 0,
  product_name: '',
  batch_no: '',
  inspection_date: '',
  inspector: '',
  result: 'pending' as 'pass' | 'fail' | 'pending',
  defects: [] as Defect[],
  remark: ''
})

const recordRules: FormRules = {
  record_no: [{ required: true, message: '请输入记录编号', trigger: 'blur' }],
  inspection_type: [{ required: true, message: '请输入检验类型', trigger: 'blur' }],
  product_name: [{ required: true, message: '请输入产品名称', trigger: 'blur' }],
  inspection_date: [{ required: true, message: '请选择检验日期', trigger: 'change' }],
  result: [{ required: true, message: '请选择结果', trigger: 'change' }]
}

const openRecordDialog = (row?: QualityRecord) => {
  if (row) {
    Object.assign(recordForm, row)
  } else {
    Object.assign(recordForm, {
      id: 0,
      record_no: '',
      inspection_type: '',
      product_id: 0,
      product_name: '',
      batch_no: '',
      inspection_date: '',
      inspector: '',
      result: 'pending',
      defects: [{ id: 0, record_id: 0, defect_type: '', defect_description: '', severity: 'minor', quantity: 0, processed: false, processed_by: '', processed_at: '', remark: '' }],
      remark: ''
    })
  }
  recordDialogVisible.value = true
}

const submitRecord = async () => {
  const valid = await recordFormRef.value?.validate()
  if (!valid) return

  recordSubmitLoading.value = true
  try {
    await createQualityRecord(recordForm)
    ElMessage.success('创建成功')
    recordDialogVisible.value = false
    fetchQualityRecords()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    recordSubmitLoading.value = false
  }
}

const addDefect = () => {
  recordForm.defects.push({
    id: 0,
    record_id: 0,
    defect_type: '',
    defect_description: '',
    severity: 'minor',
    quantity: 0,
    processed: false,
    processed_by: '',
    processed_at: '',
    remark: ''
  })
}

const removeDefect = (index: number) => {
  if (recordForm.defects.length > 1) {
    recordForm.defects.splice(index, 1)
  }
}

const recordViewVisible = ref(false)
const currentRecord = ref<QualityRecord | null>(null)

const viewRecord = (row: QualityRecord) => {
  currentRecord.value = row
  recordViewVisible.value = true
}

onMounted(() => {
  fetchQualityStandards()
  fetchQualityRecords()
})
</script>

<style scoped>
.quality-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 600; color: #303133; margin: 0; }
.filter-card { margin-bottom: 20px; }
</style>
