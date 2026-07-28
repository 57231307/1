<template>
  <div class="mrp-history-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>{{ t('mrp.history.title') }}</h2>
        <p>{{ t('mrp.history.subtitle') }}</p>
      </div>
    </el-card>

    <!-- 历史记录列表 -->
    <el-card class="table-card">
      <el-table
        v-loading="loading"
        :data="historyList"
        stripe
        border
        :aria-label="t('mrp.history.listAriaLabel')"
      >
        <el-table-column
          prop="calculation_no"
          :label="t('mrp.history.calculationNo')"
          width="180"
        />
        <el-table-column :label="t('mrp.history.product')" min-width="200">
          <template #default="{ row }">
            <el-tag
              v-for="(product, index) in row.products"
              :key="index"
              size="small"
              style="margin-right: 4px; margin-bottom: 4px"
            >
              {{ product.product_name }}
            </el-tag>
            <span v-if="!row.products || row.products.length === 0">-</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="demand_quantity"
          :label="t('mrp.history.demandQuantity')"
          width="120"
          align="right"
        />
        <el-table-column prop="demand_date" :label="t('mrp.history.demandDate')" width="130" />
        <el-table-column prop="status" :label="t('mrp.history.status')" width="120">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('mrp.history.createdAt')" width="180" />
        <el-table-column :label="t('mrp.history.operation')" width="150" fixed="right">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              :disabled="row.status !== 'completed'"
              @click="viewResult(row as MrpHistoryRecord)"
            >
              {{ t('mrp.history.viewResult') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页 -->
      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('mrp.history.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <!-- 结果详情对话框 -->
    <el-dialog
      v-model="resultVisible"
      :title="t('mrp.history.resultDialogTitle')"
      width="90%"
      top="5vh"
      :aria-label="t('mrp.history.resultDialogAriaLabel')"
    >
      <template v-if="currentResult">
        <el-descriptions :column="3" border class="result-header">
          <el-descriptions-item :label="t('mrp.history.calculationNo')">{{
            currentResult.calculation_no
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('mrp.history.demandQuantity')">{{
            currentResult.demand_quantity
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('mrp.history.demandDate')">{{
            currentResult.demand_date
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('mrp.history.createdAt')">{{
            currentResult.created_at
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('mrp.history.completedAt')">{{
            currentResult.completed_at || '-'
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('mrp.history.status')">
            <el-tag :type="getStatusType(currentResult.status)">
              {{ getStatusLabel(currentResult.status) }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item :label="t('mrp.history.product')" :span="3">
            <el-tag
              v-for="(product, index) in currentResult.products"
              :key="index"
              style="margin-right: 4px; margin-bottom: 4px"
            >
              {{ product.product_code }} - {{ product.product_name }}
            </el-tag>
          </el-descriptions-item>
        </el-descriptions>

        <el-divider content-position="left">{{ t('mrp.history.materialsDivider') }}</el-divider>

        <el-table
          :data="currentResult.materials"
          stripe
          border
          max-height="400"
          :aria-label="t('mrp.history.detailAriaLabel')"
        >
          <el-table-column
            prop="material_code"
            :label="t('mrp.history.materialCode')"
            width="140"
          />
          <el-table-column
            prop="material_name"
            :label="t('mrp.history.materialName')"
            min-width="160"
          />
          <el-table-column
            prop="specification"
            :label="t('mrp.history.specification')"
            min-width="120"
          />
          <el-table-column prop="unit" :label="t('mrp.history.unit')" width="80" />
          <el-table-column
            prop="required_quantity"
            :label="t('mrp.history.demandQuantity')"
            width="120"
            align="right"
          />
          <el-table-column
            prop="available_stock"
            :label="t('mrp.history.availableStock')"
            width="120"
            align="right"
          />
          <el-table-column
            prop="in_transit_quantity"
            :label="t('mrp.history.inTransitQuantity')"
            width="100"
            align="right"
          />
          <el-table-column
            prop="safety_stock"
            :label="t('mrp.history.safetyStock')"
            width="100"
            align="right"
          />
          <el-table-column
            prop="net_requirement"
            :label="t('mrp.history.netRequirement')"
            width="120"
            align="right"
          >
            <template #default="{ row }">
              <span :class="{ 'highlight-quantity': row.net_requirement > 0 }">{{
                row.net_requirement
              }}</span>
            </template>
          </el-table-column>
          <el-table-column
            prop="suggested_order_quantity"
            :label="t('mrp.history.suggestedOrderQuantity')"
            width="130"
            align="right"
          />
          <el-table-column
            prop="suggested_date"
            :label="t('mrp.history.suggestedDate')"
            width="130"
          />
        </el-table>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { ElMessage } from 'element-plus';
import { useI18n } from 'vue-i18n';
import { getMrpResult, type MrpHistoryRecord, type MrpCalculationResult } from '../../api/mrp';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

/**
 * 状态类型映射（el-tag type）
 */
const STATUS_TYPE_MAP: Record<string, string> = {
  pending: 'info',
  calculating: 'warning',
  completed: 'success',
  failed: 'danger',
};

const getStatusType = (status: string) => STATUS_TYPE_MAP[status] || 'info';

/**
 * 状态标签映射（基于 i18n）
 */
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: t('mrp.history.statusPending'),
    calculating: t('mrp.history.statusCalculating'),
    completed: t('mrp.history.statusCompleted'),
    failed: t('mrp.history.statusFailed'),
  };
  return map[status] || status;
};

const resultVisible = ref(false);
const currentResult = ref<MrpCalculationResult | null>(null);

// 批次 274：接入 useTableApi，消除手写 historyList/total/loading/queryForm.page/page_size + fetchHistory 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: historyList,
  loading,
  page,
  pageSize,
  total,
} = useTableApi<MrpHistoryRecord>({
  url: '/production/mrp-history',
  listKey: 'list',
  onError: (e: unknown) => {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('mrp.history.fetchListError')
    );
  },
});

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handleSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

const handleCurrentChange = (p: number) => {
  page.value = p;
};

const viewResult = async (row: MrpHistoryRecord) => {
  try {
    const res = await getMrpResult(row.id);
    currentResult.value = res.data || null;
    resultVisible.value = true;
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('mrp.history.fetchResultError')
    );
  }
};

// 批次 274：useTableApi 构造时自动初始加载，无需 onMounted 调用 fetchHistory
</script>

<style scoped>
.mrp-history-container {
  padding: 20px;
}

.header-card {
  margin-bottom: 20px;
}

.header-content h2 {
  margin: 0 0 8px 0;
  color: #303133;
}

.header-content p {
  margin: 0;
  color: #909399;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.result-header {
  margin-bottom: 16px;
}

.highlight-quantity {
  color: #e6a23c;
  font-weight: bold;
}
</style>
