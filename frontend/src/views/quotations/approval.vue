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
          <span class="title"
            >{{ t('quotations.approval.title') }} - {{ quotation.quotation_no }}</span
          >
          <el-button @click="$router.back()">{{ t('quotations.approval.back') }}</el-button>
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
        <el-descriptions-item :label="t('quotations.approval.labelCustomer')">
          {{ quotation.customer_name || quotation.customer_id }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.approval.labelAmount')">
          {{ quotation.currency }} {{ formatAmount(quotation.total_amount) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.approval.labelPriceTerms')">{{
          quotation.price_terms
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('quotations.approval.labelCurrency')">
          {{ quotation.currency }} ({{ t('quotations.approval.exchangeRateLabel') }}
          {{ quotation.exchange_rate }})
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.approval.labelQuotationDate')" :span="2">
          {{ quotation.quotation_date }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.approval.labelValidUntil')" :span="2">
          {{ quotation.valid_until }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('quotations.approval.labelApprover')" :span="2">
          {{ quotation.approved_by_name || quotation.approved_by || '-' }}
          <span v-if="quotation.approved_at" class="meta-text">
            ({{ quotation.approved_at }})
          </span>
        </el-descriptions-item>
        <el-descriptions-item
          v-if="quotation.rejection_reason"
          :label="t('quotations.approval.labelRejectionReason')"
          :span="2"
        >
          <span class="rejection-reason">{{ quotation.rejection_reason }}</span>
        </el-descriptions-item>
        <el-descriptions-item
          v-if="quotation.converted_sales_order_id"
          :label="t('quotations.approval.labelConvertedOrder')"
          :span="2"
        >
          {{ t('quotations.approval.orderIdPrefix') }}{{ quotation.converted_sales_order_id }}
          <span v-if="quotation.converted_at" class="meta-text">
            ({{ quotation.converted_at }})
          </span>
        </el-descriptions-item>
      </el-descriptions>

      <div class="actions">
        <el-button v-if="canSubmit" type="primary" :loading="submitting" @click="handleSubmit">
          {{ t('quotations.approval.submitApproval') }}
        </el-button>
        <el-button v-if="canApprove" type="success" :loading="submitting" @click="handleApprove">
          {{ t('quotations.approval.approve') }}
        </el-button>
        <el-button v-if="canApprove" type="danger" :loading="submitting" @click="handleReject">
          {{ t('quotations.approval.reject') }}
        </el-button>
        <el-button v-if="canConvert" type="success" :loading="submitting" @click="handleConvert">
          {{ t('quotations.approval.convertOrder') }}
        </el-button>
      </div>
    </el-card>

    <el-empty v-else-if="!loading" :description="t('quotations.approval.notExist')" />
  </div>
</template>

<script setup lang="ts">
// 报价单审批页脚本
import { ref, computed, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getQuotation,
  submitQuotation,
  approveQuotation,
  rejectQuotation,
  convertQuotation,
  type QuotationResponseDto,
} from '@/api/quotation';
import ApprovalProgress from './components/ApprovalProgress.vue';

const { t } = useI18n({ useScope: 'global' });

const route = useRoute();
const router = useRouter();
const loading = ref(false);
const submitting = ref(false);
const quotation = ref<QuotationResponseDto | null>(null);

/** 加载 */
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
      (e instanceof Error ? e.message : String(e)) || t('quotations.approval.loadFailed')
    );
    quotation.value = null;
  } finally {
    loading.value = false;
  }
}

const canSubmit = computed(
  () => quotation.value && ['draft', 'rejected'].includes(quotation.value.status)
);
const canApprove = computed(() => quotation.value?.status === 'pending_approval');
const canConvert = computed(() => quotation.value?.status === 'approved');

async function handleSubmit() {
  if (!quotation.value) return;
  submitting.value = true;
  try {
    await submitQuotation(quotation.value.id);
    ElMessage.success(t('quotations.approval.submitSuccess'));
    loadData();
  } finally {
    submitting.value = false;
  }
}

async function handleApprove() {
  if (!quotation.value) return;
  try {
    await ElMessageBox.confirm(
      t('quotations.approval.approveConfirmText'),
      t('quotations.approval.approveConfirmTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  submitting.value = true;
  try {
    await approveQuotation(quotation.value.id);
    ElMessage.success(t('quotations.approval.approveSuccess'));
    loadData();
  } finally {
    submitting.value = false;
  }
}

async function handleReject() {
  if (!quotation.value) return;
  let reason = '';
  try {
    const { value } = await ElMessageBox.prompt(
      t('quotations.approval.rejectPromptText'),
      t('quotations.approval.rejectTitle'),
      {
        inputValidator: (v: string) =>
          v && v.trim() ? true : t('quotations.approval.rejectReasonRequired'),
      }
    );
    reason = value;
  } catch {
    return;
  }
  submitting.value = true;
  try {
    await rejectQuotation(quotation.value.id, reason);
    ElMessage.success(t('quotations.approval.rejectSuccess'));
    loadData();
  } finally {
    submitting.value = false;
  }
}

async function handleConvert() {
  if (!quotation.value) return;
  try {
    await ElMessageBox.confirm(
      t('quotations.approval.convertConfirmText'),
      t('quotations.approval.convertTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  submitting.value = true;
  try {
    const res = await convertQuotation(quotation.value.id);
    // convertQuotation 返回 ApiResponse<ConvertResponse>，res.data 即 ConvertResponse
    const order = res.data;
    ElMessage.success(t('quotations.approval.convertSuccess', { id: order?.id }));
    if (order?.id) {
      router.push(`/sales/orders/${order.id}`);
    } else {
      loadData();
    }
  } finally {
    submitting.value = false;
  }
}

function formatAmount(value?: number): string {
  if (value === undefined || value === null) return '0.00';
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

onMounted(loadData);
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
