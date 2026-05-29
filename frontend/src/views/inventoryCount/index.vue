<script setup lang="ts">
import { ref } from 'vue'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElInput,
  ElSelect,
  ElMessageBox,
  ElMessage,
  ElRow,
  ElCol,
  ElPagination,
  ElDescriptions,
  ElDescriptionsItem,
} from 'element-plus'
import { Plus, Check, View } from '@element-plus/icons-vue'
import {
  listInventoryCounts,
  getInventoryCount,
  createInventoryCount,
  deleteInventoryCount,
  completeInventoryCount,
  getCountItems,
  updateCountItem,
  type InventoryCountEntity,
  type CountItem,
} from '@/api/inventoryCount'
import { request } from '@/api/request'

const tableData = ref<InventoryCountEntity[]>([])
const total = ref(0)
const loading = ref(false)
const searchForm = ref({
  count_no: '',
  warehouse_id: '',
  status: '',
})
const pagination = ref({
  page: 1,
  pageSize: 20,
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增盘点')
const form = ref<Partial<InventoryCountEntity>>({
  count_no: '',
  count_date: new Date().toISOString().split('T')[0],
  warehouse_id: 0,
  status: 'draft',
})

const viewDialogVisible = ref(false)
const viewData = ref<InventoryCountEntity | null>(null)
const detailData = ref<CountItem[]>([])
const editableDetailData = ref<CountItem[]>([])

const warehouseOptions = ref<{ label: string; value: number }[]>([])

const statusOptions = [
  { label: '全部', value: '' },
  { label: '进行中', value: 'draft' },
  { label: '已完成', value: 'completed' },
]

const getStatusLabel = (value: string) => {
  return statusOptions.find((s) => s.value === value)?.label || value
}

const getStatusClass = (value: string) => {
  return value === 'draft' ? 'status-draft' : 'status-completed'
}

const loadData = async () => {
  loading.value = true
  try {
    const res: any = await listInventoryCounts({
      page: pagination.value.page,
      pageSize: pagination.value.pageSize,
      count_no: searchForm.value.count_no,
      warehouse_id: searchForm.value.warehouse_id
        ? Number(searchForm.value.warehouse_id)
        : undefined,
      status: searchForm.value.status,
    })
    tableData.value = res.data!.list
    total.value = res.data!.total
  } catch (error) {
    ElMessage.error('加载失败')
  } finally {
    loading.value = false
  }
}

const loadWarehouses = async () => {
  try {
    const res: any = await request.get('/warehouses/select')
    warehouseOptions.value = res.data!
  } catch (error) {
    console.warn('加载仓库失败')
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  loadData()
}

const handleReset = () => {
  searchForm.value = {
    count_no: '',
    warehouse_id: '',
    status: '',
  }
  handleSearch()
}

const handlePageChange = (page: number) => {
  pagination.value.page = page
  loadData()
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.value.pageSize = pageSize
  loadData()
}

const openAddDialog = () => {
  dialogTitle.value = '新增盘点'
  form.value = {
    count_no: '',
    count_date: new Date().toISOString().split('T')[0],
    warehouse_id: 0,
    status: 'draft',
  }
  dialogVisible.value = true
}

const openViewDialog = async (row: InventoryCountEntity) => {
  try {
    const res: any = await getInventoryCount(row.id!)
    viewData.value = res.data!
    const itemsRes: any = await getCountItems(row.id!)
    detailData.value = itemsRes.data
    editableDetailData.value = JSON.parse(JSON.stringify(itemsRes.data))
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error('获取详情失败')
  }
}

const handleSubmit = async () => {
  if (!form.value.warehouse_id) {
    ElMessage.warning('请选择仓库')
    return
  }
  if (!form.value.count_date) {
    ElMessage.warning('请选择盘点日期')
    return
  }
  try {
    await createInventoryCount(form.value)
    ElMessage.success('新增成功')
    dialogVisible.value = false
    loadData()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  }
}

const handleDelete = async (row: InventoryCountEntity) => {
  if (row.status === 'completed') {
    ElMessage.warning('已完成的盘点不能删除')
    return
  }
  try {
    await ElMessageBox.confirm('确定要删除这个盘点吗？', '提示', {
      type: 'warning',
    })
    await deleteInventoryCount(row.id!)
    ElMessage.success('删除成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消删除')
  }
}

const handleApprove = async (row: InventoryCountEntity) => {
  try {
    await ElMessageBox.confirm('确定要审批通过这个盘点吗？', '提示', {
      type: 'warning',
    })
    await completeInventoryCount(row.id!)
    ElMessage.success('盘点已审批')
    loadData()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

const handleComplete = async (row: InventoryCountEntity) => {
  try {
    await ElMessageBox.confirm('确定要完成这个盘点吗？', '提示', {
      type: 'warning',
    })
    await completeInventoryCount(row.id!)
    ElMessage.success('盘点已完成')
    loadData()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

const updateActualQty = async (item: CountItem) => {
  item.diff_qty = item.actual_qty - item.system_qty
  item.diff_amount = item.diff_qty * item.cost_price
  await updateCountItem(item.id!, { actual_qty: item.actual_qty })
}

loadData()
loadWarehouses()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.count_no"
            placeholder="盘点单号"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElSelect v-model="searchForm.warehouse_id" placeholder="选择仓库" class="filter-item">
            <ElOption label="全部" value="" />
            <ElOption
              v-for="w in warehouseOptions"
              :key="w.value"
              :label="w.label"
              :value="String(w.value)"
            />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElSelect v-model="searchForm.status" placeholder="状态" class="filter-item">
            <ElOption v-for="s in statusOptions" :key="s.value" :label="s.label" :value="s.value" />
          </ElSelect>
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">查询</ElButton>
        <ElButton @click="handleReset">重置</ElButton>
        <ElButton type="success" @click="openAddDialog"> <Plus /> 新增盘点 </ElButton>
      </div>
    </div>

    <ElTable
      :data="tableData"
      :loading="loading"
      border
      fit
      highlight-current-row
      style="width: 100%"
    >
      <ElTableColumn prop="count_no" label="盘点单号" width="150" />
      <ElTableColumn prop="count_date" label="盘点日期" width="120" />
      <ElTableColumn prop="warehouse_name" label="仓库" width="120" />
      <ElTableColumn prop="status" label="状态" width="100">
        <template #default="{ row }">
          <span :class="['status-tag', getStatusClass(row.status)]">
            {{ getStatusLabel(row.status) }}
          </span>
        </template>
      </ElTableColumn>
      <ElTableColumn prop="created_by_name" label="创建人" width="100" />
      <ElTableColumn prop="created_at" label="创建时间" width="150" />
      <ElTableColumn prop="completed_at" label="完成时间" width="150" />
      <ElTableColumn label="操作" width="250" align="center">
        <template #default="{ row }">
          <ElButton size="small" @click="openViewDialog(row)"> <View /> 查看 </ElButton>
          <ElButton
            v-if="row.status === 'draft'"
            size="small"
            type="primary"
            @click="handleApprove(row)"
          >
            <Check /> 审批
          </ElButton>
          <ElButton
            v-if="row.status === 'approved'"
            size="small"
            type="warning"
            @click="handleComplete(row)"
          >
            <Check /> 完成盘点
          </ElButton>
          <ElButton
            v-if="row.status === 'draft'"
            size="small"
            type="danger"
            @click="handleDelete(row)"
            >删除</ElButton
          >
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination-wrapper">
      <ElPagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="handlePageSizeChange"
        @current-change="handlePageChange"
      />
    </div>

    <ElDialog
      :title="dialogTitle"
      :visible="dialogVisible"
      width="500px"
      @close="dialogVisible = false"
    >
      <ElForm :model="form" label-width="100px">
        <ElFormItem label="盘点单号" prop="count_no">
          <ElInput v-model="form.count_no" readonly />
        </ElFormItem>
        <ElFormItem label="盘点日期" prop="count_date">
          <ElDatePicker v-model="form.count_date" type="date" />
        </ElFormItem>
        <ElFormItem label="仓库" prop="warehouse_id">
          <ElSelect v-model="form.warehouse_id" placeholder="请选择仓库">
            <ElOption
              v-for="w in warehouseOptions"
              :key="w.value"
              :label="w.label"
              :value="w.value"
            />
          </ElSelect>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="dialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="handleSubmit">确定</ElButton>
      </template>
    </ElDialog>

    <ElDialog
      title="盘点详情"
      :visible="viewDialogVisible"
      width="900px"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <el-descriptions :column="4" border>
          <ElDescriptionsItem label="盘点单号">{{ viewData.count_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="盘点日期">{{ viewData.count_date }}</ElDescriptionsItem>
          <ElDescriptionsItem label="仓库">{{ viewData.warehouse_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="状态">{{
            getStatusLabel(viewData.status)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建人">{{ viewData.created_by_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建时间">{{ viewData.created_at }}</ElDescriptionsItem>
          <ElDescriptionsItem label="完成时间">{{
            viewData.completed_at || '-'
          }}</ElDescriptionsItem>
        </el-descriptions>
        <div style="margin-top: 20px">
          <h4>盘点明细</h4>
          <ElTable :data="editableDetailData" border style="width: 100%">
            <ElTableColumn prop="product_code" label="产品编码" width="120" />
            <ElTableColumn prop="product_name" label="产品名称" width="150" />
            <ElTableColumn prop="color_no" label="色号" width="100" />
            <ElTableColumn prop="grade" label="等级" width="80" />
            <ElTableColumn prop="unit" label="单位" width="80" />
            <ElTableColumn prop="system_qty" label="系统数量" width="100" align="right" />
            <ElTableColumn prop="actual_qty" label="实际数量" width="120" align="center">
              <template #default="{ row }">
                <ElInputNumber
                  v-if="viewData.status === 'draft'"
                  v-model="row.actual_qty"
                  :precision="0"
                  @change="updateActualQty(row)"
                />
                <span v-else>{{ row.actual_qty }}</span>
              </template>
            </ElTableColumn>
            <ElTableColumn prop="diff_qty" label="差异数量" width="120" align="right">
              <template #default="{ row }">
                <span :class="{ positive: row.diff_qty > 0, negative: row.diff_qty < 0 }">
                  {{ row.diff_qty }}
                </span>
              </template>
            </ElTableColumn>
            <ElTableColumn prop="diff_amount" label="差异金额" width="120" align="right">
              <template #default="{ row }">
                <span :class="{ positive: row.diff_amount > 0, negative: row.diff_amount < 0 }">
                  {{ row.diff_amount.toFixed(2) }}
                </span>
              </template>
            </ElTableColumn>
            <ElTableColumn prop="remark" label="备注" />
          </ElTable>
        </div>
      </div>
    </ElDialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.filter-container {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.filter-actions {
  margin-top: 10px;
}

.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}

.status-draft {
  background: #f5f7fa;
  color: #909399;
}

.status-completed {
  background: #f0f9eb;
  color: #67c23a;
}

.positive {
  color: #67c23a;
}

.negative {
  color: #f56c6c;
}

.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
