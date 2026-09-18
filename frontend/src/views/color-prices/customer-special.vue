<template>
  <div class="customer-special">
    <el-page-header :content="$t('colorPrices.customerSpecial.title')" @back="$router.back()" />

    <el-card style="margin-top: 20px">
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
      <el-form :model="form" label-width="110px">
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.customerId')" required>
          <el-input-number v-model="form.customer_id" :min="1" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.productId')" required>
          <el-input-number v-model="form.product_id" :min="1" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.colorId')" required>
          <el-input-number v-model="form.color_id" :min="1" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.specialPrice')" required>
          <el-input-number v-model="form.special_price" :precision="2" :min="0" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.customerSpecial.dialog.discount')">
          <el-input-number v-model="form.discount_percent" :precision="2" :min="0" :max="100" />
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
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  getCustomerSpecialPriceList,
  createCustomerSpecialPrice,
  formatPrice,
  type CustomerColorPrice,
} from '@/api/color-price';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);
const saving = ref(false);
const items = ref<CustomerColorPrice[]>([]);
const dialogVisible = ref(false);

const form = reactive({
  customer_id: 1,
  product_id: 1,
  color_id: 1,
  special_price: 0,
  discount_percent: null as number | null,
  currency: 'CNY',
  valid_from: '',
  valid_until: '',
  notes: '',
});

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
    customer_id: 1,
    product_id: 1,
    color_id: 1,
    special_price: 0,
    discount_percent: null,
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
  saving.value = true;
  try {
    await createCustomerSpecialPrice({
      customer_id: form.customer_id,
      product_id: form.product_id,
      color_id: form.color_id,
      special_price: form.special_price,
      discount_percent: form.discount_percent,
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
.customer-special {
  padding: 20px;
}
.toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
</style>
