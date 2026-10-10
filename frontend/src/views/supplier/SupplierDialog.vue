<!--
  SupplierDialog.vue - 供应商新建/编辑/查看对话框
  来源：原 supplier/index.vue 中 弹窗表单区（line 43-197）
  拆分日期：2026-06-22 P9-3 批次 E 样板 2
  拆分目的：supplier/index.vue 458 行 → 约 290 行（主文件）+ 本子组件 ~230 行
  行为完全保持一致（仅结构重构）
  表单数据同步契约：父组件（index.vue）是表单唯一权威源；本组件在对话框打开时
  从 props.formData 向下快照同步一次，用户编辑只向上回写（emit update:formData）。
  禁止 prop↔local 双向回写环 + 全局冷却标志：打开时父组件同步赋好的行数据，
  会被任何"旧快照（含空白表单）异步回写"在 watcher flush 阶段覆盖回空值，
  冷却标志还会吞掉行数据触发的向下同步——编辑弹窗可选字段因此只剩 placeholder。
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
          <el-form-item :label="t('supplier.dialog.label.email')" prop="contact_email">
            <el-input
              v-model="localFormData.contact_email"
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
import { ref, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance, FormRules } from 'element-plus';
// 表单模型与空白态以 ./supplier-form 为单一定义（与父组件 index.vue 共用，禁止两处各写一份）
import type { SupplierFormData } from './supplier-form';

const { t } = useI18n({ useScope: 'global' });

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
  supplier_short_name: [
    {
      validator: (_rule, value, callback) => {
        // 后端 CreateSupplierRequest.supplier_short_name 为 Option + length(min=2,max=100)：
        // 未填写（空串）时由父组件省略该键、validator 不触发，填写则须 2-100 字符，否则撞后端 422。
        if (value && (value.length < 2 || value.length > 100)) {
          callback(new Error(t('supplier.dialog.validation.shortNameLength')));
        } else {
          callback();
        }
      },
      trigger: 'blur',
    },
  ],
  credit_code: [
    {
      validator: (_rule, value, callback) => {
        // 后端 CreateSupplierRequest.credit_code 为 Option + length(equal=18)：
        // 未填写时省略该键，填写则必须恰为 18 位，否则撞后端 422。
        if (value && value.length !== 18) {
          callback(new Error(t('supplier.dialog.validation.creditCodeLength')));
        } else {
          callback();
        }
      },
      trigger: 'blur',
    },
  ],
}));

// 本地表单镜像：避免直接改 prop（vue/no-mutating-props），同步方向为
// 「打开时向下快照 + 编辑时向上回写」的单向环（无向下回灌 watcher，回写不可能成环）。
const localFormData = ref<SupplierFormData>({ ...props.formData });

// 打开即向下同步：此刻父组件已在同一 tick 内同步完成
// 「重置空白 → Object.assign(formData, 行数据)」，props.formData 即最终权威值；
// 每个可编辑字段按后端出参原值如实回填（缺值列由后端如实返回 null/空串，不做 || 兜底改写）。
// 同时清理上次会话遗留的校验红字。
watch(
  () => props.visible,
  open => {
    if (open) {
      localFormData.value = { ...props.formData };
      formRef.value?.clearValidate();
    }
  }
);

// 向上回写：仅把用户编辑结果同步给父组件（父组件提交时读的是它持有的 formData）。
// 打开向下同步触发的这次回写内容与父组件当前值一致，属幂等覆盖，不会洗掉数据。
watch(
  localFormData,
  newForm => {
    emit('update:formData', { ...newForm });
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

// 无 defineExpose：表单状态的清空由父组件（权威源）负责，本组件只在打开时向下同步，
// 不存在需要父组件命令子组件"先清空再赋值"的第二写入方（那正是旧快照竞态的来源）。
</script>
