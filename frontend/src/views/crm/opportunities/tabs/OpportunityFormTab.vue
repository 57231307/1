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
              v-if="estimatedAmountViewable"
              v-model="formData.estimated_amount"
              :precision="2"
              :min="0"
              style="width: 100%"
            />
            <!-- 2026-10-02 裁定：金额对"仅非本人行"不外显（后端整键移除）；
                 无权时只给中性占位，不显示任何判定原因 -->
            <span v-else class="amount-hidden-text">{{
              t('crmOpportunityForm.estimatedAmountHidden')
            }}</span>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmOpportunityForm.winProbability')" prop="win_probability">
            <el-slider
              v-model="formData.win_probability"
              :min="0"
              :max="100"
              @change="winProbabilityTouched = true"
            />
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
  // 出参真实契约：Decimal = JSON 字符串、可空；el-input-number 需数值，回填时归一
  estimated_amount: 0 as number | undefined,
  // 赢率后端权威（services/crm/opp.rs::default_win_probability_by_stage）：未上送时创建按
  // 阶段默认填充、阶段流转按新阶段默认刷新；undefined = 未设置（新建无既有值/库中无值）。
  // 前端绝不上送占位值——显式上送会覆盖后端按阶段算出的赢率。
  win_probability: undefined as number | undefined,
  expected_close_date: '',
  product_desc: '',
});

// 赢率是否由用户显式拨动过滑块（change 事件）：仅此为真时才参与提交判定，
// 未拨动一律省略该键，把赢率决定权留给后端阶段口径
const winProbabilityTouched = ref(false);

// 2026-10-02 裁定（金额"仅非本人行"不外显）在编辑表单的落地判据：
// - 行数据缺 estimated_amount 键 = 本入口不外显金额 → 中性占位、不可编辑；
// - 键在但为 null = 库中无值 → 空输入框。
// 提交遵循"未改动即不下发"：无权/未改/被清空（后端 Option=None 语义即不修改）
// 都省略 estimated_amount，绝不以默认值或空值覆盖库中真实金额。
const estimatedAmountViewable = ref(true);
const originalEstimatedAmount = ref<number | undefined>(undefined);

// Decimal 字符串 → number 归一（禁止对字符串 .toFixed()）；出参形状漂移（非数值）
// 按"无值"对待：不回填、且因非有限值不参与下发，不会把伪形写回库
const normalizeAmount = (raw: unknown): number | undefined => {
  if (raw === null || raw === undefined || raw === '') return undefined;
  const n = Number(raw);
  return Number.isFinite(n) ? n : undefined;
};

// 赢率列同为 Decimal 出参（JSON 字符串/null），el-slider 需数值：与 normalizeAmount 同形归一，
// null/缺失/伪形按"未设置"对待——不回填出假数值，也不在未拨动时下发
const normalizePercent = normalizeAmount;

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
      estimatedAmountViewable.value = true;
      originalEstimatedAmount.value = undefined;
      if (props.rowData) {
        Object.assign(formData, props.rowData);
        // win_probability 行原文是 Decimal 出参（JSON 字符串）或 null，控件需数值：如实归一回填
        formData.win_probability = normalizePercent(props.rowData.win_probability);
        // 金额键存在与否 = 字段级权限是否外显（后端对非本人行整键移除 estimated_amount）；
        // 先记录可见性与归一后的原值，提交时按"未改动即不下发"处理
        const hasAmount = 'estimated_amount' in props.rowData;
        estimatedAmountViewable.value = hasAmount;
        const normalized = hasAmount ? normalizeAmount(props.rowData.estimated_amount) : undefined;
        originalEstimatedAmount.value = normalized;
        formData.estimated_amount = normalized;
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
  formData.win_probability = undefined;
  winProbabilityTouched.value = false;
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
      expected_close_date: formData.expected_close_date || undefined,
      product_desc: formData.product_desc || undefined,
    };
    // 赢率只在用户显式拨动滑块后如实上送当前值；未拨动一律省略该键——
    // 创建时后端按阶段默认填充，编辑改阶段时后端按新阶段默认刷新，
    // 占位值/回填旧值下发都会把这两条后端口径盖掉
    const winProbabilityToSend =
      winProbabilityTouched.value && typeof formData.win_probability === 'number'
        ? formData.win_probability
        : undefined;
    if (formData.id) {
      const payload: OpportunityUpdateInput = { ...fields };
      // 2026-10-02 裁定（金额"仅非本人行"不外显）编辑侧落地：
      // 无权（键被移除）、金额未改动、或被清空（后端 Option=None 即"不修改"）一律
      // 省略 estimated_amount——绝不以默认值/空值覆盖库中真实金额，也不发 null 假装清除。
      if (
        estimatedAmountViewable.value &&
        typeof formData.estimated_amount === 'number' &&
        Number.isFinite(formData.estimated_amount) &&
        formData.estimated_amount !== originalEstimatedAmount.value
      ) {
        payload.estimated_amount = formData.estimated_amount;
      }
      if (winProbabilityToSend !== undefined) {
        payload.win_probability = winProbabilityToSend;
      }
      await updateOpportunity(formData.id, payload);
    } else {
      // 新建无既有金额可覆盖：按表单现值下发（清空即不下发，落库为无值）
      const payload: OpportunityCreateInput = {
        ...fields,
        estimated_amount:
          typeof formData.estimated_amount === 'number' &&
          Number.isFinite(formData.estimated_amount)
            ? formData.estimated_amount
            : undefined,
        win_probability: winProbabilityToSend,
      };
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
