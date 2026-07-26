<!--
  ProductionFilter.vue - 生产管理过滤栏
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-card shadow="never" class="filter-card">
    <el-form
      :inline="true"
      :model="localForm"
      :aria-label="t('production.filter.ariaLabel')"
      @submit.prevent
    >
      <el-form-item :label="t('production.filter.labelOrderNo')">
        <el-input
          v-model="localForm.order_no"
          :placeholder="t('production.filter.placeholderOrderNo')"
          clearable
          style="width: 200px"
        />
      </el-form-item>
      <el-form-item :label="t('production.filter.labelStatus')">
        <el-select
          v-model="localForm.status"
          :placeholder="t('production.filter.placeholderStatus')"
          clearable
          style="width: 150px"
        >
          <el-option :label="t('production.filter.statusDraft')" value="draft" />
          <el-option :label="t('production.filter.statusPlanned')" value="planned" />
          <el-option :label="t('production.filter.statusInProgress')" value="in_progress" />
          <el-option :label="t('production.filter.statusCompleted')" value="completed" />
          <el-option :label="t('production.filter.statusCancelled')" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('search')">{{
          t('production.filter.buttonSearch')
        }}</el-button>
        <el-button @click="emit('reset')">{{ t('production.filter.buttonReset') }}</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n({ useScope: 'global' })

// 过滤表单字段类型
interface FilterForm {
  order_no: string
  status: string
}

const props = defineProps<{
  form: FilterForm
}>()

const emit = defineEmits<{
  search: []
  reset: []
  'update:form': [form: FilterForm]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<FilterForm>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.form,
  newForm => {
    if (syncing) return
    syncing = true
    localForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true }
)

// 本地变化时通知父组件
watch(
  localForm,
  newForm => {
    if (syncing) return
    syncing = true
    emit('update:form', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true }
)
</script>

<style scoped>
.filter-card {
  margin-bottom: 16px;
}
</style>
