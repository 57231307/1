<!--
  CustomerFormTab.vue - 客户新建/编辑对话框
  来源：原 customer/index.vue 中 新建/编辑对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog
    v-model="visible"
    :title="title"
    width="700px"
    :close-on-click-modal="false"
    :aria-label="t('customer.form.ariaLabel')"
    @close="handleClose"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="120px"
      :aria-label="t('customer.form.formAriaLabel')"
    >
      <el-divider content-position="left">{{ t('customer.form.section.basic') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.customerCode')" prop="customer_code">
            <el-input
              v-model="formData.customer_code"
              :placeholder="t('customer.form.placeholder.customerCode')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.customerName')" prop="customer_name">
            <el-input
              v-model="formData.customer_name"
              :placeholder="t('customer.form.placeholder.customerName')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.contactPerson')" prop="contact_person">
            <el-input
              v-model="formData.contact_person"
              :placeholder="t('customer.form.placeholder.contactPerson')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.contactPhone')" prop="contact_phone">
            <el-input
              v-model="formData.contact_phone"
              :placeholder="t('customer.form.placeholder.contactPhone')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.email')" prop="contact_email">
            <el-input
              v-model="formData.contact_email"
              :placeholder="t('customer.form.placeholder.email')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.customerType')" prop="customer_type">
            <el-select
              v-model="formData.customer_type"
              :placeholder="t('customer.form.placeholder.customerType')"
              style="width: 100%"
            >
              <!-- value 一律是后端唯一词表 constants::customer_type::ALLOWED 的渠道 token
                   （小写精确匹配，不 trim、不归一大小写）；分层词 vip/normal 不属本列，
                   提交即被 400 VALIDATION_ERROR 拒绝。五个合法值全列，缺一即建不出该类客户。 -->
              <el-option :label="t('customer.form.option.typeRetail')" value="retail" />
              <el-option :label="t('customer.form.option.typeWholesale')" value="wholesale" />
              <el-option :label="t('customer.form.option.typeDistributor')" value="distributor" />
              <el-option :label="t('customer.form.option.typeManufacturer')" value="manufacturer" />
              <el-option :label="t('customer.form.option.typeOther')" value="other" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.industry')" prop="customer_industry">
            <el-input
              v-model="formData.customer_industry"
              :placeholder="t('customer.form.placeholder.industry')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.annualPurchase')" prop="annual_purchase">
            <el-input-number
              v-model="formData.annual_purchase"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('customer.form.section.address') }}</el-divider>
      <el-form-item :label="t('customer.form.label.address')" prop="address">
        <el-input
          v-model="formData.address"
          :placeholder="t('customer.form.placeholder.address')"
        />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.province')" prop="province">
            <el-input
              v-model="formData.province"
              :placeholder="t('customer.form.placeholder.province')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.city')" prop="city">
            <el-input v-model="formData.city" :placeholder="t('customer.form.placeholder.city')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.postalCode')" prop="postal_code">
            <el-input
              v-model="formData.postal_code"
              :placeholder="t('customer.form.placeholder.postalCode')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.country')" prop="country">
            <el-input
              v-model="formData.country"
              :placeholder="t('customer.form.placeholder.country')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('customer.form.section.finance') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.taxId')" prop="tax_id">
            <el-input
              v-model="formData.tax_id"
              :placeholder="t('customer.form.placeholder.taxId')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.creditLimit')" prop="credit_limit">
            <el-input-number
              v-model="formData.credit_limit"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.paymentTerms')" prop="payment_terms">
            <el-input-number v-model="formData.payment_terms" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.status')" prop="status">
            <el-radio-group v-model="formData.status">
              <el-radio value="active">{{ t('customer.form.status.active') }}</el-radio>
              <el-radio value="inactive">{{ t('customer.form.status.inactive') }}</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.bankName')" prop="bank_name">
            <el-input
              v-model="formData.bank_name"
              :placeholder="t('customer.form.placeholder.bankName')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.bankAccount')" prop="bank_account">
            <el-input
              v-model="formData.bank_account"
              :placeholder="t('customer.form.placeholder.bankAccount')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('customer.form.section.business') }}</el-divider>
      <el-form-item :label="t('customer.form.label.mainProducts')" prop="main_products">
        <el-input
          v-model="formData.main_products"
          :placeholder="t('customer.form.placeholder.mainProducts')"
        />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('customer.form.label.qualityRequirement')"
            prop="quality_requirement"
          >
            <el-input
              v-model="formData.quality_requirement"
              :placeholder="t('customer.form.placeholder.qualityRequirement')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('customer.form.label.inspectionStandard')"
            prop="inspection_standard"
          >
            <el-input
              v-model="formData.inspection_standard"
              :placeholder="t('customer.form.placeholder.inspectionStandard')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('customer.form.label.notes')" prop="notes">
        <el-input
          v-model="formData.notes"
          type="textarea"
          :rows="3"
          :placeholder="t('customer.form.placeholder.notes')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('customer.form.button.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('customer.form.button.save')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import type { Customer } from '@/api/customer';
import { createCustomer, updateCustomer } from '@/api/customer';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  title: string;
  rowData: Partial<Customer> | null;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const visible = ref(props.modelValue);
const submitLoading = ref(false);
const formRef = ref<FormInstance>();

// 客户类型（渠道）缺省值与后端 `constants::customer_type::validate(None)` 的缺省口径同源：
// 后端在请求未提供该字段时补 `OTHER`（语义=渠道未知）。前端未选时若替业务方断言"零售"
// 会污染渠道维度的下游筛选与统计，故初值/reset 一律写 `other`，不写 `retail`。
// 该字段后端 DTO 为 Option、无必填门（`validate_customer_type` 只在 Some 时校验取值），
// 故此处维持同等校验强度，不额外加前端必填规则。
// 显式标注 string：const 字面量的类型是 `"other"`，直接喂给 reactive 会让 customer_type
// 收窄成该单一 token，下拉改选其它渠道时模板赋值过不了 vue-tsc。
const DEFAULT_CUSTOMER_TYPE: string = 'other';

const formData = reactive({
  id: undefined as number | undefined,
  customer_code: '',
  customer_name: '',
  contact_person: '',
  contact_phone: '',
  contact_email: '',
  address: '',
  city: '',
  province: '',
  country: '',
  postal_code: '',
  customer_type: DEFAULT_CUSTOMER_TYPE,
  tax_id: '',
  credit_limit: 0,
  payment_terms: 30,
  bank_name: '',
  bank_account: '',
  status: 'active',
  notes: '',
  customer_industry: '',
  main_products: '',
  annual_purchase: 0,
  quality_requirement: '',
  inspection_standard: '',
});

const formRules: FormRules = {
  customer_code: [
    {
      required: true,
      message: t('customer.form.validation.customerCodeRequired'),
      trigger: 'blur',
    },
  ],
  customer_name: [
    {
      required: true,
      message: t('customer.form.validation.customerNameRequired'),
      trigger: 'blur',
    },
  ],
  contact_person: [
    {
      required: true,
      message: t('customer.form.validation.contactPersonRequired'),
      trigger: 'blur',
    },
  ],
  contact_phone: [
    {
      required: true,
      message: t('customer.form.validation.contactPhoneRequired'),
      trigger: 'blur',
    },
    {
      pattern: /^1[3-9]\d{9}$/,
      message: t('customer.form.validation.contactPhoneInvalid'),
      trigger: 'blur',
    },
  ],
};

watch(
  () => props.modelValue,
  val => {
    visible.value = val;
    if (val) {
      resetForm();
      if (props.rowData) {
        Object.assign(formData, props.rowData);
      }
    }
  }
);

watch(visible, val => {
  emit('update:modelValue', val);
});

const resetForm = () => {
  formData.id = undefined;
  formData.customer_code = '';
  formData.customer_name = '';
  formData.contact_person = '';
  formData.contact_phone = '';
  formData.contact_email = '';
  formData.address = '';
  formData.city = '';
  formData.province = '';
  formData.country = '';
  formData.postal_code = '';
  formData.customer_type = DEFAULT_CUSTOMER_TYPE;
  formData.tax_id = '';
  formData.credit_limit = 0;
  formData.payment_terms = 30;
  formData.bank_name = '';
  formData.bank_account = '';
  formData.status = 'active';
  formData.notes = '';
  formData.customer_industry = '';
  formData.main_products = '';
  formData.annual_purchase = 0;
  formData.quality_requirement = '';
  formData.inspection_standard = '';
  formRef.value?.clearValidate();
};

const handleClose = () => {
  resetForm();
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
    submitLoading.value = true;
    // 载荷键集逐键对齐后端 customer_handler.rs：CreateCustomerRequest(:23-72)/
    // UpdateCustomerRequest(:93-139)。此前整表单 rest 展开把 id/created_at/updated_at
    // 只读生成列（以及更新侧的 customer_code——编辑 DTO 无此键，编码建档即定）
    // 冒充可写字段提交，后端 serde 静默丢弃=契约漂移，现按 DTO 显式逐键构造。
    // contact_email 空则省略键（后端 #[validate(email)] 对 Some("") 判失败触发 422、
    // 对 None 跳过校验）；credit_limit 后端 DTO 为字符串（Option<String>，number 即 422），
    // 提交前转字符串防 422。
    const { contact_email, credit_limit } = formData;
    const payload = {
      customer_name: formData.customer_name,
      contact_person: formData.contact_person,
      contact_phone: formData.contact_phone,
      address: formData.address,
      city: formData.city,
      province: formData.province,
      postal_code: formData.postal_code,
      credit_limit: String(credit_limit ?? '0'),
      payment_terms: formData.payment_terms,
      tax_id: formData.tax_id,
      bank_name: formData.bank_name,
      bank_account: formData.bank_account,
      customer_type: formData.customer_type,
      country: formData.country,
      status: formData.status,
      customer_industry: formData.customer_industry,
      main_products: formData.main_products,
      annual_purchase: formData.annual_purchase,
      quality_requirement: formData.quality_requirement,
      inspection_standard: formData.inspection_standard,
      notes: formData.notes,
      ...(contact_email ? { contact_email } : {}),
    };
    if (formData.id) {
      await updateCustomer(formData.id, payload);
      ElMessage.success(t('customer.form.message.saveSuccess'));
    } else {
      // customer_code 仅创建 DTO 有（Option，留空则省略键、由服务端建档规则生成）
      await createCustomer({
        ...payload,
        ...(formData.customer_code ? { customer_code: formData.customer_code } : {}),
      });
      ElMessage.success(t('customer.form.message.saveSuccess'));
    }
    visible.value = false;
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('customer.form.message.validationFailed'));
    logger.warn(t('customer.form.message.validationFailed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
