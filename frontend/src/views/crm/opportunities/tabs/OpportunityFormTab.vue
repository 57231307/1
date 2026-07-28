<!--
  OpportunityFormTab.vue - 商机新建/编辑对话框
  来源：原 crm/opportunities/index.vue 中 新建/编辑对话框
-->
<template>
  <el-dialog
    v-model="visible"
    :title="title"
    width="800px"
    :close-on-click-modal="false"
    :aria-label="title"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="100px"
      :aria-label="t('crmOpportunityForm.ariaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('crmOpportunityForm.opportunityName')" prop="opportunity_name">
            <el-input
              v-model="formData.opportunity_name"
              :placeholder="t('crmOpportunityForm.opportunityNamePlaceholder')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmOpportunityForm.customer')" prop="customer_id">
            <el-select
              v-model="formData.customer_id"
              :placeholder="t('crmOpportunityForm.customerPlaceholder')"
              filterable
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
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('crmOpportunityForm.opportunityType')" prop="opportunity_type">
            <el-select
              v-model="formData.opportunity_type"
              :placeholder="t('crmOpportunityForm.opportunityTypePlaceholder')"
            >
              <el-option :label="t('crmOpportunityForm.opportunityTypeOption.new')" value="NEW" />
              <el-option
                :label="t('crmOpportunityForm.opportunityTypeOption.upsell')"
                value="UPSELL"
              />
              <el-option
                :label="t('crmOpportunityForm.opportunityTypeOption.renewal')"
                value="RENEWAL"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmOpportunityForm.opportunityStage')" prop="opportunity_stage">
            <el-select
              v-model="formData.opportunity_stage"
              :placeholder="t('crmOpportunityForm.opportunityStagePlaceholder')"
            >
              <el-option :label="t('crmOpportunityForm.stageOption.initial')" value="INITIAL" />
              <el-option
                :label="t('crmOpportunityForm.stageOption.requirement')"
                value="REQUIREMENT"
              />
              <el-option :label="t('crmOpportunityForm.stageOption.proposal')" value="PROPOSAL" />
              <el-option
                :label="t('crmOpportunityForm.stageOption.negotiation')"
                value="NEGOTIATION"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('crmOpportunityForm.estimatedAmount')" prop="estimated_amount">
            <el-input-number
              v-model="formData.estimated_amount"
              :precision="2"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmOpportunityForm.winProbability')" prop="win_probability">
            <el-slider v-model="formData.win_probability" :min="0" :max="100" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('crmOpportunityForm.expectedCloseDate')"
            prop="expected_close_date"
          >
            <el-date-picker
              v-model="formData.expected_close_date"
              type="date"
              :placeholder="t('crmOpportunityForm.expectedCloseDatePlaceholder')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmOpportunityForm.owner')" prop="owner_id">
            <el-select
              v-model="formData.owner_id"
              :placeholder="t('crmOpportunityForm.ownerPlaceholder')"
              filterable
            >
              <el-option v-for="u in users" :key="u.id" :label="u.real_name" :value="u.id" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('crmOpportunityForm.productDesc')" prop="product_desc">
        <el-input
          v-model="formData.product_desc"
          type="textarea"
          :rows="3"
          :placeholder="t('crmOpportunityForm.productDescPlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('crmOpportunityForm.remarks')" prop="remarks">
        <el-input
          v-model="formData.remarks"
          type="textarea"
          :rows="2"
          :placeholder="t('crmOpportunityForm.remarksPlaceholder')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('crmOpportunityForm.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('crmOpportunityForm.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import type { Opportunity } from '@/api/crm';
import type { User } from '@/api/user';
import type { Customer } from '@/api/customer';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  title: string;
  rowData: Partial<Opportunity> | null;
  users: User[];
  customers: Customer[];
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

const formData = reactive({
  id: null as number | null,
  opportunity_name: '',
  customer_id: '' as string | number,
  opportunity_type: '',
  opportunity_stage: '',
  estimated_amount: 0,
  win_probability: 50,
  expected_close_date: '',
  owner_id: '' as string | number,
  product_desc: '',
  remarks: '',
});

const formRules: FormRules = {
  opportunity_name: [
    {
      required: true,
      message: t('crmOpportunityForm.validation.opportunityNameRequired'),
      trigger: 'blur',
    },
  ],
  customer_id: [
    {
      required: true,
      message: t('crmOpportunityForm.validation.customerRequired'),
      trigger: 'change',
    },
  ],
  opportunity_stage: [
    {
      required: true,
      message: t('crmOpportunityForm.validation.stageRequired'),
      trigger: 'change',
    },
  ],
  owner_id: [
    {
      required: true,
      message: t('crmOpportunityForm.validation.ownerRequired'),
      trigger: 'change',
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
  formData.id = null;
  formData.opportunity_name = '';
  formData.customer_id = '';
  formData.opportunity_type = '';
  formData.opportunity_stage = '';
  formData.estimated_amount = 0;
  formData.win_probability = 50;
  formData.expected_close_date = '';
  formData.owner_id = '';
  formData.product_desc = '';
  formData.remarks = '';
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
    submitLoading.value = true;
    // 调用由父组件处理实际的保存逻辑（通过 emit）
    ElMessage.success(t('crmOpportunityForm.message.saveSuccess'));
    visible.value = false;
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    logger.warn(t('crmOpportunityForm.message.validationFailed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
