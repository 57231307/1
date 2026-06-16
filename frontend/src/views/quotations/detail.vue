<!--
  报价单详情页
  - 顶部按钮：返回 / 编辑 / 提交 / 批准 / 拒绝 / 转订单 / 取消（按状态显示）
  - 描述列表：客户/日期/价格条款/币种/含税/客户等级/MOQ/交期/状态
  - 报价明细表
  - 贸易条款 Tab
  - 金额合计
-->
<template>
  <div v-loading="loading" class="quotation-detail">
    <el-card v-if="quotation">
      <template #header>
        <div class="card-header">
          <span class="title">
            报价单详情 -
            <span class="quotation-no">{{ quotation.quotation_no }}</span>
          </span>
          <div class="actions">
            <el-button @click="$router.back()">返回</el-button>
            <el-button v-if="canEdit" @click="$router.push(`/quotations/${quotation.id}/edit`)">
              编辑
            </el-button>
            <el-button v-if="canSubmit" type="primary" @click="handleSubmit"> 提交审批 </el-button>
            <el-button v-if="canApprove" type="success" @click="handleApprove"> 批准 </el-button>
            <el-button v-if="canApprove" type="danger" @click="handleReject"> 拒绝 </el-button>
            <el-button v-if="canConvert" type="success" @click="handleConvert">
              转销售订单
            </el-button>
            <el-button v-if="canCancel" type="danger" plain @click="handleCancel"> 取消 </el-button>
          </div>
        </div>
      </template>

      <!-- 基本信息 -->
      <el-descriptions :column="3" border>
        <el-descriptions-item label="客户">
          {{ quotation.customer_name || quotation.customer_id }}
        </el-descriptions-item>
        <el-descriptions-item label="报价日期">{{ quotation.quotation_date }}</el-descriptions-item>
        <el-descriptions-item label="有效期至">{{ quotation.valid_until }}</el-descriptions-item>
        <el-descriptions-item label="价格条款">{{ quotation.price_terms }}</el-descriptions-item>
        <el-descriptions-item label="币种">
          {{ quotation.currency }}（汇率 {{ quotation.exchange_rate }}）
        </el-descriptions-item>
        <el-descriptions-item label="含税">
          {{ quotation.tax_inclusive ? '是' : '否' }}（税率 {{ quotation.tax_rate }}%）
        </el-descriptions-item>
        <el-descriptions-item label="客户等级">
          {{ quotation.customer_level || '-' }}
        </el-descriptions-item>
        <el-descriptions-item label="MOQ">
          {{ quotation.moq ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item label="交期">
          {{ quotation.lead_time_days ?? '-' }} 天
        </el-descriptions-item>
        <el-descriptions-item label="状态" :span="3">
          <el-tag :type="tagType(quotation.status) as any">
            {{ statusLabel(quotation.status) }}
          </el-tag>
          <span v-if="quotation.approved_at" class="approved-info">
            审批时间：{{ quotation.approved_at }} 审批人：{{
              quotation.approved_by_name || quotation.approved_by
            }}
          </span>
        </el-descriptions-item>
        <el-descriptions-item v-if="quotation.rejection_reason" label="拒绝原因" :span="3">
          <span class="rejection-reason">{{ quotation.rejection_reason }}</span>
        </el-descriptions-item>
        <el-descriptions-item
          v-if="quotation.converted_sales_order_id"
          label="已转销售订单"
          :span="3"
        >
          销售订单 ID：{{ quotation.converted_sales_order_id }}，转换时间：{{
            quotation.converted_at
          }}
        </el-descriptions-item>
        <el-descriptions-item v-if="quotation.notes" label="备注" :span="3">
          {{ quotation.notes }}
        </el-descriptions-item>
      </el-descriptions>

      <!-- 报价明细 -->
      <h3 class="section-title">报价明细（{{ quotation.items?.length || 0 }} 项）</h3>
      <el-table :data="quotation.items" border empty-text="无明细">
        <el-table-column type="index" label="#" width="50" align="center" />
        <el-table-column label="产品" min-width="180">
          <template #default="{ row }">
            {{ row.product_name || row.product_code || row.product_id }}
          </template>
        </el-table-column>
        <el-table-column label="色号" width="100">
          <template #default="{ row }">{{ row.color_code || '-' }}</template>
        </el-table-column>
        <el-table-column label="规格" min-width="140">
          <template #default="{ row }">{{ row.specification || '-' }}</template>
        </el-table-column>
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="quantity" label="数量" width="100" align="right" />
        <el-table-column label="单价" width="120" align="right">
          <template #default="{ row }">{{ formatAmount(row.unit_price) }}</template>
        </el-table-column>
        <el-table-column label="含税单价" width="120" align="right">
          <template #default="{ row }">{{ formatAmount(row.unit_price_with_tax) }}</template>
        </el-table-column>
        <el-table-column label="金额" width="140" align="right">
          <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
        </el-table-column>
      </el-table>

      <!-- 贸易条款 -->
      <h3 class="section-title">贸易条款</h3>
      <el-tabs v-if="hasTerms">
        <el-tab-pane
          v-for="(group, type) in groupedTerms"
          :key="type"
          :label="termTypeLabel(type as TermType)"
        >
          <div v-for="(term, idx) in group" :key="term.id || idx" class="term-item">
            <span class="term-index">{{ idx + 1 }}.</span>
            <span>{{ term.term_value }}</span>
          </div>
        </el-tab-pane>
      </el-tabs>
      <el-empty v-else description="暂无贸易条款" :image-size="60" />

      <!-- 金额合计 -->
      <div class="totals">
        <span>小计：{{ quotation.currency }} {{ formatAmount(quotation.subtotal) }}</span>
        <span>税额：{{ quotation.currency }} {{ formatAmount(quotation.tax_amount) }}</span>
        <span class="grand-total">
          合计：{{ quotation.currency }} {{ formatAmount(quotation.total_amount) }}
        </span>
      </div>

      <div class="meta">
        <span>创建：{{ quotation.created_at }}</span>
        <span>更新：{{ quotation.updated_at }}</span>
      </div>
    </el-card>

    <el-empty v-else-if="!loading" description="报价单不存在" />
  </div>
</template>

<script setup lang="ts">
// 报价单详情页脚本
// - 加载报价单
// - 按钮按状态显示
// - 提交/批准/拒绝/转订单/取消
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  getQuotation,
  submitQuotation,
  approveQuotation,
  rejectQuotation,
  cancelQuotation,
  convertQuotation,
  QUOTATION_STATUS_LABELS,
  QUOTATION_STATUS_TAG_TYPES,
  TERM_TYPE_LABELS,
  type QuotationResponseDto,
  type QuotationStatus,
  type TermType,
} from '@/api/quotation'

const route = useRoute()
const router = useRouter()
const loading = ref(false)
const quotation = ref<QuotationResponseDto | null>(null)

/** 加载详情 */
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

/** 按钮可见性（按状态） */
const canEdit = computed(
  () => quotation.value && ['draft', 'rejected'].includes(quotation.value.status)
)
const canSubmit = computed(
  () => quotation.value && ['draft', 'rejected'].includes(quotation.value.status)
)
const canApprove = computed(() => quotation.value?.status === 'pending_approval')
const canConvert = computed(() => quotation.value?.status === 'approved')
const canCancel = computed(
  () =>
    quotation.value &&
    ['draft', 'pending_approval', 'rejected', 'approved'].includes(quotation.value.status)
)

/** 贸易条款按类型分组 */
const groupedTerms = computed(() => {
  if (!quotation.value?.terms) return {} as Record<TermType, any[]>
  const groups: Record<string, any[]> = {}
  for (const t of quotation.value.terms) {
    if (!groups[t.term_type]) groups[t.term_type] = []
    groups[t.term_type].push(t)
  }
  return groups as Record<TermType, any[]>
})

const hasTerms = computed(() => quotation.value?.terms && quotation.value.terms.length > 0)

function statusLabel(s: QuotationStatus): string {
  return QUOTATION_STATUS_LABELS[s] || s
}

function tagType(s: QuotationStatus): string {
  return QUOTATION_STATUS_TAG_TYPES[s] || ''
}

function termTypeLabel(type: TermType): string {
  return TERM_TYPE_LABELS[type] || type
}

function formatAmount(value?: number): string {
  if (value === undefined || value === null) return '0.00'
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

/** 提交审批 */
async function handleSubmit() {
  if (!quotation.value) return
  await submitQuotation(quotation.value.id)
  ElMessage.success('已提交审批')
  loadData()
}

/** 批准 */
async function handleApprove() {
  if (!quotation.value) return
  try {
    await ElMessageBox.confirm('确认批准此报价单？', '批准确认', { type: 'warning' })
  } catch {
    return
  }
  await approveQuotation(quotation.value.id)
  ElMessage.success('已批准')
  loadData()
}

/** 拒绝 */
async function handleReject() {
  if (!quotation.value) return
  let reason = ''
  try {
    const { value } = await ElMessageBox.prompt('请输入拒绝原因', '拒绝', {
      inputValidator: (v: string) => (v && v.trim() ? true : '拒绝原因不能为空'),
      inputErrorMessage: '拒绝原因不能为空',
    })
    reason = value
  } catch {
    return
  }
  await rejectQuotation(quotation.value.id, reason)
  ElMessage.success('已拒绝')
  loadData()
}

/** 转销售订单 */
async function handleConvert() {
  if (!quotation.value) return
  try {
    await ElMessageBox.confirm(
      `确认将报价单 ${quotation.value.quotation_no} 转为销售订单？转订单后报价单状态将变为"已转订单"。`,
      '转订单确认',
      { type: 'warning' }
    )
  } catch {
    return
  }
  const res = await convertQuotation(quotation.value.id)
  const order = res.data as any
  ElMessage.success(`转订单成功，销售订单 ID：${order?.id}`)
  if (order?.id) {
    router.push(`/sales/orders/${order.id}`)
  } else {
    loadData()
  }
}

/** 取消 */
async function handleCancel() {
  if (!quotation.value) return
  try {
    await ElMessageBox.confirm(
      `确认取消报价单 ${quotation.value.quotation_no}？取消后无法恢复。`,
      '取消确认',
      { type: 'warning' }
    )
  } catch {
    return
  }
  await cancelQuotation(quotation.value.id)
  ElMessage.success('已取消')
  loadData()
}

onMounted(loadData)
</script>

<style scoped>
.quotation-detail {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
.quotation-no {
  color: #409eff;
  font-family: monospace;
}
.actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.section-title {
  margin: 24px 0 12px 0;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
  border-left: 3px solid #409eff;
  padding-left: 8px;
}
.term-item {
  padding: 8px 0;
  display: flex;
  gap: 8px;
}
.term-index {
  font-weight: 600;
  color: #909399;
  min-width: 24px;
}
.approved-info {
  margin-left: 12px;
  font-size: 13px;
  color: #909399;
}
.rejection-reason {
  color: #f56c6c;
}
.totals {
  text-align: right;
  margin: 20px 0;
  font-size: 15px;
  display: flex;
  justify-content: flex-end;
  gap: 24px;
}
.totals .grand-total {
  font-weight: bold;
  color: #f56c6c;
  font-size: 18px;
}
.meta {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px dashed #dcdfe6;
  font-size: 13px;
  color: #909399;
  display: flex;
  gap: 24px;
}
</style>
