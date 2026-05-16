<template>
  <div class="greige-fabrics-page">
    <div class="header">
      <h2>坯布管理</h2>
      <el-button type="primary" @click="handleCreate">新建坯布</el-button>
    </div>

    <el-table :data="greigeList" v-loading="loading" border>
      <el-table-column prop="batchNo" label="坯布批号" />
      <el-table-column prop="productId" label="产品 ID" />
      <el-table-column prop="quantity" label="数量" />
      <el-table-column prop="unit" label="单位" />
      <el-table-column prop="warehouseId" label="仓库 ID" />
      <el-table-column prop="status" label="状态">
        <template #default="{ row }">
          <el-tag :type="row.status === 'available' ? 'success' : 'warning'">
            {{ row.status === 'available' ? '可用' : '已占用' }}
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
      @close="handleDialogClose"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="100px"
      >
        <el-form-item label="坯布批号" prop="batchNo">
          <el-input v-model="formData.batchNo" placeholder="请输入坯布批号" />
        </el-form-item>
        <el-form-item label="产品 ID" prop="productId">
          <el-input-number v-model="formData.productId" :min="1" placeholder="请输入产品 ID" style="width: 100%" />
        </el-form-item>
        <el-form-item label="数量" prop="quantity">
          <el-input-number v-model="formData.quantity" :min="0" :precision="2" placeholder="请输入数量" style="width: 100%" />
        </el-form-item>
        <el-form-item label="单位" prop="unit">
          <el-select v-model="formData.unit" placeholder="请选择单位">
            <el-option label="米" value="m" />
            <el-option label="码" value="yd" />
            <el-option label="千克" value="kg" />
          </el-select>
        </el-form-item>
        <el-form-item label="仓库" prop="warehouseId">
          <el-select v-model="formData.warehouseId" placeholder="请选择仓库">
            <el-option
              v-for="wh in warehouseList"
              :key="wh.id"
              :label="wh.name"
              :value="wh.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="formData.status" placeholder="请选择状态">
            <el-option label="可用" value="available" />
            <el-option label="已占用" value="occupied" />
          </el-select>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit" :loading="submitLoading">
          确定
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listGreigeFabrics, createGreigeFabric, updateGreigeFabric, deleteGreigeFabric } from '@/api/greige-fabric'
import { warehouseApi } from '@/api/warehouse'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const formRef = ref<FormInstance>()
const greigeList = ref<any[]>([])
const warehouseList = ref<any[]>([])

const formData = reactive<any>({
  batchNo: '',
  productId: null,
  quantity: 0,
  unit: 'm',
  warehouseId: null,
  status: 'available'
})

const formRules: FormRules = {
  batchNo: [{ required: true, message: '请输入坯布批号', trigger: 'blur' }],
  productId: [{ required: true, message: '请输入产品 ID', trigger: 'blur' }],
  quantity: [{ required: true, message: '请输入数量', trigger: 'blur' }],
  unit: [{ required: true, message: '请选择单位', trigger: 'change' }],
  warehouseId: [{ required: true, message: '请选择仓库', trigger: 'change' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }]
}

const loadGreigeFabrics = async () => {
  loading.value = true
  try {
    const res = await listGreigeFabrics()
    greigeList.value = res.data || []
  } catch (error) {
    ElMessage.error('加载坯布列表失败')
  } finally {
    loading.value = false
  }
}

const loadWarehouses = async () => {
  try {
    const res = await warehouseApi.list()
    warehouseList.value = (res.data as any).list || []
  } catch (error) {
    ElMessage.error('加载仓库列表失败')
  }
}

const handleCreate = () => {
  dialogMode.value = 'create'
  Object.assign(formData, {
    id: null,
    batchNo: '',
    productId: null,
    quantity: 0,
    unit: 'm',
    warehouseId: null,
    status: 'available'
  })
  dialogVisible.value = true
}

const handleEdit = (row: any) => {
  dialogMode.value = 'edit'
  Object.assign(formData, {
    id: row.id,
    batchNo: row.batchNo,
    productId: row.productId,
    quantity: row.quantity,
    unit: row.unit,
    warehouseId: row.warehouseId,
    status: row.status
  })
  dialogVisible.value = true
}

const handleDelete = async (row: any) => {
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
      if (dialogMode.value === 'create') {
        await createGreigeFabric(formData)
        ElMessage.success('创建成功')
      } else {
        await updateGreigeFabric(formData.id, formData)
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

onMounted(() => {
  loadGreigeFabrics()
  loadWarehouses()
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
