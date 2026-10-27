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
              value-format="YYYY-MM-DD"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchase.createDlg.warehouse')" prop="warehouse_id">
            <el-select
              v-model="localForm.warehouse_id"
              :placeholder="t('purchase.createDlg.warehousePlaceholder')"
              style="width: 100%"
            >
              <el-option
                v-for="w in warehouses"
                :key="w.id"
                :label="w.warehouse_name"
                :value="w.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchase.createDlg.department')" prop="department_id">
            <el-select
              v-model="localForm.department_id"
              :placeholder="t('purchase.createDlg.departmentPlaceholder')"
              style="width: 100%"
            >
              <el-option v-for="d in departments" :key="d.id" :label="d.name" :value="d.id" />
            </el-select>
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
              value-format="YYYY-MM-DD"
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
      <el-form-item :label="t('purchase.createDlg.detail')" prop="items">
        <div class="items-table">
          <div class="items-header">
            <span class="col-product">{{ t('purchase.createDlg.colProduct') }}</span>
            <span class="col-color">{{ t('purchase.createDlg.colColor') }}</span>
            <span class="col-qty">{{ t('purchase.createDlg.colQuantity') }}</span>
            <span class="col-price">{{ t('purchase.createDlg.colUnitPriceWithTax') }}</span>
            <span class="col-tax">{{ t('purchase.createDlg.colTaxRate') }}</span>
            <span class="col-net-price">{{ t('purchase.createDlg.colNetUnitPrice') }}</span>
            <span class="col-tax-amount">{{ t('purchase.createDlg.colTaxAmount') }}</span>
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
            <!--
              含税单价是本表唯一的价格主输入（录入事实），用 el-input 承载并靠校验器管范围：
              禁用带加减号的步进控件是项目硬口径。净额与税额由它按税率除法反算派生，不可手输。
            -->
            <el-input
              v-model="item.unit_price_with_tax"
              class="col-price"
              :placeholder="t('purchase.createDlg.unitPriceWithTaxPlaceholder')"
              @change="onCalculateSubtotal(item)"
            />
            <el-input-number
              v-model="item.tax_rate"
              :min="0"
              :max="999.9999"
              :precision="4"
              :placeholder="t('purchase.createDlg.taxRatePlaceholder')"
              class="col-tax"
              @change="onCalculateSubtotal(item)"
            />
            <!-- 不含税单价是派生只读值：含税单价按本行税率除法反算，前端不乘 (1 + 税率) -->
            <span class="col-net-price">{{ formatDerived(item.unit_price) }}</span>
            <!-- 税额是派生只读值：含税金额减净额取差额，与后端 utils tax 同一口径 -->
            <span class="col-tax-amount">{{ formatDerived(item.tax_amount) }}</span>
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
            <span class="col-amount">{{ formatDerived(item.subtotal) }}</span>
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
        <el-button type="primary" :loading="creating" :disabled="creating" @click="onSubmit">{{
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
import type { Warehouse } from '@/api/warehouse';
import type { Department } from '@/api/department';
import { getProductColorList } from '@/api/product';
import { formatDecimalAmount } from '@/utils/money';
import type { CreateFormData, CreateItem } from '../composables/useCreate';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  modelValue: boolean;
  form: CreateFormData;
  rules: FormRules;
  suppliers: Supplier[];
  products: Product[];
  warehouses: Warehouse[];
  departments: Department[];
  // 建单在途（父组件管理）：为真时确认按钮转圈并禁用，避免连点重复建单
  creating: boolean;
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

/**
 * 派生只读列的展示（不含税单价、税额、净额小计）：
 * 无值（未填含税单价或未录税率）显示占位符而不是 0，避免把"没算"看成"算得 0"；
 * 有值走 money.ts 的 formatDecimalAmount 统一千分位与两位小数。
 * 入参 item.unit_price 与 item.tax_amount、item.subtotal 来自 useCreate 的 calculateSubtotal
 * 除法反算，只读展示给制单人核对，不提交给后端当权威值（后端按真实税率重算并逐分复核）。
 */
const formatDerived = (value: number | undefined): string =>
  value === undefined ? '--' : formatDecimalAmount(value);

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
  overflow-x: auto;
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
  flex: 1 0 150px;
}

/* 定宽列一律不参与收缩，对话框宽度不足时改由 items-table 横向滚动承接。
   允许收缩会把 el-input-number 的输入框压到零宽，数量与税率在界面上填不进去 */
.col-color,
.col-qty,
.col-price,
.col-tax,
.col-net-price,
.col-tax-amount,
.col-supplier-info,
.col-amount {
  flex: none;
}

.col-color {
  width: 150px;
}

.col-qty,
.col-price,
.col-tax,
.col-net-price,
.col-tax-amount,
.col-amount {
  width: 100px;
}

/* 派生只读列（不含税单价、税额、净额小计）：右对齐、不可编辑，缺值显示占位符 */
.col-net-price,
.col-tax-amount,
.col-amount {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.col-supplier-info {
  width: 160px;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 768px) {
  /* 明细行在竖屏改为逐字段堆叠，固定列宽会撑破弹窗内容区 */
  .items-header {
    display: none;
  }

  .items-row {
    flex-wrap: wrap;
    gap: 8px;
    padding: 10px 0;
  }

  .items-row > * {
    margin-right: 0;
  }

  .col-product,
  .col-color,
  .col-qty,
  .col-price,
  .col-net-price,
  .col-tax-amount,
  .col-amount,
  .col-supplier-info {
    flex: 0 0 100%;
    width: 100%;
    min-width: 0;
  }

  .col-supplier-info {
    white-space: normal;
    text-overflow: clip;
  }

  .items-row > .el-button {
    width: 100%;
    margin-left: 0;
  }
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
