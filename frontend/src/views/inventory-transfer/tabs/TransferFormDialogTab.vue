<!--
  TransferFormDialogTab.vue - 调拨单编辑对话框
  来源：原 inventoryTransfer/index.vue 中 调拨单编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? $t('inventoryTransfer.transferForm.editTitle')
        : $t('inventoryTransfer.transferForm.createTitle')
    "
    width="800px"
    :aria-label="
      mode === 'view'
        ? $t('inventoryTransfer.transferForm.viewDialogTitle')
        : formData.id
          ? $t('inventoryTransfer.transferForm.editDialogTitle')
          : $t('inventoryTransfer.transferForm.createDialogTitle')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :disabled="mode === 'view'"
      :aria-label="$t('inventoryTransfer.transferForm.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="$t('inventoryTransfer.transferForm.fromWarehouse')"
            prop="from_warehouse_id"
          >
            <el-select v-model="formData.from_warehouse_id" style="width: 100%">
              <el-option
                v-for="wh in warehouses"
                :key="wh.id"
                :label="wh.warehouse_name"
                :value="wh.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="$t('inventoryTransfer.transferForm.toWarehouse')"
            prop="to_warehouse_id"
          >
            <el-select v-model="formData.to_warehouse_id" style="width: 100%">
              <el-option
                v-for="wh in warehouses"
                :key="wh.id"
                :label="wh.warehouse_name"
                :value="wh.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="$t('inventoryTransfer.transferForm.transferDate')" prop="transfer_date">
        <el-date-picker
          v-model="formData.transfer_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
      <el-divider content-position="left">{{
        $t('inventoryTransfer.transferForm.detailDivider')
      }}</el-divider>
      <div
        v-for="(item, index) in formData.items"
        :key="index"
        style="display: flex; gap: 8px; margin-bottom: 8px; align-items: center"
      >
        <el-select
          v-model="item.product_id"
          :placeholder="$t('inventoryTransfer.transferForm.productPlaceholder')"
          style="flex: 2"
          filterable
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
          :placeholder="$t('inventoryTransfer.transferForm.quantityPlaceholder')"
          style="flex: 1"
        />
        <el-input-number
          v-model="item.cost_price"
          :min="0"
          :precision="2"
          :placeholder="$t('inventoryTransfer.transferForm.pricePlaceholder')"
          style="flex: 1"
        />
        <el-input
          v-model="item.remark"
          :placeholder="$t('inventoryTransfer.transferForm.remarkPlaceholder')"
          style="flex: 1.5"
        />
        <el-button
          type="danger"
          :icon="Delete"
          circle
          :disabled="formData.items.length <= 1"
          @click="removeItem(index)"
        />
      </div>
      <el-button type="primary" link @click="addItem">
        <el-icon><Plus /></el-icon>{{ $t('inventoryTransfer.transferForm.addItem') }}
      </el-button>
    </el-form>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        $t('inventoryTransfer.transferForm.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        $t('inventoryTransfer.transferForm.save')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance } from 'element-plus';
import { Plus, Delete } from '@element-plus/icons-vue';
import {
  createInventoryTransfer,
  updateInventoryTransfer,
  type InventoryTransferEntity,
} from '@/api/inventory-transfer';
import type { Warehouse } from '@/api/warehouse';
import type { Product } from '@/api/product';
import { logger } from '@/utils/logger';

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryTransferEntity | null;
  warehouses: Warehouse[];
  products: Product[];
  mode: 'create' | 'edit' | 'view';
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

const formData = reactive({
  id: 0,
  transfer_date: new Date().toISOString().split('T')[0],
  from_warehouse_id: undefined as number | undefined,
  to_warehouse_id: undefined as number | undefined,
  total_amount: 0,
  status: 'pending' as 'pending' | 'approved' | 'executed' | 'cancelled',
  items: [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }] as {
    product_id: number;
    quantity: number;
    cost_price: number;
    amount: number;
    remark: string;
  }[],
});

const resetForm = () => {
  formData.id = 0;
  formData.transfer_date = new Date().toISOString().split('T')[0];
  formData.from_warehouse_id = undefined;
  formData.to_warehouse_id = undefined;
  formData.total_amount = 0;
  formData.status = 'pending';
  formData.items = [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }];
};

const addItem = () => {
  formData.items.push({ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' });
};

const removeItem = (index: number) => {
  if (formData.items.length > 1) {
    formData.items.splice(index, 1);
  }
};

const calcAmount = (item: { quantity: number; cost_price: number }) => {
  item.quantity * item.cost_price;
};

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow);
        if (!formData.items || formData.items.length === 0) {
          formData.items = [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }];
        }
      } else {
        resetForm();
      }
    }
  }
);

const handleSubmit = async () => {
  submitLoading.value = true;
  try {
    formData.items.forEach(calcAmount);
    formData.total_amount = formData.items.reduce(
      (sum, item) => sum + item.cost_price * item.quantity,
      0
    );
    if (formData.id) {
      await updateInventoryTransfer(formData.id, formData as Partial<InventoryTransferEntity>);
    } else {
      await createInventoryTransfer(formData as Partial<InventoryTransferEntity>);
    }
    ElMessage.success(t('message.operationSuccess'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    ElMessage.error((error as Error).message || t('message.operationFailed'));
    logger.error(t('inventoryTransfer.transferForm.saveFailed'), (error as Error).message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
