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
        <el-input
          :model-value="numericView(form.creditScore)"
          inputmode="decimal"
          style="width: 100%"
          @change="
            (v: string | number) =>
              (form.creditScore = bindNullableNumeric(v, form.creditScore, { min: 0, max: 100 }))
          "
        />
      </el-form-item>
      <el-form-item :label="t('customerCredit.rating.label.creditLimit')" prop="creditLimit">
        <el-input
          :model-value="numericView(form.creditLimit)"
          inputmode="decimal"
          style="width: 100%"
          @change="
            (v: string | number) =>
              (form.creditLimit = bindNullableNumeric(v, form.creditLimit, {
                min: 0,
                precision: 2,
              }))
          "
        />
      </el-form-item>
      <el-form-item :label="t('customerCredit.rating.label.creditDays')" prop="creditDays">
        <el-input
          :model-value="numericView(form.creditDays)"
          inputmode="numeric"
          style="width: 100%"
          @change="
            (v: string | number) =>
              (form.creditDays = bindNullableNumeric(v, form.creditDays, { min: 0 }))
          "
        />
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
import { bindNumeric, numericView, type NumericBindOptions } from '@/utils/number-input';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { setCreditRating, type SetCreditRatingInput } from '@/api/customer-credit';
import { PERMISSIONS } from '@/constants/permissions';
import type { Customer } from '@/api/customer';
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
 * 可空数值字段的手写录入归一（change/blur 提交边界调用）：清空回 null（未填态，required 规则拦得住），
 * 非法文本维持当前值，其余走数值绑定归一的 min max precision 钳制。
 * 入参 提交文本与当前可空值与边界词表；返回值写回原字段，null 保持 null。
 */
function bindNullableNumeric(
  val: string | number,
  current: number | null,
  opts: NumericBindOptions
): number | null {
  const text = String(val).trim();
  if (text === '') return null;
  const bound = bindNumeric(text, current ?? undefined, opts);
  return bound ?? current;
}

/**
 * 未填态一律是 null / undefined，绝不预置 0。
 *
 * 缘由：async-validator 的 required 对数字 0 判为有效值——若把未填初值设成 0，
 * "用户没填"照样过校验并随请求落库（services/customer_credit_limit.rs 的
 * set_credit_rating 对 Some(v) 一律显式写入，含 Some(0)），表现为信用分 0、
 * 额度 0（可用额度随之 0）、账期 0 天——用户从未输入过的 0 被当成输入。
 *
 * 本表单的口径：
 * - 初值 null = 未填，required 拦得住（null/undefined 视为空），提交不出去；
 * - 用户显式输入的 0 是合法业务值（额度清零、现结=0 天账期、信用分 0 分），照原样上送；
 * - 清空输入框由绑定归一回到 null 未填态，而不是被 min=0 拍平成 0。
 * 三个键都不由前端替用户补默认值；后端对 None 的兜底（等级 B / 分数 60 / 账期 30）
 * 是后端的产品口径，前端不预先替它填上，也不假装它不存在。
 */
const form = reactive({
  customer_id: undefined as number | undefined,
  creditLevel: '',
  creditScore: null as number | null,
  creditLimit: null as number | null,
  creditDays: null as number | null,
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
  creditScore: [
    {
      required: true,
      message: t('customerCredit.rating.validation.creditScoreRequired'),
      trigger: 'blur',
    },
  ],
  creditLimit: [
    {
      required: true,
      message: t('customerCredit.rating.validation.creditLimitRequired'),
      trigger: 'blur',
    },
  ],
  creditDays: [
    {
      required: true,
      message: t('customerCredit.rating.validation.creditDaysRequired'),
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
    }
  }
);

watch(visible, val => {
  emit('update:modelValue', val);
});

const resetForm = () => {
  form.customer_id = undefined;
  form.creditLevel = '';
  form.creditScore = null;
  form.creditLimit = null;
  form.creditDays = null;
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
    source.creditScore === null ||
    source.creditLimit === null ||
    source.creditDays === null;
  if (unfilled) return null;
  return {
    customer_id: source.customer_id as number,
    credit_level: source.creditLevel,
    credit_score: source.creditScore as number,
    credit_limit: source.creditLimit as number,
    credit_days: source.creditDays as number,
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
