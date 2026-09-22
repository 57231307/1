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
      <el-table
        v-loading="loading"
        :data="fabrics"
        stripe
        :aria-label="t('fabric.greigeTab.tableAriaLabel')"
      >
        <el-table-column prop="fabric_no" :label="t('fabric.greigeTab.columnCode')" width="140" />
        <el-table-column
          prop="fabric_name"
          :label="t('fabric.greigeTab.columnName')"
          min-width="150"
        />
        <!--
          供应商列暂移除：后端列表端点仅返回 supplier_id，不返回 supplier_name
          （见 models/greige_fabric.rs / handlers/greige_fabric_handler.rs list），
          不得在前端编造或 ?? '-' 兜底，待后端补字段后再加回。
        -->
        <el-table-column prop="width" :label="t('fabric.greigeTab.columnWidth')" width="80" />
        <el-table-column
          prop="gram_weight"
          :label="t('fabric.greigeTab.columnWeight')"
          width="80"
        />
        <el-table-column
          prop="composition"
          :label="t('fabric.greigeTab.columnComposition')"
          width="120"
        />
        <el-table-column
          prop="weight_kg"
          :label="t('fabric.greigeTab.columnWeightKg')"
          width="110"
          align="right"
        />
        <el-table-column
          prop="length_m"
          :label="t('fabric.greigeTab.columnLengthM')"
          width="110"
          align="right"
        />
        <el-table-column
          prop="status"
          :label="t('fabric.greigeTab.columnStatus')"
          width="90"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === '在库' ? 'success' : 'info'" size="small">
              {{ row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('fabric.greigeTab.columnAction')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openEdit(row)">{{
              t('fabric.greigeTab.buttonEdit')
            }}</el-button>
            <el-button type="success" link size="small" @click="emit('openStock', 'in', row)">{{
              t('fabric.greigeTab.buttonStockIn')
            }}</el-button>
            <el-button type="warning" link size="small" @click="emit('openStock', 'out', row)">{{
              t('fabric.greigeTab.buttonStockOut')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue';
import { useI18n } from 'vue-i18n';
import { Plus } from '@element-plus/icons-vue';
import type { GreigeFabric } from '@/api/greige-fabric';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  openDialog: [row: GreigeFabric | null];
  openStock: [type: 'in' | 'out', row: GreigeFabric];
}>();

const fabrics = ref<GreigeFabric[]>([]);
const loading = ref(false);

const fetchFabrics = async () => {
  loading.value = true;
  try {
    const { getGreigeFabricList } = await import('@/api/greige-fabric');
    const res = await getGreigeFabricList();
    // 响应形态兜底：数组或 { items } 分页包装（防 el-table r is not iterable 白屏）
    const _p = res.data as unknown;
    fabrics.value = Array.isArray(_p) ? _p : ((_p as { items?: GreigeFabric[] })?.items ?? []);
  } catch (error) {
    const err = error as Error;
    logger.error(t('fabric.greigeTab.fetchFailed'), err.message);
  } finally {
    loading.value = false;
  }
};

const openCreate = () => emit('openDialog', null);
const openEdit = (row: GreigeFabric) => emit('openDialog', row);

onMounted(() => fetchFabrics());

defineExpose({ fetchFabrics });
</script>
