<template>
  <div class="supplier-evaluation">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>供应商评估管理</span>
        </div>
      </template>
      
      <el-tabs v-model="activeTab">
        <el-tab-pane label="评估记录" name="records">
          <div class="toolbar">
            <el-button type="primary" @click="handleCreateRecord">新建评估</el-button>
          </div>
          
          <el-table :data="recordList" border stripe>
            <el-table-column prop="supplierName" label="供应商名称" />
            <el-table-column prop="period" label="评估周期" />
            <el-table-column prop="totalScore" label="总分" />
            <el-table-column prop="rating" label="评级">
              <template #default="{ row }">
                <el-tag v-if="row.rating === 'A'" type="success">A</el-tag>
                <el-tag v-else-if="row.rating === 'B'" type="warning">B</el-tag>
                <el-tag v-else-if="row.rating === 'C'" type="danger">C</el-tag>
                <el-tag v-else type="info">{{ row.rating }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" />
            <el-table-column prop="evaluatorName" label="评估人" />
            <el-table-column prop="createdAt" label="创建时间" />
            <el-table-column label="操作" fixed="right" width="200">
              <template #default="{ row }">
                <el-button link type="primary" @click="handleViewRecord(row)">查看</el-button>
              </template>
            </el-table-column>
          </el-table>
          
          <el-pagination
            v-model:current-page="recordPagination.page"
            v-model:page-size="recordPagination.pageSize"
            :total="recordPagination.total"
            layout="total, prev, pager, next, jumper"
            @current-change="fetchRecords"
          />
        </el-tab-pane>
        
        <el-tab-pane label="供应商排名" name="rankings">
          <el-button type="primary" @click="fetchRankings">刷新排名</el-button>
          
          <el-table :data="rankingList" border stripe>
            <el-table-column prop="rank" label="排名" width="80">
              <template #default="{ row }">
                <span v-if="row.rank === 1" class="rank-first">🥇 {{ row.rank }}</span>
                <span v-else-if="row.rank === 2" class="rank-second">🥈 {{ row.rank }}</span>
                <span v-else-if="row.rank === 3" class="rank-third">🥉 {{ row.rank }}</span>
                <span v-else>{{ row.rank }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="supplierName" label="供应商名称" />
            <el-table-column prop="totalScore" label="总分" />
            <el-table-column prop="rating" label="评级">
              <template #default="{ row }">
                <el-tag v-if="row.rating === 'A'" type="success">A</el-tag>
                <el-tag v-else-if="row.rating === 'B'" type="warning">B</el-tag>
                <el-tag v-else-if="row.rating === 'C'" type="danger">C</el-tag>
                <el-tag v-else type="info">{{ row.rating }}</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>
    
    <!-- 创建/编辑对话框 -->
    <el-dialog v-model="recordDialogVisible" :title="isEdit ? '编辑评估' : '新建评估'" width="600px">
      <el-form :model="recordForm" :rules="recordRules" ref="recordFormRef" label-width="120px">
        <el-form-item label="供应商" prop="supplierId">
          <el-select v-model="recordForm.supplierId" placeholder="请选择供应商" style="width: 100%">
            <el-option label="供应商A" :value="1" />
            <el-option label="供应商B" :value="2" />
            <el-option label="供应商C" :value="3" />
          </el-select>
        </el-form-item>
        <el-form-item label="评估周期" prop="period">
          <el-input v-model="recordForm.period" placeholder="例如：2024-Q1" />
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="recordForm.remark" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="recordDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSaveRecord" :loading="submitLoading">保存</el-button>
      </template>
    </el-dialog>
    
    <!-- 详情对话框 -->
    <el-dialog v-model="detailDialogVisible" title="评估详情" width="700px">
      <el-descriptions :column="2" border v-if="currentRecord">
        <el-descriptions-item label="供应商">{{ currentRecord.supplierName }}</el-descriptions-item>
        <el-descriptions-item label="评估周期">{{ currentRecord.period }}</el-descriptions-item>
        <el-descriptions-item label="总分">{{ currentRecord.totalScore }}</el-descriptions-item>
        <el-descriptions-item label="评级">{{ currentRecord.rating }}</el-descriptions-item>
        <el-descriptions-item label="状态">{{ currentRecord.status }}</el-descriptions-item>
        <el-descriptions-item label="评估人">{{ currentRecord.evaluatorName }}</el-descriptions-item>
        <el-descriptions-item label="创建时间" :span="2">{{ currentRecord.createdAt }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{ currentRecord.remark || '-' }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  listEvaluationRecords,
  createEvaluationRecord,
  getSupplierRankings,
  type EvaluationRecord
} from '@/api/supplier-evaluation'

const activeTab = ref('records')
const recordList = ref<EvaluationRecord[]>([])
const rankingList = ref<any[]>([])

const recordPagination = reactive({
  page: 1,
  pageSize: 20,
  total: 0
})

const recordDialogVisible = ref(false)
const detailDialogVisible = ref(false)
const isEdit = ref(false)
const submitLoading = ref(false)
const recordFormRef = ref()
const currentRecord = ref<EvaluationRecord | null>(null)

const recordForm = reactive({
  supplierId: undefined as number | undefined,
  period: '',
  remark: ''
})

const recordRules = {
  supplierId: [{ required: true, message: '请选择供应商', trigger: 'change' }],
  period: [{ required: true, message: '请输入评估周期', trigger: 'blur' }]
}

const fetchRecords = async () => {
  try {
    const res: any = await listEvaluationRecords({
      page: recordPagination.page,
      pageSize: recordPagination.pageSize
    })
    if (res.data) {
      recordList.value = res.data!.list || res.data! || []
      recordPagination.total = res.data!.total || res.data?.length || 0
    }
  } catch (e) {
    ElMessage.error('获取评估记录失败')
  }
}

const fetchRankings = async () => {
  try {
    const res: any = await getSupplierRankings({ limit: 20 })
    if (res.data) {
      rankingList.value = (res.data! || []).map((item: any, index: number) => ({
        ...item,
        rank: index + 1
      }))
    }
  } catch (e) {
    ElMessage.error('获取排名失败')
  }
}

const handleCreateRecord = () => {
  isEdit.value = false
  Object.assign(recordForm, { supplierId: undefined, period: '', remark: '' })
  recordDialogVisible.value = true
}

const handleViewRecord = (row: EvaluationRecord) => {
  currentRecord.value = row
  detailDialogVisible.value = true
}

const handleSaveRecord = async () => {
  if (!recordFormRef.value) return
  
  await recordFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      await createEvaluationRecord(recordForm as any)
      ElMessage.success('保存成功')
      recordDialogVisible.value = false
      fetchRecords()
    } catch (e: any) {
      ElMessage.error(e.message || '保存失败')
    } finally {
      submitLoading.value = false
    }
  })
}

onMounted(() => {
  fetchRecords()
  fetchRankings()
})
</script>

<style scoped>
.supplier-evaluation .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.supplier-evaluation .toolbar {
  margin-bottom: 16px;
}

.supplier-evaluation .el-table {
  margin-bottom: 16px;
}

.supplier-evaluation .rank-first {
  color: #ffd700;
  font-weight: bold;
}

.supplier-evaluation .rank-second {
  color: #c0c0c0;
  font-weight: bold;
}

.supplier-evaluation .rank-third {
  color: #cd7f32;
  font-weight: bold;
}
</style>
