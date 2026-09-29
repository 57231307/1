<!--
  ReconciliationTab.vue - 应收对账 Tab
  来源：原 ar/index.vue 中 应收对账 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="reconciliation-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('arModule.reconciliation.title') }}</h2>
      <el-button type="primary" @click="openReconciliationDialog()">
        <el-icon><Plus /></el-icon>
        {{ $t('arModule.reconciliation.create') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="reconciliationLoading"
        :data="reconciliations"
        stripe
        :aria-label="$t('arModule.reconciliation.listAria')"
      >
        <el-table-column
          prop="reconciliation_no"
          :label="$t('arModule.reconciliation.reconciliationNo')"
          width="140"
        />
        <el-table-column
          prop="customer_name"
          :label="$t('arModule.reconciliation.customer')"
          width="150"
        />
        <el-table-column :label="$t('arModule.reconciliation.reconciliationDate')" width="200">
          <template #default="{ row }"> {{ row.period_start }} ~ {{ row.period_end }} </template>
        </el-table-column>
        <el-table-column
          :label="$t('arModule.reconciliation.invoiceAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.total_invoices) }}
          </template>
        </el-table-column>
        <el-table-column
          :label="$t('arModule.reconciliation.paymentAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.total_collections) }}
          </template>
        </el-table-column>
        <el-table-column
          :label="$t('arModule.reconciliation.difference')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            <span :class="{ 'text-red': Number(row.closing_balance) !== 0 }">
              {{ formatMoney(row.closing_balance) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column
          prop="reconciliation_status"
          :label="$t('common.status')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getReconciliationStatusType(row.reconciliation_status)" size="small">
              {{ getReconciliationStatusLabel(row.reconciliation_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_at"
          :label="$t('arModule.reconciliation.confirmedAt')"
          width="160"
        />
        <el-table-column :label="$t('common.operation')" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.reconciliation_status === 'draft'"
              type="success"
              link
              size="small"
              @click="confirmReconciliation(row)"
              >{{ $t('arModule.reconciliation.confirm') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="reconciliationDialogVisible"
      :title="$t('arModule.reconciliation.createTitle')"
      width="500px"
      :aria-label="$t('arModule.reconciliation.createAria')"
    >
      <el-form
        ref="reconciliationFormRef"
        :model="reconciliationForm"
        label-width="80px"
        :aria-label="$t('arModule.reconciliation.formAria')"
      >
        <el-form-item :label="$t('arModule.reconciliation.reconciliationNo')">
          <el-input
            v-model="reconciliationForm.reconciliation_no"
            :placeholder="$t('arModule.reconciliation.reconciliationNo')"
          />
        </el-form-item>
        <el-form-item :label="$t('arModule.reconciliation.customer')">
          <el-select
            v-model="reconciliationForm.customer_id"
            :placeholder="$t('arModule.reconciliation.customerPlaceholder')"
            style="width: 100%"
          >
            <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('arModule.reconciliation.reconciliationDate')">
          <el-date-picker
            v-model="periodRange"
            type="daterange"
            value-format="YYYY-MM-DD"
            :start-placeholder="$t('arModule.reconciliation.datePlaceholder')"
            :end-placeholder="$t('arModule.reconciliation.datePlaceholder')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('arModule.reconciliation.invoiceAmount')">
          <el-input-number
            v-model="reconciliationForm.total_invoices"
            :min="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('arModule.reconciliation.paymentAmount')">
          <el-input-number
            v-model="reconciliationForm.total_collections"
            :min="0"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reconciliationDialogVisible = false">{{
          $t('common.cancel')
        }}</el-button>
        <el-button
          type="primary"
          :loading="reconciliationSubmitLoading"
          @click="submitReconciliation"
          >{{ $t('common.confirm') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance } from 'element-plus';
import {
  getARReconciliationList,
  createARReconciliation,
  updateARReconciliationStatus,
  type ARReconciliation,
} from '@/api/ar';
import type { Customer } from '@/api/customer';

const { t } = useI18n({ useScope: 'global' });

const reconciliations = ref<ARReconciliation[]>([]);
const customers = ref<Customer[]>([]);
const reconciliationLoading = ref(false);
const reconciliationSubmitLoading = ref(false);
const reconciliationDialogVisible = ref(false);
const reconciliationFormRef = ref<FormInstance>();
const periodRange = ref<[string, string] | null>(null);

const reconciliationForm = reactive({
  reconciliation_no: '',
  customer_id: undefined as number | undefined,
  opening_balance: 0,
  total_invoices: 0,
  total_collections: 0,
});

// 后端 ReconciliationResponse 金额字段为 Decimal.to_string()，前端按字符串接收后转数值格式化
const formatMoney = (amount: number | string | null | undefined) => {
  const n = Number(amount ?? 0);
  return n.toLocaleString('zh-CN', { minimumFractionDigits: 2 });
};

const getReconciliationStatusLabel = (status: string | null) => {
  // 真实 ar_reconciliation.reconciliation_status 值集（小写）：
  // draft/sent/confirmed/disputed/closed/cancelled（backend/src/models/status/finance.rs:22 起）
  const keyMap: Record<string, string> = {
    draft: 'arModule.reconciliation.statusDraft',
    confirmed: 'arModule.reconciliation.statusConfirmed',
    disputed: 'arModule.reconciliation.statusDisputed',
  };
  const key = status ? keyMap[status] : undefined;
  return key ? t(key) : status || '';
};

const getReconciliationStatusType = (status: string | null) => {
  const map: Record<string, string> = {
    draft: 'warning',
    confirmed: 'success',
    disputed: 'danger',
  };
  return (status && map[status]) || 'info';
};

const fetchReconciliations = async () => {
  reconciliationLoading.value = true;
  try {
    const res = await getARReconciliationList();
    reconciliations.value = res.data.items;
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('arModule.reconciliation.fetchListFailed'));
  } finally {
    reconciliationLoading.value = false;
  }
};

const openReconciliationDialog = () => {
  reconciliationForm.reconciliation_no = '';
  reconciliationForm.customer_id = undefined;
  reconciliationForm.opening_balance = 0;
  reconciliationForm.total_invoices = 0;
  reconciliationForm.total_collections = 0;
  const today = new Date().toISOString().split('T')[0];
  periodRange.value = [today, today];
  reconciliationDialogVisible.value = true;
};

const submitReconciliation = async () => {
  if (!reconciliationForm.customer_id || !periodRange.value) {
    ElMessage.warning(t('arModule.reconciliation.selectCustomer'));
    return;
  }

  reconciliationSubmitLoading.value = true;
  try {
    await createARReconciliation({
      reconciliation_no: reconciliationForm.reconciliation_no,
      customer_id: reconciliationForm.customer_id,
      period_start: periodRange.value[0],
      period_end: periodRange.value[1],
      opening_balance: reconciliationForm.opening_balance,
      total_invoices: reconciliationForm.total_invoices,
      total_collections: reconciliationForm.total_collections,
    });
    ElMessage.success(t('common.message.createSuccess'));
    reconciliationDialogVisible.value = false;
    fetchReconciliations();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    reconciliationSubmitLoading.value = false;
  }
};

const confirmReconciliation = async (row: ARReconciliation) => {
  try {
    await ElMessageBox.confirm(
      t('arModule.reconciliation.confirmMessage'),
      t('arModule.reconciliation.confirmTitle'),
      { type: 'info' }
    );
    await updateARReconciliationStatus(row.id, 'confirmed');
    ElMessage.success(t('arModule.reconciliation.confirmSuccess'));
    fetchReconciliations();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

onMounted(() => {
  fetchReconciliations();
});
</script>

<style scoped>
.text-red {
  color: #f56c6c;
}
</style>
