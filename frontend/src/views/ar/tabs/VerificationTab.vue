<!--
  VerificationTab.vue - 应收核销管理
  核销列表 + 自动核销 + 手动核销 + 详情 + 取消核销
  数据源：/ar/verifications（api/ar.ts 封装）
-->
<template>
  <div class="ar-verification-tab">
    <div class="tab-toolbar">
      <el-button type="primary" @click="openManualDialog">
        {{ t('arModule.verification.manual') }}
      </el-button>
      <el-button :loading="autoVerifying" @click="handleAutoVerify">
        {{ t('arModule.verification.auto') }}
      </el-button>
      <el-button @click="fetchVerifications">{{ t('common.refresh') }}</el-button>
    </div>

    <el-table v-loading="loading" :data="verifications" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column
        prop="invoice_no"
        :label="t('arModule.verification.invoiceNo')"
        min-width="150"
      />
      <el-table-column
        prop="payment_no"
        :label="t('arModule.verification.paymentNo')"
        min-width="150"
      />
      <el-table-column
        prop="verification_date"
        :label="t('arModule.verification.date')"
        width="110"
      />
      <el-table-column
        prop="verification_amount"
        :label="t('arModule.verification.amount')"
        width="130"
        align="right"
      >
        <template #default="{ row }">{{ formatMoney(row.verification_amount) }}</template>
      </el-table-column>
      <el-table-column prop="status" :label="t('common.status')" width="100">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
            {{ row.status }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.action')" width="160" fixed="right">
        <template #default="{ row }">
          <el-button size="small" link @click="showDetail(row)">
            {{ t('common.detail') }}
          </el-button>
          <el-button
            v-if="row.status === 'active'"
            size="small"
            type="danger"
            link
            @click="cancelVerification(row)"
          >
            {{ t('arModule.verification.cancel') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 手动核销对话框 -->
    <el-dialog v-model="manualVisible" :title="t('arModule.verification.manual')" width="560px">
      <el-form ref="manualFormRef" :model="manualForm" :rules="manualRules" label-width="110px">
        <el-form-item :label="t('arModule.verification.invoice')" prop="invoice_id">
          <el-select v-model="manualForm.invoice_id" filterable>
            <el-option
              v-for="inv in unverifiedInvoices"
              :key="inv.id"
              :label="`${inv.invoice_no}（未核销 ${formatMoney(inv.unpaid_amount)}）`"
              :value="inv.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('arModule.verification.payment')" prop="payment_id">
          <el-select v-model="manualForm.payment_id" filterable>
            <el-option
              v-for="pay in unverifiedPayments"
              :key="pay.id"
              :label="`${pay.payment_no}（${formatMoney(pay.amount)}）`"
              :value="pay.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('arModule.verification.amount')" prop="verification_amount">
          <el-input-number v-model="manualForm.verification_amount" :min="0.01" :precision="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="manualVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="submitManual">
          {{ t('common.confirm') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 详情对话框 -->
    <el-dialog
      v-model="detailVisible"
      :title="t('arModule.verification.detailTitle')"
      width="600px"
    >
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="ID">{{ detailRow.id }}</el-descriptions-item>
        <el-descriptions-item :label="t('common.status')">{{
          detailRow.status
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('arModule.verification.invoiceNo')">
          {{ detailRow.invoice_no }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('arModule.verification.paymentNo')">
          {{ detailRow.payment_no }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('arModule.verification.amount')">
          {{ formatMoney(detailRow.verification_amount) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('arModule.verification.date')">
          {{ detailRow.verification_date }}
        </el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getARVerificationList,
  getARVerification,
  autoVerifyAR,
  manualVerifyAR,
  cancelARVerification,
  getUnverifiedARInvoices,
  getUnverifiedARPayments,
  type ARVerification,
  type ARInvoice,
  type ARPayment,
} from '@/api/ar';

const { t } = useI18n({ useScope: 'global' });

const verifications = ref<ARVerification[]>([]);
const loading = ref(false);
const autoVerifying = ref(false);
const manualVisible = ref(false);
const detailVisible = ref(false);
const detailRow = ref<ARVerification | null>(null);
const submitting = ref(false);
const manualFormRef = ref<FormInstance>();
const unverifiedInvoices = ref<ARInvoice[]>([]);
const unverifiedPayments = ref<ARPayment[]>([]);

const formatMoney = (amount: number | string | undefined) => {
  const n = Number(amount ?? 0);
  return n.toLocaleString('zh-CN', { minimumFractionDigits: 2 });
};

const fetchVerifications = async () => {
  loading.value = true;
  try {
    const res = await getARVerificationList();
    // 后端 ar_verification_handler::list_verifications 载荷承载数组的键为 `list`
    verifications.value = res.data.list;
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    loading.value = false;
  }
};

const handleAutoVerify = async () => {
  try {
    await ElMessageBox.confirm(
      t('arModule.verification.autoConfirm'),
      t('arModule.verification.auto'),
      { type: 'info' }
    );
  } catch {
    return;
  }
  autoVerifying.value = true;
  try {
    await autoVerifyAR();
    ElMessage.success(t('arModule.verification.autoSuccess'));
    fetchVerifications();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    autoVerifying.value = false;
  }
};

const manualForm = reactive({
  invoice_id: undefined as number | undefined,
  payment_id: undefined as number | undefined,
  verification_amount: 0,
});

const manualRules: FormRules = {
  invoice_id: [
    { required: true, message: t('arModule.verification.invoiceRequired'), trigger: 'change' },
  ],
  payment_id: [
    { required: true, message: t('arModule.verification.paymentRequired'), trigger: 'change' },
  ],
  verification_amount: [
    { required: true, message: t('arModule.verification.amountRequired'), trigger: 'blur' },
  ],
};

const openManualDialog = async () => {
  manualForm.invoice_id = undefined;
  manualForm.payment_id = undefined;
  manualForm.verification_amount = 0;
  manualVisible.value = true;
  try {
    const [invRes, payRes] = await Promise.all([
      getUnverifiedARInvoices(),
      getUnverifiedARPayments(),
    ]);
    // 后端 get_unverified_invoices/payments 直接返回裸数组
    unverifiedInvoices.value = invRes.data;
    unverifiedPayments.value = payRes.data;
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

const submitManual = async () => {
  const valid = await manualFormRef.value?.validate();
  if (!valid || !manualForm.invoice_id || !manualForm.payment_id) return;
  submitting.value = true;
  try {
    await manualVerifyAR({
      invoice_id: manualForm.invoice_id,
      payment_id: manualForm.payment_id,
      amount: manualForm.verification_amount,
    });
    ElMessage.success(t('common.success'));
    manualVisible.value = false;
    fetchVerifications();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    submitting.value = false;
  }
};

const showDetail = async (row: ARVerification) => {
  try {
    const res = await getARVerification(row.id);
    detailRow.value = (res.data as ARVerification) || row;
  } catch (error) {
    logger.error(t('arModule.verification.detailFailed'), error);
    detailRow.value = row;
  }
  detailVisible.value = true;
};

const cancelVerification = async (row: ARVerification) => {
  try {
    await ElMessageBox.confirm(
      t('arModule.verification.cancelConfirm'),
      t('arModule.verification.cancel'),
      { type: 'warning' }
    );
    await cancelARVerification(row.id);
    ElMessage.success(t('common.success'));
    fetchVerifications();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

defineExpose({ refresh: fetchVerifications });

onMounted(() => {
  fetchVerifications();
});
</script>

<style scoped>
.tab-toolbar {
  margin-bottom: 12px;
  display: flex;
  gap: 8px;
}
</style>
