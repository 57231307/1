<!--
  PurchaseReturnForm.vue - 采购退货新建/编辑表单对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="isEdit ? t('purchaseReturn.form.title.edit') : t('purchaseReturn.form.title.create')"
    width="900px"
    :aria-label="isEdit ? t('purchaseReturn.form.aria.edit') : t('purchaseReturn.form.aria.create')"
    @update:model-value="onVisibleChange"
  >
    <el-form
      ref="formRef"
      :model="localFormData"
      :rules="formRules"
      label-width="100px"
      :aria-label="t('purchaseReturn.form.aria.form')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('purchaseReturn.form.label.purchaseOrder')"
            prop="purchaseOrderId"
          >
            <el-select
              v-model="localFormData.purchaseOrderId"
              :placeholder="t('purchaseReturn.form.placeholder.purchaseOrder')"
              filterable
              @change="onOrderChange"
            >
              <el-option
                v-for="order in purchaseOrders"
                :key="order.id"
                :label="order.order_no"
                :value="order.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseReturn.form.label.returnDate')" prop="returnDate">
            <el-date-picker
              v-model="localFormData.returnDate"
              type="date"
              :placeholder="t('purchaseReturn.form.placeholder.returnDate')"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('purchaseReturn.form.label.reason')" prop="reason">
        <el-input
          v-model="localFormData.reason"
          type="textarea"
          :rows="3"
          :placeholder="t('purchaseReturn.form.placeholder.reason')"
        />
      </el-form-item>
      <el-form-item :label="t('purchaseReturn.form.label.remarks')">
        <el-input
          v-model="localFormData.remarks"
          type="textarea"
          :rows="2"
          :placeholder="t('purchaseReturn.form.placeholder.remarks')"
        />
      </el-form-item>

      <!-- 退货明细 -->
      <el-divider content-position="left">{{ t('purchaseReturn.form.itemsTitle') }}</el-divider>
      <el-button type="primary" size="small" class="mb-10" @click="onAddItem">
        {{ t('purchaseReturn.form.button.addItem') }}
      </el-button>
      <el-table
        :data="localFormData.items"
        border
        :aria-label="t('purchaseReturn.form.aria.itemsTable')"
      >
        <el-table-column
          prop="productName"
          :label="t('purchaseReturn.form.column.productName')"
          min-width="150"
        >
          <template #default="{ row }">
            <el-select
              v-model="row.productId"
              filterable
              :placeholder="t('purchaseReturn.form.placeholder.product')"
              @change="(v: number) => onProductChange(row, v)"
            >
              <el-option
                v-for="product in products"
                :key="product.id"
                :label="product.name"
                :value="product.id"
              />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column
          prop="quantity"
          :label="t('purchaseReturn.form.column.quantity')"
          width="120"
        >
          <template #default="{ row }">
            <el-input-number v-model="row.quantity" :min="1" size="small" />
          </template>
        </el-table-column>
        <el-table-column
          prop="unitPrice"
          :label="t('purchaseReturn.form.column.unitPrice')"
          width="120"
        >
          <template #default="{ row }">
            <el-input-number v-model="row.unitPrice" :min="0" :precision="2" size="small" />
          </template>
        </el-table-column>
        <el-table-column prop="amount" :label="t('purchaseReturn.form.column.amount')" width="120">
          <template #default="{ row }">
            <span>¥{{ (row.quantity * row.unitPrice).toFixed(2) }}</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="reason"
          :label="t('purchaseReturn.form.column.reason')"
          min-width="150"
        >
          <template #default="{ row }">
            <el-input v-model="row.reason" size="small" />
          </template>
        </el-table-column>
        <el-table-column :label="t('purchaseReturn.form.column.action')" width="80">
          <template #default="{ $index }">
            <el-button size="small" type="danger" @click="onRemoveItem($index)">{{
              t('purchaseReturn.form.button.delete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">{{ t('purchaseReturn.form.button.cancel') }}</el-button>
      <el-button type="primary" :loading="submitting" @click="onSubmit">{{
        t('purchaseReturn.form.button.submit')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance } from 'element-plus';
import type { PurchaseReturnItem } from '@/api/purchase-return';

const { t } = useI18n({ useScope: 'global' });

// 采购订单数据结构
interface PurchaseOrder {
  id: number;
  order_no: string;
}

// 产品数据结构
interface Product {
  id: number;
  name: string;
  price: number;
}

// 表单数据类型（所有字段可选，兼容 Partial<PurchaseReturn>）
interface FormDataType {
  id?: number | undefined;
  purchaseOrderId?: number | undefined;
  returnDate?: string;
  reason?: string;
  remarks?: string;
  items?: Partial<PurchaseReturnItem>[];
}

// 表单校验规则类型
interface FormRules {
  purchaseOrderId: Array<{ required: boolean; message: string; trigger: string }>;
  returnDate: Array<{ required: boolean; message: string; trigger: string }>;
  reason: Array<{ required: boolean; message: string; trigger: string }>;
}

const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 是否编辑模式
  isEdit: boolean;
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: FormDataType;
  // 表单校验规则
  formRules: FormRules;
  // 采购订单列表
  purchaseOrders: PurchaseOrder[];
  // 产品列表
  products: Product[];
  // 提交中
  submitting?: boolean;
}>();

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void;
  // 提交
  (e: 'submit'): void;
  // 采购订单变化
  (e: 'order-change', orderId: number): void;
  // 产品变化
  (e: 'product-change', row: Partial<PurchaseReturnItem>, productId: number): void;
  // 添加明细
  (e: 'add-item'): void;
  // 删除明细
  (e: 'remove-item', index: number): void;
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  (e: 'update:formData', formData: FormDataType): void;
}>();

// 表单引用
const formRef = ref<FormInstance>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 items 数组，需要深拷贝以保证本地修改与父组件解耦
const localFormData = ref<FormDataType>(deepClone(props.formData));

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件编辑/新建时填充数据）
watch(
  () => props.formData,
  newData => {
    if (syncing) return;
    syncing = true;
    localFormData.value = deepClone(newData);
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localFormData,
  newData => {
    if (syncing) return;
    syncing = true;
    emit('update:formData', deepClone(newData));
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};

/** 取消 */
const onCancel = () => {
  emit('update:visible', false);
};

/** 提交 */
const onSubmit = async () => {
  await formRef.value?.validate();
  emit('submit');
};

/** 采购订单变化 */
const onOrderChange = (orderId: number) => {
  emit('order-change', orderId);
};

/** 产品变化 */
const onProductChange = (row: Partial<PurchaseReturnItem>, productId: number) => {
  emit('product-change', row, productId);
};

/** 添加明细 */
const onAddItem = () => {
  emit('add-item');
};

/** 删除明细 */
const onRemoveItem = (index: number) => {
  emit('remove-item', index);
};
</script>

<style scoped>
.mb-10 {
  margin-bottom: 10px;
}
</style>
