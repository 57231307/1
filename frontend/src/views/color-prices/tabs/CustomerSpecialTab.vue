<template>
  <div class="customer-special-tab">
    <el-card>
      <div class="toolbar">
        <el-button type="primary" @click="handleCreate">{{
          $t('colorPrices.customerSpecial.create')
        }}</el-button>
        <el-button plain @click="loadList">{{
          $t('colorPrices.customerSpecial.refresh')
        }}</el-button>
      </div>

      <el-table v-loading="loading" :data="items" border stripe>
        <el-table-column prop="id" :label="$t('colorPrices.customerSpecial.table.id')" width="70" />
        <el-table-column
          prop="customer_id"
          :label="$t('colorPrices.customerSpecial.table.customerId')"
          width="100"
        />
        <el-table-column
          prop="product_id"
          :label="$t('colorPrices.customerSpecial.table.productId')"
          width="100"
        />
        <el-table-column
          prop="color_id"
          :label="$t('colorPrices.customerSpecial.table.colorId')"
          width="100"
        />
        <el-table-column
          prop="special_price"
          :label="$t('colorPrices.customerSpecial.table.specialPrice')"
          width="110"
        >
          <template #default="{ row }">
            {{ formatPrice(row.special_price, row.currency) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="discount_percent"
          :label="$t('colorPrices.customerSpecial.table.discount')"
          width="90"
        />
        <el-table-column
          prop="currency"
          :label="$t('colorPrices.customerSpecial.table.currency')"
          width="90"
        />
        <el-table-column
          prop="valid_from"
          :label="$t('colorPrices.customerSpecial.table.validFrom')"
          width="110"
        />
        <el-table-column
          prop="valid_until"
          :label="$t('colorPrices.customerSpecial.table.validUntil')"
          width="110"
        />
        <el-table-column
          prop="notes"
          :label="$t('colorPrices.customerSpecial.table.notes')"
          min-width="140"
          show-overflow-tooltip
        />
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="$t('colorPrices.customerSpecial.dialog.createTitle')"
      width="520"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="110px">
        <el-form-item
          :label="$t('colorPrices.customerSpecial.dialog.customerId')"
          prop="customer_id"
          required
        >
          <el-input v-model="form.customer_id" inputmode="numeric" />
        </el-form-item>
        <el-form-item
          :label="$t('colorPrices.customerSpecial.dialog.productId')"
          prop="product_id"
          required
        >
          <el-input v-model="form.product_id" inputmode="numeric" />
        </el-form-item>
        <el-form-item
          :label="$t('colorPrices.customerSpecial.dialog.colorId')"
          prop="color_id"
          required
        >
          <el-input v-model="form.color_id" inputmode="numeric" />
        </el-form-item>
        <el-form-item
          :label="$t('colorPrices.customerSpecial.dialog.specialPrice')"
          prop="special_price"
          required
        >
          <el-input v-model="form.special_price" inputmode="decimal" />
        </el-form-item>
        <el-form-item
          :label="$t('colorPrices.customerSpecial.dialog.discount')"
          prop="discount_percent"
        >
          <el-input v-model="form.discount_percent" inputmode="decimal" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.currency')" required>
          <el-select v-model="form.currency" style="width: 100%">
            <el-option label="CNY" value="CNY" />
            <el-option label="USD" value="USD" />
            <el-option label="EUR" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.validFrom')" required>
          <el-input v-model="form.valid_from" placeholder="2026-01-01" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.validUntil')">
          <el-input v-model="form.valid_until" placeholder="2026-12-31" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.notes')">
          <el-input v-model="form.notes" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('colorPrices.common.cancel') }}</el-button>
        <el-button type="primary" :loading="saving" @click="handleSave">{{
          $t('colorPrices.common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';
import {
  getCustomerSpecialPriceList,
  createCustomerSpecialPrice,
  formatPrice,
  type CustomerColorPrice,
} from '@/api/color-price';
import { numberInputRules } from '@/utils/number-input';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);
const saving = ref(false);
const items = ref<CustomerColorPrice[]>([]);
const dialogVisible = ref(false);

// 手写录入文本模型（数字约束收进 numberInputRules 校验器，不靠步进按钮）
// ID 类初值为 1（手输主键下界），折扣为可选项初值空串（提交回 null 三态）
const form = reactive({
  customer_id: '1',
  product_id: '1',
  color_id: '1',
  special_price: '',
  discount_percent: '',
  currency: 'CNY',
  valid_from: '',
  valid_until: '',
  notes: '',
});

// 建单对话框数值录入校验（范围与精度靠校验器，不靠步进按钮）
const formRef = ref<FormInstance>();
const rules = computed<FormRules>(() => ({
  customer_id: numberInputRules({
    label: t('colorPrices.customerSpecial.dialog.customerId'),
    required: true,
    min: 1,
    integer: true,
  }),
  product_id: numberInputRules({
    label: t('colorPrices.customerSpecial.dialog.productId'),
    required: true,
    min: 1,
    integer: true,
  }),
  color_id: numberInputRules({
    label: t('colorPrices.customerSpecial.dialog.colorId'),
    required: true,
    min: 1,
    integer: true,
  }),
  special_price: numberInputRules({
    label: t('colorPrices.customerSpecial.dialog.specialPrice'),
    required: true,
    min: 0,
    precision: 2,
  }),
  discount_percent: numberInputRules({
    label: t('colorPrices.customerSpecial.dialog.discount'),
    min: 0,
    max: 100,
    precision: 2,
  }),
}));

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getCustomerSpecialPriceList();
    items.value = res.items || [];
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  } finally {
    loading.value = false;
  }
};

const handleCreate = () => {
  Object.assign(form, {
    customer_id: '1',
    product_id: '1',
    color_id: '1',
    special_price: '',
    discount_percent: '',
    currency: 'CNY',
    valid_from: '',
    valid_until: '',
    notes: '',
  });
  dialogVisible.value = true;
};

const handleSave = async () => {
  if (!form.valid_from) {
    ElMessage.warning(t('colorPrices.customerSpecial.dialog.validFrom'));
    return;
  }
  let valid = false;
  await formRef.value?.validate(ok => {
    valid = ok;
  });
  if (!valid) return;
  saving.value = true;
  try {
    await createCustomerSpecialPrice({
      customer_id: Number(form.customer_id),
      product_id: Number(form.product_id),
      color_id: Number(form.color_id),
      special_price: Number(form.special_price),
      discount_percent: form.discount_percent.trim() === '' ? null : Number(form.discount_percent),
      currency: form.currency,
      valid_from: form.valid_from,
      valid_until: form.valid_until || null,
      notes: form.notes || null,
    });
    ElMessage.success(t('colorPrices.message.createSuccess'));
    dialogVisible.value = false;
    await loadList();
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  } finally {
    saving.value = false;
  }
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.customer-special-tab {
  padding: 20px;
}
.toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
</style>
