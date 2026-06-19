<!--
  PrdForm.vue - 生产管理订单表单（新建/编辑）
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="form.id ? '编辑生产订单' : '新建生产订单'"
    width="700px"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="订单编号" prop="order_no">
            <el-input v-model="form.order_no" placeholder="请输入订单编号" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="产品ID" prop="product_id">
            <el-input-number
              :model-value="form.product_id"
              :min="1"
              style="width: 100%"
              @update:model-value="(v: number | undefined) => (form.product_id = v ?? undefined)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="计划数量" prop="planned_quantity">
            <el-input-number
              :model-value="form.planned_quantity"
              :min="0"
              style="width: 100%"
              @update:model-value="
                (v: number | undefined) => (form.planned_quantity = v ?? undefined)
              "
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="优先级" prop="priority">
            <el-input-number
              :model-value="form.priority"
              :min="1"
              :max="10"
              style="width: 100%"
              @update:model-value="(v: number | undefined) => (form.priority = v ?? 5)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="计划开始">
            <el-date-picker
              v-model="form.scheduled_start_date"
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
              v-model="form.scheduled_end_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="工作中心ID">
        <el-input-number
          :model-value="form.work_center_id"
          :min="0"
          style="width: 100%"
          @update:model-value="
            (v: number | undefined) => (form.work_center_id = v ?? undefined)
          "
        />
      </el-form-item>
      <el-form-item label="备注">
        <el-input v-model="form.remark" type="textarea" :rows="3" placeholder="请输入备注" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="loading" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref } from 'vue'
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
  // 表单数据
  form: OrderForm
  // 提交 loading
  loading: boolean
  // 校验规则
  rules: FormRules
}>()
void props // 显式标记使用避免 TS6133

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
}>()

// 表单 ref
const formRef = ref<FormInstance>()

/** 点击确定：先校验再发 submit */
const onSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    emit('submit')
  })
}
</script>
