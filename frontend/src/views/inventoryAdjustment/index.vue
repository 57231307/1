<script setup lang="ts">
import { ref } from 'vue'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElSelect,
  ElDatePicker,
  ElInputNumber,
  ElMessageBox,
  ElMessage,
  ElRow,
  ElCol,
  ElDescriptions,
  ElPagination,
} from 'element-plus'
import { Plus, Edit, Delete, View, Check } from '@element-plus/icons-vue'
import {
  listInventoryAdjustments,
  getInventoryAdjustment,
  createInventoryAdjustment,
  updateInventoryAdjustment,
  deleteInventoryAdjustment,
  approveInventoryAdjustment,
  getAdjustmentItems,
  type InventoryAdjustmentEntity,
  type AdjustmentItem,
} from '@/api/inventoryAdjustment'
import { request } from '@/api/request'

const tableData = ref<InventoryAdjustmentEntity[]>([])
const total = ref(0)
const loading = ref(false)
const searchForm = ref({
  adjust_no: '',
  warehouse_id: '',
  status: '',
})
const pagination = ref({
  page: 1,
  pageSize: 20,
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增库存调整')
const form = ref<Partial<InventoryAdjustmentEntity>>({
  adjust_no: '',
  adjust_date: new Date().toISOString().split('T')[0],
  warehouse_id: 0,
  reason: '',
  status: 'draft',
  items: [],
})

const viewDialogVisible = ref(false)
const viewData = ref<InventoryAdjustmentEntity | null>(null)
const detailData = ref<AdjustmentItem[]>([])

const warehouseOptions = ref<{ label: string; value: number }[]>([])
const productOptions = ref<{ label: string; value: number }[]>([])

const statusOptions = [
  { label: '全部', value: '' },
  { label: '草稿', value: 'draft' },
  { label: '已审核', value: 'approved' },
]

const getStatusLabel = (value: string) => {
  return statusOptions.find((s) => s.value === value)?.label || value
}

const getStatusClass = (value: string) => {
  return value === 'draft' ? 'status-draft' : 'status-approved'
}

const loadData = async () => {
  loading.value = true
  try {
    const res: any = await listInventoryAdjustments({
      page: pagination.value.page,
      pageSize: pagination.value.pageSize,
      adjust_no: searchForm.value.adjust_no,
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

const loadProducts = async () => {
  try {
    const res: any = await request.get('/products/select')
    productOptions.value = res.data!
  } catch (error) {
    console.warn('加载产品失败')
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  loadData()
}

const handleReset = () => {
  searchForm.value = {
    adjust_no: '',
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
  dialogTitle.value = '新增库存调整'
  form.value = {
    adjust_no: '',
    adjust_date: new Date().toISOString().split('T')[0],
    warehouse_id: 0,
    reason: '',
    status: 'draft',
    items: [{ product_id: 0, quantity: 0, cost_price: 0, amount: 0 }],
  }
  dialogVisible.value = true
}

const openEditDialog = async (row: InventoryAdjustmentEntity) => {
  dialogTitle.value = '编辑库存调整'
  const res: any = await getInventoryAdjustment(row.id!)
  const itemsRes: any = await getAdjustmentItems(row.id!)
  form.value = { ...res.data, items: itemsRes.data }
  dialogVisible.value = true
}

const openViewDialog = async (row: InventoryAdjustmentEntity) => {
  try {
    const res: any = await getInventoryAdjustment(row.id!)
    viewData.value = res.data!
    const itemsRes: any = await getAdjustmentItems(row.id!)
    detailData.value = itemsRes.data
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error('获取详情失败')
  }
}

const calculateAmount = (item: any) => {
  item.amount = (item.quantity || 0) * (item.cost_price || 0)
}

const addItem = () => {
  if (!form.value.items) form.value.items = []
  form.value.items.push({ product_id: 0, quantity: 0, cost_price: 0, amount: 0 })
}

const removeItem = (index: number) => {
  if ((form.value.items || []).length > 1) {
    form.value.items!.splice(index, 1)
  }
}

const handleSubmit = async () => {
  if (!form.value.warehouse_id) {
    ElMessage.warning('请选择仓库')
    return
  }
  if (!form.value.reason) {
    ElMessage.warning('请输入调整原因')
    return
  }
  if (!form.value.adjust_date) {
    ElMessage.warning('请选择调整日期')
    return
  }
  const validItems = (form.value.items || []).filter((e) => e.product_id > 0 && e.quantity !== 0)
  if (validItems.length === 0) {
    ElMessage.warning('请至少添加一条有效的调整明细')
    return
  }
  try {
    const data = { ...form.value, items: validItems }
    if (form.value.id) {
      await updateInventoryAdjustment(form.value.id, data)
      ElMessage.success('更新成功')
    } else {
      await createInventoryAdjustment(data)
      ElMessage.success('新增成功')
    }
    dialogVisible.value = false
    loadData()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  }
}

const handleDelete = async (row: InventoryAdjustmentEntity) => {
  if (row.status === 'approved') {
    ElMessage.warning('已审核的调整单不能删除')
    return
  }
  try {
    await ElMessageBox.confirm('确定要删除这个调整单吗？', '提示', {
      type: 'warning',
    })
    await deleteInventoryAdjustment(row.id!)
    ElMessage.success('删除成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消删除')
  }
}

const handleApprove = async (row: InventoryAdjustmentEntity) => {
  try {
    await ElMessageBox.confirm('确定要审核这个调整单吗？', '提示', {
      type: 'warning',
    })
    await approveInventoryAdjustment(row.id!)
    ElMessage.success('审核成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消操作')
  }
}

loadData()
loadWarehouses()
loadProducts()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.adjust_no"
            placeholder="调整单号"
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
        <ElButton type="success" @click="openAddDialog"> <Plus /> 新增调整单 </ElButton>
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
      <ElTableColumn prop="adjust_no" label="调整单号" width="150" />
      <ElTableColumn prop="adjust_date" label="调整日期" width="120" />
      <ElTableColumn prop="warehouse_name" label="仓库" width="120" />
      <ElTableColumn prop="reason" label="调整原因" width="200" />
      <ElTableColumn prop="total_amount" label="调整金额" width="120" align="right">
        <template #default="scope">{{ scope.row.total_amount.toFixed(2) }}</template>
      </ElTableColumn>
      <ElTableColumn prop="status" label="状态" width="100">
        <template #default="scope">
          <span :class="['status-tag', getStatusClass(scope.row.status)]">
            {{ getStatusLabel(scope.row.status) }}
          </span>
        </template>
      </ElTableColumn>
      <ElTableColumn prop="created_by_name" label="创建人" width="100" />
      <ElTableColumn prop="created_at" label="创建时间" width="150" />
      <ElTableColumn label="操作" width="250" align="center">
        <template #default="scope">
          <ElButton size="small" @click="openViewDialog(scope.row as any)">
            <View />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="primary"
            @click="openEditDialog(scope.row as any)"
          >
            <Edit />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="warning"
            @click="handleApprove(scope.row as any)"
          >
            <Check /> 审核
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="danger"
            @click="handleDelete(scope.row as any)"
          >
            <Delete />
          </ElButton>
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

    <ElDialog v-model="dialogVisible" :title="dialogTitle" width="800px">
      <ElForm :model="form" label-width="100px">
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem label="调整单号" prop="adjust_no">
              <ElInput v-model="form.adjust_no" readonly />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="调整日期" prop="adjust_date">
              <ElDatePicker v-model="form.adjust_date" type="date" />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow :gutter="20">
          <ElCol :span="12">
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
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="调整原因" prop="reason">
              <ElInput v-model="form.reason" placeholder="请输入调整原因" />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElFormItem label="调整明细">
          <div class="items-table">
            <div class="items-header">
              <span class="col-product">产品</span>
              <span class="col-qty">数量</span>
              <span class="col-price">单价</span>
              <span class="col-amount">金额</span>
              <span class="col-action">操作</span>
            </div>
            <div v-for="(item, index) in form.items" :key="index" class="items-row">
              <ElSelect v-model="item.product_id" placeholder="选择产品" class="col-product">
                <ElOption
                  v-for="p in productOptions"
                  :key="p.value"
                  :label="p.label"
                  :value="p.value"
                />
              </ElSelect>
              <ElInputNumber
                v-model="item.quantity"
                class="col-qty"
                @change="calculateAmount(item)"
              />
              <ElInputNumber
                v-model="item.cost_price"
                :precision="2"
                class="col-price"
                @change="calculateAmount(item)"
              />
              <ElInputNumber v-model="item.amount" :precision="2" class="col-amount" readonly />
              <ElButton
                v-if="(form.items || []).length > 1"
                size="small"
                type="danger"
                @click="removeItem(index)"
                >删除</ElButton
              >
            </div>
            <ElButton type="text" @click="addItem">+ 添加明细</ElButton>
          </div>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="dialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="handleSubmit">确定</ElButton>
      </template>
    </ElDialog>

    <ElDialog v-model="viewDialogVisible" title="调整单详情" width="800px">
      <div v-if="viewData">
        <ElDescriptions :column="4" border>
          <ElDescriptionsItem label="调整单号">{{ viewData.adjust_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="调整日期">{{ viewData.adjust_date }}</ElDescriptionsItem>
          <ElDescriptionsItem label="仓库">{{ viewData.warehouse_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="调整原因">{{ viewData.reason }}</ElDescriptionsItem>
          <ElDescriptionsItem label="调整金额">{{
            viewData.total_amount.toFixed(2)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="状态">{{
            getStatusLabel(viewData.status)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建人">{{ viewData.created_by_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建时间">{{ viewData.created_at }}</ElDescriptionsItem>
        </ElDescriptions>
        <div style="margin-top: 20px">
          <h4>调整明细</h4>
          <ElTable :data="detailData" border style="width: 100%">
            <ElTableColumn prop="product_code" label="产品编码" width="120" />
            <ElTableColumn prop="product_name" label="产品名称" width="150" />
            <ElTableColumn prop="color_no" label="色号" width="100" />
            <ElTableColumn prop="grade" label="等级" width="80" />
            <ElTableColumn prop="quantity" label="数量" width="100" align="right" />
            <ElTableColumn prop="cost_price" label="单价" width="100" align="right">
              <template #default="scope">{{ scope.row.cost_price.toFixed(2) }}</template>
            </ElTableColumn>
            <ElTableColumn prop="amount" label="金额" width="120" align="right">
              <template #default="scope">{{ scope.row.amount.toFixed(2) }}</template>
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

.status-approved {
  background: #f0f9eb;
  color: #67c23a;
}

.items-table {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}

.items-header {
  display: flex;
  background: #f5f7fa;
  padding: 10px;
  font-weight: bold;
}

.items-row {
  display: flex;
  padding: 10px;
  border-top: 1px solid #ebeef5;
}

.col-product {
  flex: 2;
  margin-right: 10px;
}

.col-qty,
.col-price,
.col-amount {
  width: 100px;
  margin-right: 10px;
}

.col-action {
  width: 60px;
}

.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
