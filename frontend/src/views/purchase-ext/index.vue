<template>
  <div class="purchase-ext-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="采购合同" name="contract">
        <div class="page-header">
          <h2 class="page-title">采购合同管理</h2>
          <el-button type="primary" @click="openContractDialog">
            <el-icon><Plus /></el-icon>
            新建合同
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="purchaseContracts" v-loading="contractLoading" stripe>
            <el-table-column prop="contract_no" label="合同编号" width="140" />
            <el-table-column prop="supplier_name" label="供应商" min-width="150" />
            <el-table-column prop="contract_date" label="合同日期" width="120" />
            <el-table-column prop="start_date" label="开始日期" width="120" />
            <el-table-column prop="end_date" label="结束日期" width="120" />
            <el-table-column prop="total_amount" label="总金额" width="120" align="right">
              <template #default="{ row }">
                {{ formatMoney(row.total_amount) }}
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getContractStatusType(row.status)" size="small">
                  {{ getContractStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_by_name" label="创建人" width="100" />
            <el-table-column label="操作" width="240" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewContract(row)">查看</el-button>
                <el-button v-if="row.status === 'draft'" type="primary" link size="small" @click="openContractDialog(row)">编辑</el-button>
                <el-button v-if="row.status === 'draft'" type="success" link size="small" @click="approveContract(row)">审批</el-button>
                <el-button v-if="row.status === 'pending'" type="warning" link size="small" @click="executeContract(row)">执行</el-button>
                <el-button v-if="['draft', 'pending'].includes(row.status)" type="danger" link size="small" @click="cancelContract(row)">取消</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="采购价格" name="price">
        <div class="page-header">
          <h2 class="page-title">采购价格管理</h2>
          <el-button type="primary" @click="openPriceDialog">
            <el-icon><Plus /></el-icon>
            新建价格
          </el-button>
        </div>

        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="priceQuery">
            <el-form-item label="产品">
              <el-input v-model="priceQuery.product_name" placeholder="产品名称" clearable />
            </el-form-item>
            <el-form-item label="供应商">
              <el-input v-model="priceQuery.supplier_name" placeholder="供应商名称" clearable />
            </el-form-item>
            <el-form-item label="状态">
              <el-select v-model="priceQuery.status" placeholder="选择状态" clearable>
                <el-option label="启用" value="active" />
                <el-option label="禁用" value="inactive" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchPurchasePrices">查询</el-button>
              <el-button @click="resetPriceQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card shadow="hover">
          <el-table :data="purchasePrices" v-loading="priceLoading" stripe>
            <el-table-column prop="product_name" label="产品名称" min-width="150" />
            <el-table-column prop="product_code" label="产品编码" width="120" />
            <el-table-column prop="supplier_name" label="供应商" min-width="150" />
            <el-table-column prop="price" label="价格" width="120" align="right">
              <template #default="{ row }">
                {{ formatMoney(row.price) }}
              </template>
            </el-table-column>
            <el-table-column prop="currency" label="货币" width="80" />
            <el-table-column prop="unit" label="单位" width="80" />
            <el-table-column prop="effective_date" label="生效日期" width="120" />
            <el-table-column prop="expiry_date" label="失效日期" width="120" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
                  {{ row.status === 'active' ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="180" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openPriceDialog(row)">编辑</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="采购退货" name="return">
        <div class="page-header">
          <h2 class="page-title">采购退货管理</h2>
          <el-button type="primary" @click="openReturnDialog">
            <el-icon><Plus /></el-icon>
            新建退货
          </el-button>
        </div>

        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="returnQuery">
            <el-form-item label="退货单号">
              <el-input v-model="returnQuery.returnNo" placeholder="退货单号" clearable />
            </el-form-item>
            <el-form-item label="供应商">
              <el-input v-model="returnQuery.supplierName" placeholder="供应商名称" clearable />
            </el-form-item>
            <el-form-item label="状态">
              <el-select v-model="returnQuery.status" placeholder="选择状态" clearable>
                <el-option label="草稿" value="draft" />
                <el-option label="待审核" value="pending" />
                <el-option label="已批准" value="approved" />
                <el-option label="已拒绝" value="rejected" />
                <el-option label="已完成" value="completed" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchPurchaseReturns">查询</el-button>
              <el-button @click="resetReturnQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card shadow="hover">
          <el-table :data="purchaseReturns" v-loading="returnLoading" stripe>
            <el-table-column prop="returnNo" label="退货单号" width="140" />
            <el-table-column prop="supplierName" label="供应商" min-width="150" />
            <el-table-column prop="purchaseOrderNo" label="订单号" width="140" />
            <el-table-column prop="returnDate" label="退货日期" width="120" />
            <el-table-column prop="totalAmount" label="总金额" width="120" align="right">
              <template #default="{ row }">
                {{ formatMoney(row.totalAmount) }}
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getReturnStatusType(row.status)" size="small">
                  {{ getReturnStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="createdBy" label="创建人" width="100" />
            <el-table-column label="操作" width="180" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewReturn(row)">查看</el-button>
                <el-button v-if="row.status === 'draft'" type="primary" link size="small" @click="openReturnDialog(row)">编辑</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="contractDialogVisible" :title="contractForm.id ? '编辑采购合同' : '新建采购合同'" width="800px">
      <el-form ref="contractFormRef" :model="contractForm" :rules="contractRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="合同编号" prop="contract_no">
              <el-input v-model="contractForm.contract_no" :disabled="!!contractForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="供应商" prop="supplier_name">
              <el-input v-model="contractForm.supplier_name" placeholder="请选择供应商" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="合同日期" prop="contract_date">
              <el-date-picker v-model="contractForm.contract_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="开始日期" prop="start_date">
              <el-date-picker v-model="contractForm.start_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="结束日期" prop="end_date">
              <el-date-picker v-model="contractForm.end_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="货币" prop="currency">
              <el-select v-model="contractForm.currency" placeholder="选择货币" style="width: 100%">
                <el-option label="CNY" value="CNY" />
                <el-option label="USD" value="USD" />
                <el-option label="EUR" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="总金额" prop="total_amount">
              <el-input-number v-model="contractForm.total_amount" :min="0" :precision="2" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider>合同明细</el-divider>
        <el-table :data="contractForm.items" border style="width: 100%">
          <el-table-column prop="product_name" label="产品名称" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.product_name" placeholder="产品名称" />
            </template>
          </el-table-column>
          <el-table-column prop="product_code" label="产品编码" width="120">
            <template #default="{ row }">
              <el-input v-model="row.product_code" placeholder="编码" />
            </template>
          </el-table-column>
          <el-table-column prop="quantity" label="数量" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.quantity" :min="0" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="unit" label="单位" width="80">
            <template #default="{ row }">
              <el-input v-model="row.unit" placeholder="单位" />
            </template>
          </el-table-column>
          <el-table-column prop="price" label="单价" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.price" :min="0" :precision="2" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="amount" label="金额" width="100">
            <template #default="{ row }">
              {{ formatMoney(row.quantity * row.price) }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="{ $index }">
              <el-button type="danger" link size="small" @click="removeContractItem($index)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" link style="margin-top: 8px" @click="addContractItem">添加产品</el-button>
        <el-form-item label="付款条款" prop="payment_terms">
          <el-input v-model="contractForm.payment_terms" type="textarea" />
        </el-form-item>
        <el-form-item label="交货条款" prop="delivery_terms">
          <el-input v-model="contractForm.delivery_terms" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="contractDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="contractSubmitLoading" @click="submitContract">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="contractViewVisible" title="采购合同详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="合同编号">{{ currentContract?.contract_no }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{ currentContract?.supplier_name }}</el-descriptions-item>
        <el-descriptions-item label="合同日期">{{ currentContract?.contract_date }}</el-descriptions-item>
        <el-descriptions-item label="有效日期">{{ currentContract?.start_date }} ~ {{ currentContract?.end_date }}</el-descriptions-item>
        <el-descriptions-item label="货币">{{ currentContract?.currency }}</el-descriptions-item>
        <el-descriptions-item label="总金额">{{ formatMoney(currentContract?.total_amount || 0) }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getContractStatusType(currentContract?.status)">
            {{ getContractStatusLabel(currentContract?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建人">{{ currentContract?.created_by_name }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>合同明细</el-divider>
      <el-table :data="currentContract?.items || []" stripe>
        <el-table-column prop="product_name" label="产品名称" min-width="150" />
        <el-table-column prop="product_code" label="产品编码" width="120" />
        <el-table-column prop="quantity" label="数量" width="100" align="right" />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="price" label="单价" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="amount" label="金额" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="120" />
      </el-table>
      <el-divider>条款</el-divider>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="付款条款">{{ currentContract?.payment_terms }}</el-descriptions-item>
        <el-descriptions-item label="交货条款">{{ currentContract?.delivery_terms }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <el-dialog v-model="priceDialogVisible" :title="priceForm.id ? '编辑采购价格' : '新建采购价格'" width="600px">
      <el-form ref="priceFormRef" :model="priceForm" :rules="priceRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="产品名称" prop="product_name">
              <el-input v-model="priceForm.product_name" placeholder="产品名称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="产品编码" prop="product_code">
              <el-input v-model="priceForm.product_code" placeholder="产品编码" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="供应商" prop="supplier_name">
          <el-input v-model="priceForm.supplier_name" placeholder="供应商名称" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="价格" prop="price">
              <el-input-number v-model="priceForm.price" :min="0" :precision="2" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="货币" prop="currency">
              <el-select v-model="priceForm.currency" placeholder="货币" style="width: 100%">
                <el-option label="CNY" value="CNY" />
                <el-option label="USD" value="USD" />
                <el-option label="EUR" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单位" prop="unit">
              <el-input v-model="priceForm.unit" placeholder="单位" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="生效日期" prop="effective_date">
              <el-date-picker v-model="priceForm.effective_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="失效日期" prop="expiry_date">
              <el-date-picker v-model="priceForm.expiry_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="状态">
          <el-select v-model="priceForm.status" placeholder="状态" style="width: 100%">
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="priceForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="priceDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="priceSubmitLoading" @click="submitPrice">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="returnDialogVisible" :title="returnForm.id ? '编辑采购退货' : '新建采购退货'" width="800px">
      <el-form ref="returnFormRef" :model="returnForm" :rules="returnRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="退货单号" prop="returnNo">
              <el-input v-model="returnForm.returnNo" :disabled="!!returnForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="供应商" prop="supplierName">
              <el-input v-model="returnForm.supplierName" placeholder="供应商名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="关联订单号" prop="purchaseOrderNo">
              <el-input v-model="returnForm.purchaseOrderNo" placeholder="订单号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="退货日期" prop="returnDate">
              <el-date-picker v-model="returnForm.returnDate" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="退货原因" prop="reason">
          <el-input v-model="returnForm.reason" type="textarea" />
        </el-form-item>
        <el-divider>退货明细</el-divider>
        <el-table :data="returnForm.items" border style="width: 100%">
          <el-table-column prop="productName" label="产品名称" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.productName" placeholder="产品名称" />
            </template>
          </el-table-column>
          <el-table-column prop="productCode" label="产品编码" width="120">
            <template #default="{ row }">
              <el-input v-model="row.productCode" placeholder="编码" />
            </template>
          </el-table-column>
          <el-table-column prop="quantity" label="数量" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.quantity" :min="0" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="unit" label="单位" width="80">
            <template #default="{ row }">
              <el-input v-model="row.unit" placeholder="单位" />
            </template>
          </el-table-column>
          <el-table-column prop="price" label="单价" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.price" :min="0" :precision="2" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="amount" label="金额" width="100">
            <template #default="{ row }">
              {{ formatMoney(row.quantity * row.price) }}
            </template>
          </el-table-column>
          <el-table-column prop="reason" label="退货原因" min-width="120">
            <template #default="{ row }">
              <el-input v-model="row.reason" placeholder="退货原因" />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="{ $index }">
              <el-button type="danger" link size="small" @click="removeReturnItem($index)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" link style="margin-top: 8px" @click="addReturnItem">添加产品</el-button>
      </el-form>
      <template #footer>
        <el-button @click="returnDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="returnSubmitLoading" @click="submitReturn">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="returnViewVisible" title="采购退货详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="退货单号">{{ currentReturn?.returnNo }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{ currentReturn?.supplierName }}</el-descriptions-item>
        <el-descriptions-item label="关联订单">{{ currentReturn?.purchaseOrderNo }}</el-descriptions-item>
        <el-descriptions-item label="退货日期">{{ currentReturn?.returnDate }}</el-descriptions-item>
        <el-descriptions-item label="总金额">{{ formatMoney(currentReturn?.totalAmount || 0) }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getReturnStatusType(currentReturn?.status)">
            {{ getReturnStatusLabel(currentReturn?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建人">{{ currentReturn?.createdBy }}</el-descriptions-item>
        <el-descriptions-item label="审批人">{{ currentReturn?.approvedByName }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>退货原因</el-divider>
      <p>{{ currentReturn?.reason }}</p>
      <el-divider>退货明细</el-divider>
      <el-table :data="currentReturn?.items || []" stripe>
        <el-table-column prop="productName" label="产品名称" min-width="150" />
        <el-table-column prop="productCode" label="产品编码" width="120" />
        <el-table-column prop="quantity" label="数量" width="100" align="right" />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="price" label="单价" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="amount" label="金额" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="reason" label="退货原因" min-width="120" />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listPurchaseContracts,
  getPurchaseContract,
  createPurchaseContract,
  updatePurchaseContract,
  approvePurchaseContract,
  executePurchaseContract,
  cancelPurchaseContract,
  type PurchaseContract,
  type ContractItem as PurchaseContractItem
} from '@/api/purchase-contract'
import {
  listPurchasePrices,
  getPurchasePrice,
  createPurchasePrice,
  updatePurchasePrice,
  type PurchasePrice
} from '@/api/purchase-price'
import { purchaseReturnApi, type PurchaseReturn, type PurchaseReturnItem } from '@/api/purchase-return'

const activeTab = ref('contract')

const purchaseContracts = ref<PurchaseContract[]>([])
const purchasePrices = ref<PurchasePrice[]>([])
const purchaseReturns = ref<PurchaseReturn[]>([])
const contractLoading = ref(false)
const priceLoading = ref(false)
const returnLoading = ref(false)

const priceQuery = reactive({
  product_name: '',
  supplier_name: '',
  status: ''
})

const returnQuery = reactive({
  returnNo: '',
  supplierName: '',
  status: ''
})

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const fetchPurchaseContracts = async () => {
  contractLoading.value = true
  try {
    const res = await listPurchaseContracts()
    purchaseContracts.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取采购合同失败')
  } finally {
    contractLoading.value = false
  }
}

const fetchPurchasePrices = async () => {
  priceLoading.value = true
  try {
    const res = await listPurchasePrices(priceQuery)
    purchasePrices.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取采购价格失败')
  } finally {
    priceLoading.value = false
  }
}

const fetchPurchaseReturns = async () => {
  returnLoading.value = true
  try {
    const res = await purchaseReturnApi.list(returnQuery)
    purchaseReturns.value = res.data?.list || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取采购退货失败')
  } finally {
    returnLoading.value = false
  }
}

const resetPriceQuery = () => {
  priceQuery.product_name = ''
  priceQuery.supplier_name = ''
  priceQuery.status = ''
  fetchPurchasePrices()
}

const resetReturnQuery = () => {
  returnQuery.returnNo = ''
  returnQuery.supplierName = ''
  returnQuery.status = ''
  fetchPurchaseReturns()
}

const getContractStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审核',
    active: '执行中',
    completed: '已完成',
    cancelled: '已取消'
  }
  return map[status || ''] || status || ''
}

const getContractStatusType = (status?: string) => {
  const map: Record<string, any> = {
    draft: 'info',
    pending: 'warning',
    active: 'primary',
    completed: 'success',
    cancelled: 'danger'
  }
  return map[status || ''] || 'info'
}

const getReturnStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审核',
    approved: '已批准',
    rejected: '已拒绝',
    completed: '已完成'
  }
  return map[status || ''] || status || ''
}

const getReturnStatusType = (status?: string) => {
  const map: Record<string, any> = {
    draft: 'info',
    pending: 'warning',
    approved: 'success',
    rejected: 'danger',
    completed: 'success'
  }
  return map[status || ''] || 'info'
}

const contractDialogVisible = ref(false)
const contractFormRef = ref<FormInstance>()
const contractSubmitLoading = ref(false)
const contractForm = reactive({
  id: 0,
  contract_no: '',
  supplier_id: 0,
  supplier_name: '',
  contract_date: '',
  start_date: '',
  end_date: '',
  total_amount: 0,
  currency: 'CNY',
  status: 'draft' as 'draft' | 'pending' | 'active' | 'completed' | 'cancelled',
  items: [] as PurchaseContractItem[],
  payment_terms: '',
  delivery_terms: ''
})

const contractRules: FormRules = {
  contract_no: [{ required: true, message: '请输入合同编号', trigger: 'blur' }],
  supplier_name: [{ required: true, message: '请输入供应商名称', trigger: 'blur' }],
  contract_date: [{ required: true, message: '请选择合同日期', trigger: 'change' }],
  total_amount: [{ required: true, message: '请输入总金额', trigger: 'blur' }]
}

const openContractDialog = async (row?: PurchaseContract) => {
  if (row) {
    const res = await getPurchaseContract(row.id)
    Object.assign(contractForm, res.data)
  } else {
    Object.assign(contractForm, {
      id: 0,
      contract_no: '',
      supplier_id: 0,
      supplier_name: '',
      contract_date: '',
      start_date: '',
      end_date: '',
      total_amount: 0,
      currency: 'CNY',
      status: 'draft',
      items: [{ id: 0, contract_id: 0, product_id: 0, product_name: '', product_code: '', quantity: 0, unit: '', price: 0, amount: 0, remark: '' }],
      payment_terms: '',
      delivery_terms: ''
    })
  }
  contractDialogVisible.value = true
}

const submitContract = async () => {
  const valid = await contractFormRef.value?.validate()
  if (!valid) return

  contractSubmitLoading.value = true
  try {
    if (contractForm.id) {
      await updatePurchaseContract(contractForm.id, contractForm)
      ElMessage.success('更新成功')
    } else {
      await createPurchaseContract(contractForm)
      ElMessage.success('创建成功')
    }
    contractDialogVisible.value = false
    fetchPurchaseContracts()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    contractSubmitLoading.value = false
  }
}

const viewContract = async (row: PurchaseContract) => {
  const res = await getPurchaseContract(row.id)
  currentContract.value = res.data
  contractViewVisible.value = true
}

const approveContract = async (row: PurchaseContract) => {
  try {
    await ElMessageBox.confirm('确定审批此合同吗？', '确认', { type: 'info' })
    await approvePurchaseContract(row.id)
    ElMessage.success('审批成功')
    fetchPurchaseContracts()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '操作失败')
  }
}

const executeContract = async (row: PurchaseContract) => {
  try {
    await ElMessageBox.confirm('确定执行此合同吗？', '确认', { type: 'info' })
    await executePurchaseContract(row.id)
    ElMessage.success('执行成功')
    fetchPurchaseContracts()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '操作失败')
  }
}

const cancelContract = async (row: PurchaseContract) => {
  try {
    await ElMessageBox.confirm('确定取消此合同吗？', '确认', { type: 'warning' })
    await cancelPurchaseContract(row.id)
    ElMessage.success('取消成功')
    fetchPurchaseContracts()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '操作失败')
  }
}

const addContractItem = () => {
  contractForm.items.push({ id: 0, contract_id: 0, product_id: 0, product_name: '', product_code: '', quantity: 0, unit: '', price: 0, amount: 0, remark: '' })
}

const removeContractItem = (index: number) => {
  if (contractForm.items.length > 1) {
    contractForm.items.splice(index, 1)
  }
}

const contractViewVisible = ref(false)
const currentContract = ref<PurchaseContract | null>(null)

const priceDialogVisible = ref(false)
const priceFormRef = ref<FormInstance>()
const priceSubmitLoading = ref(false)
const priceForm = reactive({
  id: 0,
  product_id: 0,
  product_name: '',
  product_code: '',
  supplier_id: 0,
  supplier_name: '',
  price: 0,
  currency: 'CNY',
  unit: '',
  effective_date: '',
  expiry_date: '',
  status: 'active' as 'active' | 'inactive',
  remark: ''
})

const priceRules: FormRules = {
  product_name: [{ required: true, message: '请输入产品名称', trigger: 'blur' }],
  supplier_name: [{ required: true, message: '请输入供应商名称', trigger: 'blur' }],
  price: [{ required: true, message: '请输入价格', trigger: 'blur' }],
  effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }]
}

const openPriceDialog = async (row?: PurchasePrice) => {
  if (row) {
    const res = await getPurchasePrice(row.id)
    Object.assign(priceForm, res.data)
  } else {
    Object.assign(priceForm, {
      id: 0,
      product_id: 0,
      product_name: '',
      product_code: '',
      supplier_id: 0,
      supplier_name: '',
      price: 0,
      currency: 'CNY',
      unit: '',
      effective_date: '',
      expiry_date: '',
      status: 'active',
      remark: ''
    })
  }
  priceDialogVisible.value = true
}

const submitPrice = async () => {
  const valid = await priceFormRef.value?.validate()
  if (!valid) return

  priceSubmitLoading.value = true
  try {
    if (priceForm.id) {
      await updatePurchasePrice(priceForm.id, priceForm)
      ElMessage.success('更新成功')
    } else {
      await createPurchasePrice(priceForm)
      ElMessage.success('创建成功')
    }
    priceDialogVisible.value = false
    fetchPurchasePrices()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    priceSubmitLoading.value = false
  }
}

const returnDialogVisible = ref(false)
const returnFormRef = ref<FormInstance>()
const returnSubmitLoading = ref(false)
const returnForm = reactive({
  id: 0,
  returnNo: '',
  supplierId: 0,
  supplierName: '',
  orderId: 0,
  purchaseOrderNo: '',
  returnDate: '',
  totalAmount: 0,
  reason: '',
  status: 'draft' as 'draft' | 'pending' | 'approved' | 'rejected' | 'completed',
  items: [] as PurchaseReturnItem[]
})

const returnRules: FormRules = {
  returnNo: [{ required: true, message: '请输入退货单号', trigger: 'blur' }],
  supplierName: [{ required: true, message: '请输入供应商名称', trigger: 'blur' }],
  returnDate: [{ required: true, message: '请选择退货日期', trigger: 'change' }],
  reason: [{ required: true, message: '请输入退货原因', trigger: 'blur' }]
}

const openReturnDialog = async (row?: PurchaseReturn) => {
  if (row) {
    const res = await purchaseReturnApi.getById(row.id)
    Object.assign(returnForm, res.data)
  } else {
    Object.assign(returnForm, {
      id: 0,
      returnNo: '',
      supplierId: 0,
      supplierName: '',
      orderId: 0,
      purchaseOrderNo: '',
      returnDate: '',
      totalAmount: 0,
      reason: '',
      status: 'draft',
      items: [{ id: 0, returnId: 0, productId: 0, productName: '', productCode: '', quantity: 0, unit: '', price: 0, amount: 0, reason: '' }]
    })
  }
  returnDialogVisible.value = true
}

const submitReturn = async () => {
  const valid = await returnFormRef.value?.validate()
  if (!valid) return

  returnSubmitLoading.value = true
  try {
    if (returnForm.id) {
      await purchaseReturnApi.update(returnForm.id, returnForm)
      ElMessage.success('更新成功')
    } else {
      await purchaseReturnApi.create(returnForm)
      ElMessage.success('创建成功')
    }
    returnDialogVisible.value = false
    fetchPurchaseReturns()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    returnSubmitLoading.value = false
  }
}

const viewReturn = async (row: PurchaseReturn) => {
  const res = await purchaseReturnApi.getById(row.id)
  currentReturn.value = res.data
  returnViewVisible.value = true
}

const addReturnItem = () => {
  returnForm.items.push({ id: 0, returnId: 0, productId: 0, productName: '', productCode: '', quantity: 0, unit: '', price: 0, amount: 0, reason: '' })
}

const removeReturnItem = (index: number) => {
  if (returnForm.items.length > 1) {
    returnForm.items.splice(index, 1)
  }
}

const returnViewVisible = ref(false)
const currentReturn = ref<PurchaseReturn | null>(null)

onMounted(() => {
  fetchPurchaseContracts()
  fetchPurchasePrices()
  fetchPurchaseReturns()
})
</script>

<style scoped>
.purchase-ext-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 600; color: #303133; margin: 0; }
.filter-card { margin-bottom: 20px; }
</style>
