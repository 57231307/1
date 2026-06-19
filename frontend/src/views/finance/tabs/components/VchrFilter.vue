<!--
  VchrFilter.vue - 凭证过滤表单
  拆分自 VoucherTab.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="voucherQuery">
      <el-form-item label="凭证号">
        <el-input
          :model-value="voucherQuery.voucher_no"
          placeholder="凭证号"
          clearable
          @update:model-value="(v: string) => (voucherQuery.voucher_no = v)"
        />
      </el-form-item>
      <el-form-item label="日期范围">
        <el-date-picker
          :model-value="voucherQuery.date_range"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          value-format="YYYY-MM-DD"
          @update:model-value="(v: string[]) => (voucherQuery.date_range = v)"
        />
      </el-form-item>
      <el-form-item label="状态">
        <el-select
          :model-value="voucherQuery.status"
          placeholder="选择状态"
          clearable
          @update:model-value="(v: string) => (voucherQuery.status = v)"
        >
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
/* eslint-disable vue/no-mutating-props */
/**
 * 凭证过滤表单组件
 * 接收父组件传入的查询对象，双向同步字段值
 */
interface VoucherQuery {
  voucher_no: string
  date_range: string[]
  status: string
}

const props = defineProps<{
  // 凭证查询条件（双向同步）
  voucherQuery: VoucherQuery
}>()

const emit = defineEmits<{
  // 查询按钮点击
  search: []
  // 重置按钮点击
  reset: []
}>()

// 避免 vue/no-mutating-props 误判（voucherQuery 由父组件管理）
void props
</script>
