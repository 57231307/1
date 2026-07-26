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
        {{ t('common.afterSales.createTicket') }}
      </el-button>
    </div>

    <el-table :data="afterSales" border stripe :empty-text="t('common.afterSales.emptyText')" :aria-label="t('common.afterSales.tableAriaLabel')">
      <el-table-column :label="t('common.afterSales.colType')" width="100">
        <template #default="{ row }">
          <el-tag>{{ getIssueTypeLabel(row.issue_type) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" :label="t('common.afterSales.colDescription')" min-width="200" show-overflow-tooltip />
      <el-table-column :label="t('common.afterSales.colStatus')" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.afterSales.colRefundAmount')" width="140" align="right">
        <template #default="{ row }">
          <span v-if="row.refund_amount">{{ row.refund_amount }}</span>
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.afterSales.colOpenedAt')" width="170">
        <template #default="{ row }">
          {{ formatDate(row.opened_at) }}
        </template>
      </el-table-column>
      <el-table-column :label="t('common.afterSales.colClosedAt')" width="170">
        <template #default="{ row }">
          {{ formatDate(row.closed_at) }}
        </template>
      </el-table-column>
      <el-table-column prop="resolution" :label="t('common.afterSales.colResolution')" min-width="180" show-overflow-tooltip />
      <el-table-column :label="t('common.afterSales.colOperation')" width="180" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'opened'"
            size="small"
            type="primary"
            link
            @click="handleUpdate(row, 'processing')"
          >
            {{ t('common.afterSales.process') }}
          </el-button>
          <el-button
            v-if="row.status === 'opened' || row.status === 'processing'"
            size="small"
            type="success"
            link
            @click="showResolveDialog(row)"
          >
            {{ t('common.afterSales.resolve') }}
          </el-button>
          <el-button
            v-if="row.status === 'resolved'"
            size="small"
            link
            @click="handleUpdate(row, 'closed')"
          >
            {{ t('common.afterSales.close') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 创建工单 -->
    <el-dialog v-model="createVisible" :title="t('common.afterSales.createDialogTitle')" :aria-label="t('common.afterSales.createDialogAriaLabel')" width="540px">
      <el-form :model="form" :rules="rules" ref="formRef" label-width="100px" :aria-label="t('common.afterSales.createFormAriaLabel')">
        <el-form-item :label="t('common.afterSales.formIssueType')" prop="issue_type">
          <el-radio-group v-model="form.issue_type">
            <el-radio-button label="complaint">{{ t('common.afterSales.issueType.complaint') }}</el-radio-button>
            <el-radio-button label="repair">{{ t('common.afterSales.issueType.repair') }}</el-radio-button>
            <el-radio-button label="exchange">{{ t('common.afterSales.issueType.exchange') }}</el-radio-button>
            <el-radio-button label="refund">{{ t('common.afterSales.issueType.refund') }}</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item :label="t('common.afterSales.formCustomer')" prop="customer_id">
          <el-input-number v-model="form.customer_id" :min="1" />
        </el-form-item>
        <el-form-item v-if="form.issue_type === 'refund'" :label="t('common.afterSales.formRefundAmount')" prop="refund_amount">
          <el-input-number v-model="form.refund_amount" :min="0" :precision="2" :step="100" />
        </el-form-item>
        <el-form-item :label="t('common.afterSales.formDescription')" prop="description">
          <el-input v-model="form.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">{{ t('common.afterSales.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleCreateSubmit">{{ t('common.afterSales.submit') }}</el-button>
      </template>
    </el-dialog>

    <!-- 解决工单 -->
    <el-dialog v-model="resolveVisible" :title="t('common.afterSales.resolveDialogTitle')" :aria-label="t('common.afterSales.resolveDialogAriaLabel')" width="500px">
      <el-form :model="resolveForm" label-width="80px" :aria-label="t('common.afterSales.resolveFormAriaLabel')">
        <el-form-item :label="t('common.afterSales.formResolution')" required>
          <el-input v-model="resolveForm.resolution" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="resolveVisible = false">{{ t('common.afterSales.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleResolveSubmit">{{ t('common.afterSales.confirm') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import {
  createAfterSales,
  updateAfterSales,
  type AfterSales,
} from '@/api/custom-order'

const { t } = useI18n({ useScope: 'global' })

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

const rules = computed(() => ({
  issue_type: [{ required: true, message: t('common.afterSales.rule.issueTypeRequired'), trigger: 'change' }],
  customer_id: [{ required: true, message: t('common.afterSales.rule.customerRequired'), trigger: 'blur' }],
  description: [{ required: true, message: t('common.afterSales.rule.descriptionRequired'), trigger: 'blur' }],
  refund_amount: [
    {
      // v11 批次 167 P2-1 修复：validator 参数类型化
      validator: (_rule: unknown, val: unknown, cb: (error?: Error) => void) => {
        if (form.value.issue_type === 'refund' && (val === undefined || val === null)) {
          cb(new Error(t('common.afterSales.rule.refundAmountRequired')))
        } else {
          cb()
        }
      },
      trigger: 'blur',
    },
  ],
}))

/** 售后类型标签映射（响应式 t() 求值，替代导入的 AFTER_SALES_TYPE 中文常量） */
function getIssueTypeLabel(s: string): string {
  const known = ['complaint', 'repair', 'exchange', 'refund']
  if (!known.includes(s)) return s
  return t(`common.afterSales.issueType.${s}`)
}

/** 状态标签映射（响应式 t() 求值，替代导入的 AFTER_SALES_STATUS 中文常量） */
function getStatusLabel(s: string): string {
  const known = ['opened', 'processing', 'resolved', 'closed', 'rejected']
  if (!known.includes(s)) return s
  return t(`common.afterSales.status.${s}`)
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
    ElMessage.success(t('common.afterSales.createSuccess'))
    createVisible.value = false
    emit('refresh')
  } catch (e: unknown) {
    // v11 批次 167 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || t('common.afterSales.createFailed'))
  } finally {
    submitting.value = false
  }
}

async function handleUpdate(row: AfterSales, status: string) {
  try {
    await updateAfterSales(row.id, { status, resolution: row.resolution })
    ElMessage.success(t('common.afterSales.statusUpdated'))
    emit('refresh')
  } catch (e: unknown) {
    // v11 批次 167 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || t('common.afterSales.updateFailed'))
  }
}

function showResolveDialog(row: AfterSales) {
  currentRecord.value = row
  resolveForm.value = { resolution: '' }
  resolveVisible.value = true
}

async function handleResolveSubmit() {
  if (!resolveForm.value.resolution) {
    ElMessage.warning(t('common.afterSales.pleaseInputResolution'))
    return
  }
  submitting.value = true
  try {
    // v11 批次 167 CI1 修复：currentRecord.value 可能为 null，添加非空守卫
    const recordId = currentRecord.value?.id
    if (!recordId) {
      ElMessage.warning(t('common.afterSales.pleaseSelectTicket'))
      return
    }
    await updateAfterSales(recordId, {
      status: 'resolved',
      resolution: resolveForm.value.resolution,
    })
    ElMessage.success(t('common.afterSales.resolveSuccess'))
    resolveVisible.value = false
    emit('refresh')
  } catch (e: unknown) {
    // v11 批次 167 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || t('common.afterSales.resolveFailed'))
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
