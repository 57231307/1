<!--
  PurchaseReceiptForm.vue - 采购入库新增/编辑对话框
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="1080px"
    :aria-label="title"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      :aria-label="t('purchaseReceipt.form.aria.form')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseReceipt.form.label.receiptNo')" prop="receipt_no">
            <el-input v-model="localForm.receipt_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseReceipt.form.label.receiptDate')" prop="receipt_date">
            <el-date-picker v-model="localForm.receipt_date" type="date" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseReceipt.form.label.supplier')" prop="supplier_id">
            <el-select
              :model-value="localForm.supplier_id"
              :placeholder="t('purchaseReceipt.form.placeholder.supplier')"
              @update:model-value="(v: number | undefined) => (localForm.supplier_id = v)"
            >
              <el-option v-for="s in suppliers" :key="s.value" :label="s.label" :value="s.value" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseReceipt.form.label.warehouse')" prop="warehouse_id">
            <el-select
              :model-value="localForm.warehouse_id"
              :placeholder="t('purchaseReceipt.form.placeholder.warehouse')"
              @update:model-value="(v: number | undefined) => (localForm.warehouse_id = v)"
            >
              <el-option v-for="w in warehouses" :key="w.value" :label="w.label" :value="w.value" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('purchaseReceipt.form.label.items')">
        <div class="items-table">
          <div class="items-header">
            <span class="col-product">{{ t('purchaseReceipt.form.itemsHeader.product') }}</span>
            <span class="col-qty">{{ t('purchaseReceipt.form.itemsHeader.quantity') }}</span>
            <span class="col-qty">
              {{ t('purchaseReceipt.form.itemsHeader.quantityAlt') }}
            </span>
            <span class="col-unit">{{ t('purchaseReceipt.form.itemsHeader.unit') }}</span>
            <span class="col-price">{{ t('purchaseReceipt.form.itemsHeader.price') }}</span>
            <span class="col-amount">{{ t('purchaseReceipt.form.itemsHeader.amount') }}</span>
            <span class="col-action">{{ t('purchaseReceipt.form.itemsHeader.action') }}</span>
          </div>
          <div v-for="(item, index) in localForm.items || []" :key="index" class="items-block">
            <div class="items-row">
              <el-select
                :model-value="item.product_id"
                :placeholder="t('purchaseReceipt.form.placeholder.product')"
                class="col-product"
                @update:model-value="(v: number) => onProductChange(item, v)"
              >
                <el-option v-for="p in products" :key="p.value" :label="p.label" :value="p.value" />
              </el-select>
              <el-input-number
                :model-value="item.quantity"
                class="col-qty"
                @update:model-value="
                  (v: number | undefined) => {
                    item.quantity = v ?? 0;
                    emit('calc-amount', item);
                  }
                "
              />
              <el-input-number
                :model-value="item.quantity_alt"
                class="col-qty"
                @update:model-value="
                  (v: number | undefined) => {
                    item.quantity_alt = v ?? 0;
                  }
                "
              />
              <!-- 主单位取自产品档案，不可手工录入（后端 unit_master 为必填契约字段） -->
              <span class="col-unit">{{ item.unit_master || '-' }}</span>
              <el-input-number
                :model-value="item.unit_price"
                :precision="2"
                class="col-price"
                @update:model-value="
                  (v: number | undefined) => {
                    item.unit_price = v ?? 0;
                    emit('calc-amount', item);
                  }
                "
              />
              <el-input-number
                :model-value="item.amount"
                :precision="2"
                class="col-amount"
                readonly
              />
              <el-button
                v-if="(localForm.items || []).length > 1"
                size="small"
                type="danger"
                @click="emit('remove-item', index)"
                >{{ t('purchaseReceipt.form.button.delete') }}</el-button
              >
            </div>
            <!--
              收货实测维度行：批次号（后端建单期强校验的四维之一，逐行必填）+ 染色布追溯维度
              （色号/缸号/等级，透传至 CreateReceiptItemRequest 的 batch_no/color_code/lot_no/grade）。
              无自动来源（本页不关联采购订单行、产品档案也不含这些维度），由操作人录入，禁止造假值。
            -->
            <div class="items-trace">
              <label class="trace-field">
                <span class="trace-label required">
                  {{ t('purchaseReceipt.form.itemsHeader.batch') }}
                </span>
                <el-input
                  v-model="item.batch_no"
                  size="small"
                  :placeholder="t('purchaseReceipt.form.placeholder.batch')"
                />
              </label>
              <label class="trace-field">
                <span class="trace-label">
                  {{ t('purchaseReceipt.form.itemsHeader.colorNo') }}
                </span>
                <el-input
                  v-model="item.color_code"
                  size="small"
                  :placeholder="t('purchaseReceipt.form.placeholder.colorNo')"
                />
              </label>
              <label class="trace-field">
                <span class="trace-label">
                  {{ t('purchaseReceipt.form.itemsHeader.lotNo') }}
                </span>
                <el-input
                  v-model="item.lot_no"
                  size="small"
                  :placeholder="t('purchaseReceipt.form.placeholder.lotNo')"
                />
              </label>
              <label class="trace-field">
                <span class="trace-label">
                  {{ t('purchaseReceipt.form.itemsHeader.grade') }}
                </span>
                <el-input
                  v-model="item.grade"
                  size="small"
                  :placeholder="t('purchaseReceipt.form.placeholder.grade')"
                />
              </label>
            </div>
          </div>
          <el-button type="text" @click="emit('add-item')">{{
            t('purchaseReceipt.form.button.addItem')
          }}</el-button>
        </div>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('purchaseReceipt.form.button.cancel')
      }}</el-button>
      <el-button type="primary" @click="onSubmit">{{
        t('purchaseReceipt.form.button.submit')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance, FormRules } from 'element-plus';
import type { ReceiptItem } from '@/api/purchase-receipt';

const { t } = useI18n({ useScope: 'global' });

// 选项类型（产品选项额外带产品档案主数据：编码/名称/主单位）
interface OptItem {
  label: string;
  value: number;
  code?: string;
  name?: string;
  unit?: string;
}

// 表单类型
interface PurchaseReceiptFormModel {
  id?: number;
  receipt_no?: string;
  receipt_date?: string;
  supplier_id?: number;
  warehouse_id?: number;
  status?: string;
  items?: ReceiptItem[];
  [key: string]: unknown;
}

const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 对话框标题
  title: string;
  // 表单数据（由父组件管理，子组件通过 emit('update:form') 回写）
  form: PurchaseReceiptFormModel;
  // 校验规则
  rules: FormRules;
  // 供应商选项
  suppliers: OptItem[];
  // 仓库选项
  warehouses: OptItem[];
  // 产品选项
  products: OptItem[];
}>();

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void;
  (e: 'add-item'): void;
  (e: 'remove-item', index: number): void;
  (e: 'calc-amount', item: ReceiptItem): void;
  (e: 'submit'): void;
  // 整体回写表单（父组件监听此事件并回写到自己的 form）
  (e: 'update:form', form: PurchaseReceiptFormModel): void;
}>();

// 表单 ref
const formRef = ref<FormInstance>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 items 数组，需要深拷贝以保证本地修改与父组件解耦
const localForm = ref<PurchaseReceiptFormModel>(deepClone(props.form));

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件打开新增/编辑时填充数据）
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

// 本地变化时通知父组件（用户输入）
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

/**
 * 选择产品：把产品档案的编码/名称/主单位带入明细行
 * （后端 CreateReceiptItemRequest 的 material_code/material_name/unit_master 为必填，
 * 只能来源于产品主数据，不允许提交时再拼假值）
 */
const onProductChange = (item: ReceiptItem, productId: number) => {
  item.product_id = productId;
  const opt = props.products.find(p => p.value === productId);
  if (!opt) return;
  item.material_code = opt.code;
  item.material_name = opt.name;
  item.unit_master = opt.unit;
};

/** 点击确定：先校验再发 submit */
const onSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return;
    emit('submit');
  });
};
</script>

<style scoped>
.items-table {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}
.items-header {
  display: flex;
  background: #f5f7fa;
  padding: 10px;
  font-weight: bold;
}
.items-row {
  display: flex;
  padding: 10px 10px 4px;
}
.items-block {
  border-top: 1px solid #ebeef5;
}
.items-trace {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 16px;
  padding: 4px 10px 10px;
}
.trace-field {
  display: flex;
  align-items: center;
  gap: 6px;
}
.trace-label {
  font-size: 12px;
  color: #606266;
  white-space: nowrap;
}
.trace-label.required::before {
  content: '*';
  color: #f56c6c;
  margin-right: 2px;
}
.trace-field :deep(.el-input) {
  width: 150px;
}
.col-product {
  flex: 2;
  margin-right: 10px;
}
.col-qty,
.col-price,
.col-amount {
  width: 100px;
  margin-right: 10px;
}
.col-unit {
  width: 56px;
  margin-right: 10px;
  line-height: 32px;
}
.col-action {
  width: 60px;
}
</style>
