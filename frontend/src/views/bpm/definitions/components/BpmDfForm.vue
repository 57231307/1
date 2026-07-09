<!--
  BpmDfForm.vue - BPM 流程定义新建/编辑对话框（含节点配置子表）
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  注意：formData 含 nodes 数组，需要深拷贝以保证本地修改与父组件解耦
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="isEdit ? '编辑流程定义' : '新建流程定义'"
    width="900px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="localFormData" :rules="rules" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="流程标识" prop="process_key">
            <el-input
              v-model="localFormData.process_key"
              :disabled="isEdit"
              placeholder="请输入流程标识（英文字母）"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="流程名称" prop="process_name">
            <el-input
              v-model="localFormData.process_name"
              placeholder="请输入流程名称"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="分类" prop="category">
            <el-select
              v-model="localFormData.category"
              placeholder="请选择分类"
              style="width: 100%"
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
          v-model="localFormData.description"
          type="textarea"
          :rows="3"
          placeholder="请输入描述"
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
      <el-table :data="localFormData.nodes" border>
        <el-table-column label="节点类型" width="120">
          <template #default="{ row }">
            <el-select
              v-model="row.type"
              size="small"
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
              v-model="row.name"
              size="small"
            />
          </template>
        </el-table-column>
        <el-table-column label="审批人类型" width="140">
          <template #default="{ row }">
            <el-select
              v-model="row.assignee_type"
              size="small"
              clearable
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
              v-model="row.condition"
              size="small"
              placeholder="如：amount > 1000"
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
import { deepClone } from '@/utils'
import { ref, watch, nextTick } from 'vue'
import { type FormInstance, type FormRules } from 'element-plus'
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
  // 表单数据（由父组件管理，子组件通过 emit 回写）
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
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  'update:formData': [v: BpmDfFormData]
  // 提交（父组件处理 API）
  submit: []
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：formData 含 nodes 数组，需要深拷贝以保证本地修改与父组件解耦
const localFormData = ref<BpmDfFormData>(deepClone(props.formData))

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件打开新建/编辑时填充数据）
watch(
  () => props.formData,
  (newData) => {
    if (syncing) return
    syncing = true
    localFormData.value = deepClone(newData)
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localFormData,
  (newData) => {
    if (syncing) return
    syncing = true
    emit('update:formData', deepClone(newData))
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 暴露给父组件
defineExpose({ formRef })

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
