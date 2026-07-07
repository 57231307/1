<!--
  售后工单面板
  - 4 种类型：客诉/维修/换货/退款
  - 创建工单 + 更新状态
-->
<template>
  <div class="after-sales-panel">
    <div class="action-bar">
      <el-button type="primary" @click="showCreateDialog">
        <el-icon><Plus /></el-icon>
        创建售后工单
      </el-button>
    </div>

    <el-table :data="afterSales" border stripe empty-text="暂无售后工单">
      <el-table-column label="类型" width="100">
        <template #default="{ row }">
          <el-tag>{{ AFTER_SALES_TYPE[row.issue_type] || row.issue_type }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" min-width="200" show-overflow-tooltip />
      <el-table-column label="状态" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ AFTER_SALES_STATUS[row.status] || row.status }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="退款金额" width="140" align="right">
        <template #default="{ row }">
          <span v-if="row.refund_amount">{{ row.refund_amount }}</span>
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column label="开单时间" width="170">
        <template #default="{ row }">
          {{ formatDate(row.opened_at) }}
        </template>
      </el-table-column>
      <el-table-column label="解决时间" width="170">
        <template #default="{ row }">
          {{ formatDate(row.closed_at) }}
        </template>
      </el-table-column>
      <el-table-column prop="resolution" label="解决方案" min-width="180" show-overflow-tooltip />
      <el-table-column label="操作" width="180" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'opened'"
            size="small"
            type="primary"
            link
            @click="handleUpdate(row, 'processing')"
          >
            处理
          </el-button>
          <el-button
            v-if="row.status === 'opened' || row.status === 'processing'"
            size="small"
            type="success"
            link
            @click="showResolveDialog(row)"
          >
            解决
          </el-button>
          <el-button
            v-if="row.status === 'resolved'"
            size="small"
            link
            @click="handleUpdate(row, 'closed')"
          >
            关闭
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 创建工单 -->
    <el-dialog v-model="createVisible" title="创建售后工单" width="540px">
      <el-form :model="form" :rules="rules" ref="formRef" label-width="100px">
        <el-form-item label="售后类型" prop="issue_type">
          <el-radio-group v-model="form.issue_type">
            <el-radio-button label="complaint">客诉</el-radio-button>
            <el-radio-button label="repair">维修</el-radio-button>
            <el-radio-button label="exchange">换货</el-radio-button>
            <el-radio-button label="refund">退款</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="客户" prop="customer_id">
          <el-input-number v-model="form.customer_id" :min="1" />
        </el-form-item>
        <el-form-item v-if="form.issue_type === 'refund'" label="退款金额" prop="refund_amount">
          <el-input-number v-model="form.refund_amount" :min="0" :precision="2" :step="100" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="form.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleCreateSubmit">提交</el-button>
      </template>
    </el-dialog>

    <!-- 解决工单 -->
    <el-dialog v-model="resolveVisible" title="解决售后工单" width="500px">
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
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  createAfterSales,
  updateAfterSales,
  AFTER_SALES_TYPE,
  AFTER_SALES_STATUS,
  type AfterSales,
} from '@/api/custom-order'

const props = defineProps<{
  orderId: number
  afterSales: AfterSales[]
}>()

const emit = defineEmits<{ (e: 'refresh'): void }>()

const createVisible = ref(false)
const resolveVisible = ref(false)
const submitting = ref(false)
const formRef = ref()
const currentRecord = ref<AfterSales | null>(null)

const form = ref({
  issue_type: 'complaint',
  customer_id: undefined as number | undefined,
  description: '',
  refund_amount: undefined as number | undefined,
})

const resolveForm = ref({ resolution: '' })

const rules = {
  issue_type: [{ required: true, message: '类型必填', trigger: 'change' }],
  customer_id: [{ required: true, message: '客户必填', trigger: 'blur' }],
  description: [{ required: true, message: '描述必填', trigger: 'blur' }],
  refund_amount: [
    {
      // v11 批次 167 P2-1 修复：validator 参数类型化
      validator: (_rule: unknown, val: unknown, cb: (error?: Error) => void) => {
        if (form.value.issue_type === 'refund' && (val === undefined || val === null)) {
          cb(new Error('退款类型必须填写金额'))
        } else {
          cb()
        }
      },
      trigger: 'blur',
    },
  ],
}

// v11 批次 167 P2-1 修复：Record<string, any> 改为联合字面量类型
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger'

function getStatusType(s: string): TagType {
  const map: Record<string, TagType> = {
    opened: 'warning',
    processing: 'primary',
    resolved: 'success',
    closed: 'info',
    rejected: 'danger',
  }
  return map[s] || 'info'
}

function formatDate(d: string | undefined) {
  if (!d) return '-'
  return new Date(d).toLocaleString('zh-CN')
}

function showCreateDialog() {
  form.value = {
    issue_type: 'complaint',
    customer_id: undefined,
    description: '',
    refund_amount: undefined,
  }
  createVisible.value = true
}

async function handleCreateSubmit() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  submitting.value = true
  try {
    await createAfterSales(props.orderId, form.value)
    ElMessage.success('创建成功')
    createVisible.value = false
    emit('refresh')
  } catch (e: unknown) {
    // v11 批次 167 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || '创建失败')
  } finally {
    submitting.value = false
  }
}

async function handleUpdate(row: AfterSales, status: string) {
  try {
    await updateAfterSales(row.id, { status, resolution: row.resolution })
    ElMessage.success('状态已更新')
    emit('refresh')
  } catch (e: unknown) {
    // v11 批次 167 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || '更新失败')
  }
}

function showResolveDialog(row: AfterSales) {
  currentRecord.value = row
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
    await updateAfterSales(currentRecord.value.id, {
      status: 'resolved',
      resolution: resolveForm.value.resolution,
    })
    ElMessage.success('解决成功')
    resolveVisible.value = false
    emit('refresh')
  } catch (e: unknown) {
    // v11 批次 167 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || '解决失败')
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.after-sales-panel {
  padding: 8px 0;
}
.action-bar {
  margin-bottom: 12px;
}
</style>
