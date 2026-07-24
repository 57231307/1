<!--
  ProductionFilter.vue - 生产管理过滤栏
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-card shadow="never" class="filter-card">
    <el-form :inline="true" :model="localForm" @submit.prevent aria-label="生产订单筛选表单">
      <el-form-item label="订单编号">
        <el-input
          v-model="localForm.order_no"
          placeholder="请输入订单编号"
          clearable
          style="width: 200px"
        />
      </el-form-item>
      <el-form-item label="状态">
        <el-select
          v-model="localForm.status"
          placeholder="请选择状态"
          clearable
          style="width: 150px"
        >
          <el-option label="草稿" value="draft" />
          <el-option label="已计划" value="planned" />
          <el-option label="进行中" value="in_progress" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('search')">查询</el-button>
        <el-button @click="emit('reset')">重置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'

// 过滤表单字段类型
interface FilterForm {
  order_no: string
  status: string
}

const props = defineProps<{
  // 过滤表单数据（由父组件管理，子组件通过 emit('update:form') 回写）
  form: FilterForm
}>()

const emit = defineEmits<{
  search: []
  reset: []
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 form）
  'update:form': [form: FilterForm]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<FilterForm>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
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
</script>

<style scoped>
.filter-card {
  margin-bottom: 16px;
}
</style>
