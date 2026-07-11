<!--
  VchrFilter.vue - 凭证过滤表单
  拆分自 VoucherTab.vue（P14 批 1 B3 I-2）
  批次 289：改造为 localQuery + handleSearch 模式，接入 useTableApi queryParams
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQuery">
      <el-form-item label="凭证号">
        <el-input
          v-model="localQuery.voucher_no"
          placeholder="凭证号"
          clearable
        />
      </el-form-item>
      <el-form-item label="日期范围">
        <el-date-picker
          v-model="localQuery.date_range"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          value-format="YYYY-MM-DD"
        />
      </el-form-item>
      <el-form-item label="状态">
        <el-select v-model="localQuery.status" placeholder="选择状态" clearable>
          <el-option label="草稿" value="draft" />
          <el-option label="已提交" value="submitted" />
          <el-option label="已审核" value="reviewed" />
          <el-option label="已过账" value="posted" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">查询</el-button>
        <el-button @click="handleReset">重置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue'

/**
 * 凭证过滤表单组件
 * 接收父组件传入的 queryParams，通过 emit('update:queryParams') 同步筛选条件
 * 查询/重置时先同步 queryParams 再触发 fetch
 */
const props = defineProps<{
  // 查询条件（由父组件 useTableApi 管理，类型放宽为 Record 兼容 useTableApi）
  queryParams: Record<string, unknown>
}>()

const emit = defineEmits<{
  // 触发查询（父组件监听后调用 handleSearch 重置页码并加载）
  (e: 'fetch'): void
  // 同步查询条件到父组件
  (e: 'update:queryParams', params: Record<string, unknown>): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// date_range 是数组，需要深拷贝以保证本地修改与父组件解耦
const localQuery = reactive({
  voucher_no: (props.queryParams.voucher_no as string) ?? '',
  date_range: [...((props.queryParams.date_range as string[]) ?? [])],
  status: (props.queryParams.status as string) ?? '',
})

/** 查询：先同步筛选条件到父组件，再触发 fetch */
const handleSearch = () => {
  emit('update:queryParams', {
    ...localQuery,
    date_range: [...localQuery.date_range],
  })
  emit('fetch')
}

/** 重置：清空本地筛选条件，同步后触发 fetch */
const handleReset = () => {
  localQuery.voucher_no = ''
  localQuery.date_range = []
  localQuery.status = ''
  emit('update:queryParams', {
    ...localQuery,
    date_range: [...localQuery.date_range],
  })
  emit('fetch')
}
</script>
