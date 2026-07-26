<!--
  GreigeTab.vue - 坯布管理 Tab
  来源：原 fabric/index.vue 中 坯布管理 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="greige-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('fabric.greigeTab.title') }}</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        {{ t('fabric.greigeTab.buttonCreate') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="fabrics" stripe :aria-label="t('fabric.greigeTab.tableAriaLabel')">
        <el-table-column prop="fabric_code" :label="t('fabric.greigeTab.columnCode')" width="120" />
        <el-table-column prop="fabric_name" :label="t('fabric.greigeTab.columnName')" min-width="150" />
        <el-table-column prop="supplier_name" :label="t('fabric.greigeTab.columnSupplier')" width="150" />
        <el-table-column prop="width" :label="t('fabric.greigeTab.columnWidth')" width="80" />
        <el-table-column prop="weight" :label="t('fabric.greigeTab.columnWeight')" width="80" />
        <el-table-column prop="composition" :label="t('fabric.greigeTab.columnComposition')" width="120" />
        <el-table-column prop="quantity" :label="t('fabric.greigeTab.columnQuantity')" width="100" align="right" />
        <el-table-column prop="status" :label="t('fabric.greigeTab.columnStatus')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? t('fabric.greigeTab.statusActive') : t('fabric.greigeTab.statusInactive') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('fabric.greigeTab.columnAction')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openEdit(row)">{{ t('fabric.greigeTab.buttonEdit') }}</el-button>
            <el-button type="success" link size="small" @click="emit('openStock', 'in', row)"
              >{{ t('fabric.greigeTab.buttonStockIn') }}</el-button
            >
            <el-button type="warning" link size="small" @click="emit('openStock', 'out', row)"
              >{{ t('fabric.greigeTab.buttonStockOut') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue'
import { useI18n } from 'vue-i18n'
import { Plus } from '@element-plus/icons-vue'
import type { GreigeFabric } from '@/api/greige-fabric'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

const emit = defineEmits<{
  openDialog: [row: GreigeFabric | null]
  openStock: [type: 'in' | 'out', row: GreigeFabric]
}>()

const fabrics = ref<GreigeFabric[]>([])
const loading = ref(false)

const fetchFabrics = async () => {
  loading.value = true
  try {
    const { getGreigeFabricList } = await import('@/api/greige-fabric')
    const res = await getGreigeFabricList()
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
