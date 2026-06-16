<!--
  贸易条款编辑器
  - 4 个标签页：物流/付款/样品/检验
  - 每个标签页支持添加/删除条款
  - 通过 v-model 双向绑定 terms 数组
-->
<template>
  <div class="term-editor">
    <el-tabs v-model="activeTab" type="border-card">
      <el-tab-pane
        v-for="(label, type) in TERM_TYPE_LABELS"
        :key="type"
        :label="label"
        :name="type"
      >
        <div v-for="(term, idx) in getTerms(type as TermType)" :key="idx" class="term-row">
          <el-form-item :label="`条款 ${idx + 1}`">
            <el-input
              v-model="term.term_value"
              type="textarea"
              :rows="2"
              :placeholder="`请输入${label}内容`"
            />
          </el-form-item>
          <el-button link type="danger" @click="handleRemove(type as TermType, idx)">
            删除
          </el-button>
        </div>
        <el-button type="primary" plain @click="handleAdd(type as TermType)">
          <el-icon><Plus /></el-icon>
          添加{{ label }}
        </el-button>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
// 贸易条款编辑器脚本
import { ref } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import { TERM_TYPE_LABELS, type CreateQuotationTermDto, type TermType } from '@/api/quotation'

const props = defineProps<{
  modelValue: CreateQuotationTermDto[]
}>()
const emit = defineEmits<{
  (e: 'update:modelValue', value: CreateQuotationTermDto[]): void
}>()

const activeTab = ref<TermType>('logistics')

/** 获取指定类型的条款 */
function getTerms(type: TermType): CreateQuotationTermDto[] {
  return props.modelValue.filter(t => t.term_type === type)
}

/** 添加条款 */
function handleAdd(type: TermType) {
  const newTerm: CreateQuotationTermDto = {
    term_type: type,
    term_key: '',
    term_value: '',
    sequence: props.modelValue.length,
  }
  emit('update:modelValue', [...props.modelValue, newTerm])
}

/** 删除条款 */
function handleRemove(type: TermType, idxInType: number) {
  const arr = [...props.modelValue]
  // 找到 type 类型中的第 idxInType 个
  let found = -1
  let count = 0
  for (let i = 0; i < arr.length; i++) {
    if (arr[i].term_type === type) {
      if (count === idxInType) {
        found = i
        break
      }
      count++
    }
  }
  if (found >= 0) {
    arr.splice(found, 1)
    emit('update:modelValue', arr)
  }
}
</script>

<style scoped>
.term-editor {
  width: 100%;
}
.term-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 8px;
}
.term-row .el-form-item {
  flex: 1;
  margin-bottom: 0;
}
</style>
