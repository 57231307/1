<!--
  GreigeTab.vue - 坯布管理 Tab
  来源：原 fabric/index.vue 中 坯布管理 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="greige-tab">
    <div class="page-header">
      <h2 class="page-title">坯布管理</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        新建坯布
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="fabrics" stripe>
        <el-table-column prop="fabric_code" label="编号" width="120" />
        <el-table-column prop="fabric_name" label="名称" min-width="150" />
        <el-table-column prop="supplier_name" label="供应商" width="150" />
        <el-table-column prop="width" label="幅宽" width="80" />
        <el-table-column prop="weight" label="克重" width="80" />
        <el-table-column prop="composition" label="成分" width="120" />
        <el-table-column prop="quantity" label="库存" width="100" align="right" />
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '正常' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openEdit(row)">编辑</el-button>
            <el-button type="success" link size="small" @click="emit('openStock', 'in', row)"
              >入库</el-button
            >
            <el-button type="warning" link size="small" @click="emit('openStock', 'out', row)"
              >出库</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import type { GreigeFabric } from '@/api/greige-fabric'
import { logger } from '@/utils/logger'

const emit = defineEmits<{
  openDialog: [row: GreigeFabric | null]
  openStock: [type: 'in' | 'out', row: GreigeFabric]
}>()

const fabrics = ref<GreigeFabric[]>([])
const loading = ref(false)

const fetchFabrics = async () => {
  loading.value = true
  try {
    const { listGreigeFabrics } = await import('@/api/greige-fabric')
    const res = await listGreigeFabrics()
    fabrics.value = (res.data as GreigeFabric[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取坯布列表失败', err.message)
  } finally {
    loading.value = false
  }
}

const openCreate = () => emit('openDialog', null)
const openEdit = (row: GreigeFabric) => emit('openDialog', row)

onMounted(() => fetchFabrics())

defineExpose({ fetchFabrics })
</script>
