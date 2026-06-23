<!--
  SupplierDialog.vue - 供应商新建/编辑/查看对话框
  来源：原 supplier/index.vue 中 弹窗表单区（line 43-197）
  拆分日期：2026-06-22 P9-3 批次 E 样板 2
  拆分目的：supplier/index.vue 458 行 → 约 290 行（主文件）+ 本子组件 ~230 行
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="800px"
    :close-on-click-modal="false"
    @update:model-value="onVisibleChange"
    @close="emit('close')"
  >
    <el-form ref="formRef" :model="localFormData" :rules="formRules" label-width="120px">
      <el-divider content-position="left">基本信息</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="供应商编码" prop="supplier_code">
            <el-input v-model="localFormData.supplier_code" placeholder="请输入供应商编码" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="供应商名称" prop="supplier_name">
            <el-input v-model="localFormData.supplier_name" placeholder="请输入供应商名称" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="供应商简称" prop="supplier_short_name">
            <el-input v-model="localFormData.supplier_short_name" placeholder="请输入简称" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="供应商类型" prop="supplier_type">
            <el-select
              v-model="localFormData.supplier_type"
              placeholder="请选择类型"
              style="width: 100%"
            >
              <el-option label="生产商" value="manufacturer" />
              <el-option label="经销商" value="distributor" />
              <el-option label="服务商" value="service" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="信用代码" prop="credit_code">
            <el-input v-model="localFormData.credit_code" placeholder="请输入统一社会信用代码" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="法人代表" prop="legal_representative">
            <el-input v-model="localFormData.legal_representative" placeholder="请输入法人代表" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">联系信息</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="联系电话" prop="contact_phone">
            <el-input v-model="localFormData.contact_phone" placeholder="请输入联系电话" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="邮箱" prop="email">
            <el-input v-model="localFormData.email" placeholder="请输入邮箱" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="网址" prop="website">
            <el-input v-model="localFormData.website" placeholder="请输入网址" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="传真" prop="fax">
            <el-input v-model="localFormData.fax" placeholder="请输入传真" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="注册地址" prop="registered_address">
        <el-input v-model="localFormData.registered_address" placeholder="请输入注册地址" />
      </el-form-item>
      <el-form-item label="经营地址" prop="business_address">
        <el-input v-model="localFormData.business_address" placeholder="请输入经营地址" />
      </el-form-item>
      <el-divider content-position="left">财务信息</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="纳税人类型" prop="taxpayer_type">
            <el-select
              v-model="localFormData.taxpayer_type"
              placeholder="请选择类型"
              style="width: 100%"
            >
              <el-option label="一般纳税人" value="general" />
              <el-option label="小规模纳税人" value="small" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="注册资本(万)" prop="registered_capital">
            <el-input-number
              v-model="localFormData.registered_capital"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="开户银行" prop="bank_name">
            <el-input v-model="localFormData.bank_name" placeholder="请输入开户银行" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="银行账号" prop="bank_account">
            <el-input v-model="localFormData.bank_account" placeholder="请输入银行账号" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">业务信息</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="等级" prop="grade">
            <el-select v-model="localFormData.grade" placeholder="请选择等级" style="width: 100%">
              <el-option label="A级" value="A" />
              <el-option label="B级" value="B" />
              <el-option label="C级" value="C" />
              <el-option label="D级" value="D" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="状态" prop="status">
            <el-radio-group v-model="localFormData.status">
              <el-radio value="active">启用</el-radio>
              <el-radio value="inactive">停用</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="主营业务" prop="main_business">
        <el-input v-model="localFormData.main_business" placeholder="请输入主营业务" />
      </el-form-item>
      <el-form-item label="备注" prop="remarks">
        <el-input v-model="localFormData.remarks" type="textarea" :rows="3" placeholder="请输入备注" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">取消</el-button>
      <el-button type="primary" :loading="submitLoading" :disabled="readonly" @click="onSubmit">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'

// 表单数据结构（与 supplier/index.vue 中 formData 完全一致）
interface SupplierFormData {
  id: number | undefined
  supplier_code: string
  supplier_name: string
  supplier_short_name: string
  supplier_type: string
  credit_code: string
  registered_address: string
  business_address: string
  legal_representative: string
  registered_capital: number
  contact_phone: string
  fax: string
  website: string
  email: string
  main_business: string
  taxpayer_type: string
  bank_name: string
  bank_account: string
  grade: string
  status: string
  remarks: string
}

// 默认空表单（用于 reset）
const emptyForm = (): SupplierFormData => ({
  id: undefined,
  supplier_code: '',
  supplier_name: '',
  supplier_short_name: '',
  supplier_type: '',
  credit_code: '',
  registered_address: '',
  business_address: '',
  legal_representative: '',
  registered_capital: 0,
  contact_phone: '',
  fax: '',
  website: '',
  email: '',
  main_business: '',
  taxpayer_type: '',
  bank_name: '',
  bank_account: '',
  grade: '',
  status: 'active',
  remarks: '',
})

const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 标题
  title: string
  // 模式：add / edit / view
  mode: 'add' | 'edit' | 'view'
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: SupplierFormData
  // 提交 loading
  submitLoading: boolean
}>()

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean]
  // 关闭后（用于父组件 reset）
  close: []
  // 提交表单
  submit: []
  // 整体回写表单
  'update:formData': [formData: SupplierFormData]
}>()

// 表单引用
const formRef = ref<FormInstance>()

// 只读模式（view 模式禁用保存按钮）
const readonly = computed(() => props.mode === 'view')

// 表单校验规则
const formRules: FormRules = {
  supplier_code: [{ required: true, message: '请输入供应商编码', trigger: 'blur' }],
  supplier_name: [{ required: true, message: '请输入供应商名称', trigger: 'blur' }],
  contact_phone: [
    { required: true, message: '请输入联系电话', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号', trigger: 'blur' },
  ],
}

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFormData = ref<SupplierFormData>({ ...props.formData })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.formData,
  (newForm) => {
    if (syncing) return
    syncing = true
    localFormData.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件
watch(
  localFormData,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:formData', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}

/** 取消按钮 */
const onCancel = () => {
  emit('update:visible', false)
}

/** 提交按钮（触发父组件 validate + save） */
const onSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    emit('submit')
  })
}

// 暴露 reset 方法供父组件调用（通过 defineExpose）
/** 重置表单到初始状态 */
const resetForm = () => {
  localFormData.value = emptyForm()
  formRef.value?.clearValidate()
}

defineExpose({ resetForm })
</script>
