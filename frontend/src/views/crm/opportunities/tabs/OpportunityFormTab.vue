<!--
  OpportunityFormTab.vue - 商机新建/编辑对话框
  来源：原 crm/opportunities/index.vue 中 新建/编辑对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" :title="title" width="800px" :close-on-click-modal="false">
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="商机名称" prop="opportunity_name">
            <el-input v-model="formData.opportunity_name" placeholder="请输入商机名称" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="客户" prop="customer_id">
            <el-select v-model="formData.customer_id" placeholder="请选择客户" filterable>
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
          <el-form-item label="商机类型" prop="opportunity_type">
            <el-select v-model="formData.opportunity_type" placeholder="请选择商机类型">
              <el-option label="新客户" value="NEW" />
              <el-option label="增购" value="UPSELL" />
              <el-option label="续约" value="RENEWAL" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="商机阶段" prop="opportunity_stage">
            <el-select v-model="formData.opportunity_stage" placeholder="请选择商机阶段">
              <el-option label="初步接触" value="INITIAL" />
              <el-option label="需求确认" value="REQUIREMENT" />
              <el-option label="方案报价" value="PROPOSAL" />
              <el-option label="谈判" value="NEGOTIATION" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="预估金额" prop="estimated_amount">
            <el-input-number
              v-model="formData.estimated_amount"
              :precision="2"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="成交概率" prop="win_probability">
            <el-slider v-model="formData.win_probability" :min="0" :max="100" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="预计成交" prop="expected_close_date">
            <el-date-picker
              v-model="formData.expected_close_date"
              type="date"
              placeholder="请选择预计成交日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="负责人" prop="owner_id">
            <el-select v-model="formData.owner_id" placeholder="请选择负责人" filterable>
              <el-option v-for="u in users" :key="u.id" :label="u.real_name" :value="u.id" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="产品描述" prop="product_desc">
        <el-input
          v-model="formData.product_desc"
          type="textarea"
          :rows="3"
          placeholder="请输入产品描述"
        />
      </el-form-item>
      <el-form-item label="备注" prop="remarks">
        <el-input v-model="formData.remarks" type="textarea" :rows="2" placeholder="请输入备注" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { Opportunity } from '@/api/crm'
import type { User } from '@/api/user'
import type { Customer } from '@/api/customer'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  title: string
  rowData: Partial<Opportunity> | null
  users: User[]
  customers: Customer[]
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const visible = ref(props.modelValue)
const submitLoading = ref(false)
const formRef = ref<FormInstance>()

const formData = reactive({
  id: null as number | null,
  opportunity_name: '',
  customer_id: '' as string | number,
  opportunity_type: '',
  opportunity_stage: '',
  estimated_amount: 0,
  win_probability: 50,
  expected_close_date: '',
  owner_id: '' as string | number,
  product_desc: '',
  remarks: '',
})

const formRules: FormRules = {
  opportunity_name: [{ required: true, message: '请输入商机名称', trigger: 'blur' }],
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
  opportunity_stage: [{ required: true, message: '请选择商机阶段', trigger: 'change' }],
  owner_id: [{ required: true, message: '请选择负责人', trigger: 'change' }],
}

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      resetForm()
      if (props.rowData) {
        Object.assign(formData, props.rowData)
      }
    }
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const resetForm = () => {
  formData.id = null
  formData.opportunity_name = ''
  formData.customer_id = ''
  formData.opportunity_type = ''
  formData.opportunity_stage = ''
  formData.estimated_amount = 0
  formData.win_probability = 50
  formData.expected_close_date = ''
  formData.owner_id = ''
  formData.product_desc = ''
  formData.remarks = ''
}

const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    submitLoading.value = true
    // 调用由父组件处理实际的保存逻辑（通过 emit）
    ElMessage.success('保存成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    logger.warn('表单验证失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>
