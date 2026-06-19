<!--
  BpmDfForm.vue - BPM 流程定义新建/编辑对话框（含节点配置子表）
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="isEdit ? '编辑流程定义' : '新建流程定义'"
    width="900px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="formData" :rules="rules" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="流程标识" prop="process_key">
            <el-input
              :model-value="formData.process_key"
              :disabled="isEdit"
              placeholder="请输入流程标识（英文字母）"
              @update:model-value="(v: string) => (formData.process_key = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="流程名称" prop="process_name">
            <el-input
              :model-value="formData.process_name"
              placeholder="请输入流程名称"
              @update:model-value="(v: string) => (formData.process_name = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="分类" prop="category">
            <el-select
              :model-value="formData.category"
              placeholder="请选择分类"
              style="width: 100%"
              @update:model-value="(v: string) => (formData.category = v)"
            >
              <el-option label="财务" value="finance" />
              <el-option label="人事" value="hr" />
              <el-option label="采购" value="purchase" />
              <el-option label="销售" value="sales" />
              <el-option label="生产" value="production" />
              <el-option label="库存" value="inventory" />
              <el-option label="其他" value="other" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="描述">
        <el-input
          :model-value="formData.description"
          type="textarea"
          :rows="3"
          placeholder="请输入描述"
          @update:model-value="(v: string) => (formData.description = v)"
        />
      </el-form-item>

      <!-- 节点配置 -->
      <el-divider content-position="left">节点配置</el-divider>
      <div class="node-actions">
        <el-button type="primary" size="small" @click="handleAddNode">
          <el-icon><Plus /></el-icon>
          添加节点
        </el-button>
      </div>
      <el-table :data="formData.nodes" border>
        <el-table-column label="节点类型" width="120">
          <template #default="{ row }">
            <el-select
              :model-value="row.type"
              size="small"
              @update:model-value="(v: string) => (row.type = v as never)"
            >
              <el-option label="开始" value="start" />
              <el-option label="审批" value="approval" />
              <el-option label="条件" value="condition" />
              <el-option label="通知" value="notify" />
              <el-option label="结束" value="end" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="节点名称" min-width="140">
          <template #default="{ row }">
            <el-input
              :model-value="row.name"
              size="small"
              @update:model-value="(v: string) => (row.name = v)"
            />
          </template>
        </el-table-column>
        <el-table-column label="审批人类型" width="140">
          <template #default="{ row }">
            <el-select
              :model-value="row.assignee_type"
              size="small"
              clearable
              @update:model-value="(v: string) => (row.assignee_type = v as never)"
            >
              <el-option label="指定用户" value="user" />
              <el-option label="指定角色" value="role" />
              <el-option label="指定部门" value="department" />
              <el-option label="动态计算" value="dynamic" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="审批人/角色ID" width="140">
          <template #default="{ row }">
            <el-input
              :model-value="String(row.assignee_value ?? '')"
              size="small"
              @update:model-value="(v: string) => (row.assignee_value = v)"
            />
          </template>
        </el-table-column>
        <el-table-column label="条件表达式" min-width="160">
          <template #default="{ row }">
            <el-input
              :model-value="row.condition"
              size="small"
              placeholder="如：amount > 1000"
              @update:model-value="(v: string) => (row.condition = v)"
            />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="80" align="center" fixed="right">
          <template #default="{ $index }">
            <el-button type="danger" link size="small" @click="handleRemoveNode($index)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { ProcessNode } from '@/api/bpm-enhanced'

// 表单数据类型
interface BpmDfFormData {
  id?: number
  process_key: string
  process_name: string
  description: string
  category: string
  nodes: ProcessNode[]
}

/**
 * 表单组件（含节点配置子表）
 */
const props = defineProps<{
  // 可见性
  visible: boolean
  // 是否编辑
  isEdit: boolean
  // 表单数据
  formData: BpmDfFormData
  // 验证规则
  rules: FormRules
  // 提交加载
  submitLoading: boolean
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  // 添加节点
  'add-node': []
  // 删除节点
  'remove-node': [index: number]
  // 提交（父组件处理 API）
  submit: []
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 暴露给父组件
defineExpose({ formRef, props })

/** 添加节点（父组件处理节点对象） */
const handleAddNode = () => {
  emit('add-node')
}

/** 删除节点 */
const handleRemoveNode = (index: number) => {
  emit('remove-node', index)
}

/** 提交（先校验，再通知父组件） */
const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    emit('submit')
  } catch {
    // 校验失败
  }
}
</script>

<style scoped>
.node-actions {
  margin-bottom: 12px;
}
</style>
