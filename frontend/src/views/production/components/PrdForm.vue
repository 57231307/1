<!--
  PrdForm.vue - 生产管理订单表单（新建/编辑）
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="localForm.id ? '编辑生产订单' : '新建生产订单'"
    width="700px"
    destroy-on-close
    :aria-label="localForm.id ? '编辑生产订单对话框' : '新建生产订单对话框'"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="localForm" :rules="rules" label-width="100px" aria-label="生产订单表单">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="订单编号" prop="order_no">
            <el-input v-model="localForm.order_no" placeholder="请输入订单编号" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="产品ID" prop="product_id">
            <el-input-number
              v-model="localForm.product_id"
              :min="1"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="计划数量" prop="planned_quantity">
            <el-input-number
              v-model="localForm.planned_quantity"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="优先级" prop="priority">
            <el-input-number v-model="localForm.priority" :min="1" :max="10" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="计划开始">
            <el-date-picker
              v-model="localForm.scheduled_start_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="计划结束">
            <el-date-picker
              v-model="localForm.scheduled_end_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="工作中心ID">
        <el-input-number v-model="localForm.work_center_id" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item label="备注">
        <el-input v-model="localForm.remark" type="textarea" :rows="3" placeholder="请输入备注" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="loading" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'

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
  // 对话框可见性
  visible: boolean
  // 表单数据（由父组件管理，子组件通过 emit('update:form') 回写）
  form: OrderForm
  // 提交 loading
  loading: boolean
  // 校验规则
  rules: FormRules
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
  // 整体回写表单
  'update:form': [form: OrderForm]
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<OrderForm>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件编辑时重置）
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
