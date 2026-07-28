<!--
  CurrencyListTab.vue - 币种管理 Tab
  来源：原 currency/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="currency-list-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('currency.page.title') }}</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ t('currency.page.newCurrency') }}
        </el-button>
        <el-button @click="openRateDialog()">
          <el-icon><Plus /></el-icon>{{ t('currency.page.newRate') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="currencyList"
        stripe
        :aria-label="t('currency.table.ariaLabel')"
      >
        <el-table-column prop="code" :label="t('currency.table.code')" width="80" />
        <el-table-column prop="name" :label="t('currency.table.name')" width="120" />
        <el-table-column
          prop="symbol"
          :label="t('currency.table.symbol')"
          width="60"
          align="center"
        />
        <el-table-column
          prop="precision"
          :label="t('currency.table.precision')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="isBase"
          :label="t('currency.table.isBase')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag v-if="row.isBase" type="success" size="small">{{
              t('currency.table.baseTag')
            }}</el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="isActive"
          :label="t('currency.table.status')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.isActive ? 'success' : 'info'" size="small">
              {{ getStatusLabel(row.isActive) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('currency.table.operation')" width="160" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openRateDialog(row.code)">{{
              t('currency.table.rate')
            }}</el-button>
            <el-button v-if="!row.isBase" type="warning" link size="small" @click="setBase(row)">{{
              t('currency.table.setBase')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="t('currency.dialog.createTitle')"
      width="500px"
      :aria-label="t('currency.dialog.createAriaLabel')"
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="100px"
        :aria-label="t('currency.dialog.formAriaLabel')"
      >
        <el-form-item :label="t('currency.dialog.code')" prop="code">
          <el-input v-model="form.code" :placeholder="t('currency.dialog.codePlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('currency.dialog.name')" prop="name">
          <el-input v-model="form.name" :placeholder="t('currency.dialog.namePlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('currency.dialog.symbol')">
          <el-input v-model="form.symbol" :placeholder="t('currency.dialog.symbolPlaceholder')" />
        </el-form-item>
        <el-form-item :label="t('currency.dialog.precision')">
          <el-input-number v-model="form.precision" :min="0" :max="6" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="t('currency.dialog.isBase')">
          <el-switch v-model="form.isBase" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('currency.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          t('currency.dialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="rateDialogVisible"
      :title="t('currency.rateDialog.createTitle')"
      width="500px"
      :aria-label="t('currency.rateDialog.createAriaLabel')"
    >
      <el-form
        ref="rateFormRef"
        :model="rateForm"
        :rules="rateRules"
        label-width="100px"
        :aria-label="t('currency.rateDialog.formAriaLabel')"
      >
        <el-form-item :label="t('currency.rateDialog.fromCurrency')" prop="fromCurrency">
          <el-select
            v-model="rateForm.fromCurrency"
            :placeholder="t('currency.rateDialog.fromCurrencyPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="c in currencyList"
              :key="c.code"
              :label="`${c.code} - ${c.name}`"
              :value="c.code"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('currency.rateDialog.toCurrency')" prop="toCurrency">
          <el-select
            v-model="rateForm.toCurrency"
            :placeholder="t('currency.rateDialog.toCurrencyPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="c in currencyList"
              :key="c.code"
              :label="`${c.code} - ${c.name}`"
              :value="c.code"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('currency.rateDialog.rate')" prop="rate">
          <el-input-number
            v-model="rateForm.rate"
            :min="0"
            :precision="6"
            :step="0.0001"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('currency.rateDialog.effectiveDate')" prop="effectiveDate">
          <el-date-picker
            v-model="rateForm.effectiveDate"
            type="date"
            :placeholder="t('currency.rateDialog.effectiveDatePlaceholder')"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('currency.rateDialog.source')">
          <el-input
            v-model="rateForm.source"
            :placeholder="t('currency.rateDialog.sourcePlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="rateDialogVisible = false">{{
          t('currency.rateDialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="rateSubmitLoading" @click="handleRateSubmit">{{
          t('currency.rateDialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import { useI18n } from 'vue-i18n';
import {
  getCurrencyList,
  createCurrency,
  createExchangeRate,
  setBaseCurrency,
  type Currency,
  type CreateCurrencyRequest,
  type CreateExchangeRateRequest,
} from '@/api/currency';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);
const submitLoading = ref(false);
const rateSubmitLoading = ref(false);
const dialogVisible = ref(false);
const rateDialogVisible = ref(false);
const currencyList = ref<Currency[]>([]);
const formRef = ref<FormInstance>();
const rateFormRef = ref<FormInstance>();

const form = reactive<CreateCurrencyRequest>({
  code: '',
  name: '',
  symbol: '',
  isBase: false,
  precision: 2,
});

const rateForm = reactive<CreateExchangeRateRequest>({
  fromCurrency: '',
  toCurrency: '',
  rate: 1,
  effectiveDate: new Date().toISOString().split('T')[0],
  source: '',
});

const rules: FormRules = {
  code: [{ required: true, message: t('currency.dialog.codeRequired'), trigger: 'blur' }],
  name: [{ required: true, message: t('currency.dialog.nameRequired'), trigger: 'blur' }],
};

const rateRules: FormRules = {
  fromCurrency: [
    { required: true, message: t('currency.rateDialog.fromCurrencyRequired'), trigger: 'change' },
  ],
  toCurrency: [
    { required: true, message: t('currency.rateDialog.toCurrencyRequired'), trigger: 'change' },
  ],
  rate: [{ required: true, message: t('currency.rateDialog.rateRequired'), trigger: 'blur' }],
  effectiveDate: [
    { required: true, message: t('currency.rateDialog.effectiveDateRequired'), trigger: 'change' },
  ],
};

/**
 * 状态标签映射（基于 i18n）
 */
const getStatusLabel = (isActive: boolean) => {
  return isActive ? t('currency.table.statusActive') : t('currency.table.statusInactive');
};

const fetchCurrencies = async () => {
  loading.value = true;
  try {
    const res = await getCurrencyList();
    const d = (res as { data?: unknown }).data as
      | Currency[]
      | { items?: Currency[]; data?: Currency[]; list?: Currency[] };
    currencyList.value = Array.isArray(d) ? d : d?.items || d?.data || d?.list || [];
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('currency.message.fetchListFailed'));
  } finally {
    loading.value = false;
  }
};

const openDialog = () => {
  formRef.value?.resetFields();
  form.code = '';
  form.name = '';
  form.symbol = '';
  form.isBase = false;
  form.precision = 2;
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      await createCurrency(form);
      ElMessage.success(t('currency.message.createSuccess'));
      dialogVisible.value = false;
      fetchCurrencies();
    } catch (e) {
      const err = e as Error;
      ElMessage.error(err.message || t('currency.message.createFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

const openRateDialog = (defaultFromCode?: string) => {
  rateFormRef.value?.resetFields();
  rateForm.fromCurrency = defaultFromCode || '';
  rateForm.toCurrency = '';
  rateForm.rate = 1;
  rateForm.effectiveDate = new Date().toISOString().split('T')[0];
  rateForm.source = '';
  rateDialogVisible.value = true;
};

const handleRateSubmit = async () => {
  if (!rateFormRef.value) return;
  await rateFormRef.value.validate(async valid => {
    if (!valid) return;
    rateSubmitLoading.value = true;
    try {
      await createExchangeRate(rateForm);
      ElMessage.success(t('currency.message.rateCreateSuccess'));
      rateDialogVisible.value = false;
    } catch (e) {
      const err = e as Error;
      ElMessage.error(err.message || t('currency.message.rateCreateFailed'));
    } finally {
      rateSubmitLoading.value = false;
    }
  });
};

// 批次 157d-1 修复：接入 setBaseCurrency API
const setBase = async (row: Currency) => {
  if (!row.id) {
    ElMessage.warning(t('currency.message.currencyIdMissing'));
    return;
  }
  try {
    await ElMessageBox.confirm(
      t('currency.message.setBaseConfirm', { code: row.code, name: row.name }),
      t('currency.message.setBaseTitle'),
      { type: 'warning' }
    );
    await setBaseCurrency(row.id);
    ElMessage.success(t('currency.message.setBaseSuccess'));
    fetchCurrencies();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('currency.message.setBaseFailed'));
    }
  }
};

onMounted(() => {
  fetchCurrencies();
});
</script>
