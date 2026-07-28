<!--
  ReturnEditDialog.vue - 销售退货新建/编辑对话框
  任务编号: P14 批 2 I-3 第 7 批
  拆分原 sales-returns/index.vue 的新建/编辑对话框部分
  内部维护 local formData，通过 props.initialData 同步初始值
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="
      dialogMode === 'create'
        ? t('salesReturns.editDialog.titleCreate')
        : t('salesReturns.editDialog.titleEdit')
    "
    width="900px"
    :aria-label="
      dialogMode === 'create'
        ? t('salesReturns.editDialog.titleCreate')
        : t('salesReturns.editDialog.titleEdit')
    "
    @update:model-value="onClose"
    @close="onDialogClose"
  >
    <el-form
      ref="formRef"
      :model="localFormData"
      :rules="formRules"
      label-width="120px"
      :aria-label="t('salesReturns.editDialog.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelSalesOrderNo')" prop="salesOrderId">
            <el-select
              v-model="localFormData.salesOrderId"
              :placeholder="t('salesReturns.editDialog.placeholderSalesOrder')"
              style="width: 100%"
              filterable
              @change="onSalesOrderChange"
            >
              <el-option
                v-for="order in salesOrderList"
                :key="order.id"
                :label="order.order_no"
                :value="order.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelCustomer')" prop="customerId">
            <el-select
              v-model="localFormData.customerId"
              :placeholder="t('salesReturns.editDialog.placeholderCustomer')"
              style="width: 100%"
              filterable
            >
              <el-option
                v-for="customer in customerList"
                :key="customer.id"
                :label="customer.customer_name"
                :value="customer.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelReturnDate')" prop="returnDate">
            <el-date-picker
              v-model="localFormData.returnDate"
              type="date"
              :placeholder="t('salesReturns.editDialog.placeholderReturnDate')"
              style="width: 100%"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelReason')" prop="reason">
            <el-select
              v-model="localFormData.reason"
              :placeholder="t('salesReturns.editDialog.placeholderReason')"
              style="width: 100%"
            >
              <el-option :label="t('salesReturns.editDialog.optionQuality')" value="quality" />
              <el-option :label="t('salesReturns.editDialog.optionQuantity')" value="quantity" />
              <el-option
                :label="t('salesReturns.editDialog.optionSpecification')"
                value="specification"
              />
              <el-option :label="t('salesReturns.editDialog.optionPackaging')" value="packaging" />
              <el-option :label="t('salesReturns.editDialog.optionOther')" value="other" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="24">
          <el-form-item :label="t('salesReturns.editDialog.labelRemarks')" prop="remarks">
            <el-input
              v-model="localFormData.remarks"
              type="textarea"
              :rows="3"
              :placeholder="t('salesReturns.editDialog.placeholderRemarks')"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-divider />

      <el-form-item :label="t('salesReturns.editDialog.labelReturnDetails')">
        <el-button type="primary" size="small" style="margin-bottom: 10px" @click="onAddItem">
          {{ t('salesReturns.editDialog.buttonAddDetail') }}
        </el-button>
        <el-table
          :data="localFormData.items"
          border
          style="width: 100%"
          :aria-label="t('salesReturns.editDialog.detailsTableAriaLabel')"
        >
          <el-table-column :label="t('salesReturns.editDialog.columnProductName')" width="200">
            <template #default="{ row }">
              <el-select
                v-model="row.productId"
                :placeholder="t('salesReturns.editDialog.placeholderProduct')"
                style="width: 100%"
                filterable
              >
                <el-option
                  v-for="product in productList"
                  :key="product.id"
                  :label="product.product_name"
                  :value="product.id"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnQuantity')" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.quantity"
                :min="1"
                :precision="2"
                style="width: 100%"
                @change="onCalculate"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnUnitPrice')" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.unitPrice"
                :min="0"
                :precision="2"
                style="width: 100%"
                @change="onCalculate"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnAmount')" width="120">
            <template #default="{ row }">
              {{ (row.quantity * row.unitPrice).toFixed(2) }}
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnReason')" width="150">
            <template #default="{ row }">
              <el-input
                v-model="row.reason"
                :placeholder="t('salesReturns.editDialog.placeholderReasonShort')"
                size="small"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnAction')" width="80">
            <template #default="{ $index }">
              <el-button type="danger" size="small" @click="onRemoveItem($index)">{{
                t('salesReturns.editDialog.buttonDelete')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-form-item>

      <el-row :gutter="20">
        <el-col :span="12" :offset="12">
          <el-form-item :label="t('salesReturns.editDialog.labelTotalAmount')">
            <el-input-number
              v-model="localFormData.totalAmount"
              :precision="2"
              :disabled="true"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>

    <template #footer>
      <el-button @click="onClose(false)">{{ t('salesReturns.editDialog.buttonCancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="onSubmit">{{
        t('salesReturns.editDialog.buttonConfirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, watch, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance, FormRules } from 'element-plus';
// v11 批次 174 P2-1 修复：从 useSr 导入具体类型替代 any
import type {
  ReturnForm,
  SalesOrderOption,
  CustomerOption,
  ProductOption,
} from '../composables/useSr';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  visible: boolean;
  dialogMode: 'create' | 'edit';
  formData: ReturnForm;
  salesOrderList: SalesOrderOption[];
  customerList: CustomerOption[];
  productList: ProductOption[];
  formRules: FormRules;
  submitLoading: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void;
  (e: 'submit', data: ReturnForm): void;
  (e: 'salesOrderChange', orderId: number): void;
  (e: 'addItem'): void;
  (e: 'removeItem', index: number): void;
  (e: 'calculate'): void;
  (e: 'dialogClose'): void;
}>();

const formRef = ref<FormInstance>();

// 浅拷贝 props.formData 到 local（避免直接修改 prop）
// v11 批次 174 P2-1 修复：reactive<any>({}) 改为 reactive<Partial<ReturnForm>>({})
const localFormData = reactive<Partial<ReturnForm>>({});
watch(
  () => props.formData,
  newVal => {
    // v11 批次 174 P2-1 修复：使用 keyof ReturnForm 断言避免 string 索引错误
    Object.keys(localFormData).forEach(k => {
      delete (localFormData as Record<string, unknown>)[k];
    });
    Object.assign(localFormData, deepClone(newVal));
  },
  { immediate: true, deep: true }
);

const onClose = (val: boolean) => {
  emit('update:visible', val);
};

const onSubmit = () => {
  // v11 批次 174 P2-1 修复：localFormData 是 Partial<ReturnForm>，emit 期望 ReturnForm
  emit('submit', localFormData as ReturnForm);
};

const onSalesOrderChange = (orderId: number) => {
  emit('salesOrderChange', orderId);
};

const onAddItem = () => {
  emit('addItem');
};

const onRemoveItem = (index: number) => {
  emit('removeItem', index);
};

const onCalculate = () => {
  emit('calculate');
};

const onDialogClose = () => {
  emit('dialogClose');
};

defineExpose({ formRef });
</script>
