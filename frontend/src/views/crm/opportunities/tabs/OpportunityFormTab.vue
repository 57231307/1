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
              <el-option
                :label="t('crmOpportunityForm.stageOption.qualification')"
                :value="OPPORTUNITY_STAGE.QUALIFICATION"
              />
              <el-option
                :label="t('crmOpportunityForm.stageOption.needs_analysis')"
                :value="OPPORTUNITY_STAGE.NEEDS_ANALYSIS"
              />
              <el-option
                :label="t('crmOpportunityForm.stageOption.proposal')"
                :value="OPPORTUNITY_STAGE.PROPOSAL"
              />
              <el-option
                :label="t('crmOpportunityForm.stageOption.negotiation')"
                :value="OPPORTUNITY_STAGE.NEGOTIATION"
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
              value-format="YYYY-MM-DD"
              :placeholder="t('crmOpportunityForm.expectedCloseDatePlaceholder')"
              style="width: 100%"
            />
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
import {
  createOpportunity,
  updateOpportunity,
  type OpportunityCreateInput,
  type OpportunityUpdateInput,
} from '@/api/crm';
import type { Customer } from '@/api/customer';
import { OPPORTUNITY_STAGE } from '@/utils/crm-status';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  title: string;
  rowData: Partial<Opportunity> | null;
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
  product_desc: '',
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
  formData.product_desc = '';
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
    submitLoading.value = true;
    // 载荷 = 后端 CreateOpportunityRequest/UpdateOpportunityRequest 真实键集（crm_dto.rs:50-69/146-164）。
    // 负责人不在本表单落库范围：services/crm/opp.rs:101 创建时以登录用户为 owner，DTO 无 owner_id/remarks 键。
    // expected_close_date 为 el-date-picker（后端 Option<NaiveDate>）：手输/未选时 v-model 为空串，
    // 空串会被后端 serde 反序列化成非法 NaiveDate → 整单 400，故空串省略该字段。
    const fields = {
      opportunity_name: formData.opportunity_name,
      customer_id: Number(formData.customer_id),
      opportunity_type: formData.opportunity_type || undefined,
      opportunity_stage: formData.opportunity_stage || undefined,
      estimated_amount: formData.estimated_amount,
      win_probability: formData.win_probability,
      expected_close_date: formData.expected_close_date || undefined,
      product_desc: formData.product_desc || undefined,
    };
    if (formData.id) {
      const payload: OpportunityUpdateInput = { ...fields };
      await updateOpportunity(formData.id, payload);
    } else {
      const payload: OpportunityCreateInput = { ...fields };
      await createOpportunity(payload);
    }
    ElMessage.success(t('crmOpportunityForm.message.saveSuccess'));
    visible.value = false;
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('crmOpportunityForm.message.validationFailed'));
    logger.warn(t('crmOpportunityForm.message.validationFailed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
