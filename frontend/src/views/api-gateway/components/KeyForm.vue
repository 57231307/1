<!--
  KeyForm.vue - API 密钥新建/编辑对话框
  本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件；
  description / expires_at 按后端可空列真值回显（null 不冒充空串），
  过期时间以 unix 秒承载、提交时转 ISO 8601。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="form?.id ? t('apiGateway.keyForm.editTitle') : t('apiGateway.keyForm.createTitle')"
    width="600px"
    :aria-label="
      form?.id ? t('apiGateway.keyForm.editAriaLabel') : t('apiGateway.keyForm.createAriaLabel')
    "
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :ref="(el: unknown) => (formRefValue = el as FormInstance)"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      :aria-label="t('apiGateway.keyForm.formAriaLabel')"
    >
      <el-form-item :label="t('apiGateway.keyForm.keyName')" prop="key_name">
        <el-input
          :model-value="localForm.key_name ?? ''"
          :placeholder="t('apiGateway.keyForm.keyNamePlaceholder')"
          @update:model-value="(v: string) => (localForm.key_name = v ?? '')"
        />
      </el-form-item>
      <el-form-item :label="t('apiGateway.keyForm.description')" prop="description">
        <!-- 后端可空列回显：null（未填/已清空）在文本域里就是空文本；提交时空文本转显式 null -->
        <el-input
          :model-value="localForm.description ?? ''"
          type="textarea"
          :aria-label="t('apiGateway.keyForm.description')"
          :rows="3"
          :placeholder="t('apiGateway.keyForm.descriptionPlaceholder')"
          @update:model-value="(v: string) => (localForm.description = v ?? '')"
        />
      </el-form-item>
      <el-form-item :label="t('apiGateway.keyForm.permissions')" prop="permissions">
        <el-input
          :model-value="permissionsText"
          :placeholder="t('apiGateway.keyForm.permissionsPlaceholder')"
          @update:model-value="(v: string) => emit('update:permissionsText', v ?? '')"
        />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('apiGateway.keyForm.rateLimit')" prop="rate_limit">
            <el-input-number
              :model-value="localForm.rate_limit"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (localForm.rate_limit = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('apiGateway.keyForm.expiresAt')" prop="expires_at">
            <!-- value-format="X"：表单模型持 unix 秒（number），提交时转 ISO 8601 送后端；
                 清空选择器 → null → 载荷显式送 null（后端落 NULL = 永不过期）。
                 模型里的 null 在绑定处转 undefined（el-date-picker 的 value 类型不含 null，
                 语义不变：输入框为空 = 未填/已清空） -->
            <el-date-picker
              :model-value="localForm.expires_at ?? undefined"
              type="datetime"
              value-format="X"
              :aria-label="t('apiGateway.keyForm.expiresAt')"
              :placeholder="t('apiGateway.keyForm.expiresAtPlaceholder')"
              style="width: 100%"
              @update:model-value="(v: number | null) => (localForm.expires_at = v ?? null)"
            />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('apiGateway.keyForm.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="emit('submit')">{{
        t('apiGateway.keyForm.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance, FormRules } from 'element-plus';
import type { ApiKeyFormValues } from '../composables/useApiKey';

const { t } = useI18n({ useScope: 'global' });

/**
 * API 密钥新建/编辑对话框
 * 父组件通过 v-model 双向同步 permissionsText
 * 子组件通过 emit('update:*') 通知父组件更新
 * 表单数据通过本地 ref 镜像 + 双向 watch 防循环 + emit('update:form') 整体回写
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 表单实例（可选，通过 v-model:formRef 双向同步）
  formRef?: FormInstance | undefined;
  // 表单数据（由父组件管理，子组件通过 emit 回写；可空列以 null 表达"未填/已清空"）
  form?: Partial<ApiKeyFormValues>;
  // 提交中状态
  submitLoading: boolean;
  // 校验规则
  rules: FormRules;
  // 权限文本（父组件通过 v-model 双向同步）
  permissionsText: string;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
  // 通过 emit 通知父组件 formRef 变化
  'update:formRef': [value: FormInstance | undefined];
  'update:permissionsText': [v: string];
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 form）
  'update:form': [form: Partial<ApiKeyFormValues>];
  submit: [];
}>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<Partial<ApiKeyFormValues>>({ ...(props.form ?? {}) });

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件打开新建/编辑时填充数据）
watch(
  () => props.form,
  newForm => {
    if (syncing) return;
    syncing = true;
    localForm.value = { ...(newForm ?? {}) };
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:form', { ...newForm });
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 将 el-form 的 ref 实例通过 emit 通知父组件
const formRefValue = ref<FormInstance | undefined>(undefined);
watch(
  formRefValue,
  val => {
    if (val) emit('update:formRef', val);
  },
  { immediate: true, flush: 'post' }
);
</script>
