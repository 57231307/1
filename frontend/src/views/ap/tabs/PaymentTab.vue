<!--
  PaymentTab.vue - 付款管理 Tab
  来源：原 ap/index.vue 中 付款管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="payment-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('apModule.payment.title') }}</h2>
      <el-button type="primary" @click="openPaymentDialog()">
        <el-icon><Plus /></el-icon> {{ $t('apModule.payment.create') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="paymentLoading"
        :data="payments"
        stripe
        :aria-label="$t('apModule.payment.listAria')"
      >
        <el-table-column prop="payment_no" :label="$t('apModule.payment.paymentNo')" width="140" />
        <el-table-column :label="$t('apModule.payment.supplier')" width="150">
          <template #default="{ row }">{{ supplierLabel(row.supplier_id) }}</template>
        </el-table-column>
        <el-table-column
          prop="payment_date"
          :label="$t('apModule.payment.paymentDate')"
          width="120"
        />
        <el-table-column :label="$t('apModule.payment.paymentAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.payment_amount) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="payment_method"
          :label="$t('apModule.payment.paymentMethod')"
          width="100"
        >
          <template #default="{ row }">
            {{ getPaymentMethodLabel(row.payment_method) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="payment_status"
          :label="$t('common.status')"
          width="90"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.payment_status === 'CONFIRMED' ? 'success' : 'warning'" size="small">
              {{
                row.payment_status === 'CONFIRMED'
                  ? $t('apModule.payment.statusConfirmed')
                  : $t('apModule.payment.statusRegistered')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="bank_account"
          :label="$t('apModule.payment.bankAccount')"
          width="150"
        />
        <el-table-column prop="created_at" :label="$t('common.createTime')" width="160" />
        <el-table-column :label="$t('common.operation')" width="160" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.payment_status !== 'CONFIRMED'"
              type="success"
              link
              size="small"
              @click="confirmPayment(row as unknown as APPayment)"
              >{{ $t('apModule.payment.confirm') }}</el-button
            >
            <el-button
              type="primary"
              link
              size="small"
              @click="printPayment(row as unknown as APPayment)"
              >{{ $t('common.print') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="paymentDialogVisible"
      :title="$t('apModule.payment.createTitle')"
      width="600px"
      :aria-label="$t('apModule.payment.createAria')"
    >
      <el-form
        ref="paymentFormRef"
        :model="paymentForm"
        :rules="paymentRules"
        label-width="100px"
        :aria-label="$t('apModule.payment.formAria')"
      >
        <el-form-item :label="$t('apModule.payment.requestSelect')" prop="request_id">
          <el-select
            v-model="paymentForm.request_id"
            :loading="approvedLoading"
            :placeholder="$t('apModule.payment.requestSelectPlaceholder')"
            :no-data-text="$t('apModule.payment.requestSelectEmpty')"
            filterable
            style="width: 100%"
          >
            <el-option
              v-for="r in approvableRequests"
              :key="r.id"
              :label="requestOptionLabel(r)"
              :value="r.id"
            />
          </el-select>
        </el-form-item>

        <!-- 供应商 / 金额 / 方式 / 银行从所选已审批申请派生，只读展示，不作为可编辑输入发给后端 -->
        <template v-if="selectedRequest">
          <el-form-item :label="$t('apModule.payment.supplier')">
            <span>{{ supplierLabel(selectedRequest.supplier_id) }}</span>
          </el-form-item>
          <el-form-item :label="$t('apModule.paymentRequest.requestAmount')">
            <span>{{ formatMoney(selectedRequest.request_amount) }}</span>
          </el-form-item>
          <el-form-item :label="$t('apModule.payment.paymentMethod')">
            <span>{{ getPaymentMethodLabel(selectedRequest.payment_method ?? '') }}</span>
          </el-form-item>
          <el-form-item
            v-if="selectedRequest.bank_account"
            :label="$t('apModule.payment.bankAccount')"
          >
            <span>{{ selectedRequest.bank_account }}</span>
          </el-form-item>
        </template>

        <el-form-item :label="$t('apModule.payment.paymentDate')" prop="payment_date">
          <el-date-picker
            v-model="paymentForm.payment_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('apModule.payment.remark')">
          <el-input v-model="paymentForm.notes" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="paymentDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="paymentSubmitLoading" @click="submitPayment">{{
          $t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getAPPaymentList,
  createAPPayment,
  updateAPPayment,
  confirmAPPayment,
  getAPPaymentMethodText,
  type APPayment,
} from '@/api/ap-payment';
import { getAPPaymentRequestList, printAPPaymentDocx, type APPaymentRequest } from '@/api/ap';
import { getSupplierList, type Supplier } from '@/api/supplier';
import { isDialogDismissal } from '@/utils/monitor';

const { t } = useI18n({ useScope: 'global' });

const payments = ref<APPayment[]>([]);
const paymentLoading = ref(false);
const suppliers = ref<Supplier[]>([]);
// 列表响应只带 supplier_id（后端无 JOIN），名称用本页已加载的供应商主数据映射
const supplierLabel = (id: number) =>
  suppliers.value.find(s => s.id === id)?.supplier_name ?? String(id);

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00';
};

const getPaymentMethodLabel = (method: string) => {
  const keyMap: Record<string, string> = {
    bank_transfer: 'apModule.payment.methodBankTransfer',
    cash: 'apModule.payment.methodCash',
    check: 'apModule.payment.methodCheck',
    bill: 'apModule.payment.methodBill',
  };
  const key = keyMap[method];
  if (key) return t(key);
  return getAPPaymentMethodText(method) || method;
};

// 后端列表返回 PaginatedResponse{items,total,...}（作为 ApiResponse.data），兼容裸数组形态
const unwrapList = <T,>(d: { items?: T[] } | T[] | undefined): T[] =>
  Array.isArray(d) ? d : d?.items || [];

const fetchSuppliers = async () => {
  try {
    const res = await getSupplierList({ page_size: 500 });
    suppliers.value = res.data?.items || [];
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('apModule.payment.supplierFetchFailed'));
  }
};

const fetchPayments = async () => {
  paymentLoading.value = true;
  try {
    const res = await getAPPaymentList();
    payments.value = unwrapList<APPayment>(res.data);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('apModule.payment.fetchListFailed'));
  } finally {
    paymentLoading.value = false;
  }
};

const paymentDialogVisible = ref(false);
const paymentFormRef = ref<FormInstance>();
const paymentSubmitLoading = ref(false);
// 新建付款：从「已审批且尚未生成付款单」的付款申请中选择一张，仅填付款日期 + 可选备注
const approvableRequests = ref<APPaymentRequest[]>([]);
const approvedLoading = ref(false);
const paymentForm = reactive({
  request_id: undefined as number | undefined,
  payment_date: new Date().toISOString().split('T')[0],
  notes: '',
});

const selectedRequest = computed(
  () =>
    approvableRequests.value.find(r => r.id === paymentForm.request_id) as
      APPaymentRequest | undefined
);

const requestOptionLabel = (r: APPaymentRequest) =>
  `${r.request_no} · ${supplierLabel(r.supplier_id)} · ${formatMoney(r.request_amount)}`;

const paymentRules: FormRules = {
  request_id: [
    { required: true, message: t('apModule.payment.requestSelectRequired'), trigger: 'change' },
  ],
  payment_date: [
    { required: true, message: t('apModule.payment.dateRequired'), trigger: 'change' },
  ],
};

const loadApprovableRequests = async () => {
  approvedLoading.value = true;
  try {
    const [reqRes, payRes] = await Promise.all([
      getAPPaymentRequestList({ approval_status: 'APPROVED', page_size: 200 }),
      getAPPaymentList({ page_size: 500 }),
    ]);
    const requests = unwrapList<APPaymentRequest>(reqRes.data);
    const existingPayments = unwrapList<APPayment>(payRes.data);
    // 后端 create 有防重复（一申请一付款单，ap_payment_service.rs:86-94），前端预选过滤已生成的申请
    const usedRequestIds = new Set(
      existingPayments.map(p => p.request_id).filter((id): id is number => id != null)
    );
    approvableRequests.value = requests.filter(r => !usedRequestIds.has(r.id));
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('apModule.payment.requestFetchFailed'));
  } finally {
    approvedLoading.value = false;
  }
};

const openPaymentDialog = async () => {
  paymentFormRef.value?.resetFields();
  paymentForm.request_id = undefined;
  paymentForm.payment_date = new Date().toISOString().split('T')[0];
  paymentForm.notes = '';
  paymentDialogVisible.value = true;
  await loadApprovableRequests();
};

const submitPayment = async () => {
  const valid = await paymentFormRef.value?.validate().catch(() => false);
  if (!valid) return;
  paymentSubmitLoading.value = true;
  try {
    // 请求体仅含后端 CreateApPaymentRequest 接受的字段（供应商/金额/方式由服务端从申请派生）
    await createAPPayment({
      request_id: paymentForm.request_id as number,
      payment_date: paymentForm.payment_date,
      notes: paymentForm.notes || undefined,
    });
    ElMessage.success(t('common.success'));
    paymentDialogVisible.value = false;
    await fetchPayments();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    paymentSubmitLoading.value = false;
  }
};

const confirmPayment = async (row: APPayment) => {
  try {
    await ElMessageBox.confirm(
      t('apModule.payment.confirmConfirm'),
      t('apModule.payment.confirmTitle'),
      { type: 'info' }
    );
  } catch {
    return; // 用户取消确认弹窗，非错误
  }
  try {
    // 后端 confirm 强制 transaction_no 非空（ap_payment_service.rs:231-239）：
    // 未回填则先弹子对话框采集交易流水号，PUT 回填成功后再 confirm
    if (!row.transaction_no) {
      const { value } = await ElMessageBox.prompt(
        t('apModule.payment.transactionNoPrompt'),
        t('apModule.payment.transactionNoTitle'),
        {
          inputPlaceholder: t('apModule.payment.transactionNoPlaceholder'),
          inputValidator: (v: string) =>
            v && v.trim() ? true : t('apModule.payment.transactionNoRequired'),
        }
      );
      await updateAPPayment(row.id, { transaction_no: value.trim() });
    }
    await confirmAPPayment(row.id);
    ElMessage.success(t('apModule.payment.confirmSuccess'));
    await fetchPayments();
  } catch (e) {
    if (isDialogDismissal(e)) return;
    const err = e as { message?: string };
    ElMessage.error(err.message || t('apModule.payment.confirmFailed'));
  }
};

const printPayment = async (row: APPayment) => {
  try {
    const blob = await printAPPaymentDocx(row.id);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${row.payment_no}.docx`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('apModule.payment.printFailed'));
  }
};

defineExpose({ refresh: fetchPayments });

onMounted(() => {
  fetchPayments();
  fetchSuppliers();
});
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>
