<!--
  LogisticsForm.vue - 物流管理新建/编辑运单对话框
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="isEdit ? t('logistics.form.title.edit') : t('logistics.form.title.create')"
    width="600px"
    :aria-label="isEdit ? t('logistics.form.aria.editDialog') : t('logistics.form.aria.createDialog')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="localForm" :rules="rules" label-width="100px" :aria-label="t('logistics.form.aria.form')">
      <el-form-item :label="t('logistics.form.label.relatedOrder')" prop="order_id">
        <el-select v-model="localForm.order_id" :placeholder="t('logistics.form.placeholder.relatedOrder')" filterable>
          <el-option
            v-for="order in orders"
            :key="order.id"
            :label="order.order_no"
            :value="order.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('logistics.form.label.logisticsCompany')" prop="logistics_company">
        <el-select v-model="localForm.logistics_company" :placeholder="t('logistics.form.placeholder.logisticsCompany')">
          <el-option :label="t('logistics.common.company.sf')" value="顺丰速运" />
          <el-option :label="t('logistics.common.company.zto')" value="中通快递" />
          <el-option :label="t('logistics.common.company.yto')" value="圆通速递" />
          <el-option :label="t('logistics.common.company.yunda')" value="韵达快递" />
          <el-option :label="t('logistics.common.company.jd')" value="京东物流" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('logistics.form.label.trackingNumber')" prop="tracking_number">
        <el-input v-model="localForm.tracking_number" :placeholder="t('logistics.form.placeholder.trackingNumber')" />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('logistics.form.label.driverName')">
            <el-input v-model="localForm.driver_name" :placeholder="t('logistics.form.placeholder.driverName')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('logistics.form.label.driverPhone')">
            <el-input v-model="localForm.driver_phone" :placeholder="t('logistics.form.placeholder.driverPhone')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('logistics.form.label.freight')">
            <el-input-number v-model="localForm.freight_fee" :min="0" :precision="2" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('logistics.form.label.expectedArrival')">
            <el-date-picker
              v-model="localForm.expected_arrival"
              type="date"
              :placeholder="t('logistics.form.placeholder.expectedArrival')"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('logistics.form.label.notes')">
        <el-input v-model="localForm.notes" type="textarea" :rows="3" :placeholder="t('logistics.form.placeholder.notes')" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{ t('logistics.form.button.cancel') }}</el-button>
      <el-button type="primary" :loading="loading" @click="onSubmit">{{ t('logistics.form.button.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { FormInstance, FormRules } from 'element-plus'

const { t } = useI18n({ useScope: 'global' })

// 订单选项类型
interface OrderOption {
  id: number
  order_no: string
}

// 订单表单字段类型
interface LogisticsForm {
  id?: number | undefined
  order_id?: number | undefined
  logistics_company?: string
  tracking_number?: string
  driver_name?: string
  driver_phone?: string
  freight_fee?: number
  expected_arrival?: string
  notes?: string
}

/**
 * 物流运单表单对话框（新建/编辑）
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 是否编辑模式
  isEdit: boolean
  // 提交 loading
  loading: boolean
  // 关联订单
  orders: OrderOption[]
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  form: LogisticsForm
  // 校验规则
  rules: FormRules
}>()

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void
  (e: 'submit'): void
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 form）
  (e: 'update:form', form: LogisticsForm): void
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<LogisticsForm>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件打开新建/编辑时填充数据）
watch(
  () => props.form,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:form', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
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
