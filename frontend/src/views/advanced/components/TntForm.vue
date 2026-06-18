<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
/**
 * TntForm - 租户创建/编辑对话框组件（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 租户对话框）
 * 数据与函数全部由父组件通过 props 传入
 */
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
  form: TenantFormData
  onSubmit: () => Promise<void>
  onCancel: () => void
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="title"
    width="600px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form :model="form" label-width="100px">
      <el-form-item label="租户名称" required>
        <el-input v-model="form.name" placeholder="请输入租户名称" />
      </el-form-item>
      <el-form-item label="租户编码" required>
        <el-input v-model="form.code" placeholder="请输入租户编码" />
      </el-form-item>
      <el-form-item label="联系人">
        <el-input v-model="form.contact_person" placeholder="请输入联系人" />
      </el-form-item>
      <el-form-item label="联系电话">
        <el-input v-model="form.contact_phone" placeholder="请输入联系电话" />
      </el-form-item>
      <el-form-item label="邮箱">
        <el-input v-model="form.email" placeholder="请输入邮箱" />
      </el-form-item>
      <el-form-item label="地址">
        <el-input v-model="form.address" placeholder="请输入地址" />
      </el-form-item>
      <el-form-item label="状态">
        <el-select
          v-model="form.status"
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
