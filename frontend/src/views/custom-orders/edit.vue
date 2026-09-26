<!--
  定制订单编辑页（/custom-orders/:id/edit）
  与创建页同构：加载已有订单回填表单，提交走更新接口，
  仅草稿状态可编辑（入口由详情页编辑按钮控制）
-->
<template>
  <div class="custom-order-edit">
    <el-card>
      <template #header>
        <div class="card-header">
          <span class="title">{{ t('customOrders.create.title') }}</span>
          <el-button @click="router.push(`/custom-orders/${orderId}`)">{{
            t('customOrders.create.buttonBack')
          }}</el-button>
        </div>
      </template>

      <el-form
        ref="formRef"
        v-loading="loading"
        :model="form"
        :rules="rules"
        label-width="120px"
        :aria-label="t('customOrders.create.formAriaLabel')"
      >
        <el-form-item :label="t('customOrders.create.labelCustomer')" prop="customer_id">
          <el-input-number
            v-model="form.customer_id"
            :min="1"
            :placeholder="t('customOrders.create.placeholderCustomer')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelProduct')" prop="product_id">
          <el-input-number
            v-model="form.product_id"
            :min="1"
            :placeholder="t('customOrders.create.placeholderProduct')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelColor')">
          <el-input-number
            v-model="form.color_id"
            :min="1"
            :placeholder="t('customOrders.create.placeholderColor')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelSpec')" prop="spec">
          <el-input v-model="form.spec" :placeholder="t('customOrders.create.placeholderSpec')" />
        </el-form-item>
        <el-form-item
          :label="t('customOrders.create.labelQuantity')"
          prop="quantity"
          for="co-edit-quantity"
        >
          <el-input-number
            id="co-edit-quantity"
            v-model="form.quantity"
            :min="0.01"
            :precision="2"
            :step="1"
          />
          <el-select v-model="form.unit" style="width: 100px; margin-left: 8px">
            <el-option :label="t('customOrders.create.unitMeter')" value="m" />
            <el-option :label="t('customOrders.create.unitKilogram')" value="kg" />
            <el-option :label="t('customOrders.create.unitPiece')" value="pcs" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelYarnSpec')">
          <el-input
            v-model="form.yarn_spec"
            :placeholder="t('customOrders.create.placeholderYarnSpec')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelDyeMethod')">
          <el-select
            v-model="form.dye_method"
            clearable
            :placeholder="t('customOrders.create.placeholderDyeMethod')"
          >
            <el-option :label="t('customOrders.create.dyeMethodReactive')" value="reactive" />
            <el-option :label="t('customOrders.create.dyeMethodDisperse')" value="disperse" />
            <el-option :label="t('customOrders.create.dyeMethodVat')" value="vat" />
            <el-option :label="t('customOrders.create.dyeMethodAcid')" value="acid" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelFinishingMethod')">
          <el-select
            v-model="form.finishing_method"
            clearable
            :placeholder="t('customOrders.create.placeholderFinishingMethod')"
          >
            <el-option :label="t('customOrders.create.finishingSoftening')" value="softening" />
            <el-option :label="t('customOrders.create.finishingWaterproof')" value="waterproof" />
            <el-option
              :label="t('customOrders.create.finishingFlameRetardant')"
              value="flame_retardant"
            />
            <el-option :label="t('customOrders.create.finishingEasyCare')" value="easy_care" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelExpectedDelivery')">
          <el-date-picker
            v-model="form.expected_delivery_date"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="t('customOrders.create.placeholderExpectedDelivery')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelTotalAmount')" for="co-edit-total-amount">
          <el-input-number
            id="co-edit-total-amount"
            v-model="form.total_amount"
            :min="0"
            :precision="2"
            :step="100"
          />
          <el-select v-model="form.currency" style="width: 100px; margin-left: 8px">
            <el-option label="CNY" value="CNY" />
            <el-option label="USD" value="USD" />
            <el-option label="EUR" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelSalesOrder')">
          <el-input-number
            v-model="form.sales_order_id"
            :min="1"
            :placeholder="t('customOrders.create.placeholderSalesOrder')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelCustomReq')">
          <el-input
            v-model="customReqText"
            type="textarea"
            :rows="3"
            :placeholder="t('customOrders.create.placeholderCustomReq')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelNotes')">
          <el-input
            v-model="form.notes"
            type="textarea"
            :rows="2"
            :placeholder="t('customOrders.create.placeholderNotes')"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">{{
            t('common.save')
          }}</el-button>
          <el-button @click="router.push(`/custom-orders/${orderId}`)">{{
            t('customOrders.create.buttonCancel')
          }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { getCustomOrder, updateCustomOrder } from '@/api/custom-order';

const route = useRoute();
const router = useRouter();
const { t } = useI18n({ useScope: 'global' });
const formRef = ref();
const submitting = ref(false);
const loading = ref(false);
const customReqText = ref('');
const orderId = Number(route.params.id);

const form = ref({
  customer_id: undefined as number | undefined,
  product_id: undefined as number | undefined,
  color_id: undefined as number | undefined,
  spec: '',
  quantity: 1,
  unit: 'm',
  yarn_spec: '',
  dye_method: '',
  finishing_method: '',
  expected_delivery_date: '',
  total_amount: undefined as number | undefined,
  currency: 'CNY',
  sales_order_id: undefined as number | undefined,
  notes: '',
});

const rules = {
  customer_id: [
    {
      required: true,
      message: t('customOrders.create.validationCustomerRequired'),
      trigger: 'blur',
    },
  ],
  product_id: [
    {
      required: true,
      message: t('customOrders.create.validationProductRequired'),
      trigger: 'blur',
    },
  ],
  spec: [
    { required: true, message: t('customOrders.create.validationSpecRequired'), trigger: 'blur' },
  ],
  quantity: [
    {
      required: true,
      message: t('customOrders.create.validationQuantityRequired'),
      trigger: 'blur',
    },
  ],
};

async function handleSubmit() {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
  } catch {
    return;
  }

  submitting.value = true;
  try {
    const custom_requirements = customReqText.value ? { note: customReqText.value } : null;

    // 表单验证通过后 narrowing 必填字段，满足更新 DTO 类型
    if (!form.value.customer_id || !form.value.product_id) {
      throw new Error(t('customOrders.create.messageCustomerProductRequired'));
    }
    // 后端 expected_delivery_date 为日期类型，空串 "" 反序列化失败，空串时不携带该字段
    const { expected_delivery_date, ...restForm } = form.value;
    const payload = {
      ...restForm,
      customer_id: form.value.customer_id,
      product_id: form.value.product_id,
      ...(expected_delivery_date ? { expected_delivery_date } : {}),
      custom_requirements,
    };
    await updateCustomOrder(orderId, payload);
    ElMessage.success(t('common.save') + ' ✅');
    router.push(`/custom-orders/${orderId}`);
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    ElMessage.error(msg || t('common.save') + ' 失败');
  } finally {
    submitting.value = false;
  }
}

onMounted(async () => {
  loading.value = true;
  try {
    const res = await getCustomOrder(orderId);
    const detail = res.data;
    if (detail) {
      form.value = {
        customer_id: detail.customer_id,
        product_id: detail.product_id,
        color_id: detail.color_id,
        spec: detail.spec || '',
        quantity: Number(detail.quantity ?? 1),
        unit: detail.unit || 'm',
        yarn_spec: detail.yarn_spec || '',
        dye_method: detail.dye_method || '',
        finishing_method: detail.finishing_method || '',
        expected_delivery_date: detail.expected_delivery_date?.slice(0, 10) || '',
        total_amount: detail.total_amount ? Number(detail.total_amount) : undefined,
        currency: detail.currency || 'CNY',
        sales_order_id: detail.sales_order_id,
        notes: detail.notes || '',
      };
      // 定制要求 JSONB 回填（note 字段）
      const req = detail.custom_requirements as { note?: string } | null | undefined;
      customReqText.value = req?.note || '';
    }
  } finally {
    loading.value = false;
  }
});
</script>

<style scoped>
.custom-order-edit {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
</style>
