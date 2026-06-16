<template>
  <div class="component-demo-page">
    <el-tabs v-model="activeTab" type="border-card">
      <el-tab-pane label="图表组件" name="charts">
        <ChartsDemo />
      </el-tab-pane>
      <el-tab-pane label="批量操作" name="batch">
        <BatchActionsDemo />
      </el-tab-pane>
      <el-tab-pane label="高级筛选" name="filter">
        <AdvancedFilterDemo />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import ChartsDemo from './ChartsDemo.vue'
import BatchActionsDemo from './BatchActionsDemo.vue'
import AdvancedFilterDemo from './AdvancedFilterDemo.vue'

const activeTab = ref('charts')
</script>

<style scoped>
.component-demo-page {
  padding: 20px;
}
</style>
