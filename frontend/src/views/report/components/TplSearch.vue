<script setup lang="ts">
/**
 * TplSearch - 报表模板搜索表单（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 顶部搜索）
 */
import { Plus } from '@element-plus/icons-vue'

interface Props {
  searchForm: {
    name: string
    type: string
    category: string
  }
  templateTypes: { label: string; value: string }[]
  categories: { label: string; value: string }[]
  onSearch: () => void
  onReset: () => void
  onCreate: () => void
}

defineProps<Props>()
</script>

<template>
  <div class="filter-container">
    <el-row :gutter="20">
      <el-col :span="7">
        <el-input
          v-model="searchForm.name"
          placeholder="模板名称"
          clearable
          @keyup.enter="onSearch"
        />
      </el-col>
      <el-col :span="5">
        <el-select v-model="searchForm.type" placeholder="报表类型" clearable>
          <el-option
            v-for="t in templateTypes"
            :key="t.value"
            :label="t.label"
            :value="t.value"
          />
        </el-select>
      </el-col>
      <el-col :span="5">
        <el-select v-model="searchForm.category" placeholder="分类" clearable>
          <el-option v-for="c in categories" :key="c.value" :label="c.label" :value="c.value" />
        </el-select>
      </el-col>
      <el-col :span="7">
        <el-button type="primary" @click="onSearch">查询</el-button>
        <el-button @click="onReset">重置</el-button>
        <el-button type="success" @click="onCreate">
          <el-icon><Plus /></el-icon> 创建模板
        </el-button>
      </el-col>
    </el-row>
  </div>
</template>
