<script setup lang="ts">
/**
 * TplPgn - 报表模板分页（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 分页）
 */
interface Props {
  page: number
  pageSize: number
  total: number
  onPageChange: (page: number) => void
  onPageSizeChange: (pageSize: number) => void
}

defineProps<Props>()
</script>

<template>
  <el-pagination
    :current-page="page"
    :page-size="pageSize"
    :total="total"
    :page-sizes="[10, 20, 50, 100]"
    layout="total, sizes, prev, pager, next"
    class="pagination-container"
    @current-change="onPageChange"
    @size-change="onPageSizeChange"
  />
</template>
