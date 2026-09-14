<template>
  <div class="chemicals-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>化学品主数据</h2>
        <div class="header-actions">
          <el-input
            v-model="keyword"
            placeholder="搜索编码 / 名称"
            clearable
            style="width: 220px"
            @keyup.enter="handleFilter"
            @clear="handleFilter"
          >
            <template #append>
              <el-button :icon="Search" @click="handleFilter" />
            </template>
          </el-input>
          <el-button type="primary" @click="handleCreate">新建化学品</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="chemicalList" border>
        <el-table-column prop="chemical_code" label="化学品编码" min-width="130" />
        <el-table-column prop="chemical_name" label="名称" min-width="140" show-overflow-tooltip />
        <el-table-column prop="chemical_type" label="类型" width="110" align="center" />
        <el-table-column prop="cas_number" label="CAS 号" width="130">
          <template #default="{ row }">{{ row.cas_number || '-' }}</template>
        </el-table-column>
        <el-table-column prop="unit" label="单位" width="80" align="center" />
        <el-table-column prop="standard_price" label="标准价" width="100" align="right">
          <template #default="{ row }">{{ row.standard_price ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="ghs_classification" label="GHS 分类" width="120" show-overflow-tooltip>
          <template #default="{ row }">{{ row.ghs_classification || '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as ChemicalStatus] ?? 'info'">
              {{ statusTextMap[row.status as ChemicalStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="handleDetail(row)">详情</el-button>
            <el-button size="small" link type="primary" @click="handleEdit(row)">编辑</el-button>
            <el-button
              v-if="row.status === 'active'"
              size="small"
              link
              type="danger"
              @click="handleDelete(row)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50]"
        layout="total, sizes, prev, pager, next"
        style="margin-top: 16px; justify-content: flex-end"
        @current-change="loadList"
        @size-change="handleFilter"
      />
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="editingId ? '编辑化学品' : '新建化学品'"
      width="640px"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="110px">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="化学品编码" prop="chemical_code">
              <el-input v-model="formData.chemical_code" :disabled="!!editingId" placeholder="必填" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="化学品名称" prop="chemical_name">
              <el-input v-model="formData.chemical_name" placeholder="必填" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="类型" prop="chemical_type">
              <el-select v-model="formData.chemical_type" placeholder="必选" style="width: 100%">
                <el-option label="染料" value="dye" />
                <el-option label="助剂" value="auxiliary" />
                <el-option label="化工原料" value="raw_material" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="CAS 号" prop="cas_number">
              <el-input v-model="formData.cas_number" placeholder="选填" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="单位" prop="unit">
              <el-input v-model="formData.unit" placeholder="kg / L" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="标准价格" prop="standard_price">
              <el-input-number
                v-model="formData.standard_price"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="GHS 分类" prop="ghs_classification">
              <el-input v-model="formData.ghs_classification" placeholder="选填" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="UN 编号" prop="un_number">
              <el-input v-model="formData.un_number" placeholder="选填" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="保质期(天)" prop="shelf_life_days">
              <el-input-number
                v-model="formData.shelf_life_days"
                :min="0"
                :precision="0"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="储存条件" prop="storage_condition">
              <el-input v-model="formData.storage_condition" placeholder="选填" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="MSDS 链接" prop="msds_url">
          <el-input v-model="formData.msds_url" placeholder="选填" />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitForm">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="化学品详情" width="640px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="化学品编码">{{ detailRow.chemical_code }}</el-descriptions-item>
        <el-descriptions-item label="名称">{{ detailRow.chemical_name }}</el-descriptions-item>
        <el-descriptions-item label="英文名">{{ detailRow.chemical_name_en || '-' }}</el-descriptions-item>
        <el-descriptions-item label="类型">{{ detailRow.chemical_type }}</el-descriptions-item>
        <el-descriptions-item label="CAS 号">{{ detailRow.cas_number || '-' }}</el-descriptions-item>
        <el-descriptions-item label="规格">{{ detailRow.specification || '-' }}</el-descriptions-item>
        <el-descriptions-item label="单位">{{ detailRow.unit }}</el-descriptions-item>
        <el-descriptions-item label="标准价格">{{ detailRow.standard_price }}</el-descriptions-item>
        <el-descriptions-item label="成本价格">{{ detailRow.cost_price }}</el-descriptions-item>
        <el-descriptions-item label="GHS 分类">{{ detailRow.ghs_classification || '-' }}</el-descriptions-item>
        <el-descriptions-item label="信号词">{{ detailRow.signal_word || '-' }}</el-descriptions-item>
        <el-descriptions-item label="保质期(天)">{{ detailRow.shelf_life_days ?? '-' }}</el-descriptions-item>
        <el-descriptions-item label="储存条件">{{ detailRow.storage_condition || '-' }}</el-descriptions-item>
        <el-descriptions-item label="安全库存">{{ detailRow.safety_stock }}</el-descriptions-item>
        <el-descriptions-item label="补货点">{{ detailRow.reorder_point }}</el-descriptions-item>
        <el-descriptions-item label="补货量">{{ detailRow.reorder_quantity }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="statusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="MSDS 版本">{{ detailRow.msds_version || '-' }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{ detailRow.remarks || '-' }}</el-descriptions-item>
      </el-descriptions>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { Search } from '@element-plus/icons-vue';
import {
  getChemicals,
  createChemical,
  updateChemical,
  deleteChemical,
  type Chemical,
  type ChemicalStatus,
} from '@/api/chemical';

const statusTextMap: Record<ChemicalStatus, string> = {
  active: '在用',
  inactive: '停用',
  discontinued: '已淘汰',
};

const statusTagMap: Record<ChemicalStatus, 'info' | 'warning' | 'success'> = {
  active: 'success',
  inactive: 'info',
  discontinued: 'warning',
};

const loading = ref(false);
const submitLoading = ref(false);
const chemicalList = ref<Chemical[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const keyword = ref('');
const dialogVisible = ref(false);
const detailVisible = ref(false);
const detailRow = ref<Chemical | null>(null);
const editingId = ref<number | null>(null);
const formRef = ref<FormInstance>();

const formData = reactive({
  chemical_code: '',
  chemical_name: '',
  chemical_type: '',
  cas_number: '',
  unit: '',
  standard_price: undefined as number | undefined,
  ghs_classification: '',
  un_number: '',
  shelf_life_days: undefined as number | undefined,
  storage_condition: '',
  msds_url: '',
  remarks: '',
});

const formRules: FormRules = {
  chemical_code: [{ required: true, message: '请输入化学品编码', trigger: 'blur' }],
  chemical_name: [{ required: true, message: '请输入化学品名称', trigger: 'blur' }],
  chemical_type: [{ required: true, message: '请选择化学品类型', trigger: 'change' }],
};

/** 响应解包防御：兼容数组 / { items } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): Chemical[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: Chemical[] } | null;
  return paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getChemicals({
      page: page.value,
      page_size: pageSize.value,
      keyword: keyword.value || undefined,
    });
    chemicalList.value = unwrapList(res.data);
    total.value = res.data && !Array.isArray(res.data) ? (res.data.total ?? 0) : chemicalList.value.length;
  } catch {
    ElMessage.error('加载化学品列表失败');
  } finally {
    loading.value = false;
  }
};

const handleFilter = () => {
  page.value = 1;
  loadList();
};

const handleCreate = () => {
  editingId.value = null;
  dialogVisible.value = true;
};

const handleEdit = (row: Chemical) => {
  editingId.value = row.id;
  formData.chemical_code = row.chemical_code;
  formData.chemical_name = row.chemical_name;
  formData.chemical_type = row.chemical_type;
  formData.cas_number = row.cas_number ?? '';
  formData.unit = row.unit;
  formData.standard_price = row.standard_price === null ? undefined : Number(row.standard_price);
  formData.ghs_classification = row.ghs_classification ?? '';
  formData.un_number = row.un_number ?? '';
  formData.shelf_life_days = row.shelf_life_days ?? undefined;
  formData.storage_condition = row.storage_condition ?? '';
  formData.msds_url = row.msds_url ?? '';
  formData.remarks = row.remarks ?? '';
  dialogVisible.value = true;
};

const resetForm = () => {
  formData.chemical_code = '';
  formData.chemical_name = '';
  formData.chemical_type = '';
  formData.cas_number = '';
  formData.unit = '';
  formData.standard_price = undefined;
  formData.ghs_classification = '';
  formData.un_number = '';
  formData.shelf_life_days = undefined;
  formData.storage_condition = '';
  formData.msds_url = '';
  formData.remarks = '';
  formRef.value?.resetFields();
};

const handleDetail = (row: Chemical) => {
  detailRow.value = row;
  detailVisible.value = true;
};

const buildPayload = () => ({
  chemical_code: formData.chemical_code,
  chemical_name: formData.chemical_name,
  chemical_type: formData.chemical_type,
  cas_number: formData.cas_number || undefined,
  unit: formData.unit || undefined,
  standard_price: formData.standard_price,
  ghs_classification: formData.ghs_classification || undefined,
  un_number: formData.un_number || undefined,
  shelf_life_days: formData.shelf_life_days,
  storage_condition: formData.storage_condition || undefined,
  msds_url: formData.msds_url || undefined,
  remarks: formData.remarks || undefined,
});

const submitForm = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (editingId.value) {
        await updateChemical(editingId.value, buildPayload());
        ElMessage.success('化学品更新成功');
      } else {
        await createChemical(buildPayload());
        ElMessage.success('化学品创建成功');
      }
      dialogVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error(editingId.value ? '化学品更新失败' : '化学品创建失败');
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDelete = async (row: Chemical) => {
  try {
    await ElMessageBox.confirm(`确认删除化学品 ${row.chemical_name} 吗？`, '删除确认', {
      confirmButtonText: '确认删除',
      cancelButtonText: '取消',
      type: 'warning',
    });
    await deleteChemical(row.id);
    ElMessage.success('删除成功');
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error('删除失败');
  }
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.chemicals-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 18px;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}
</style>
