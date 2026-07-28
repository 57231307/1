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
            {{ t('quotations.detail.title') }} -
            <span class="quotation-no">{{ quotation.quotation_no }}</span>
          </span>
          <div class="actions">
            <el-button @click="$router.back()">{{ t('quotations.detail.back') }}</el-button>
            <el-button v-if="canEdit" @click="$router.push(`/quotations/${quotation.id}/edit`)">
              {{ t('quotations.detail.edit') }}
            </el-button>
            <el-button v-if="canSubmit" type="primary" @click="handleSubmit">
              {{ t('quotations.detail.submitApproval') }}
            </el-button>
            <el-button v-if="canApprove" type="success" @click="handleApprove">
              {{ t('quotations.detail.approve') }}
            </el-button>
            <el-button v-if="canApprove" type="danger" @click="handleReject">
              {{ t('quotations.detail.reject') }}
            </el-button>
            <el-button v-if="canConvert" type="success" @click="handleConvert">
              {{ t('quotations.detail.convertOrder') }}
            </el-button>
            <el-button v-if="canCancel" type="danger" plain @click="handleCancel">
              {{ t('quotations.detail.cancel') }}
            </el-button>
          </div>
        </div>
      </template>

      <!-- 基本信息 -->
      <el-descriptions :column="3" border>
        <el-descriptions-item :label="t('quotations.detail.labelCustomer')">
          {{ quotation.customer_name || quotation.customer_id }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelQuotationDate')">{{
          quotation.quotation_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelValidUntil')">{{
          quotation.valid_until
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelPriceTerms')">{{
          quotation.price_terms
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelCurrency')">
          {{ quotation.currency }} ({{ t('quotations.detail.exchangeRateLabel') }}
          {{ quotation.exchange_rate }})
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelTaxInclusive')">
          {{ quotation.tax_inclusive ? t('quotations.detail.yes') : t('quotations.detail.no') }} ({{
            t('quotations.detail.taxRateLabel')
          }}
          {{ quotation.tax_rate }}%)
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelCustomerLevel')">
          {{ quotation.customer_level || '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelMoq')">
          {{ quotation.moq ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelLeadTime')">
          {{ quotation.lead_time_days ?? '-' }} {{ t('quotations.detail.leadTimeUnit') }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.detail.labelStatus')" :span="3">
          <el-tag :type="tagType(quotation.status)">
            {{ statusLabel(quotation.status) }}
          </el-tag>
          <span v-if="quotation.approved_at" class="approved-info">
            {{ t('quotations.detail.approvalTimeLabel') }}{{ quotation.approved_at }}
            {{ t('quotations.detail.approverLabel')
            }}{{ quotation.approved_by_name || quotation.approved_by }}
          </span>
        </el-descriptions-item>
        <el-descriptions-item
          v-if="quotation.rejection_reason"
          :label="t('quotations.detail.labelRejectionReason')"
          :span="3"
        >
          <span class="rejection-reason">{{ quotation.rejection_reason }}</span>
        </el-descriptions-item>
        <el-descriptions-item
          v-if="quotation.converted_sales_order_id"
          :label="t('quotations.detail.labelConvertedOrder')"
          :span="3"
        >
          {{ t('quotations.detail.salesOrderIdPrefix') }}{{ quotation.converted_sales_order_id
          }}{{ t('quotations.detail.convertedTimeLabel') }}{{ quotation.converted_at }}
        </el-descriptions-item>
        <el-descriptions-item
          v-if="quotation.notes"
          :label="t('quotations.detail.labelRemark')"
          :span="3"
        >
          {{ quotation.notes }}
        </el-descriptions-item>
      </el-descriptions>

      <!-- 报价明细 -->
      <h3 class="section-title">
        {{ t('quotations.detail.itemsTitle', { count: quotation.items?.length || 0 }) }}
      </h3>
      <el-table
        :data="quotation.items"
        border
        :empty-text="t('quotations.detail.emptyItems')"
        :aria-label="t('quotations.detail.itemsTableAriaLabel')"
      >
        <el-table-column type="index" label="#" width="50" align="center" />
        <el-table-column :label="t('quotations.detail.colProduct')" min-width="180">
          <template #default="{ row }">
            {{ row.product_name || row.product_code || row.product_id }}
          </template>
        </el-table-column>
        <el-table-column :label="t('quotations.detail.colColor')" width="100">
          <template #default="{ row }">{{ row.color_code || '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('quotations.detail.colSpec')" min-width="140">
          <template #default="{ row }">{{ row.specification || '-' }}</template>
        </el-table-column>
        <el-table-column prop="unit" :label="t('quotations.detail.colUnit')" width="80" />
        <el-table-column
          prop="quantity"
          :label="t('quotations.detail.colQuantity')"
          width="100"
          align="right"
        />
        <el-table-column :label="t('quotations.detail.colUnitPrice')" width="120" align="right">
          <template #default="{ row }">{{ formatAmount(row.unit_price) }}</template>
        </el-table-column>
        <el-table-column
          :label="t('quotations.detail.colUnitPriceWithTax')"
          width="120"
          align="right"
        >
          <template #default="{ row }">{{ formatAmount(row.unit_price_with_tax) }}</template>
        </el-table-column>
        <el-table-column :label="t('quotations.detail.colAmount')" width="140" align="right">
          <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
        </el-table-column>
      </el-table>

      <!-- 贸易条款 -->
      <h3 class="section-title">{{ t('quotations.detail.sectionTerms') }}</h3>
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
      <el-empty v-else :description="t('quotations.detail.emptyTerms')" :image-size="60" />

      <!-- 金额合计 -->
      <div class="totals">
        <span
          >{{ t('quotations.detail.subtotal') }}{{ quotation.currency }}
          {{ formatAmount(quotation.subtotal) }}</span
        >
        <span
          >{{ t('quotations.detail.taxAmount') }}{{ quotation.currency }}
          {{ formatAmount(quotation.tax_amount) }}</span
        >
        <span class="grand-total">
          {{ t('quotations.detail.total') }}{{ quotation.currency }}
          {{ formatAmount(quotation.total_amount) }}
        </span>
      </div>

      <div class="meta">
        <span>{{ t('quotations.detail.createdAt') }}{{ quotation.created_at }}</span>
        <span>{{ t('quotations.detail.updatedAt') }}{{ quotation.updated_at }}</span>
      </div>
    </el-card>

    <el-empty v-else-if="!loading" :description="t('quotations.detail.notExist')" />
  </div>
</template>

<script setup lang="ts">
// 报价单详情页脚本
// - 加载报价单
// - 按钮按状态显示
// - 提交/批准/拒绝/转订单/取消
import { ref, computed, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
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
  type QuotationTermResponseDto,
  type ConvertResponse,
} from '@/api/quotation';

/** el-tag 类型联合（与 element-plus TagProps.type 对齐） */
type TagType = '' | 'success' | 'warning' | 'info' | 'danger';

const { t } = useI18n({ useScope: 'global' });

const route = useRoute();
const router = useRouter();
const loading = ref(false);
const quotation = ref<QuotationResponseDto | null>(null);

/** 加载详情 */
async function loadData() {
  const id = Number(route.params.id);
  if (!id) return;
  loading.value = true;
  try {
    const res = await getQuotation(id);
    quotation.value = res.data as QuotationResponseDto;
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.detail.loadFailed')
    );
    quotation.value = null;
  } finally {
    loading.value = false;
  }
}

/** 按钮可见性（按状态） */
const canEdit = computed(
  () => quotation.value && ['draft', 'rejected'].includes(quotation.value.status)
);
const canSubmit = computed(
  () => quotation.value && ['draft', 'rejected'].includes(quotation.value.status)
);
const canApprove = computed(() => quotation.value?.status === 'pending_approval');
const canConvert = computed(() => quotation.value?.status === 'approved');
const canCancel = computed(
  () =>
    quotation.value &&
    ['draft', 'pending_approval', 'rejected', 'approved'].includes(quotation.value.status)
);

/** 贸易条款按类型分组 */
const groupedTerms = computed(() => {
  if (!quotation.value?.terms) return {} as Record<TermType, QuotationTermResponseDto[]>;
  const groups: Record<string, QuotationTermResponseDto[]> = {};
  for (const t of quotation.value.terms) {
    if (!groups[t.term_type]) groups[t.term_type] = [];
    groups[t.term_type].push(t);
  }
  return groups as Record<TermType, QuotationTermResponseDto[]>;
});

const hasTerms = computed(() => quotation.value?.terms && quotation.value.terms.length > 0);

function statusLabel(s: QuotationStatus): string {
  return QUOTATION_STATUS_LABELS[s] || s;
}

function tagType(s: QuotationStatus): TagType {
  return (QUOTATION_STATUS_TAG_TYPES[s] || '') as TagType;
}

function termTypeLabel(type: TermType): string {
  return TERM_TYPE_LABELS[type] || type;
}

function formatAmount(value?: number): string {
  if (value === undefined || value === null) return '0.00';
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

/** 提交审批 */
async function handleSubmit() {
  if (!quotation.value) return;
  await submitQuotation(quotation.value.id);
  ElMessage.success(t('quotations.detail.submitSuccess'));
  loadData();
}

/** 批准 */
async function handleApprove() {
  if (!quotation.value) return;
  try {
    await ElMessageBox.confirm(
      t('quotations.detail.approveConfirmText'),
      t('quotations.detail.approveConfirmTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  await approveQuotation(quotation.value.id);
  ElMessage.success(t('quotations.detail.approveSuccess'));
  loadData();
}

/** 拒绝 */
async function handleReject() {
  if (!quotation.value) return;
  let reason = '';
  try {
    const { value } = await ElMessageBox.prompt(
      t('quotations.detail.rejectPromptText'),
      t('quotations.detail.rejectTitle'),
      {
        inputValidator: (v: string) =>
          v && v.trim() ? true : t('quotations.detail.rejectReasonRequired'),
        inputErrorMessage: t('quotations.detail.rejectReasonRequired'),
      }
    );
    reason = value;
  } catch {
    return;
  }
  await rejectQuotation(quotation.value.id, reason);
  ElMessage.success(t('quotations.detail.rejectSuccess'));
  loadData();
}

/** 转销售订单 */
async function handleConvert() {
  if (!quotation.value) return;
  try {
    await ElMessageBox.confirm(
      t('quotations.detail.convertConfirmText', { no: quotation.value.quotation_no }),
      t('quotations.detail.convertConfirmTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  const res = await convertQuotation(quotation.value.id);
  const order: ConvertResponse | undefined = res.data;
  ElMessage.success(t('quotations.detail.convertSuccess', { id: order?.id }));
  if (order?.id) {
    router.push(`/sales/orders/${order.id}`);
  } else {
    loadData();
  }
}

/** 取消 */
async function handleCancel() {
  if (!quotation.value) return;
  try {
    await ElMessageBox.confirm(
      t('quotations.detail.cancelConfirmText', { no: quotation.value.quotation_no }),
      t('quotations.detail.cancelConfirmTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  await cancelQuotation(quotation.value.id);
  ElMessage.success(t('quotations.detail.cancelSuccess'));
  loadData();
}

onMounted(loadData);
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
