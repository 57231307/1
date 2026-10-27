<!--
  RatingDialogTab.vue - 客户信用评级对话框
  来源：原 customerCredit/index.vue 中 设置评级对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog
    v-model="visible"
    :title="t('customerCredit.rating.title')"
    width="500px"
    :aria-label="t('customerCredit.rating.ariaLabel')"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="120px"
      :aria-label="t('customerCredit.rating.formAriaLabel')"
    >
      <el-form-item :label="t('customerCredit.rating.label.customer')" prop="customer_id">
        <el-select
          v-model="form.customer_id"
          :placeholder="t('customerCredit.rating.placeholder.customer')"
          style="width: 100%"
        >
          <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('customerCredit.rating.label.creditLevel')" prop="creditLevel">
        <!-- 档位取值词表单一真相源在 constants/credit-level.ts，与列表列的着色映射同一份 -->
        <el-select
          v-model="form.creditLevel"
          :placeholder="t('customerCredit.rating.placeholder.creditLevel')"
          style="width: 100%"
        >
          <el-option
            v-for="level in CREDIT_LEVEL_VALUES"
            :key="level"
            :label="level"
            :value="level"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('customerCredit.rating.label.creditScore')" prop="creditScore">
        <el-input v-model="form.creditScore" style="width: 100%" inputmode="numeric" />
      </el-form-item>
      <el-form-item :label="t('customerCredit.rating.label.creditLimit')" prop="creditLimit">
        <el-input v-model="form.creditLimit" style="width: 100%" inputmode="decimal" />
      </el-form-item>
      <el-form-item :label="t('customerCredit.rating.label.creditDays')" prop="creditDays">
        <el-input v-model="form.creditDays" style="width: 100%" inputmode="numeric" />
      </el-form-item>
      <el-form-item :label="t('customerCredit.rating.label.remark')" prop="remark">
        <el-input v-model="form.remark" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('customerCredit.rating.button.cancel') }}</el-button>
      <!-- 提交打的是设置评级写端点，路径末段 rating 不在后端动作关键字表内，
           按方法派生成 create 键，与列表页设置评级入口同键。 -->
      <el-button
        v-permission="PERMISSIONS.CUSTOMER_CREDIT_CREATE"
        type="primary"
        :loading="submitLoading"
        @click="handleSubmit"
        >{{ t('customerCredit.rating.button.save') }}</el-button
      >
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { setCreditRating, type SetCreditRatingInput } from '@/api/customer-credit';
import { PERMISSIONS } from '@/constants/permissions';
import type { Customer } from '@/api/customer';
import { numberInputRules } from '@/utils/number-input';
import { logger } from '@/utils/logger';
import { CREDIT_LEVEL_VALUES } from '@/constants/credit-level';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
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

/**
 * 未填态一律是空串，绝不预置 0。
 *
 * 缘由：数字录入项换成 el-input 后 v-model 是字符串，空串代表用户没填。
 * 若把未填初值设成 0，required 认 0 为有效值照样过校验并随请求落库
 * （services/customer_credit_limit.rs 的 set_credit_rating 对 Some(v) 一律显式写入，含 Some(0)），
 * 表现为信用分 0、额度 0（可用额度随之 0）、账期 0 天——用户从未输入过的 0 被当成输入。
 *
 * 本表单的口径：
 * - 初值空串 = 未填，required 拦得住（空串视为空），提交不出去；
 * - 用户显式输入的 0 是合法业务值（额度清零、现结=0 天账期、信用分 0 分），照原样上送；
 * - 范围与精度靠 rules 数字校验器钉住（分数 0 到 100 整数、额度不小于 0 两位小数、账期不小于 0 整数），
 *   不再靠加减按钮；清空输入框回到空串未填态。
 * 三个键都不由前端替用户补默认值；后端对 None 的兜底（等级 B / 分数 60 / 账期 30）
 * 是后端的产品口径，前端不预先替它填上，也不假装它不存在。
 */
const form = reactive({
  customer_id: undefined as number | undefined,
  creditLevel: '',
  creditScore: '',
  creditLimit: '',
  creditDays: '',
  remark: '',
});

const rules: FormRules = {
  customer_id: [
    {
      required: true,
      message: t('customerCredit.rating.validation.customerRequired'),
      trigger: 'change',
    },
  ],
  creditLevel: [
    {
      required: true,
      message: t('customerCredit.rating.validation.creditLevelRequired'),
      trigger: 'change',
    },
  ],
  creditScore: numberInputRules({
    label: t('customerCredit.rating.label.creditScore'),
    required: true,
    min: 0,
    max: 100,
    integer: true,
    messages: { empty: t('customerCredit.rating.validation.creditScoreRequired') },
  }),
  creditLimit: numberInputRules({
    label: t('customerCredit.rating.label.creditLimit'),
    required: true,
    min: 0,
    precision: 2,
    messages: { empty: t('customerCredit.rating.validation.creditLimitRequired') },
  }),
  creditDays: numberInputRules({
    label: t('customerCredit.rating.label.creditDays'),
    required: true,
    min: 0,
    integer: true,
    messages: { empty: t('customerCredit.rating.validation.creditDaysRequired') },
  }),
};

watch(
  () => props.modelValue,
  val => {
    visible.value = val;
    if (val) {
      resetForm();
    }
  }
);

watch(visible, val => {
  emit('update:modelValue', val);
});

const resetForm = () => {
  form.customer_id = undefined;
  form.creditLevel = '';
  form.creditScore = '';
  form.creditLimit = '';
  form.creditDays = '';
  form.remark = '';
  formRef.value?.clearValidate();
};

/**
 * 纯函数：把表单态换算成 PUT/POST 载荷，未填时返回 null 而不是补默认值。
 * 存在意义是"校验已过但状态仍不完整"这条不该发生的路径——它必须让提交显式失败，
 * 而不是悄悄把 0/'' 写进库（required 认 0 为有效值，补位即等于替用户输入）。
 * 入参：表单快照；传给谁：仅 handleSubmit 用于判定与上送；不存储、不打印。
 */
function buildRatingPayload(source: typeof form): SetCreditRatingInput | null {
  const unfilled =
    typeof source.customer_id !== 'number' ||
    source.creditLevel === '' ||
    source.creditScore === '' ||
    source.creditLimit === '' ||
    source.creditDays === '';
  if (unfilled) return null;
  return {
    customer_id: source.customer_id as number,
    credit_level: source.creditLevel,
    credit_score: Number(source.creditScore),
    credit_limit: Number(source.creditLimit),
    credit_days: Number(source.creditDays),
    remark: source.remark,
  };
}

const handleSubmit = async () => {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
    const payload = buildRatingPayload(form);
    if (payload === null) {
      // 校验通过却仍判定为未填：不做任何默认值补位，点名报错并留日志
      ElMessage.error(t('customerCredit.rating.validation.incompleteForm'));
      logger.warn(
        t('customerCredit.rating.log.submissionBlocked'),
        JSON.stringify({
          creditLevel: form.creditLevel,
          creditScore: form.creditScore,
          creditLimit: form.creditLimit,
          creditDays: form.creditDays,
        })
      );
      return;
    }
    submitLoading.value = true;
    // 请求体对齐后端 CreditRatingRequestDto（api/customer-credit.ts 的 SetCreditRatingInput）：
    // customer_id 必填走请求体（该 handler 不读路径 id，routes/crm.rs 的 {id} 只是占位）；
    // credit_level/credit_score/credit_limit/credit_days/remark 五个采集项全量上送，
    // 任何一项缺席都会让后端按 None 走自己的兜底而不是落用户输入的真值。
    await setCreditRating(payload.customer_id, payload);
    ElMessage.success(t('customerCredit.rating.message.success'));
    visible.value = false;
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('customerCredit.rating.message.failed'));
    logger.warn(t('customerCredit.rating.log.failed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
