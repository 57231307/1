<!--
  报价单审批页
  - 顶部：审批进度（ApprovalProgress 组件）
  - 描述列表：客户/金额/审批人/拒绝原因
  - 审批操作按钮（按状态）
-->
<template>
  <div v-loading="loading" class="approval-page">
    <el-card v-if="quotation">
      <template #header>
        <div class="card-header">
          <span class="title">报价单审批 - {{ quotation.quotation_no }}</span>
          <el-button @click="$router.back()">返回</el-button>
        </div>
      </template>

      <ApprovalProgress
        :status="quotation.status"
        :approved-at="quotation.approved_at"
        :approved-by-name="quotation.approved_by_name"
        :rejection-reason="quotation.rejection_reason"
        :converted-at="quotation.converted_at"
        :converted-order-id="quotation.converted_sales_order_id"
      />

      <el-descriptions :column="2" border style="margin-top: 24px">
        <el-descriptions-item label="客户">
          {{ quotation.customer_name || quotation.customer_id }}
        </el-descriptions-item>
        <el-descriptions-item label="金额">
          {{ quotation.currency }} {{ formatAmount(quotation.total_amount) }}
        </el-descriptions-item>
        <el-descriptions-item label="价格条款">{{ quotation.price_terms }}</el-descriptions-item>
        <el-descriptions-item label="币种">
          {{ quotation.currency }}（汇率 {{ quotation.exchange_rate }}）
        </el-descriptions-item>
        <el-descriptions-item label="报价日期" :span="2">
          {{ quotation.quotation_date }}
        </el-descriptions-item>
        <el-descriptions-item label="有效期至" :span="2">
          {{ quotation.valid_until }}
        </el-descriptions-item>
        <el-descriptions-item label="审批人" :span="2">
          {{ quotation.approved_by_name || quotation.approved_by || '-' }}
          <span v-if="quotation.approved_at" class="meta-text">
            （{{ quotation.approved_at }}）
          </span>
        </el-descriptions-item>
        <el-descriptions-item v-if="quotation.rejection_reason" label="拒绝原因" :span="2">
          <span class="rejection-reason">{{ quotation.rejection_reason }}</span>
        </el-descriptions-item>
        <el-descriptions-item
          v-if="quotation.converted_sales_order_id"
          label="已转销售订单"
          :span="2"
        >
          订单 ID：{{ quotation.converted_sales_order_id }}
          <span v-if="quotation.converted_at" class="meta-text">
            （{{ quotation.converted_at }}）
          </span>
        </el-descriptions-item>
      </el-descriptions>

      <div class="actions">
        <el-button v-if="canSubmit" type="primary" :loading="submitting" @click="handleSubmit">
          提交审批
        </el-button>
        <el-button v-if="canApprove" type="success" :loading="submitting" @click="handleApprove">
          批准
        </el-button>
        <el-button v-if="canApprove" type="danger" :loading="submitting" @click="handleReject">
          拒绝
        </el-button>
        <el-button v-if="canConvert" type="success" :loading="submitting" @click="handleConvert">
          转销售订单
        </el-button>
      </div>
    </el-card>

    <el-empty v-else-if="!loading" description="报价单不存在" />
  </div>
</template>

<script setup lang="ts">
// 报价单审批页脚本
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  getQuotation,
  submitQuotation,
  approveQuotation,
  rejectQuotation,
  convertQuotation,
  type QuotationResponseDto,
} from '@/api/quotation'
import ApprovalProgress from './components/ApprovalProgress.vue'

const route = useRoute()
const router = useRouter()
const loading = ref(false)
const submitting = ref(false)
const quotation = ref<QuotationResponseDto | null>(null)

/** 加载 */
async function loadData() {
  const id = Number(route.params.id)
  if (!id) return
  loading.value = true
  try {
    const res = await getQuotation(id)
    quotation.value = res.data as QuotationResponseDto
  } catch (e: any) {
    ElMessage.error(e?.message || '加载报价单失败')
    quotation.value = null
  } finally {
    loading.value = false
  }
}

const canSubmit = computed(
  () => quotation.value && ['draft', 'rejected'].includes(quotation.value.status)
)
const canApprove = computed(() => quotation.value?.status === 'pending_approval')
const canConvert = computed(() => quotation.value?.status === 'approved')

async function handleSubmit() {
  if (!quotation.value) return
  submitting.value = true
  try {
    await submitQuotation(quotation.value.id)
    ElMessage.success('已提交审批')
    loadData()
  } finally {
    submitting.value = false
  }
}

async function handleApprove() {
  if (!quotation.value) return
  try {
    await ElMessageBox.confirm('确认批准此报价单？', '批准确认', { type: 'warning' })
  } catch {
    return
  }
  submitting.value = true
  try {
    await approveQuotation(quotation.value.id)
    ElMessage.success('已批准')
    loadData()
  } finally {
    submitting.value = false
  }
}

async function handleReject() {
  if (!quotation.value) return
  let reason = ''
  try {
    const { value } = await ElMessageBox.prompt('请输入拒绝原因', '拒绝', {
      inputValidator: (v: string) => (v && v.trim() ? true : '拒绝原因不能为空'),
    })
    reason = value
  } catch {
    return
  }
  submitting.value = true
  try {
    await rejectQuotation(quotation.value.id, reason)
    ElMessage.success('已拒绝')
    loadData()
  } finally {
    submitting.value = false
  }
}

async function handleConvert() {
  if (!quotation.value) return
  try {
    await ElMessageBox.confirm('确认转销售订单？', '转订单', { type: 'warning' })
  } catch {
    return
  }
  submitting.value = true
  try {
    const res = await convertQuotation(quotation.value.id)
    const order = res.data as any
    ElMessage.success(`转订单成功，销售订单 ID：${order?.id}`)
    if (order?.id) {
      router.push(`/sales/orders/${order.id}`)
    } else {
      loadData()
    }
  } finally {
    submitting.value = false
  }
}

function formatAmount(value?: number): string {
  if (value === undefined || value === null) return '0.00'
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

onMounted(loadData)
</script>

<style scoped>
.approval-page {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
.meta-text {
  margin-left: 8px;
  color: #909399;
  font-size: 13px;
}
.rejection-reason {
  color: #f56c6c;
}
.actions {
  margin-top: 24px;
  display: flex;
  gap: 12px;
  justify-content: center;
}
</style>
