<template>
  <div class="trading-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="采购合同" name="purchase-contract">
        <div class="page-header">
          <h2 class="page-title">采购合同管理</h2>
          <el-button type="primary" @click="openPurchaseContractDialog()">
            <el-icon><Plus /></el-icon>
            新建合同
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="purchaseContractLoading" :data="purchaseContracts" stripe>
            <el-table-column prop="contract_no" label="合同编号" width="140" />
            <el-table-column prop="supplier_name" label="供应商" width="150" />
            <el-table-column prop="contract_date" label="合同日期" width="120" />
            <el-table-column prop="total_amount" label="总金额" width="120" align="right">
              <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">{{
                  getContractStatusLabel(row.status)
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button
                  type="primary"
                  link
                  size="small"
                  @click="viewPurchaseContract(row as any)"
                  >查看</el-button
                >
                <el-button
                  v-if="row.status === 'draft'"
                  type="success"
                  link
                  size="small"
                  @click="approvePurchaseContract(row as any)"
                  >审批</el-button
                >
                <el-button
                  v-if="row.status === 'approved'"
                  type="warning"
                  link
                  size="small"
                  @click="executePurchaseContract(row as any)"
                  >执行</el-button
                >
                <el-button
                  type="danger"
                  link
                  size="small"
                  @click="deletePurchaseContract(row as any)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="采购价格" name="purchase-price">
        <div class="page-header">
          <h2 class="page-title">采购价格管理</h2>
          <el-button type="primary" @click="openPurchasePriceDialog()">
            <el-icon><Plus /></el-icon>
            新建价格
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="purchasePriceLoading" :data="purchasePrices" stripe>
            <el-table-column prop="product_name" label="产品" width="150" />
            <el-table-column prop="supplier_name" label="供应商" width="150" />
            <el-table-column prop="price" label="价格" width="100" align="right">
              <template #default="{ row }">{{ formatMoney(row.price) }}</template>
            </el-table-column>
            <el-table-column prop="currency" label="币种" width="80" />
            <el-table-column prop="unit" label="单位" width="80" />
            <el-table-column prop="effective_date" label="生效日期" width="120" />
            <el-table-column prop="expiry_date" label="失效日期" width="120" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">{{
                  row.status === 'active' ? '有效' : '无效'
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120">
              <template #default="{ row }">
                <el-button
                  type="primary"
                  link
                  size="small"
                  @click="openPurchasePriceDialog(row as any)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="deletePurchasePrice(row as any)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="销售合同" name="sales-contract">
        <div class="page-header">
          <h2 class="page-title">销售合同管理</h2>
          <el-button type="primary" @click="openSalesContractDialog()">
            <el-icon><Plus /></el-icon>
            新建合同
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="salesContractLoading" :data="salesContracts" stripe>
            <el-table-column prop="contract_no" label="合同编号" width="140" />
            <el-table-column prop="customer_name" label="客户" width="150" />
            <el-table-column prop="contract_date" label="合同日期" width="120" />
            <el-table-column prop="total_amount" label="总金额" width="120" align="right">
              <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">{{
                  getContractStatusLabel(row.status)
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewSalesContract(row as any)"
                  >查看</el-button
                >
                <el-button
                  v-if="row.status === 'draft'"
                  type="success"
                  link
                  size="small"
                  @click="approveSalesContract(row as any)"
                  >审批</el-button
                >
                <el-button type="danger" link size="small" @click="deleteSalesContract(row as any)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="销售价格" name="sales-price">
        <div class="page-header">
          <h2 class="page-title">销售价格管理</h2>
          <el-button type="primary" @click="openSalesPriceDialog()">
            <el-icon><Plus /></el-icon>
            新建价格
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="salesPriceLoading" :data="salesPrices" stripe>
            <el-table-column prop="product_name" label="产品" width="150" />
            <el-table-column prop="customer_name" label="客户" width="150" />
            <el-table-column prop="price" label="价格" width="100" align="right">
              <template #default="{ row }">{{ formatMoney(row.price) }}</template>
            </el-table-column>
            <el-table-column prop="currency" label="币种" width="80" />
            <el-table-column prop="unit" label="单位" width="80" />
            <el-table-column prop="effective_date" label="生效日期" width="120" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">{{
                  row.status === 'active' ? '有效' : '无效'
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120">
              <template #default="{ row }">
                <el-button
                  type="primary"
                  link
                  size="small"
                  @click="openSalesPriceDialog(row as any)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="deleteSalesPrice(row as any)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="销售退货" name="sales-return">
        <div class="page-header">
          <h2 class="page-title">销售退货管理</h2>
          <el-button type="primary" @click="openReturnDialog()">
            <el-icon><Plus /></el-icon>
            新建退货
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="returnLoading" :data="salesReturns" stripe>
            <el-table-column prop="return_no" label="退货单号" width="140" />
            <el-table-column prop="customer_name" label="客户" width="150" />
            <el-table-column prop="order_no" label="销售单号" width="140" />
            <el-table-column prop="return_date" label="退货日期" width="120" />
            <el-table-column prop="total_amount" label="退货金额" width="120" align="right">
              <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
            </el-table-column>
            <el-table-column prop="reason" label="退货原因" min-width="150" />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">{{
                  getReturnStatusLabel(row.status)
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openReturnDialog(row as any)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="deleteSalesReturn(row as any)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <!-- 采购合同对话框 -->
    <el-dialog
      v-model="purchaseContractDialogVisible"
      :title="purchaseContractDialogTitle"
      width="600px"
    >
      <el-form :model="purchaseContractForm" label-width="100px">
        <el-form-item label="合同编号">
          <el-input v-model="purchaseContractForm.contract_no" placeholder="请输入合同编号" />
        </el-form-item>
        <el-form-item label="供应商">
          <el-input v-model="purchaseContractForm.supplier_name" placeholder="请输入供应商名称" />
        </el-form-item>
        <el-form-item label="合同日期">
          <el-date-picker
            v-model="purchaseContractForm.contract_date"
            type="date"
            placeholder="选择日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="总金额">
          <el-input-number
            v-model="purchaseContractForm.total_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select
            v-model="purchaseContractForm.status"
            placeholder="请选择状态"
            style="width: 100%"
          >
            <el-option label="草稿" value="draft" />
            <el-option label="待审批" value="pending" />
            <el-option label="已审批" value="approved" />
            <el-option label="执行中" value="active" />
            <el-option label="已完成" value="completed" />
            <el-option label="已取消" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item label="备注">
          <el-input
            v-model="purchaseContractForm.remarks"
            type="textarea"
            placeholder="请输入备注"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="purchaseContractDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitPurchaseContract">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 采购价格对话框 -->
    <el-dialog v-model="purchasePriceDialogVisible" :title="purchasePriceDialogTitle" width="600px">
      <el-form :model="purchasePriceForm" label-width="100px">
        <el-form-item label="产品">
          <el-input v-model="purchasePriceForm.product_name" placeholder="请输入产品名称" />
        </el-form-item>
        <el-form-item label="供应商">
          <el-input v-model="purchasePriceForm.supplier_name" placeholder="请输入供应商名称" />
        </el-form-item>
        <el-form-item label="价格">
          <el-input-number
            v-model="purchasePriceForm.price"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="币种">
          <el-select
            v-model="purchasePriceForm.currency"
            placeholder="请选择币种"
            style="width: 100%"
          >
            <el-option label="人民币" value="CNY" />
            <el-option label="美元" value="USD" />
            <el-option label="欧元" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item label="单位">
          <el-input v-model="purchasePriceForm.unit" placeholder="请输入单位" />
        </el-form-item>
        <el-form-item label="生效日期">
          <el-date-picker
            v-model="purchasePriceForm.effective_date"
            type="date"
            placeholder="选择日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="失效日期">
          <el-date-picker
            v-model="purchasePriceForm.expiry_date"
            type="date"
            placeholder="选择日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select
            v-model="purchasePriceForm.status"
            placeholder="请选择状态"
            style="width: 100%"
          >
            <el-option label="有效" value="active" />
            <el-option label="无效" value="inactive" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="purchasePriceDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitPurchasePrice">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 销售合同对话框 -->
    <el-dialog v-model="salesContractDialogVisible" :title="salesContractDialogTitle" width="600px">
      <el-form :model="salesContractForm" label-width="100px">
        <el-form-item label="合同编号">
          <el-input v-model="salesContractForm.contract_no" placeholder="请输入合同编号" />
        </el-form-item>
        <el-form-item label="客户">
          <el-input v-model="salesContractForm.customer_name" placeholder="请输入客户名称" />
        </el-form-item>
        <el-form-item label="合同日期">
          <el-date-picker
            v-model="salesContractForm.contract_date"
            type="date"
            placeholder="选择日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="总金额">
          <el-input-number
            v-model="salesContractForm.total_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select
            v-model="salesContractForm.status"
            placeholder="请选择状态"
            style="width: 100%"
          >
            <el-option label="草稿" value="draft" />
            <el-option label="待审批" value="pending" />
            <el-option label="已审批" value="approved" />
            <el-option label="执行中" value="active" />
            <el-option label="已完成" value="completed" />
            <el-option label="已取消" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="salesContractForm.remarks" type="textarea" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="salesContractDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitSalesContract">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 销售价格对话框 -->
    <el-dialog v-model="salesPriceDialogVisible" :title="salesPriceDialogTitle" width="600px">
      <el-form :model="salesPriceForm" label-width="100px">
        <el-form-item label="产品">
          <el-input v-model="salesPriceForm.product_name" placeholder="请输入产品名称" />
        </el-form-item>
        <el-form-item label="客户">
          <el-input v-model="salesPriceForm.customer_name" placeholder="请输入客户名称" />
        </el-form-item>
        <el-form-item label="价格">
          <el-input-number
            v-model="salesPriceForm.price"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="币种">
          <el-select v-model="salesPriceForm.currency" placeholder="请选择币种" style="width: 100%">
            <el-option label="人民币" value="CNY" />
            <el-option label="美元" value="USD" />
            <el-option label="欧元" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item label="单位">
          <el-input v-model="salesPriceForm.unit" placeholder="请输入单位" />
        </el-form-item>
        <el-form-item label="生效日期">
          <el-date-picker
            v-model="salesPriceForm.effective_date"
            type="date"
            placeholder="选择日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="salesPriceForm.status" placeholder="请选择状态" style="width: 100%">
            <el-option label="有效" value="active" />
            <el-option label="无效" value="inactive" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="salesPriceDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitSalesPrice">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 销售退货对话框 -->
    <el-dialog v-model="returnDialogVisible" :title="returnDialogTitle" width="600px">
      <el-form :model="returnForm" label-width="100px">
        <el-form-item label="退货单号">
          <el-input v-model="returnForm.return_no" placeholder="请输入退货单号" />
        </el-form-item>
        <el-form-item label="客户">
          <el-input v-model="returnForm.customer_name" placeholder="请输入客户名称" />
        </el-form-item>
        <el-form-item label="销售单号">
          <el-input v-model="returnForm.order_no" placeholder="请输入销售单号" />
        </el-form-item>
        <el-form-item label="退货日期">
          <el-date-picker
            v-model="returnForm.return_date"
            type="date"
            placeholder="选择日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="退货金额">
          <el-input-number
            v-model="returnForm.total_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="退货原因">
          <el-input v-model="returnForm.reason" type="textarea" placeholder="请输入退货原因" />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="returnForm.status" placeholder="请选择状态" style="width: 100%">
            <el-option label="草稿" value="draft" />
            <el-option label="待审批" value="pending" />
            <el-option label="已审批" value="approved" />
            <el-option label="已拒绝" value="rejected" />
            <el-option label="已完成" value="completed" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="returnDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitReturn">确定</el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import {
  listTradingPurchaseContracts,
  createTradingPurchaseContract,
  updateTradingPurchaseContract,
  deleteTradingPurchaseContract as apiDeletePurchaseContract,
  approveTradingPurchaseContract as apiApprovePurchaseContract,
  executeTradingPurchaseContract as apiExecutePurchaseContract,
  type TradingPurchaseContract,
  listTradingPurchasePrices,
  createTradingPurchasePrice,
  updateTradingPurchasePrice,
  deleteTradingPurchasePrice as apiDeletePurchasePrice,
  type TradingPurchasePrice,
  listTradingSalesContracts,
  createTradingSalesContract,
  updateTradingSalesContract,
  deleteTradingSalesContract as apiDeleteSalesContract,
  approveTradingSalesContract as apiApproveSalesContract,
  type TradingSalesContract,
  listTradingSalesPrices,
  createTradingSalesPrice,
  updateTradingSalesPrice,
  deleteTradingSalesPrice as apiDeleteSalesPrice,
  type TradingSalesPrice,
  listTradingSalesReturns,
  createTradingSalesReturn,
  updateTradingSalesReturn,
  deleteTradingSalesReturn as apiDeleteSalesReturn,
  type TradingSalesReturn,
} from '@/api/trading'

const activeTab = ref('purchase-contract')
const purchaseContracts = ref<TradingPurchaseContract[]>([])
const purchasePrices = ref<TradingPurchasePrice[]>([])
const salesContracts = ref<TradingSalesContract[]>([])
const salesPrices = ref<TradingSalesPrice[]>([])
const salesReturns = ref<TradingSalesReturn[]>([])
const purchaseContractLoading = ref(false)
const purchasePriceLoading = ref(false)
const salesContractLoading = ref(false)
const salesPriceLoading = ref(false)
const returnLoading = ref(false)

// 对话框状态
const purchaseContractDialogVisible = ref(false)
const purchasePriceDialogVisible = ref(false)
const salesContractDialogVisible = ref(false)
const salesPriceDialogVisible = ref(false)
const returnDialogVisible = ref(false)

// 对话框标题
const purchaseContractDialogTitle = ref('新建采购合同')
const purchasePriceDialogTitle = ref('新建采购价格')
const salesContractDialogTitle = ref('新建销售合同')
const salesPriceDialogTitle = ref('新建销售价格')
const returnDialogTitle = ref('新建销售退货')

// 表单数据
const purchaseContractForm = ref({
  id: null as number | null,
  contract_no: '',
  supplier_name: '',
  contract_date: '',
  total_amount: 0,
  status: 'draft',
  remarks: '',
})

const purchasePriceForm = ref({
  id: null as number | null,
  product_name: '',
  supplier_name: '',
  price: 0,
  currency: 'CNY',
  unit: '',
  effective_date: '',
  expiry_date: '',
  status: 'active',
})

const salesContractForm = ref({
  id: null as number | null,
  contract_no: '',
  customer_name: '',
  contract_date: '',
  total_amount: 0,
  status: 'draft',
  remarks: '',
})

const salesPriceForm = ref({
  id: null as number | null,
  product_name: '',
  customer_name: '',
  price: 0,
  currency: 'CNY',
  unit: '',
  effective_date: '',
  status: 'active',
})

const returnForm = ref({
  id: null as number | null,
  return_no: '',
  customer_name: '',
  order_no: '',
  return_date: '',
  total_amount: 0,
  reason: '',
  status: 'draft',
})

const fetchPurchaseContracts = async () => {
  purchaseContractLoading.value = true
  try {
    const res: any = await listTradingPurchaseContracts()
    purchaseContracts.value = res.data! || []
  } finally {
    purchaseContractLoading.value = false
  }
}

const fetchPurchasePrices = async () => {
  purchasePriceLoading.value = true
  try {
    const res: any = await listTradingPurchasePrices()
    purchasePrices.value = res.data! || []
  } finally {
    purchasePriceLoading.value = false
  }
}

const fetchSalesContracts = async () => {
  salesContractLoading.value = true
  try {
    const res: any = await listTradingSalesContracts()
    salesContracts.value = res.data! || []
  } finally {
    salesContractLoading.value = false
  }
}

const fetchSalesPrices = async () => {
  salesPriceLoading.value = true
  try {
    const res: any = await listTradingSalesPrices()
    salesPrices.value = res.data! || []
  } finally {
    salesPriceLoading.value = false
  }
}

const fetchSalesReturns = async () => {
  returnLoading.value = true
  try {
    const res: any = await listTradingSalesReturns()
    salesReturns.value = res.data! || []
  } finally {
    returnLoading.value = false
  }
}

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getStatusType = (status: string) => {
  const map: Record<string, any> = {
    draft: 'info',
    pending: 'warning',
    approved: 'success',
    active: 'primary',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

const getContractStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审批',
    approved: '已审批',
    active: '执行中',
    completed: '已完成',
    cancelled: '已取消',
  }
  return map[status] || status
}

const getReturnStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审批',
    approved: '已审批',
    rejected: '已拒绝',
    completed: '已完成',
  }
  return map[status] || status
}

// 采购合同操作
const openPurchaseContractDialog = (row?: TradingPurchaseContract) => {
  if (row) {
    purchaseContractDialogTitle.value = '编辑采购合同'
    purchaseContractForm.value = { ...row, remarks: '' }
  } else {
    purchaseContractDialogTitle.value = '新建采购合同'
    purchaseContractForm.value = {
      id: null,
      contract_no: '',
      supplier_name: '',
      contract_date: '',
      total_amount: 0,
      status: 'draft',
      remarks: '',
    }
  }
  purchaseContractDialogVisible.value = true
}

const viewPurchaseContract = (row: TradingPurchaseContract) => {
  openPurchaseContractDialog(row)
}

const submitPurchaseContract = async () => {
  try {
    if (purchaseContractForm.value.id) {
      await updateTradingPurchaseContract(purchaseContractForm.value.id, purchaseContractForm.value)
      ElMessage.success('更新成功')
    } else {
      await createTradingPurchaseContract(purchaseContractForm.value)
      ElMessage.success('创建成功')
    }
    purchaseContractDialogVisible.value = false
    fetchPurchaseContracts()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const approvePurchaseContract = async (row: TradingPurchaseContract) => {
  try {
    await ElMessageBox.confirm('确定审批此采购合同吗？', '确认', { type: 'info' })
    await apiApprovePurchaseContract(row.id)
    ElMessage.success('审批成功')
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

const executePurchaseContract = async (row: TradingPurchaseContract) => {
  try {
    await ElMessageBox.confirm('确定执行此采购合同吗？', '确认', { type: 'info' })
    await apiExecutePurchaseContract(row.id)
    ElMessage.success('执行成功')
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

const deletePurchaseContract = async (row: TradingPurchaseContract) => {
  try {
    await ElMessageBox.confirm('确定删除此采购合同吗？', '确认', { type: 'warning' })
    await apiDeletePurchaseContract(row.id)
    ElMessage.success('删除成功')
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

// 采购价格操作
const openPurchasePriceDialog = (row?: TradingPurchasePrice) => {
  if (row) {
    purchasePriceDialogTitle.value = '编辑采购价格'
    purchasePriceForm.value = { ...row }
  } else {
    purchasePriceDialogTitle.value = '新建采购价格'
    purchasePriceForm.value = {
      id: null,
      product_name: '',
      supplier_name: '',
      price: 0,
      currency: 'CNY',
      unit: '',
      effective_date: '',
      expiry_date: '',
      status: 'active',
    }
  }
  purchasePriceDialogVisible.value = true
}

const submitPurchasePrice = async () => {
  try {
    if (purchasePriceForm.value.id) {
      await updateTradingPurchasePrice(purchasePriceForm.value.id, purchasePriceForm.value)
      ElMessage.success('更新成功')
    } else {
      await createTradingPurchasePrice(purchasePriceForm.value)
      ElMessage.success('创建成功')
    }
    purchasePriceDialogVisible.value = false
    fetchPurchasePrices()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const deletePurchasePrice = async (row: TradingPurchasePrice) => {
  try {
    await ElMessageBox.confirm('确定删除此采购价格吗？', '确认', { type: 'warning' })
    await apiDeletePurchasePrice(row.id)
    ElMessage.success('删除成功')
    fetchPurchasePrices()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

// 销售合同操作
const openSalesContractDialog = (row?: TradingSalesContract) => {
  if (row) {
    salesContractDialogTitle.value = '编辑销售合同'
    salesContractForm.value = { ...row, remarks: '' }
  } else {
    salesContractDialogTitle.value = '新建销售合同'
    salesContractForm.value = {
      id: null,
      contract_no: '',
      customer_name: '',
      contract_date: '',
      total_amount: 0,
      status: 'draft',
      remarks: '',
    }
  }
  salesContractDialogVisible.value = true
}

const viewSalesContract = (row: TradingSalesContract) => {
  openSalesContractDialog(row)
}

const submitSalesContract = async () => {
  try {
    if (salesContractForm.value.id) {
      await updateTradingSalesContract(salesContractForm.value.id, salesContractForm.value)
      ElMessage.success('更新成功')
    } else {
      await createTradingSalesContract(salesContractForm.value)
      ElMessage.success('创建成功')
    }
    salesContractDialogVisible.value = false
    fetchSalesContracts()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const approveSalesContract = async (row: TradingSalesContract) => {
  try {
    await ElMessageBox.confirm('确定审批此销售合同吗？', '确认', { type: 'info' })
    await apiApproveSalesContract(row.id)
    ElMessage.success('审批成功')
    fetchSalesContracts()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

const deleteSalesContract = async (row: TradingSalesContract) => {
  try {
    await ElMessageBox.confirm('确定删除此销售合同吗？', '确认', { type: 'warning' })
    await apiDeleteSalesContract(row.id)
    ElMessage.success('删除成功')
    fetchSalesContracts()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

// 销售价格操作
const openSalesPriceDialog = (row?: TradingSalesPrice) => {
  if (row) {
    salesPriceDialogTitle.value = '编辑销售价格'
    salesPriceForm.value = { ...row }
  } else {
    salesPriceDialogTitle.value = '新建销售价格'
    salesPriceForm.value = {
      id: null,
      product_name: '',
      customer_name: '',
      price: 0,
      currency: 'CNY',
      unit: '',
      effective_date: '',
      status: 'active',
    }
  }
  salesPriceDialogVisible.value = true
}

const submitSalesPrice = async () => {
  try {
    if (salesPriceForm.value.id) {
      await updateTradingSalesPrice(salesPriceForm.value.id, salesPriceForm.value)
      ElMessage.success('更新成功')
    } else {
      await createTradingSalesPrice(salesPriceForm.value)
      ElMessage.success('创建成功')
    }
    salesPriceDialogVisible.value = false
    fetchSalesPrices()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const deleteSalesPrice = async (row: TradingSalesPrice) => {
  try {
    await ElMessageBox.confirm('确定删除此销售价格吗？', '确认', { type: 'warning' })
    await apiDeleteSalesPrice(row.id)
    ElMessage.success('删除成功')
    fetchSalesPrices()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

// 销售退货操作
const openReturnDialog = (row?: TradingSalesReturn) => {
  if (row) {
    returnDialogTitle.value = '编辑销售退货'
    returnForm.value = { ...row }
  } else {
    returnDialogTitle.value = '新建销售退货'
    returnForm.value = {
      id: null,
      return_no: '',
      customer_name: '',
      order_no: '',
      return_date: '',
      total_amount: 0,
      reason: '',
      status: 'draft',
    }
  }
  returnDialogVisible.value = true
}

const submitReturn = async () => {
  try {
    if (returnForm.value.id) {
      await updateTradingSalesReturn(returnForm.value.id, returnForm.value)
      ElMessage.success('更新成功')
    } else {
      await createTradingSalesReturn(returnForm.value)
      ElMessage.success('创建成功')
    }
    returnDialogVisible.value = false
    fetchSalesReturns()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const deleteSalesReturn = async (row: TradingSalesReturn) => {
  try {
    await ElMessageBox.confirm('确定删除此销售退货吗？', '确认', { type: 'warning' })
    await apiDeleteSalesReturn(row.id)
    ElMessage.success('删除成功')
    fetchSalesReturns()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

const hasLoaded = createLazyLoader()

onMounted(() => {
  fetchPurchaseContracts()
  loadIfNot('purchasePrices', fetchPurchasePrices, hasLoaded)
  loadIfNot('salesContracts', fetchSalesContracts, hasLoaded)
  loadIfNot('salesPrices', fetchSalesPrices, hasLoaded)
  loadIfNot('salesReturns', fetchSalesReturns, hasLoaded)
})
</script>

<style scoped>
.trading-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>
