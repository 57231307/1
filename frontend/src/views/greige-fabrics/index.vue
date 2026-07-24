<template>
  <div class="greige-fabrics-page">
    <div class="header">
      <h2>坯布管理</h2>
      <el-button type="primary" @click="handleCreate">新建坯布</el-button>
    </div>

    <el-table v-loading="loading" :data="greigeList" border aria-label="坯布列表">
      <el-table-column prop="fabric_code" label="面料编码" min-width="120" />
      <el-table-column prop="fabric_name" label="面料名称" min-width="120" />
      <el-table-column prop="fabric_type" label="面料类型" width="100" />
      <el-table-column prop="supplier_name" label="供应商" width="120" />
      <el-table-column prop="quantity" label="数量" width="100" align="right" />
      <el-table-column prop="unit" label="单位" width="80" />
      <el-table-column prop="status" label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'">
            {{ row.status === 'active' ? '启用' : '停用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200">
        <template #default="{ row }">
          <el-button size="small" @click="handleEdit(row)">编辑</el-button>
          <el-button size="small" type="danger" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogMode === 'create' ? '新建坯布' : '编辑坯布'"
      width="600px"
      aria-label="坯布编辑对话框"
      @close="handleDialogClose"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px" aria-label="坯布信息表单">
        <el-form-item label="面料编码" prop="fabric_code">
          <el-input v-model="formData.fabric_code" placeholder="请输入面料编码" />
        </el-form-item>
        <el-form-item label="面料名称" prop="fabric_name">
          <el-input v-model="formData.fabric_name" placeholder="请输入面料名称" />
        </el-form-item>
        <el-form-item label="面料类型" prop="fabric_type">
          <el-input v-model="formData.fabric_type" placeholder="请输入面料类型" />
        </el-form-item>
        <el-form-item label="供应商" prop="supplier_id">
          <el-select v-model="formData.supplier_id" placeholder="请选择供应商" filterable>
            <el-option
              v-for="s in supplierList"
              :key="s.id"
              :label="s.supplier_name"
              :value="s.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="宽度" prop="width">
          <el-input-number
            v-model="formData.width"
            :min="0"
            :precision="2"
            placeholder="请输入宽度"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="克重" prop="weight">
          <el-input-number
            v-model="formData.weight"
            :min="0"
            :precision="2"
            placeholder="请输入克重"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="单位" prop="unit">
          <el-select v-model="formData.unit" placeholder="请选择单位">
            <el-option label="米" value="m" />
            <el-option label="码" value="yd" />
            <el-option label="千克" value="kg" />
          </el-select>
        </el-form-item>
        <el-form-item label="成分" prop="composition">
          <el-input v-model="formData.composition" placeholder="请输入成分" />
        </el-form-item>
        <el-form-item label="数量" prop="quantity">
          <el-input-number
            v-model="formData.quantity"
            :min="0"
            :precision="2"
            placeholder="请输入数量"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="最小起订量" prop="min_order_quantity">
          <el-input-number
            v-model="formData.min_order_quantity"
            :min="0"
            :precision="2"
            placeholder="请输入最小起订量"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="formData.status" placeholder="请选择状态">
            <el-option label="启用" value="active" />
            <el-option label="停用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="formData.description" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit"> 确定 </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getGreigeFabricList,
  createGreigeFabric,
  updateGreigeFabric,
  deleteGreigeFabric,
  type GreigeFabric,
} from '@/api/greige-fabric'
import { getSupplierList, type Supplier } from '@/api/supplier'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const formRef = ref<FormInstance>()
// v11 批次 181 P2-1 修复：使用 API 的 GreigeFabric 类型，替代本地 GreigeFabricRow
const greigeList = ref<GreigeFabric[]>([])
// v11 批次 181 P2-1 修复：使用 Supplier 类型，替代本地 WarehouseOption
const supplierList = ref<Supplier[]>([])

// 表单数据类型，与 GreigeFabric 对齐（id 创建时为空，编辑时由后端返回）
interface GreigeFabricForm {
  id?: number
  fabric_code: string
  fabric_name: string
  fabric_type: string
  supplier_id: number
  supplier_name: string
  width: number
  weight: number
  unit: string
  composition: string
  quantity: number
  min_order_quantity: number
  status: 'active' | 'inactive'
  description: string
}

const formData = reactive<GreigeFabricForm>({
  fabric_code: '',
  fabric_name: '',
  fabric_type: '',
  supplier_id: 0,
  supplier_name: '',
  width: 0,
  weight: 0,
  unit: 'm',
  composition: '',
  quantity: 0,
  min_order_quantity: 0,
  status: 'active',
  description: '',
})

const formRules: FormRules = {
  fabric_code: [{ required: true, message: '请输入面料编码', trigger: 'blur' }],
  fabric_name: [{ required: true, message: '请输入面料名称', trigger: 'blur' }],
  fabric_type: [{ required: true, message: '请输入面料类型', trigger: 'blur' }],
  supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
  unit: [{ required: true, message: '请选择单位', trigger: 'change' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

const loadGreigeFabrics = async () => {
  loading.value = true
  try {
    const res = await getGreigeFabricList()
    // v11 批次 181 P2-1 修复：API 返回 GreigeFabric[]，前端直接使用，无需类型转换
    greigeList.value = res.data || []
  } catch (error) {
    ElMessage.error('加载坯布列表失败')
  } finally {
    loading.value = false
  }
}

const loadSuppliers = async () => {
  try {
    // v11 批次 181 P2-1 修复：使用 supplier API 替代 warehouse API
    // getSupplierList 返回 { list: Supplier[]; total: number }，提取 list 字段
    const res = await getSupplierList()
    const d = res.data
    if (d && typeof d === 'object' && 'list' in d) {
      supplierList.value = d.list || []
    } else {
      supplierList.value = []
    }
  } catch (error) {
    ElMessage.error('加载供应商列表失败')
  }
}

const resetForm = () => {
  Object.assign(formData, {
    id: undefined,
    fabric_code: '',
    fabric_name: '',
    fabric_type: '',
    supplier_id: 0,
    supplier_name: '',
    width: 0,
    weight: 0,
    unit: 'm',
    composition: '',
    quantity: 0,
    min_order_quantity: 0,
    status: 'active',
    description: '',
  })
}

const handleCreate = () => {
  dialogMode.value = 'create'
  resetForm()
  dialogVisible.value = true
}

const handleEdit = (row: GreigeFabric) => {
  dialogMode.value = 'edit'
  // 同步供应商名称展示
  const supplier = supplierList.value.find(s => s.id === row.supplier_id)
  Object.assign(formData, {
    id: row.id,
    fabric_code: row.fabric_code,
    fabric_name: row.fabric_name,
    fabric_type: row.fabric_type,
    supplier_id: row.supplier_id,
    supplier_name: supplier?.supplier_name || row.supplier_name || '',
    width: row.width,
    weight: row.weight,
    unit: row.unit,
    composition: row.composition,
    quantity: row.quantity,
    min_order_quantity: row.min_order_quantity,
    status: row.status,
    description: row.description,
  })
  dialogVisible.value = true
}

const handleDelete = async (row: GreigeFabric) => {
  if (!row.id) return

  try {
    await deleteGreigeFabric(row.id)
    ElMessage.success('删除成功')
    await loadGreigeFabrics()
  } catch (error) {
    ElMessage.error('删除失败')
  }
}

const handleSubmit = async () => {
  if (!formRef.value) return

  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      // 同步 supplier_name 到表单数据
      const supplier = supplierList.value.find(s => s.id === formData.supplier_id)
      if (supplier) {
        formData.supplier_name = supplier.supplier_name
      }

      if (dialogMode.value === 'create') {
        // v11 批次 181 P2-1 修复：GreigeFabricForm 与 Partial<GreigeFabric> 字段一致，直接传入
        await createGreigeFabric(formData)
        ElMessage.success('创建成功')
      } else {
        // edit 模式下 formData.id 由 handleEdit 从 row.id 赋值
        await updateGreigeFabric(formData.id!, formData)
        ElMessage.success('更新成功')
      }
      dialogVisible.value = false
      await loadGreigeFabrics()
    } catch (error) {
      ElMessage.error(dialogMode.value === 'create' ? '创建失败' : '更新失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDialogClose = () => {
  formRef.value?.resetFields()
}

const hasLoaded = createLazyLoader()

onMounted(() => {
  loadGreigeFabrics()
  loadIfNot('suppliers', loadSuppliers, hasLoaded)
})
</script>

<style scoped>
.greige-fabrics-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>
