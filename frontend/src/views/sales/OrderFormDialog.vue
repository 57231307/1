<!--
  OrderFormDialog.vue - 销售订单表单对话框
  来源：原 sales/index.vue 中 订单表单 dialog
  拆分日期：2026-06-15 B3-1
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="900px"
    destroy-on-close
    :aria-label="title"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localData"
      :rules="formRules"
      label-width="100px"
      :aria-label="t('sales.orderForm.formAriaLabel')"
    >
      <el-divider content-position="left">{{ t('sales.orderForm.basicInfo') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('sales.orderForm.customer')" prop="customer_id">
            <el-select
              v-model="localData.customer_id"
              :placeholder="t('sales.orderForm.customerPlaceholder')"
              style="width: 100%"
              @change="handleCustomerChange"
            >
              <el-option
                v-for="c in customers"
                :key="c.id"
                :label="c.customer_name"
                :value="c.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('sales.orderForm.orderDate')" prop="order_date">
            <el-date-picker
              v-model="localData.order_date"
              type="date"
              :placeholder="t('sales.orderForm.datePlaceholder')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('sales.orderForm.requiredDate')" prop="required_date">
            <el-date-picker
              v-model="localData.required_date"
              type="date"
              :placeholder="t('sales.orderForm.datePlaceholder')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('sales.orderForm.contactPerson')" prop="contact_person">
            <el-input
              v-model="localData.contact_person"
              :placeholder="t('sales.orderForm.contactPersonPlaceholder')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('sales.orderForm.contactPhone')" prop="contact_phone">
        <el-input
          v-model="localData.contact_phone"
          :placeholder="t('sales.orderForm.contactPhonePlaceholder')"
          style="width: 50%"
        />
      </el-form-item>
      <el-form-item :label="t('sales.orderForm.deliveryAddress')" prop="delivery_address">
        <el-input
          v-model="localData.delivery_address"
          type="textarea"
          :rows="2"
          :placeholder="t('sales.orderForm.deliveryAddressPlaceholder')"
        />
      </el-form-item>

      <el-divider content-position="left">{{ t('sales.orderForm.orderItems') }}</el-divider>
      <div class="order-items">
        <el-table
          :data="localData.items"
          border
          style="width: 100%"
          :aria-label="t('sales.orderForm.itemsTableAriaLabel')"
        >
          <el-table-column :label="t('sales.orderForm.product')" width="200">
            <template #default="{ row, $index }">
              <el-select
                v-model="row.product_id"
                :placeholder="t('sales.orderForm.productPlaceholder')"
                @change="(v: number) => handleProductSelect($index, v)"
              >
                <el-option
                  v-for="p in products"
                  :key="p.id"
                  :label="p.product_name"
                  :value="p.id"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column prop="quantity" :label="t('sales.orderForm.quantity')" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.quantity"
                :min="1"
                size="small"
                @change="() => calculateSubtotal(row)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="unit" :label="t('sales.orderForm.unit')" width="80" />
          <el-table-column prop="unit_price" :label="t('sales.orderForm.unitPrice')" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.unit_price"
                :min="0"
                :precision="2"
                size="small"
                @change="() => calculateSubtotal(row)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="subtotal" :label="t('sales.orderForm.subtotal')" width="120">
            <template #default="{ row }">
              <span class="amount">¥{{ (row.subtotal || 0).toLocaleString() }}</span>
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.orderForm.operation')" width="80">
            <template #default="{ $index }">
              <el-button type="danger" link size="small" @click="removeItem($index)">{{
                t('sales.orderForm.delete')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" plain size="small" style="margin-top: 10px" @click="addItem">
          <el-icon><Plus /></el-icon> {{ t('sales.orderForm.addItem') }}
        </el-button>
      </div>

      <el-divider content-position="left">{{ t('sales.orderForm.otherInfo') }}</el-divider>
      <el-form-item :label="t('sales.orderForm.remark')">
        <el-input
          v-model="localData.remark"
          type="textarea"
          :rows="3"
          :placeholder="t('sales.orderForm.remarkPlaceholder')"
        />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="8">
          <el-form-item :label="t('sales.orderForm.orderTotal')">
            <div class="total-amount">¥{{ calculateTotal().toLocaleString() }}</div>
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item :label="t('sales.orderForm.taxAmount')">
            <div class="total-amount">¥{{ (calculateTotal() * 0.13).toLocaleString() }}</div>
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item :label="t('sales.orderForm.totalWithTax')">
            <div class="total-amount highlight">
              ¥{{ (calculateTotal() * 1.13).toLocaleString() }}
            </div>
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('sales.orderForm.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">{{
        t('sales.orderForm.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, reactive, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import type { Customer } from '@/api/customer';
import type { Product } from '@/api/product';

const { t } = useI18n({ useScope: 'global' });

interface OrderItemForm {
  id: number;
  product_id: number | undefined;
  product_name: string;
  product_code: string;
  quantity: number;
  unit: string;
  unit_price: number;
  subtotal: number;
}

interface OrderForm {
  id?: number;
  customer_id: number | undefined;
  customer_name: string;
  order_date: Date | string;
  required_date: string;
  contact_person: string;
  contact_phone: string;
  delivery_address: string;
  remark: string;
  items: OrderItemForm[];
  total_amount?: number;
}

const props = defineProps<{
  visible: boolean;
  title: string;
  formData: OrderForm;
  customers: Customer[];
  products: Product[];
  submitting?: boolean;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
  submit: [data: OrderForm];
}>();

const formRef = ref<FormInstance>();

// 本地数据副本：避免 vue/no-mutating-props 警告；
// watch 监听 props.formData 变化以同步父组件重置
const localData = reactive<OrderForm>({
  id: 0,
  customer_id: undefined,
  customer_name: '',
  order_date: new Date(),
  required_date: '',
  contact_person: '',
  contact_phone: '',
  delivery_address: '',
  remark: '',
  items: [],
  total_amount: 0,
});

watch(
  () => props.formData,
  newData => {
    Object.assign(localData, deepClone(newData));
  },
  { deep: true, immediate: true }
);

const formRules = computed<FormRules>(() => ({
  customer_id: [
    { required: true, message: t('sales.orderForm.customerRequired'), trigger: 'change' },
  ],
  order_date: [
    { required: true, message: t('sales.orderForm.orderDateRequired'), trigger: 'change' },
  ],
  required_date: [
    { required: true, message: t('sales.orderForm.requiredDateRequired'), trigger: 'change' },
  ],
  contact_person: [
    { required: true, message: t('sales.orderForm.contactPersonRequired'), trigger: 'blur' },
  ],
  contact_phone: [
    { required: true, message: t('sales.orderForm.contactPhoneRequired'), trigger: 'blur' },
    {
      pattern: /^1[3-9]\d{9}$/,
      message: t('sales.orderForm.contactPhoneInvalid'),
      trigger: 'blur',
    },
  ],
  delivery_address: [
    { required: true, message: t('sales.orderForm.deliveryAddressRequired'), trigger: 'blur' },
  ],
}));

const handleCustomerChange = (customerId: number) => {
  const customer = props.customers.find(c => c.id === customerId);
  if (customer) {
    localData.customer_name = customer.customer_name;
  }
};

const handleProductSelect = (index: number, _v: number) => {
  const product = props.products.find(p => p.id === localData.items[index].product_id);
  if (product) {
    localData.items[index].product_name = product.product_name;
    localData.items[index].product_code = product.product_code;
    localData.items[index].unit_price = product.price || 0;
    calculateSubtotal(localData.items[index]);
  }
};

const calculateSubtotal = (item: OrderItemForm) => {
  item.subtotal = (item.quantity || 0) * (item.unit_price || 0);
};

const calculateTotal = () => {
  return localData.items.reduce((sum, item) => sum + (item.subtotal || 0), 0);
};

const addItem = () => {
  localData.items.push({
    id: Date.now(),
    product_id: undefined,
    product_name: '',
    product_code: '',
    quantity: 1,
    unit: t('sales.orderForm.defaultUnit'),
    unit_price: 0,
    subtotal: 0,
  });
};

const removeItem = (index: number) => {
  if (localData.items.length > 1) {
    localData.items.splice(index, 1);
  } else {
    ElMessage.warning(t('sales.orderForm.atLeastOneItem'));
  }
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
    const validItems = localData.items.filter(
      item => item.product_id && item.quantity > 0 && item.unit_price > 0
    );
    if (validItems.length === 0) {
      ElMessage.warning(t('sales.orderForm.addValidItem'));
      return;
    }
    localData.total_amount = calculateTotal();
    emit('submit', localData);
  } catch (error) {
    const err = error as { message?: string };
    if (err.message) {
      ElMessage.error(err.message || t('sales.orderForm.operationFailed'));
    }
  }
};
</script>

<style scoped>
.order-items {
  margin-bottom: 20px;
}
.amount {
  font-weight: 600;
  color: #f56c6c;
}
.total-amount {
  font-size: 20px;
  font-weight: 700;
  color: #303133;
}
.total-amount.highlight {
  color: #f56c6c;
}
</style>
