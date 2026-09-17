<!--
  PaymentTab.vue - 应收收款管理
  收款列表 + 新建/编辑 + 确认收款 + 详情查看
  数据源：/ar/payments（api/ar.ts 封装）
-->
<template>
  <div class="ar-payment-tab">
    <div class="tab-toolbar">
      <el-button type="primary" :icon="Plus" @click="openCreateDialog">
        {{ t('arModule.payment.create') }}
      </el-button>
      <el-button @click="fetchPayments">{{ t('common.refresh') || '刷新' }}</el-button>
    </div>

    <el-table v-loading="loading" :data="payments" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="payment_no" :label="t('arModule.payment.paymentNo')" min-width="150" />
      <el-table-column prop="customer_id" :label="t('arModule.payment.customer')" width="90" />
      <el-table-column prop="payment_date" :label="t('arModule.payment.paymentDate')" width="110" />
      <el-table-column prop="payment_method" :label="t('arModule.payment.method')" width="110" />
      <el-table-column
        prop="payment_amount"
        :label="t('arModule.payment.amount')"
        width="130"
        align="right"
      >
        <template #default="{ row }">{{ formatMoney(row.payment_amount) }}</template>
      </el-table-column>
      <el-table-column prop="status" :label="t('common.status')" width="100">
        <template #default="{ row }">
          <el-tag :type="statusTagType(row.status)" size="small">{{ row.status }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.action')" width="220" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'draft' || row.status === 'pending'"
            size="small"
            type="primary"
            link
            @click="openEditDialog(row)"
          >
            {{ t('common.edit') }}
          </el-button>
          <el-button
            v-if="row.status === 'draft' || row.status === 'pending' || row.status === 'confirmed'"
            size="small"
            type="success"
            link
            @click="confirmPayment(row)"
          >
            {{ t('arModule.payment.confirm') }}
          </el-button>
          <el-button size="small" link @click="showDetail(row)">
            {{ t('common.detail') || '详情' }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog
      v-model="dialogVisible"
      :title="editId ? t('arModule.payment.edit') : t('arModule.payment.create')"
      width="540px"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="110px">
        <el-form-item :label="t('arModule.payment.customer')" prop="customer_id">
          <el-input-number v-model="form.customer_id" :min="1" />
        </el-form-item>
        <el-form-item :label="t('arModule.payment.paymentDate')" prop="payment_date">
          <el-date-picker v-model="form.payment_date" type="date" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item :label="t('arModule.payment.method')" prop="payment_method">
          <el-select v-model="form.payment_method">
            <el-option label="银行转账" value="bank_transfer" />
            <el-option label="现金" value="cash" />
            <el-option label="支票" value="check" />
            <el-option label="承兑" value="bill" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('arModule.payment.amount')" prop="payment_amount">
          <el-input-number v-model="form.payment_amount" :min="0.01" :precision="2" />
        </el-form-item>
        <el-form-item :label="t('arModule.payment.notes')">
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

    <el-dialog v-model="detailVisible" :title="t('common.detail') || '收款详情'" width="600px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="ID">{{ detailRow.id }}</el-descriptions-item>
        <el-descriptions-item :label="t('arModule.payment.paymentNo')">
          {{ detailRow.payment_no }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('arModule.payment.customer')">
          {{ detailRow.customer_id }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('common.status')">{{
          detailRow.status
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('arModule.payment.amount')">
          {{ formatMoney(detailRow.payment_amount) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('arModule.payment.paymentDate')">
          {{ detailRow.payment_date }}
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
  getARPaymentList,
  getARPayment,
  createARPayment,
  updateARPayment,
  confirmARPayment,
  type ARPayment,
} from '@/api/ar';

const { t } = useI18n({ useScope: 'global' });

const payments = ref<ARPayment[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const detailVisible = ref(false);
const editId = ref<number | null>(null);
const detailRow = ref<ARPayment | null>(null);
const formRef = ref<FormInstance>();
const submitting = ref(false);

const formatMoney = (amount: number | string | undefined) => {
  const n = Number(amount ?? 0);
  return n.toLocaleString('zh-CN', { minimumFractionDigits: 2 });
};

const statusTagType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    confirmed: 'success',
    completed: 'success',
    cancelled: 'info',
  };
  return map[status] || 'info';
};

const fetchPayments = async () => {
  loading.value = true;
  try {
    const res = await getARPaymentList();
    const d = res.data as unknown as
      { list?: ARPayment[]; items?: ARPayment[] } | ARPayment[] | undefined;
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      payments.value = d.list || d.items || [];
    } else {
      payments.value = (d as ARPayment[]) || [];
    }
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    loading.value = false;
  }
};

const form = reactive({
  customer_id: undefined as number | undefined,
  payment_date: new Date().toISOString().split('T')[0],
  payment_method: 'bank_transfer',
  payment_amount: 0,
  notes: '',
});

const rules: FormRules = {
  customer_id: [
    { required: true, message: t('arModule.payment.customerRequired'), trigger: 'change' },
  ],
  payment_date: [
    { required: true, message: t('arModule.payment.dateRequired'), trigger: 'change' },
  ],
  payment_amount: [
    { required: true, message: t('arModule.payment.amountRequired'), trigger: 'blur' },
  ],
};

const openCreateDialog = () => {
  editId.value = null;
  form.customer_id = undefined;
  form.payment_date = new Date().toISOString().split('T')[0];
  form.payment_method = 'bank_transfer';
  form.payment_amount = 0;
  form.notes = '';
  dialogVisible.value = true;
};

const openEditDialog = (row: ARPayment) => {
  editId.value = row.id;
  form.customer_id = row.customer_id;
  form.payment_date = row.payment_date;
  form.payment_method = row.payment_method;
  form.payment_amount = Number(row.payment_amount ?? 0);
  form.notes = row.notes || '';
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  const valid = await formRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    if (editId.value) {
      await updateARPayment(editId.value, form);
    } else {
      await createARPayment(form);
    }
    ElMessage.success(t('common.success'));
    dialogVisible.value = false;
    fetchPayments();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    submitting.value = false;
  }
};

const confirmPayment = async (row: ARPayment) => {
  try {
    await ElMessageBox.confirm(
      t('arModule.payment.confirmMessage'),
      t('arModule.payment.confirm'),
      { type: 'warning' }
    );
    await confirmARPayment(row.id);
    ElMessage.success(t('common.success'));
    fetchPayments();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const showDetail = async (row: ARPayment) => {
  try {
    const res = await getARPayment(row.id);
    detailRow.value = (res.data as ARPayment) || row;
  } catch {
    detailRow.value = row;
  }
  detailVisible.value = true;
};

defineExpose({ refresh: fetchPayments });

onMounted(() => {
  fetchPayments();
});
</script>

<style scoped>
.tab-toolbar {
  margin-bottom: 12px;
  display: flex;
  gap: 8px;
}
</style>
