<!--
  CreateDlg - 新建采购单对话框
  任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 新建采购单对话框）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  新增：色号选择 + resolveSkuMapping 回填只读供应商品/色号/协议价
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('purchase.createDlg.title')"
    width="900px"
    :aria-label="t('purchase.createDlg.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form
      ref="localFormRef"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      :aria-label="t('purchase.createDlg.formAria')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchase.createDlg.supplier')" prop="supplier_id">
            <el-select
              v-model="localForm.supplier_id"
              :placeholder="t('purchase.createDlg.supplierPlaceholder')"
              style="width: 100%"
              @change="onSupplierChange"
            >
              <el-option
                v-for="s in suppliers"
                :key="s.id"
                :label="s.supplier_name"
                :value="s.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchase.createDlg.orderDate')" prop="order_date">
            <el-date-picker
              v-model="localForm.order_date"
              type="date"
              :placeholder="t('purchase.createDlg.datePlaceholder')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchase.createDlg.requiredDate')">
            <el-date-picker
              v-model="localForm.required_date"
              type="date"
              :placeholder="t('purchase.createDlg.datePlaceholder')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchase.createDlg.remark')">
            <el-input
              v-model="localForm.remark"
              :placeholder="t('purchase.createDlg.remarkPlaceholder')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('purchase.createDlg.detail')">
        <div class="items-table">
          <div class="items-header">
            <span class="col-product">{{ t('purchase.createDlg.colProduct') }}</span>
            <span class="col-color">{{ t('purchase.createDlg.colColor') }}</span>
            <span class="col-qty">{{ t('purchase.createDlg.colQuantity') }}</span>
            <span class="col-price">{{ t('purchase.createDlg.colUnitPrice') }}</span>
            <span class="col-supplier-info">{{ t('purchase.createDlg.colSupplierInfo') }}</span>
            <span class="col-amount">{{ t('purchase.createDlg.colAmount') }}</span>
            <span class="col-action">{{ t('purchase.createDlg.colOperation') }}</span>
          </div>
          <div v-for="(item, index) in localForm.items" :key="index" class="items-row">
            <el-select
              v-model="item.product_id"
              :placeholder="t('purchase.createDlg.productPlaceholder')"
              class="col-product"
              filterable
              @change="onProductSelect(index)"
            >
              <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
            </el-select>
            <el-select-v2
              v-model="item.color_id"
              :options="getColorOptions(index)"
              :placeholder="t('purchase.createDlg.colorPlaceholder')"
              :loading="itemColorLoading[index]"
              filterable
              clearable
              class="col-color"
              @change="onColorSelect(index)"
            />
            <el-input-number
              v-model="item.quantity"
              :min="1"
              class="col-qty"
              @change="onCalculateSubtotal(item)"
            />
            <el-input-number
              v-model="item.unit_price"
              :min="0"
              :precision="2"
              class="col-price"
              @change="onCalculateSubtotal(item)"
            />
            <div class="col-supplier-info">
              <template v-if="item.resolved">
                <span class="resolved-text">
                  {{ item.resolved.supplier_product_code }}
                  <template v-if="item.resolved.supplier_color_no">
                    / {{ item.resolved.supplier_color_no }}
                  </template>
                </span>
              </template>
              <span v-else class="resolved-placeholder">--</span>
            </div>
            <el-input-number v-model="item.subtotal" :precision="2" class="col-amount" readonly />
            <el-button
              v-if="localForm.items.length > 1"
              size="small"
              type="danger"
              @click="onRemoveItem(index)"
              >{{ t('purchase.createDlg.delete') }}</el-button
            >
          </div>
          <el-button type="text" @click="onAddItem">{{
            t('purchase.createDlg.addItem')
          }}</el-button>
        </div>
      </el-form-item>
      <el-form-item :label="t('purchase.createDlg.totalAmount')">
        <span class="total-amount">¥{{ calculateTotal().toLocaleString() }}</span>
      </el-form-item>
    </el-form>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="onCancel">{{ t('purchase.createDlg.cancel') }}</el-button>
        <el-button type="primary" @click="onSubmit">{{
          t('purchase.createDlg.confirm')
        }}</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormRules, FormInstance } from 'element-plus';
import type { Supplier } from '@/api/supplier';
import type { Product, ProductColor } from '@/api/product';
import { getProductColorList } from '@/api/product';
import type { CreateFormData, CreateItem } from '../composables/useCreate';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  modelValue: boolean;
  form: CreateFormData;
  rules: FormRules;
  suppliers: Supplier[];
  products: Product[];
  onSubmit: () => void;
  onCancel: () => void;
  onAddItem: () => void;
  onRemoveItem: (index: number) => void;
  onProductSelect: (index: number) => void;
  onColorSelect: (index: number) => void;
  onSupplierChange: () => void;
  onCalculateSubtotal: (item: CreateItem) => void;
  calculateTotal: () => number;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
  (e: 'update:form', form: CreateFormData): void;
  (e: 'update:formRef', ref: FormInstance | undefined): void;
}>();

const localForm = ref<CreateFormData>(deepClone(props.form));

const localFormRef = ref<FormInstance>();
watch(localFormRef, v => emit('update:formRef', v));

let syncing = false;

watch(
  () => props.form,
  newForm => {
    if (syncing) return;
    syncing = true;
    localForm.value = deepClone(newForm);
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

watch(
  localForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:form', deepClone(newForm));
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 每行色号选项（懒加载缓存）
const itemColorCache = ref<Record<number, ProductColor[]>>({});
const itemColorLoading = ref<Record<number, boolean>>({});

const getColorOptions = (index: number) => {
  const pid = localForm.value.items[index]?.product_id;
  if (!pid) return [];
  return (itemColorCache.value[pid] ?? []).map(c => ({
    value: c.id,
    label: `${c.color_no} - ${c.color_name}`,
  }));
};

watch(
  () => localForm.value.items.map(i => i.product_id),
  async (ids, oldIds) => {
    for (let i = 0; i < ids.length; i++) {
      const pid = ids[i];
      if (pid && pid !== oldIds?.[i] && !itemColorCache.value[pid]) {
        itemColorLoading.value[i] = true;
        try {
          const res = await getProductColorList(pid);
          itemColorCache.value[pid] = res.data;
        } catch (e) {
          console.error('[purchase] load colors error:', e);
        } finally {
          itemColorLoading.value[i] = false;
        }
      }
    }
  },
  { deep: true }
);
</script>

<style scoped>
.items-table {
  width: 100%;
}

.items-header {
  display: flex;
  gap: 6px;
  padding: 8px 0;
  font-weight: 600;
  color: #303133;
  border-bottom: 1px solid #ebeef5;
}

.items-row {
  display: flex;
  gap: 6px;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;
}

.col-product {
  flex: 1;
  min-width: 150px;
}

.col-color {
  width: 150px;
}

.col-qty,
.col-price,
.col-amount {
  width: 100px;
}

.col-supplier-info {
  width: 160px;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.resolved-text {
  color: #409eff;
}

.resolved-placeholder {
  color: #c0c4cc;
}

.total-amount {
  font-size: 18px;
  font-weight: 700;
  color: #f56c6c;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
