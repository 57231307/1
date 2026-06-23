<script setup lang="ts">
/**
 * TntForm - 租户创建/编辑对话框组件
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 租户对话框）
 * 数据与函数全部由父组件通过 props 传入
 * P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
 */
import { ref, watch, nextTick } from 'vue'

interface TenantFormData {
  id: number | null
  name: string
  code: string
  contact_person: string
  contact_phone: string
  email: string
  address: string
  status: string
}

interface Props {
  modelValue: boolean
  title: string
  // 租户表单（由父组件管理，子组件通过 emit('update:form') 回写）
  form: TenantFormData
  onSubmit: () => Promise<void>
  onCancel: () => void
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  // 整体回写表单
  (e: 'update:form', form: TenantFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<TenantFormData>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
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

// 本地变化时通知父组件
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
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="title"
    width="600px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form :model="localForm" label-width="100px">
      <el-form-item label="租户名称" required>
        <el-input v-model="localForm.name" placeholder="请输入租户名称" />
      </el-form-item>
      <el-form-item label="租户编码" required>
        <el-input v-model="localForm.code" placeholder="请输入租户编码" />
      </el-form-item>
      <el-form-item label="联系人">
        <el-input v-model="localForm.contact_person" placeholder="请输入联系人" />
      </el-form-item>
      <el-form-item label="联系电话">
        <el-input v-model="localForm.contact_phone" placeholder="请输入联系电话" />
      </el-form-item>
      <el-form-item label="邮箱">
        <el-input v-model="localForm.email" placeholder="请输入邮箱" />
      </el-form-item>
      <el-form-item label="地址">
        <el-input v-model="localForm.address" placeholder="请输入地址" />
      </el-form-item>
      <el-form-item label="状态">
        <el-select
          v-model="localForm.status"
          placeholder="请选择状态"
          style="width: 100%"
        >
          <el-option label="正常" value="active" />
          <el-option label="停用" value="inactive" />
          <el-option label="暂停" value="suspended" />
        </el-select>
      </el-form-item>
    </el-form>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="onCancel">取消</el-button>
        <el-button type="primary" @click="onSubmit">确定</el-button>
      </span>
    </template>
  </el-dialog>
</template>
