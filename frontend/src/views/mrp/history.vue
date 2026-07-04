<template>
  <div class="mrp-history-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>MRP 计算历史</h2>
        <p>查看历史 MRP 计算记录及结果</p>
      </div>
    </el-card>

    <!-- 历史记录列表 -->
    <el-card class="table-card">
      <el-table v-loading="loading" :data="historyList" stripe border>
        <el-table-column prop="calculation_no" label="计算编号" width="180" />
        <el-table-column label="产品" min-width="200">
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
        <el-table-column prop="demand_quantity" label="需求数量" width="120" align="right" />
        <el-table-column prop="demand_date" label="需求日期" width="130" />
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="STATUS_MAP[row.status as keyof typeof STATUS_MAP]?.type || 'info'">
              {{ STATUS_MAP[row.status as keyof typeof STATUS_MAP]?.label || row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="计算时间" width="180" />
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              :disabled="row.status !== 'completed'"
              @click="viewResult(row as any)"
            >
              查看结果
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页 -->
      <div class="pagination-container">
        <el-pagination
          v-model:current-page="queryForm.page"
          v-model:page-size="queryForm.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchHistory"
          @current-change="fetchHistory"
        />
      </div>
    </el-card>

    <!-- 结果详情对话框 -->
    <el-dialog v-model="resultVisible" title="MRP 计算结果" width="90%" top="5vh">
      <template v-if="currentResult">
        <el-descriptions :column="3" border class="result-header">
          <el-descriptions-item label="计算编号">{{
            currentResult.calculation_no
          }}</el-descriptions-item>
          <el-descriptions-item label="需求数量">{{
            currentResult.demand_quantity
          }}</el-descriptions-item>
          <el-descriptions-item label="需求日期">{{
            currentResult.demand_date
          }}</el-descriptions-item>
          <el-descriptions-item label="计算时间">{{
            currentResult.created_at
          }}</el-descriptions-item>
          <el-descriptions-item label="完成时间">{{
            currentResult.completed_at || '-'
          }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="STATUS_MAP[currentResult.status as keyof typeof STATUS_MAP]?.type">
              {{ STATUS_MAP[currentResult.status as keyof typeof STATUS_MAP]?.label }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="产品" :span="3">
            <el-tag
              v-for="(product, index) in currentResult.products"
              :key="index"
              style="margin-right: 4px; margin-bottom: 4px"
            >
              {{ product.product_code }} - {{ product.product_name }}
            </el-tag>
          </el-descriptions-item>
        </el-descriptions>

        <el-divider content-position="left">物料需求清单</el-divider>

        <el-table :data="currentResult.materials" stripe border max-height="400">
          <el-table-column prop="material_code" label="物料编码" width="140" />
          <el-table-column prop="material_name" label="物料名称" min-width="160" />
          <el-table-column prop="specification" label="规格" min-width="120" />
          <el-table-column prop="unit" label="单位" width="80" />
          <el-table-column prop="required_quantity" label="需求数量" width="120" align="right" />
          <el-table-column prop="available_stock" label="可用库存" width="120" align="right" />
          <el-table-column prop="in_transit_quantity" label="在途量" width="100" align="right" />
          <el-table-column prop="safety_stock" label="安全库存" width="100" align="right" />
          <el-table-column prop="net_requirement" label="净需求" width="120" align="right">
            <template #default="{ row }">
              <span :class="{ 'highlight-quantity': row.net_requirement > 0 }">{{
                row.net_requirement
              }}</span>
            </template>
          </el-table-column>
          <el-table-column
            prop="suggested_order_quantity"
            label="建议订单量"
            width="130"
            align="right"
          />
          <el-table-column prop="suggested_date" label="建议日期" width="130" />
        </el-table>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getMrpHistory,
  getMrpResult,
  type MrpHistoryRecord,
  type MrpCalculationResult,
} from '../../api/mrp'

const STATUS_MAP = {
  pending: { label: '待计算', type: 'info' },
  calculating: { label: '计算中', type: 'warning' },
  completed: { label: '已完成', type: 'success' },
  failed: { label: '计算失败', type: 'danger' },
}

const loading = ref(false)
const historyList = ref<MrpHistoryRecord[]>([])
const total = ref(0)
const resultVisible = ref(false)
const currentResult = ref<MrpCalculationResult | null>(null)

const queryForm = reactive({
  page: 1,
  page_size: 20,
})

const fetchHistory = async () => {
  loading.value = true
  try {
    const res = await getMrpHistory(queryForm)
    historyList.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || '获取历史记录失败')
  } finally {
    loading.value = false
  }
}

const viewResult = async (row: MrpHistoryRecord) => {
  try {
    const res = await getMrpResult(row.id)
    currentResult.value = res.data || null
    resultVisible.value = true
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || '获取计算结果失败')
  }
}

onMounted(() => {
  fetchHistory()
})
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
