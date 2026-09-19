<!--
  PaymentRequestTab.vue - 应付付款申请管理
  覆盖付款申请全生命周期：创建/编辑/删除/提交/审批/驳回 + 详情查看
  数据源：/ap/payment-requests（api/ap.ts 封装）
-->
<template>
  <div class="payment-request-tab">
    <div class="tab-toolbar">
      <el-button type="primary" :icon="Plus" @click="openCreateDialog">
        {{ t('apModule.paymentRequest.create') }}
      </el-button>
      <el-button @click="fetchRequests">{{ t('common.refresh') || '刷新' }}</el-button>
    </div>

    <el-table v-loading="loading" :data="requests" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column
        prop="request_no"
        :label="t('apModule.paymentRequest.requestNo')"
        min-width="150"
      />
      <el-table-column prop="supplier_id" :label="t('apModule.payment.supplier')" width="90" />
      <el-table-column
        prop="request_date"
        :label="t('apModule.paymentRequest.requestDate')"
        width="110"
      />
      <el-table-column
        prop="payment_type"
        :label="t('apModule.paymentRequest.paymentType')"
        width="100"
      />
      <el-table-column prop="payment_method" :label="t('apModule.payment.method')" width="110" />
      <el-table-column
        prop="request_amount"
        :label="t('apModule.paymentRequest.requestAmount')"
        width="130"
        align="right"
      >
        <template #default="{ row }">{{ formatMoney(row.request_amount) }}</template>
      </el-table-column>
      <el-table-column prop="currency" label="币种" width="70" />
      <el-table-column prop="status" :label="t('common.status')" width="100">
        <template #default="{ row }">
          <el-tag :type="statusTagType(row.status)" size="small">{{ row.status }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.action')" width="280" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'draft' || row.status === 'rejected'"
            size="small"
            type="primary"
            link
            @click="openEditDialog(row)"
          >
            {{ t('common.edit') }}
          </el-button>
          <el-button
            v-if="row.status === 'draft' || row.status === 'rejected'"
            size="small"
            type="warning"
            link
            @click="submitRequest(row)"
          >
            {{ t('apModule.paymentRequest.submit') }}
          </el-button>
          <el-button
            v-if="row.status === 'pending_approval'"
            size="small"
            type="success"
            link
            @click="approveRequest(row)"
          >
            {{ t('apModule.paymentRequest.approve') }}
          </el-button>
          <el-button
            v-if="row.status === 'pending_approval'"
            size="small"
            type="danger"
            link
            @click="rejectRequest(row)"
          >
            {{ t('apModule.paymentRequest.reject') }}
          </el-button>
          <el-button
            v-if="row.status === 'draft' || row.status === 'rejected'"
            size="small"
            type="danger"
            link
            @click="removeRequest(row)"
          >
            {{ t('common.delete') }}
          </el-button>
          <el-button size="small" link @click="showDetail(row)">
            {{ t('common.detail') || '详情' }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="editId ? t('apModule.paymentRequest.edit') : t('apModule.paymentRequest.create')"
      width="560px"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="110px">
        <el-form-item :label="t('apModule.payment.supplier')" prop="supplier_id">
          <el-input-number v-model="form.supplier_id" :min="1" />
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.requestDate')" prop="request_date">
          <el-date-picker v-model="form.request_date" type="date" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.paymentType')" prop="payment_type">
          <el-select v-model="form.payment_type">
            <el-option label="采购付款" value="purchase" />
            <el-option label="费用付款" value="expense" />
            <el-option label="预付款" value="prepayment" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('apModule.payment.method')" prop="payment_method">
          <el-select v-model="form.payment_method">
            <el-option label="银行转账" value="bank_transfer" />
            <el-option label="现金" value="cash" />
            <el-option label="支票" value="check" />
            <el-option label="承兑" value="bill" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.requestAmount')" prop="request_amount">
          <el-input-number v-model="form.request_amount" :min="0.01" :precision="2" />
        </el-form-item>
        <el-form-item label="币种" prop="currency">
          <el-select v-model="form.currency" style="width: 120px">
            <el-option label="CNY" value="CNY" />
            <el-option label="USD" value="USD" />
            <el-option label="EUR" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.bankName')">
          <el-input v-model="form.bank_name" />
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.notes')">
          <el-input v-model="form.notes" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmit">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 详情对话框 -->
    <el-dialog v-model="detailVisible" :title="t('common.detail') || '付款申请详情'" width="620px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="ID">{{ detailRow.id }}</el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.requestNo')">
          {{ detailRow.request_no }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('apModule.payment.supplier')">
          {{ detailRow.supplier_id }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('common.status')">{{
          detailRow.status
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.requestAmount')">
          {{ formatMoney(detailRow.request_amount) }} {{ detailRow.currency }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.requestDate')">
          {{ detailRow.request_date }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.bankName')">
          {{ detailRow.bank_name || '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.notes')">
          {{ detailRow.notes || '-' }}
        </el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getAPPaymentRequestList,
  getAPPaymentRequest,
  createAPPaymentRequest,
  updateAPPaymentRequest,
  deleteAPPaymentRequest,
  submitAPPaymentRequest,
  approveAPPaymentRequest,
  rejectAPPaymentRequest,
  type APPaymentRequest,
} from '@/api/ap';

const { t } = useI18n({ useScope: 'global' });

const requests = ref<APPaymentRequest[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const detailVisible = ref(false);
const editId = ref<number | null>(null);
const detailRow = ref<APPaymentRequest | null>(null);
const formRef = ref<FormInstance>();
const submitting = ref(false);

const formatMoney = (amount: number | string | undefined) => {
  const n = Number(amount ?? 0);
  return n.toLocaleString('zh-CN', { minimumFractionDigits: 2 });
};

const statusTagType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending_approval: 'warning',
    approved: 'success',
    rejected: 'danger',
    paid: 'success',
    cancelled: 'info',
  };
  return map[status] || 'info';
};

const fetchRequests = async () => {
  loading.value = true;
  try {
    const res = await getAPPaymentRequestList();
    const d = res.data as unknown as
      { list?: APPaymentRequest[]; items?: APPaymentRequest[] } | APPaymentRequest[] | undefined;
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      requests.value = d.list || d.items || [];
    } else {
      requests.value = (d as APPaymentRequest[]) || [];
    }
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    loading.value = false;
  }
};

const form = reactive({
  supplier_id: undefined as number | undefined,
  request_date: new Date().toISOString().split('T')[0],
  payment_type: 'purchase',
  payment_method: 'bank_transfer',
  request_amount: 0,
  currency: 'CNY',
  bank_name: '',
  notes: '',
});

const rules: FormRules = {
  supplier_id: [
    { required: true, message: t('apModule.payment.supplierRequired'), trigger: 'change' },
  ],
  request_date: [
    { required: true, message: t('apModule.payment.dateRequired'), trigger: 'change' },
  ],
  request_amount: [
    { required: true, message: t('apModule.payment.amountRequired'), trigger: 'blur' },
  ],
};

const resetForm = () => {
  form.supplier_id = undefined;
  form.request_date = new Date().toISOString().split('T')[0];
  form.payment_type = 'purchase';
  form.payment_method = 'bank_transfer';
  form.request_amount = 0;
  form.currency = 'CNY';
  form.bank_name = '';
  form.notes = '';
};

const openCreateDialog = () => {
  editId.value = null;
  resetForm();
  dialogVisible.value = true;
};

const openEditDialog = (row: APPaymentRequest) => {
  editId.value = row.id;
  form.supplier_id = row.supplier_id;
  form.request_date = row.request_date;
  form.payment_type = row.payment_type || '';
  form.payment_method = row.payment_method || '';
  form.request_amount = Number(row.request_amount ?? 0);
  form.currency = row.currency || 'CNY';
  form.bank_name = row.bank_name || '';
  form.notes = row.notes || '';
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  const valid = await formRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    if (editId.value) {
      await updateAPPaymentRequest(editId.value, form);
    } else {
      await createAPPaymentRequest(form);
    }
    ElMessage.success(t('common.success'));
    dialogVisible.value = false;
    fetchRequests();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    submitting.value = false;
  }
};

const submitRequest = async (row: APPaymentRequest) => {
  try {
    await ElMessageBox.confirm(t('apModule.paymentRequest.submitConfirm'), t('common.confirm'), {
      type: 'info',
    });
    await submitAPPaymentRequest(row.id);
    ElMessage.success(t('common.success'));
    fetchRequests();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const approveRequest = async (row: APPaymentRequest) => {
  try {
    await ElMessageBox.confirm(t('apModule.paymentRequest.approveConfirm'), t('common.confirm'), {
      type: 'warning',
    });
    await approveAPPaymentRequest(row.id);
    ElMessage.success(t('common.success'));
    fetchRequests();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const rejectRequest = async (row: APPaymentRequest) => {
  try {
    const { value } = await ElMessageBox.prompt(
      t('apModule.paymentRequest.rejectReason'),
      t('apModule.paymentRequest.reject'),
      {
        type: 'warning',
        inputPattern: /\S+/,
        inputErrorMessage: t('apModule.paymentRequest.rejectReasonRequired'),
      }
    );
    await rejectAPPaymentRequest(row.id, value);
    ElMessage.success(t('common.success'));
    fetchRequests();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const removeRequest = async (row: APPaymentRequest) => {
  try {
    await ElMessageBox.confirm(t('apModule.paymentRequest.deleteConfirm'), t('common.delete'), {
      type: 'warning',
    });
    await deleteAPPaymentRequest(row.id);
    ElMessage.success(t('common.success'));
    fetchRequests();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const showDetail = async (row: APPaymentRequest) => {
  try {
    const res = await getAPPaymentRequest(row.id);
    detailRow.value = (res.data as APPaymentRequest) || row;
  } catch {
    detailRow.value = row;
  }
  detailVisible.value = true;
};

defineExpose({ refresh: fetchRequests });

onMounted(() => {
  fetchRequests();
});
</script>

<style scoped>
.tab-toolbar {
  margin-bottom: 12px;
  display: flex;
  gap: 8px;
}
</style>
