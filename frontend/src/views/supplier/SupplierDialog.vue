<!--
  SupplierDialog.vue - 供应商新建/编辑/查看对话框
  来源：原 supplier/index.vue 中 弹窗表单区（line 43-197）
  拆分日期：2026-06-22 P9-3 批次 E 样板 2
  拆分目的：supplier/index.vue 458 行 → 约 290 行（主文件）+ 本子组件 ~230 行
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    :aria-label="title"
    width="800px"
    :close-on-click-modal="false"
    @update:model-value="onVisibleChange"
    @close="emit('close')"
  >
    <el-form
      ref="formRef"
      :model="localFormData"
      :rules="formRules"
      label-width="120px"
      :aria-label="t('supplier.dialog.formAriaLabel')"
    >
      <el-divider content-position="left">{{ t('supplier.dialog.section.basic') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.supplierCode')" prop="supplier_code">
            <el-input
              v-model="localFormData.supplier_code"
              :placeholder="t('supplier.dialog.placeholder.supplierCode')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.supplierName')" prop="supplier_name">
            <el-input
              v-model="localFormData.supplier_name"
              :placeholder="t('supplier.dialog.placeholder.supplierName')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.shortName')" prop="supplier_short_name">
            <el-input
              v-model="localFormData.supplier_short_name"
              :placeholder="t('supplier.dialog.placeholder.shortName')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.supplierType')" prop="supplier_type">
            <el-select
              v-model="localFormData.supplier_type"
              :placeholder="t('supplier.dialog.placeholder.supplierType')"
              style="width: 100%"
            >
              <el-option :label="t('supplier.dialog.option.manufacturer')" value="manufacturer" />
              <el-option :label="t('supplier.dialog.option.distributor')" value="distributor" />
              <el-option :label="t('supplier.dialog.option.service')" value="service" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.creditCode')" prop="credit_code">
            <el-input
              v-model="localFormData.credit_code"
              :placeholder="t('supplier.dialog.placeholder.creditCode')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('supplier.dialog.label.legalRepresentative')"
            prop="legal_representative"
          >
            <el-input
              v-model="localFormData.legal_representative"
              :placeholder="t('supplier.dialog.placeholder.legalRepresentative')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('supplier.dialog.section.contact') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.contactPhone')" prop="contact_phone">
            <el-input
              v-model="localFormData.contact_phone"
              :placeholder="t('supplier.dialog.placeholder.contactPhone')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.email')" prop="email">
            <el-input
              v-model="localFormData.email"
              :placeholder="t('supplier.dialog.placeholder.email')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.website')" prop="website">
            <el-input
              v-model="localFormData.website"
              :placeholder="t('supplier.dialog.placeholder.website')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.fax')" prop="fax">
            <el-input
              v-model="localFormData.fax"
              :placeholder="t('supplier.dialog.placeholder.fax')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('supplier.dialog.label.registeredAddress')" prop="registered_address">
        <el-input
          v-model="localFormData.registered_address"
          :placeholder="t('supplier.dialog.placeholder.registeredAddress')"
        />
      </el-form-item>
      <el-form-item :label="t('supplier.dialog.label.businessAddress')" prop="business_address">
        <el-input
          v-model="localFormData.business_address"
          :placeholder="t('supplier.dialog.placeholder.businessAddress')"
        />
      </el-form-item>
      <el-divider content-position="left">{{ t('supplier.dialog.section.financial') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.taxpayerType')" prop="taxpayer_type">
            <el-select
              v-model="localFormData.taxpayer_type"
              :placeholder="t('supplier.dialog.placeholder.taxpayerType')"
              style="width: 100%"
            >
              <el-option :label="t('supplier.dialog.option.generalTaxpayer')" value="general" />
              <el-option :label="t('supplier.dialog.option.smallTaxpayer')" value="small" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('supplier.dialog.label.registeredCapital')"
            prop="registered_capital"
          >
            <el-input-number
              v-model="localFormData.registered_capital"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.bankName')" prop="bank_name">
            <el-input
              v-model="localFormData.bank_name"
              :placeholder="t('supplier.dialog.placeholder.bankName')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.bankAccount')" prop="bank_account">
            <el-input
              v-model="localFormData.bank_account"
              :placeholder="t('supplier.dialog.placeholder.bankAccount')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('supplier.dialog.section.business') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.grade')" prop="grade">
            <el-select
              v-model="localFormData.grade"
              :placeholder="t('supplier.dialog.placeholder.grade')"
              style="width: 100%"
            >
              <el-option :label="t('supplier.dialog.option.gradeA')" value="A" />
              <el-option :label="t('supplier.dialog.option.gradeB')" value="B" />
              <el-option :label="t('supplier.dialog.option.gradeC')" value="C" />
              <el-option :label="t('supplier.dialog.option.gradeD')" value="D" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('supplier.dialog.label.status')" prop="status">
            <el-radio-group v-model="localFormData.status">
              <el-radio value="active">{{ t('supplier.dialog.option.statusActive') }}</el-radio>
              <el-radio value="inactive">{{ t('supplier.dialog.option.statusInactive') }}</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('supplier.dialog.label.mainBusiness')" prop="main_business">
        <el-input
          v-model="localFormData.main_business"
          :placeholder="t('supplier.dialog.placeholder.mainBusiness')"
        />
      </el-form-item>
      <el-form-item :label="t('supplier.dialog.label.remarks')" prop="remarks">
        <el-input
          v-model="localFormData.remarks"
          type="textarea"
          :rows="3"
          :placeholder="t('supplier.dialog.placeholder.remarks')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">{{ t('supplier.dialog.button.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" :disabled="readonly" @click="onSubmit">{{
        t('supplier.dialog.button.save')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance, FormRules } from 'element-plus';

const { t } = useI18n({ useScope: 'global' });

// 表单数据结构（与 supplier/index.vue 中 formData 完全一致）
interface SupplierFormData {
  id: number | undefined;
  supplier_code: string;
  supplier_name: string;
  supplier_short_name: string;
  supplier_type: string;
  credit_code: string;
  registered_address: string;
  business_address: string;
  legal_representative: string;
  registered_capital: number;
  contact_phone: string;
  fax: string;
  website: string;
  email: string;
  main_business: string;
  taxpayer_type: string;
  bank_name: string;
  bank_account: string;
  grade: string;
  status: string;
  remarks: string;
}

// 默认空表单（用于 reset）
const emptyForm = (): SupplierFormData => ({
  id: undefined,
  supplier_code: '',
  supplier_name: '',
  supplier_short_name: '',
  supplier_type: '',
  credit_code: '',
  registered_address: '',
  business_address: '',
  legal_representative: '',
  registered_capital: 0,
  contact_phone: '',
  fax: '',
  website: '',
  email: '',
  main_business: '',
  taxpayer_type: '',
  bank_name: '',
  bank_account: '',
  grade: '',
  status: 'active',
  remarks: '',
});

const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 标题
  title: string;
  // 模式：add / edit / view
  mode: 'add' | 'edit' | 'view';
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: SupplierFormData;
  // 提交 loading
  submitLoading: boolean;
}>();

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean];
  // 关闭后（用于父组件 reset）
  close: [];
  // 提交表单
  submit: [];
  // 整体回写表单
  'update:formData': [formData: SupplierFormData];
}>();

// 表单引用
const formRef = ref<FormInstance>();

// 只读模式（view 模式禁用保存按钮）
const readonly = computed(() => props.mode === 'view');

// 表单校验规则
const formRules = computed<FormRules>(() => ({
  supplier_code: [
    {
      required: true,
      message: t('supplier.dialog.validation.supplierCodeRequired'),
      trigger: 'blur',
    },
  ],
  supplier_name: [
    {
      required: true,
      message: t('supplier.dialog.validation.supplierNameRequired'),
      trigger: 'blur',
    },
  ],
  contact_phone: [
    {
      required: true,
      message: t('supplier.dialog.validation.contactPhoneRequired'),
      trigger: 'blur',
    },
    {
      pattern: /^1[3-9]\d{9}$/,
      message: t('supplier.dialog.validation.phoneFormat'),
      trigger: 'blur',
    },
  ],
}));

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFormData = ref<SupplierFormData>({ ...props.formData });

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local
watch(
  () => props.formData,
  newForm => {
    if (syncing) return;
    syncing = true;
    localFormData.value = { ...newForm };
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件
watch(
  localFormData,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:formData', { ...newForm });
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

/** 取消按钮 */
const onCancel = () => {
  emit('update:visible', false);
};

/** 提交按钮（触发父组件 validate + save） */
const onSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    emit('submit');
  });
};

// 暴露 reset 方法供父组件调用（通过 defineExpose）
/** 重置表单到初始状态 */
const resetForm = () => {
  localFormData.value = emptyForm();
  formRef.value?.clearValidate();
};

defineExpose({ resetForm });
</script>
