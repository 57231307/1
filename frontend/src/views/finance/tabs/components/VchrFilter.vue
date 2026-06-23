<!--
  VchrFilter.vue - 凭证过滤表单
  拆分自 VoucherTab.vue（P14 批 1 B3 I-2）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localVoucherQuery">
      <el-form-item label="凭证号">
        <el-input
          v-model="localVoucherQuery.voucher_no"
          placeholder="凭证号"
          clearable
        />
      </el-form-item>
      <el-form-item label="日期范围">
        <el-date-picker
          v-model="localVoucherQuery.date_range"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          value-format="YYYY-MM-DD"
        />
      </el-form-item>
      <el-form-item label="状态">
        <el-select v-model="localVoucherQuery.status" placeholder="选择状态" clearable>
          <el-option label="草稿" value="draft" />
          <el-option label="已提交" value="submitted" />
          <el-option label="已审核" value="reviewed" />
          <el-option label="已过账" value="posted" />
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

/**
 * 凭证过滤表单组件
 * 接收父组件传入的查询对象，通过 emit('update:voucherQuery') 回写
 */
interface VoucherQuery {
  voucher_no: string
  date_range: string[]
  status: string
}

const props = defineProps<{
  // 凭证查询条件（由父组件管理，子组件通过 emit 回写）
  voucherQuery: VoucherQuery
}>()

const emit = defineEmits<{
  // 查询按钮点击
  (e: 'search'): void
  // 重置按钮点击
  (e: 'reset'): void
  // 整体回写查询条件（父组件监听此事件并 Object.assign 到自己的 voucherQuery）
  (e: 'update:voucherQuery', voucherQuery: VoucherQuery): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：date_range 是数组，需要深拷贝以保证本地修改与父组件解耦
const localVoucherQuery = ref<VoucherQuery>({
  ...props.voucherQuery,
  date_range: [...(props.voucherQuery.date_range || [])],
})

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.voucherQuery,
  (newQuery) => {
    if (syncing) return
    syncing = true
    localVoucherQuery.value = {
      ...newQuery,
      date_range: [...(newQuery.date_range || [])],
    }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localVoucherQuery,
  (newQuery) => {
    if (syncing) return
    syncing = true
    emit('update:voucherQuery', {
      ...newQuery,
      date_range: [...(newQuery.date_range || [])],
    })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>
