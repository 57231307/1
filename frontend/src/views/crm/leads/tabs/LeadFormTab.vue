<!--
  LeadFormTab.vue - 线索新建/编辑对话框
  来源：原 crm/leads/index.vue 中 新建/编辑对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" :title="title" width="800px" :close-on-click-modal="false" :aria-label="title">
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px" aria-label="线索表单">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="线索来源" prop="lead_source">
            <el-select v-model="formData.lead_source" placeholder="请选择线索来源">
              <el-option label="网站" value="WEBSITE" />
              <el-option label="电话" value="PHONE" />
              <el-option label="展会" value="EXHIBITION" />
              <el-option label="推荐" value="REFERRAL" />
              <el-option label="其他" value="OTHER" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="优先级" prop="priority">
            <el-select v-model="formData.priority" placeholder="请选择优先级">
              <el-option label="低" value="LOW" />
              <el-option label="中" value="MEDIUM" />
              <el-option label="高" value="HIGH" />
              <el-option label="紧急" value="URGENT" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="公司名称" prop="company_name">
            <el-input v-model="formData.company_name" placeholder="请输入公司名称" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="联系人" prop="contact_name">
            <el-input v-model="formData.contact_name" placeholder="请输入联系人姓名" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="手机号" prop="mobile_phone">
            <el-input v-model="formData.mobile_phone" placeholder="请输入手机号" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="邮箱" prop="email">
            <el-input v-model="formData.email" placeholder="请输入邮箱" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="职位" prop="contact_title">
            <el-input v-model="formData.contact_title" placeholder="请输入职位" />
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
      <el-form-item label="需求描述" prop="requirement_desc">
        <el-input
          v-model="formData.requirement_desc"
          type="textarea"
          :rows="3"
          placeholder="请输入需求描述"
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
import type { Lead } from '@/api/crm'
import type { User } from '@/api/user'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  title: string
  rowData: Partial<Lead> | null
  users: User[]
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
  lead_source: '',
  company_name: '',
  contact_name: '',
  contact_title: '',
  mobile_phone: '',
  email: '',
  priority: 'MEDIUM',
  owner_id: '' as string | number,
  requirement_desc: '',
  remarks: '',
})

const formRules: FormRules = {
  lead_source: [{ required: true, message: '请选择线索来源', trigger: 'change' }],
  contact_name: [{ required: true, message: '请输入联系人姓名', trigger: 'blur' }],
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
  formData.lead_source = ''
  formData.company_name = ''
  formData.contact_name = ''
  formData.contact_title = ''
  formData.mobile_phone = ''
  formData.email = ''
  formData.priority = 'MEDIUM'
  formData.owner_id = ''
  formData.requirement_desc = ''
  formData.remarks = ''
}

const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    submitLoading.value = true
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
