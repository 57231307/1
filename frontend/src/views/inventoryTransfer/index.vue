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
} from 'element-plus'
import { Plus, Edit, Delete, View, Check, ArrowRight } from '@element-plus/icons-vue'
import {
  listInventoryTransfers,
  getInventoryTransfer,
  createInventoryTransfer,
  updateInventoryTransfer,
  deleteInventoryTransfer,
  approveInventoryTransfer,
  getTransferItems,
  type InventoryTransferEntity,
  type TransferItem,
} from '@/api/inventoryTransfer'
import { request } from '@/api/request'

const tableData = ref<InventoryTransferEntity[]>([])
const total = ref(0)
const loading = ref(false)
const searchForm = ref({
  transfer_no: '',
  from_warehouse_id: '',
  to_warehouse_id: '',
  status: '',
})
const pagination = ref({
  page: 1,
  pageSize: 20,
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增调拨')
const form = ref<Partial<InventoryTransferEntity>>({
  transfer_no: '',
  transfer_date: new Date().toISOString().split('T')[0],
  from_warehouse_id: 0,
  to_warehouse_id: 0,
  status: 'draft',
  items: [],
})

const viewDialogVisible = ref(false)
const viewData = ref<InventoryTransferEntity | null>(null)
const detailData = ref<TransferItem[]>([])

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
    const res: any = await listInventoryTransfers({
      page: pagination.value.page,
      pageSize: pagination.value.pageSize,
      transfer_no: searchForm.value.transfer_no,
      from_warehouse_id: searchForm.value.from_warehouse_id
        ? Number(searchForm.value.from_warehouse_id)
        : undefined,
      to_warehouse_id: searchForm.value.to_warehouse_id
        ? Number(searchForm.value.to_warehouse_id)
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
    transfer_no: '',
    from_warehouse_id: '',
    to_warehouse_id: '',
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
  dialogTitle.value = '新增调拨'
  form.value = {
    transfer_no: '',
    transfer_date: new Date().toISOString().split('T')[0],
    from_warehouse_id: 0,
    to_warehouse_id: 0,
    status: 'draft',
    items: [{ product_id: 0, quantity: 0, cost_price: 0, amount: 0 }],
  }
  dialogVisible.value = true
}

const openEditDialog = async (row: InventoryTransferEntity) => {
  dialogTitle.value = '编辑调拨'
  const res: any = await getInventoryTransfer(row.id!)
  const itemsRes: any = await getTransferItems(row.id!)
  form.value = { ...res.data, items: itemsRes.data }
  dialogVisible.value = true
}

const openViewDialog = async (row: InventoryTransferEntity) => {
  try {
    const res: any = await getInventoryTransfer(row.id!)
    viewData.value = res.data!
    const itemsRes: any = await getTransferItems(row.id!)
    detailData.value = itemsRes.data
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error('获取详情失败')
  }
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
  if (!form.value.from_warehouse_id || !form.value.to_warehouse_id) {
    ElMessage.warning('请选择调出和调入仓库')
    return
  }
  if (form.value.from_warehouse_id === form.value.to_warehouse_id) {
    ElMessage.warning('调出和调入仓库不能相同')
    return
  }
  const validItems = (form.value.items || []).filter((e) => e.product_id > 0 && e.quantity !== 0)
  if (validItems.length === 0) {
    ElMessage.warning('请至少添加一条有效的调拨明细')
    return
  }
  try {
    const data = { ...form.value, items: validItems }
    if (form.value.id) {
      await updateInventoryTransfer(form.value.id, data)
      ElMessage.success('更新成功')
    } else {
      await createInventoryTransfer(data)
      ElMessage.success('新增成功')
    }
    dialogVisible.value = false
    loadData()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const handleDelete = async (row: InventoryTransferEntity) => {
  if (row.status === 'approved') {
    ElMessage.warning('已审核的调拨单不能删除')
    return
  }
  try {
    await ElMessageBox.confirm('确定要删除这个调拨单吗？', '提示', {
      type: 'warning',
    })
    await deleteInventoryTransfer(row.id!)
    ElMessage.success('删除成功')
    loadData()
  } catch (error) {
    ElMessage.info('取消删除')
  }
}

const handleApprove = async (row: InventoryTransferEntity) => {
  try {
    await ElMessageBox.confirm('确定要审核这个调拨单吗？', '提示', {
      type: 'warning',
    })
    await approveInventoryTransfer(row.id!)
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
            v-model="searchForm.transfer_no"
            placeholder="调拨单号"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.from_warehouse_id"
            placeholder="调出仓库"
            class="filter-item"
          >
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
          <ElSelect v-model="searchForm.to_warehouse_id" placeholder="调入仓库" class="filter-item">
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
        <ElButton type="success" @click="openAddDialog"> <Plus /> 新增调拨单 </ElButton>
      </div>
    </div>

    <ElTable
      :data="tableData"
      :total="total"
      :loading="loading"
      :page-size="pagination.pageSize"
      :current-page="pagination.page"
      border
      fit
      highlight-current-row
      style="width: 100%"
      @current-change="handlePageChange"
      @size-change="handlePageSizeChange"
    >
      <ElTableColumn prop="transfer_no" label="调拨单号" width="150" />
      <ElTableColumn prop="transfer_date" label="调拨日期" width="120" />
      <ElTableColumn prop="from_warehouse_name" label="调出仓库" width="150" />
      <ElTableColumn prop="to_warehouse_name" label="调入仓库" width="150" />
      <ElTableColumn prop="total_amount" label="调拨金额" width="120" align="right">
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
          <ElButton size="small" @click="openViewDialog(scope.row)">
            <View />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="primary"
            @click="openEditDialog(scope.row)"
          >
            <Edit />
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="warning"
            @click="handleApprove(scope.row)"
          >
            <Check /> 审核
          </ElButton>
          <ElButton
            v-if="scope.row.status === 'draft'"
            size="small"
            type="danger"
            @click="handleDelete(scope.row)"
          >
            <Delete />
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <ElDialog
      :title="dialogTitle"
      :visible="dialogVisible"
      width="800px"
      @close="dialogVisible = false"
    >
      <ElForm :model="form" label-width="100px">
        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElFormItem label="调拨单号" prop="transfer_no">
              <ElInput v-model="form.transfer_no" readonly />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="调拨日期" prop="transfer_date">
              <ElDatePicker v-model="form.transfer_date" type="date" />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow :gutter="20" class="warehouse-row">
          <ElCol :span="10">
            <ElFormItem label="调出仓库" prop="from_warehouse_id">
              <ElSelect v-model="form.from_warehouse_id" placeholder="请选择调出仓库">
                <ElOption
                  v-for="w in warehouseOptions"
                  :key="w.value"
                  :label="w.label"
                  :value="w.value"
                />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="4" class="arrow-col">
            <ArrowRight />
          </ElCol>
          <ElCol :span="10">
            <ElFormItem label="调入仓库" prop="to_warehouse_id">
              <ElSelect v-model="form.to_warehouse_id" placeholder="请选择调入仓库">
                <ElOption
                  v-for="w in warehouseOptions"
                  :key="w.value"
                  :label="w.label"
                  :value="w.value"
                />
              </ElSelect>
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElFormItem label="调拨明细">
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
              <ElInputNumber v-model="item.quantity" class="col-qty" />
              <ElInputNumber v-model="item.cost_price" :precision="2" class="col-price" />
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

    <ElDialog
      title="调拨单详情"
      :visible="viewDialogVisible"
      width="800px"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <ElDescriptions :column="4" border>
          <ElDescriptionsItem label="调拨单号">{{ viewData.transfer_no }}</ElDescriptionsItem>
          <ElDescriptionsItem label="调拨日期">{{ viewData.transfer_date }}</ElDescriptionsItem>
          <ElDescriptionsItem label="调出仓库">{{
            viewData.from_warehouse_name
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="调入仓库">{{ viewData.to_warehouse_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="调拨金额">{{
            viewData.total_amount.toFixed(2)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="状态">{{
            getStatusLabel(viewData.status)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建人">{{ viewData.created_by_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建时间">{{ viewData.created_at }}</ElDescriptionsItem>
        </ElDescriptions>
        <div style="margin-top: 20px">
          <h4>调拨明细</h4>
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

.warehouse-row {
  align-items: center;
}

.arrow-col {
  display: flex;
  align-items: center;
  justify-content: center;
  color: #909399;
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
</style>
