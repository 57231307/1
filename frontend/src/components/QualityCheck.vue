<!--
  质量检查组件
  - 异常列表
  - 上报异常（GB/T 26377 色差 + ISO 105 色牢度）
  - 解决异常
-->
<template>
  <div class="quality-check">
    <div class="action-bar">
      <el-button type="primary" @click="showReportDialog">
        <el-icon><Plus /></el-icon>
        上报异常
      </el-button>
    </div>

    <el-table :data="issues" border stripe empty-text="暂无质量异常">
      <el-table-column prop="issue_type" label="异常类型" width="140" />
      <el-table-column label="严重度" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="ISSUE_SEVERITY_COLORS[row.severity] || 'info'">
            {{ ISSUE_SEVERITY[row.severity] || row.severity }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" min-width="200" show-overflow-tooltip />
      <el-table-column label="发现时间" width="170">
        <template #default="{ row }">
          {{ formatDate(row.discovered_at) }}
        </template>
      </el-table-column>
      <el-table-column label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="row.status === 'open' ? 'danger' : (row.status === 'resolved' ? 'success' : 'info')">
            {{ row.status }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="resolution" label="解决方案" min-width="200" show-overflow-tooltip />
      <el-table-column label="操作" width="120" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'open' || row.status === 'investigating'"
            size="small"
            type="success"
            link
            @click="handleResolve(row)"
          >
            解决
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 上报异常对话框 -->
    <el-dialog v-model="reportVisible" title="上报质量异常" width="540px">
      <el-form :model="reportForm" :rules="reportRules" ref="reportFormRef" label-width="100px">
        <el-form-item label="异常类型" prop="issue_type">
          <el-select v-model="reportForm.issue_type" placeholder="选择异常类型">
            <el-option label="色差 (GB/T 26377)" value="color_diff" />
            <el-option label="色牢度 (ISO 105)" value="color_fastness" />
            <el-option label="规格不符" value="spec" />
            <el-option label="破损" value="damage" />
            <el-option label="其他" value="other" />
          </el-select>
        </el-form-item>
        <el-form-item label="严重度" prop="severity">
          <el-radio-group v-model="reportForm.severity">
            <el-radio-button label="low">低</el-radio-button>
            <el-radio-button label="medium">中</el-radio-button>
            <el-radio-button label="high">高</el-radio-button>
            <el-radio-button label="critical">严重</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item v-if="reportForm.issue_type === 'color_diff'" label="色差 ΔE">
          <el-input-number v-model="reportForm.color_delta_e" :min="0" :precision="2" :step="0.5" />
          <span style="margin-left: 8px; color: #909399; font-size: 12px">
            GB/T 26377-2022 标准，&gt;5 提示可感知色差
          </span>
        </el-form-item>
        <el-form-item v-if="reportForm.issue_type === 'color_fastness'" label="色牢度等级">
          <el-select v-model="reportForm.color_fastness_grade" placeholder="ISO 105 等级 1-5">
            <el-option :label="`${i} 级`" :value="i" v-for="i in [1, 2, 3, 4, 5]" :key="i" />
          </el-select>
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="reportForm.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reportVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleReportSubmit">提交</el-button>
      </template>
    </el-dialog>

    <!-- 解决异常对话框 -->
    <el-dialog v-model="resolveVisible" title="解决质量异常" width="500px">
      <el-form :model="resolveForm" label-width="80px">
        <el-form-item label="解决方案" required>
          <el-input v-model="resolveForm.resolution" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="resolveVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleResolveSubmit">确认</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  reportQualityIssue,
  resolveQualityIssue,
  ISSUE_SEVERITY,
  ISSUE_SEVERITY_COLORS,
} from '@/api/custom-order'

const props = defineProps<{
  orderId: number
  issues: any[]
}>()

const emit = defineEmits<{ (e: 'refresh'): void }>()

const reportVisible = ref(false)
const resolveVisible = ref(false)
const submitting = ref(false)
const reportFormRef = ref()
const currentIssue = ref<any>(null)

const reportForm = ref({
  issue_type: '',
  severity: 'medium',
  description: '',
  color_delta_e: undefined as number | undefined,
  color_fastness_grade: undefined as number | undefined,
})

const resolveForm = ref({ resolution: '' })

const reportRules = {
  issue_type: [{ required: true, message: '异常类型必填', trigger: 'change' }],
  severity: [{ required: true, message: '严重度必填', trigger: 'change' }],
  description: [{ required: true, message: '描述必填', trigger: 'blur' }],
}

function formatDate(d: any) {
  if (!d) return '-'
  return new Date(d).toLocaleString('zh-CN')
}

function showReportDialog() {
  reportForm.value = {
    issue_type: '',
    severity: 'medium',
    description: '',
    color_delta_e: undefined,
    color_fastness_grade: undefined,
  }
  reportVisible.value = true
}

async function handleReportSubmit() {
  if (!reportFormRef.value) return
  try {
    await reportFormRef.value.validate()
  } catch {
    return
  }
  submitting.value = true
  try {
    await reportQualityIssue(props.orderId, reportForm.value)
    ElMessage.success('上报成功')
    reportVisible.value = false
    emit('refresh')
  } catch (e: any) {
    ElMessage.error(e?.message || '上报失败')
  } finally {
    submitting.value = false
  }
}

function handleResolve(row: any) {
  currentIssue.value = row
  resolveForm.value = { resolution: '' }
  resolveVisible.value = true
}

async function handleResolveSubmit() {
  if (!resolveForm.value.resolution) {
    ElMessage.warning('请输入解决方案')
    return
  }
  submitting.value = true
  try {
    await resolveQualityIssue(currentIssue.value.id, {
      resolution: resolveForm.value.resolution,
      operator_id: 1,
    })
    ElMessage.success('解决成功')
    resolveVisible.value = false
    emit('refresh')
  } catch (e: any) {
    ElMessage.error(e?.message || '解决失败')
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.quality-check {
  padding: 8px 0;
}
.action-bar {
  margin-bottom: 12px;
}
</style>
