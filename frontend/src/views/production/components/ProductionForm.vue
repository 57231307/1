<!--
  ProductionForm.vue - 生产管理订单表单（新建/编辑）
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="localForm.id ? t('production.form.titleEdit') : t('production.form.titleCreate')"
    width="700px"
    destroy-on-close
    :aria-label="
      localForm.id ? t('production.form.ariaLabelEdit') : t('production.form.ariaLabelCreate')
    "
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      :aria-label="t('production.form.ariaLabelForm')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('production.form.labelOrderNo')" prop="order_no">
            <el-input
              v-model="localForm.order_no"
              :placeholder="t('production.form.placeholderOrderNo')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('production.form.labelProductId')" prop="product_id">
            <el-input-number v-model="localForm.product_id" :min="1" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('production.form.labelPlannedQuantity')" prop="planned_quantity">
            <el-input-number v-model="localForm.planned_quantity" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('production.form.labelPriority')" prop="priority">
            <el-input-number v-model="localForm.priority" :min="1" :max="10" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('production.form.labelScheduledStart')">
            <el-date-picker
              v-model="localForm.scheduled_start_date"
              type="date"
              :placeholder="t('production.form.placeholderDate')"
              style="width: 100%"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('production.form.labelScheduledEnd')">
            <el-date-picker
              v-model="localForm.scheduled_end_date"
              type="date"
              :placeholder="t('production.form.placeholderDate')"
              style="width: 100%"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('production.form.labelWorkCenterId')">
        <el-input-number v-model="localForm.work_center_id" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item :label="t('production.form.labelRemark')">
        <el-input
          v-model="localForm.remark"
          type="textarea"
          :rows="3"
          :placeholder="t('production.form.placeholderRemark')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('production.form.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="loading" @click="onSubmit">{{
        t('production.form.buttonConfirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { FormInstance, FormRules } from 'element-plus'

const { t } = useI18n({ useScope: 'global' })

// 订单表单字段类型
interface OrderForm {
  id?: number | undefined
  order_no?: string
  product_id?: number | undefined
  planned_quantity?: number | undefined
  scheduled_start_date?: string
  scheduled_end_date?: string
  priority?: number
  work_center_id?: number | undefined
  remark?: string
  status?: string
}

const props = defineProps<{
  visible: boolean
  form: OrderForm
  loading: boolean
  rules: FormRules
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
  'update:form': [form: OrderForm]
}>()

const formRef = ref<FormInstance>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<OrderForm>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.form,
  newForm => {
    if (syncing) return
    syncing = true
    localForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true }
)

// 本地变化时通知父组件
watch(
  localForm,
  newForm => {
    if (syncing) return
    syncing = true
    emit('update:form', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true }
)

/** 点击确定：先校验再发 submit */
const onSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    emit('submit')
  })
}
</script>
