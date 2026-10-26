<!--
  PrintDocumentDialog.vue - 从业务单据发起真实打印任务（受理 + 轮询真实终态）

  调用方：单据查看/详情面（如销售订单详情对话框），传入 docType（后端 print_service 支持的单据
  类型键）、docId（该行真实主键）与可选 docLabel（单据号，仅用于界面回显）。
  数据来源：GET /print-templates 取该 docType 的启用模板、GET /print-targets 取启用中的打印目标。
  提交：POST /print-tasks（受理），随后按 GET /print-tasks/{id} 轮询真实 status，
  到终态才判定成败并把后端 error_message 原文呈现；绝不伪造百分比、不宣称"已打印完成"。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('printDialog.title')"
    width="520px"
    :aria-label="t('printDialog.dialogAriaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="form" :aria-label="t('printDialog.formAriaLabel')" label-width="110px">
      <el-form-item :label="t('printDialog.document')">
        <el-input :model-value="docLabel || String(docId ?? '')" readonly />
      </el-form-item>
      <el-form-item :label="t('printDialog.template')" prop="template_id">
        <el-select
          v-model="form.template_id"
          :placeholder="t('printDialog.templatePlaceholder')"
          :loading="templatesLoading"
          style="width: 100%"
          filterable
          @change="applyTemplateDefaults"
        >
          <el-option
            v-for="tpl in templateOptions"
            :key="tpl.id"
            :label="tpl.template_name"
            :value="tpl.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('printDialog.target')" prop="target_name">
        <el-select
          v-model="form.target_name"
          :placeholder="t('printDialog.targetPlaceholder')"
          :loading="targetsLoading"
          style="width: 100%"
          filterable
        >
          <el-option
            v-for="target in targetOptions"
            :key="target.target_code"
            :label="targetLabel(target)"
            :value="target.target_code"
          />
        </el-select>
      </el-form-item>
      <el-form-item
        :label="t('printDialog.copies')"
        prop="copies"
        :rules="copiesRules"
      >
        <!-- 手写录入 打印份数 正整数 1 到 99（原步进控件 min max 与默认精度 0 落到 rules 与提交门） -->
        <el-input v-model="form.copies" inputmode="numeric" />
      </el-form-item>
    </el-form>

    <el-alert v-if="blockingHint" type="warning" :closable="false" show-icon>
      {{ blockingHint }}
    </el-alert>

    <div v-if="task" class="print-task-result">
      <p>
        {{ t('printDialog.jobCode') }}
        <strong>{{ task.job_code }}</strong>
      </p>
      <p>
        {{ t('printDialog.status') }}
        <el-tag :type="printTaskStatusTagType(task.status)" size="small">
          {{ t(printTaskStatusLabelKey(task.status)) }}
        </el-tag>
      </p>
      <p v-if="task.error_message">
        {{ t('printDialog.errorMessage') }}
        {{ task.error_message }}
      </p>
      <p v-if="!isPrintTaskTerminal(task.status)">{{ t('printDialog.stillInFlight') }}</p>
    </div>

    <template #footer>
      <el-button @click="emit('update:visible', false)">{{ t('common.cancel') }}</el-button>
      <el-button
        v-permission="PERMISSIONS.PRINT_TASK_CREATE"
        type="primary"
        :loading="submitting"
        :disabled="!canSubmit"
        @click="handleSubmit"
      >
        {{ t('printDialog.submit') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { getPrintTemplateList, type PrintTemplate } from '@/api/print-templates';
import { getPrintTargetList, type PrintTarget } from '@/api/print-targets';
import { createPrintTask, getPrintTask, type PrintTask } from '@/api/print-tasks';
import { PERMISSIONS } from '@/constants/permissions';
import {
  isPrintTaskTerminal,
  printTaskStatusLabelKey,
  printTaskStatusTagType,
} from '@/constants/print-task-status';
import { logAuxLoadFailure } from '@/utils/logger';
import { numberInputRules } from '@/utils/number-input';

const { t } = useI18n({ useScope: 'global' });

/** 轮询真实终态的次数与间隔（观测窗口，非进度计时器；窗口用尽仍保留最后一次读到的状态） */
const POLL_INTERVAL_MS = 1500;
const POLL_MAX_ATTEMPTS = 20;
const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

const props = defineProps<{
  visible: boolean;
  /** 后端 print_service 支持的单据类型键，与所选模板的 doc_type 一致 */
  docType: string;
  /** 要打的那一张单据的真实主键；缺失即组件用法错误，提交被禁用 */
  docId: number | null;
  /** 单据号等人类可读标识，仅用于界面回显 */
  docLabel?: string;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const templates = ref<PrintTemplate[]>([]);
const targets = ref<PrintTarget[]>([]);
const templatesLoading = ref(false);
const targetsLoading = ref(false);
const submitting = ref(false);
const task = ref<PrintTask | null>(null);

const form = reactive({
  template_id: undefined as number | undefined,
  target_name: undefined as string | undefined,
  // 视图态数值字段以字符串承载（el-input 手写录入），提交边界显式转回契约数字
  copies: '1',
});

/** 份数录入的提交门 与行内 rules 同源 整数 1 到 99 */
const copiesRules = numberInputRules({
  label: t('printDialog.copies'),
  required: true,
  min: 1,
  max: 99,
  integer: true,
});

/** 份数录入的提交门 与行内 rules 同源 整数 1 到 99 */
const copiesValid = computed(() => {
  const raw = form.copies.trim();
  if (!/^\d+$/.test(raw)) return false;
  const n = Number(raw);
  return Number.isInteger(n) && n >= 1 && n <= 99;
});

/** 该单据类型下「启用」的模板：停用的模板后端不会用于渲染，列出即误导 */
const templateOptions = computed(() =>
  templates.value.filter(tpl => tpl.doc_type === props.docType && tpl.status === 'active')
);

/** 启用中的打印目标：受理侧只接受 enabled 行（禁用行必 400「未配置或未启用」） */
const targetOptions = computed(() => targets.value.filter(target => target.enabled));

function targetLabel(target: PrintTarget): string {
  return `${target.display_name}（${target.target_code}）`;
}

/** 模板默认值带出：选中模板后按其缺省份数与缺省目标预填，用户仍可覆盖 */
function applyTemplateDefaults() {
  const tpl = templateOptions.value.find(item => item.id === form.template_id);
  if (!tpl) return;
  // 模板缺省份数带出时钳到 1 到 99（原步进控件的边界钳制语义保留），以字符串灌入手写录入框
  form.copies = String(Math.min(99, Math.max(1, tpl.copies_default)));
  form.target_name = tpl.default_target_name ?? form.target_name;
}

/** 不可提交时给出行内提示（fail-visible，不静默禁用后让用户猜原因） */
const blockingHint = computed(() => {
  if (!props.docId) return t('printDialog.missingDocument');
  if (props.visible && !templatesLoading.value && templateOptions.value.length === 0)
    return t('printDialog.noActiveTemplate');
  // 候选仍在加载时不下"没有可用目标"的结论，避免把加载中的空数组误报成配置缺失
  if (!targetsLoading.value && !targetOptions.value.length && !form.target_name)
    return t('printDialog.noEnabledTarget');
  return '';
});

const canSubmit = computed(
  () =>
    !!props.docId &&
    !!form.target_name &&
    copiesValid.value &&
    templateOptions.value.length > 0 &&
    !submitting.value &&
    !blockingHint.value
);

const loadAux = async () => {
  templatesLoading.value = true;
  targetsLoading.value = true;
  try {
    const res = await getPrintTemplateList();
    templates.value = res.data;
  } catch (error: unknown) {
    logAuxLoadFailure('加载打印模板失败:', error);
    templates.value = [];
  } finally {
    templatesLoading.value = false;
  }
  try {
    const res = await getPrintTargetList();
    targets.value = res.data;
  } catch (error: unknown) {
    logAuxLoadFailure('加载打印目标失败:', error);
    targets.value = [];
  } finally {
    targetsLoading.value = false;
  }
};

/** 每次打开都重头采集：上一次的模板/目标与任务结论不得残留在本次对话框 */
watch(
  () => props.visible,
  async v => {
    if (!v) return;
    task.value = null;
    form.template_id = undefined;
    form.target_name = undefined;
    form.copies = '1';
    await loadAux();
    const preferred = templateOptions.value.find(tpl => tpl.is_default) ?? templateOptions.value[0];
    if (preferred) {
      form.template_id = preferred.id;
      applyTemplateDefaults();
      if (!form.target_name && targetOptions.value.length === 1) {
        form.target_name = targetOptions.value[0].target_code;
      }
    }
  }
);

/**
 * 轮询受理后的真实终态：到 succeeded/failed 即返回；窗口用尽仍在途则返回最后一次读到的任务，
 * 交调用方如实呈现（后端派发可能因设备不可达而慢失败），绝不把"已受理"说成"已打印"。
 */
const pollUntilTerminal = async (id: number): Promise<PrintTask | null> => {
  let last: PrintTask | null = null;
  for (let attempt = 0; attempt < POLL_MAX_ATTEMPTS; attempt++) {
    try {
      const res = await getPrintTask(id);
      last = res.data;
      task.value = res.data;
      if (isPrintTaskTerminal(res.data.status)) return last;
    } catch (error: unknown) {
      logAuxLoadFailure('打印任务状态轮询失败:', error);
    }
    await sleep(POLL_INTERVAL_MS);
  }
  return last;
};

const handleSubmit = async () => {
  if (!canSubmit.value || !props.docId) return;
  submitting.value = true;
  task.value = null;
  try {
    const res = await createPrintTask({
      doc_type: props.docType,
      doc_id: props.docId,
      target_name: form.target_name,
      template_id: form.template_id,
      // 提交边界：字符串编辑态转回契约数字（canSubmit 已保证为 1 到 99 整数）
      copies: Number(form.copies),
    });
    const settled = await pollUntilTerminal(res.data.id);
    if (settled?.status === 'succeeded') {
      ElMessage.success(t('printDialog.printSucceeded'));
    } else if (settled?.status === 'failed') {
      ElMessage.error(t('printDialog.printFailed'));
    } else {
      ElMessage.info(t('printDialog.acceptedInFlight'));
    }
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) || t('printDialog.submitFailed')
    );
  } finally {
    submitting.value = false;
  }
};
</script>

<style scoped>
.print-task-result {
  margin-top: 8px;
  padding: 8px 12px;
  background: #f5f7fa;
  border-radius: 4px;
  font-size: 13px;
}
.print-task-result p {
  margin: 4px 0;
}
</style>
