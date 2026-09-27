<!--
  TransferDialog.vue - 新建调拨单对话框
  任务编号: P14 批 2 I-3 第 8 批
  拆分原 inventory/index.vue 的新建调拨单对话框
  使用 props.initialForm 初始化 + 内部 localForm
  submit 时 emit submitWithForm(localForm) 把当前 form 回传

  修复（缺陷B）：每行明细增加产品选择器（el-select），绑定 item.product_id。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('inventory.transferDialog.title')"
    width="700px"
    :close-on-click-modal="false"
    :aria-label="t('inventory.transferDialog.ariaLabel')"
    @update:model-value="onClose"
  >
    <el-form
      :model="localForm"
      label-width="100px"
      :aria-label="t('inventory.transferDialog.formAria')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('inventory.transferDialog.fromWarehouse')">
            <el-select
              v-model="localForm.from_warehouse_id"
              :placeholder="t('inventory.transferDialog.fromWarehousePlaceholder')"
              style="width: 100%"
            >
              <el-option
                v-for="wh in warehouses"
                :key="wh.id"
                :label="getWarehouseLabel(wh)"
                :value="wh.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('inventory.transferDialog.toWarehouse')">
            <el-select
              v-model="localForm.to_warehouse_id"
              :placeholder="t('inventory.transferDialog.toWarehousePlaceholder')"
              style="width: 100%"
            >
              <el-option
                v-for="wh in warehouses"
                :key="wh.id"
                :label="getWarehouseLabel(wh)"
                :value="wh.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('inventory.transferDialog.divider') }}</el-divider>
      <el-form-item
        v-for="(item, index) in localForm.items"
        :key="index"
        :label="t('inventory.transferDialog.itemProduct')"
      >
        <div style="display: flex; gap: 10px; width: 100%">
          <el-select
            v-model="item.product_id"
            filterable
            :placeholder="t('inventory.transferDialog.productPlaceholder')"
            style="flex: 2"
          >
            <el-option
              v-for="p in products"
              :key="p.id"
              :label="`${p.product_code} - ${p.product_name}`"
              :value="p.id"
            />
          </el-select>
          <el-input-number
            v-model="item.quantity"
            :min="1"
            :placeholder="t('inventory.transferDialog.quantityPlaceholder')"
            style="flex: 1"
          />
          <el-button
            type="danger"
            :icon="Delete"
            circle
            :disabled="localForm.items.length <= 1"
            @click="emit('removeItem', index)"
          />
        </div>
      </el-form-item>
      <el-button type="primary" link @click="emit('addItem')">
        <el-icon><Plus /></el-icon>
        {{ t('inventory.transferDialog.addProduct') }}
      </el-button>
      <el-form-item :label="t('inventory.transferDialog.remark')" style="margin-top: 16px">
        <el-input
          v-model="localForm.remark"
          type="textarea"
          :rows="2"
          :placeholder="t('inventory.transferDialog.remarkPlaceholder')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onClose(false)">{{ t('inventory.transferDialog.cancel') }}</el-button>
      <el-button type="primary" @click="onSubmit">{{
        t('inventory.transferDialog.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { deepClone } from '@/utils';
import { Delete, Plus } from '@element-plus/icons-vue';
import { reactive, watch } from 'vue';
import { getWarehouseLabel } from '../composables/invFmts';
import type { Warehouse } from '@/api/warehouse';
import type { Product } from '@/api/product';

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' });

/** 调拨单明细行 */
interface TransferFormItem {
  product_id: number | null;
  quantity: number;
}

/** 调拨单表单数据 */
interface TransferForm {
  from_warehouse_id: number | null;
  to_warehouse_id: number | null;
  items: TransferFormItem[];
  remark: string;
}

const props = defineProps<{
  visible: boolean;
  initialForm: TransferForm;
  warehouses: Warehouse[];
  products: Product[];
}>();

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void;
  (e: 'submit', data: TransferForm): void;
  (e: 'addItem'): void;
  (e: 'removeItem', index: number): void;
}>();

// 浅拷贝 initialForm 同步初始值（不直接突变 prop）
const localForm = reactive<TransferForm>({
  from_warehouse_id: null,
  to_warehouse_id: null,
  items: [],
  remark: '',
});
watch(
  () => props.initialForm,
  newVal => {
    // TransferForm 字段固定，直接 Object.assign 覆盖即可（无需逐键 delete）
    Object.assign(localForm, deepClone(newVal));
  },
  { immediate: true, deep: true }
);

const onClose = (val: boolean) => {
  emit('update:visible', val);
};

const onSubmit = () => {
  emit('submit', deepClone(localForm));
};
</script>
