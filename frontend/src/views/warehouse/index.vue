<template>
  <div class="warehouse-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">仓库管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>仓储管理</el-breadcrumb-item>
          <el-breadcrumb-item>仓库管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建仓库
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><OfficeBuilding /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">仓库总数</div>
              <div class="stat-value">{{ stats.totalWarehouses }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon active-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">启用仓库</div>
              <div class="stat-value">{{ stats.activeWarehouses }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon location-icon">
              <el-icon><Location /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">库位总数</div>
              <div class="stat-value">{{ stats.totalLocations }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon capacity-icon">
              <el-icon><Box /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">总容量</div>
              <div class="stat-value">{{ formatNumber(stats.totalCapacity) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="仓库编码/名称" clearable />
        </el-form-item>
        <el-form-item label="仓库类型">
          <el-select v-model="queryParams.warehouse_type" placeholder="选择类型" clearable>
            <el-option label="原材料仓库" value="raw_material" />
            <el-option label="成品仓库" value="finished_goods" />
            <el-option label="在制品仓库" value="wip" />
            <el-option label="退货仓库" value="return" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="warehouses" stripe>
        <el-table-column prop="warehouse_code" label="仓库编码" width="140" fixed />
        <el-table-column prop="warehouse_name" label="仓库名称" min-width="180" fixed />
        <el-table-column prop="warehouse_type" label="仓库类型" width="120">
          <template #default="{ row }">
            <el-tag size="small">{{ getWarehouseTypeText(row.warehouse_type) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="address" label="地址" min-width="200" show-overflow-tooltip />
        <el-table-column prop="contact_person" label="联系人" width="100" />
        <el-table-column prop="phone" label="联系电话" width="130" />
        <el-table-column prop="capacity" label="容量" width="100" align="right">
          <template #default="{ row }">
            <span>{{ formatNumber(row.capacity || 0) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="is_default" label="默认" width="80">
          <template #default="{ row }">
            <el-tag v-if="row.is_default" type="success" size="small">是</el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">详情</el-button>
            <el-button type="primary" link size="small" @click="handleEdit(row)">编辑</el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleQuery"
          @current-change="handleQuery"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, OfficeBuilding, CircleCheck, Location, Box, Printer, Download } from '@element-plus/icons-vue'
import printJS from 'print-js'

const loading = ref(false)
const warehouses = ref<any[]>([])
const total = ref(0)

const stats = ref({
  totalWarehouses: 5,
  activeWarehouses: 4,
  totalLocations: 128,
  totalCapacity: 50000
})

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  warehouse_type: '',
  status: ''
})

const formatNumber = (num: number) => num.toLocaleString()

const getWarehouseTypeText = (type: string) => {
  const map: Record<string, string> = {
    raw_material: '原材料',
    finished_goods: '成品',
    wip: '在制品',
    return: '退货'
  }
  return map[type] || type
}

const fetchData = async () => {
  loading.value = true
  try {
    warehouses.value = [
      { id: 1, warehouse_code: 'WH001', warehouse_name: 'A仓库-原材料库', warehouse_type: 'raw_material', address: '上海市浦东新区XX路1号', contact_person: '张仓管', phone: '13800001001', capacity: 15000, is_default: true, status: 'active' },
      { id: 2, warehouse_code: 'WH002', warehouse_name: 'B仓库-成品库', warehouse_type: 'finished_goods', address: '上海市浦东新区XX路2号', contact_person: '李仓管', phone: '13800001002', capacity: 20000, is_default: false, status: 'active' },
      { id: 3, warehouse_code: 'WH003', warehouse_name: 'C仓库-在制品库', warehouse_type: 'wip', address: '上海市浦东新区XX路3号', contact_person: '王仓管', phone: '13800001003', capacity: 10000, is_default: false, status: 'active' },
      { id: 4, warehouse_code: 'WH004', warehouse_name: 'D仓库-退货库', warehouse_type: 'return', address: '上海市浦东新区XX路4号', contact_person: '赵仓管', phone: '13800001004', capacity: 5000, is_default: false, status: 'inactive' }
    ]
    total.value = 4
    ElMessage.info('使用演示数据')
  } finally {
    loading.value = false
  }
}

const handlePrint = () => {
  const printData = warehouses.value.map((item: any, index: number) => ({
    '序号': index + 1,
    '仓库编码': item.warehouse_code,
    '仓库名称': item.warehouse_name,
    '地址': item.address,
    '负责人': item.manager,
    '容量': item.capacity,
    '状态': item.status === 1 ? '启用' : '停用'
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}),
    type: 'table' as any,
    header: '仓库列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;'
  })
}

const handleExport = () => {
  const csvContent = [
    ['仓库编码', '仓库名称', '地址', '负责人', '容量', '状态'],
    ...warehouses.value.map((item: any) => [item.warehouse_code, item.warehouse_name, item.address, item.manager, item.capacity, item.status === 1 ? '启用' : '停用'])
  ].map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(',')).join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `仓库列表_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

const handleQuery = () => { fetchData() }
const handleReset = () => { queryParams.keyword = ''; queryParams.warehouse_type = ''; queryParams.status = ''; handleQuery() }
const handleView = (row: any) => { ElMessage.info(`查看仓库 ${row.warehouse_name}`) }

onMounted(() => { fetchData() })
</script>

<style scoped>
.warehouse-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
.header-left .page-title { font-size: 28px; font-weight: 600; color: #303133; margin: 0 0 12px 0; }
.header-actions { display: flex; gap: 12px; }
.stats-row { margin-bottom: 20px; }
.stat-card { border-radius: 12px; transition: all 0.3s ease; }
.stat-card:hover { transform: translateY(-4px); box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12); }
.stat-card.highlight { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); }
.stat-card.highlight .stat-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-card.highlight .stat-label, .stat-card.highlight .stat-value { color: white; }
.stat-card.warning { background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%); }
.stat-card.warning .stat-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-card.warning .stat-label, .stat-card.warning .stat-value { color: white; }
.stat-content { display: flex; align-items: center; gap: 16px; }
.stat-icon { width: 56px; height: 56px; border-radius: 12px; display: flex; align-items: center; justify-content: center; font-size: 28px; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
.stat-icon.total-icon { background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%); }
.stat-icon.active-icon { background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%); }
.stat-icon.location-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-icon.capacity-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-info { flex: 1; }
.stat-label { font-size: 14px; color: #909399; margin-bottom: 4px; }
.stat-value { font-size: 28px; font-weight: 700; color: #303133; line-height: 1.2; }
.filter-card { margin-bottom: 20px; }
.table-card { margin-bottom: 20px; }
.pagination-wrapper { margin-top: 20px; display: flex; justify-content: flex-end; }
:deep(.el-card__header) { padding: 16px 20px; border-bottom: 1px solid #ebeef5; }
:deep(.el-card__body) { padding: 20px; }
</style>
