<template>
  <div class="warehouse-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('warehouse.index.pageTitle') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            t('warehouse.index.breadcrumbHome')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('warehouse.index.breadcrumbMasterData') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('warehouse.index.breadcrumbWarehouse') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          {{ t('warehouse.index.buttonCreate') }}
        </el-button>
        <el-button v-permission="'warehouse.print'" @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ t('warehouse.index.buttonPrint') }}
        </el-button>
        <el-button v-permission="'warehouse.export'" @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ t('warehouse.index.buttonExport') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="t('warehouse.index.ariaFilterForm')"
      >
        <el-form-item :label="t('warehouse.index.filterKeyword')">
          <el-input
            v-model="queryParams.keyword"
            :placeholder="t('warehouse.index.placeholderKeyword')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('warehouse.index.filterType')">
          <el-select
            v-model="queryParams.warehouse_type"
            :placeholder="t('warehouse.index.placeholderType')"
            clearable
          >
            <el-option :label="t('warehouse.index.optionRaw')" value="raw" />
            <el-option :label="t('warehouse.index.optionFinished')" value="finished" />
            <el-option :label="t('warehouse.index.optionSemi')" value="semi" />
            <el-option :label="t('warehouse.index.optionReturn')" value="return" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('warehouse.index.filterStatus')">
          <el-select
            v-model="queryParams.status"
            :placeholder="t('warehouse.index.placeholderStatus')"
            clearable
          >
            <el-option :label="t('warehouse.index.optionActive')" value="active" />
            <el-option :label="t('warehouse.index.optionInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            t('warehouse.index.buttonQuery')
          }}</el-button>
          <el-button @click="handleReset">{{ t('warehouse.index.buttonReset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="warehouses"
        stripe
        :aria-label="t('warehouse.index.ariaTable')"
      >
        <el-table-column
          prop="warehouse_code"
          :label="t('warehouse.index.colCode')"
          width="120"
          fixed
        />
        <el-table-column
          prop="warehouse_name"
          :label="t('warehouse.index.colName')"
          min-width="180"
          fixed
        />
        <el-table-column prop="warehouse_type" :label="t('warehouse.index.colType')" width="100">
          <template #default="{ row }">
            <el-tag :type="getWarehouseTypeTag(row.warehouse_type)" size="small">
              {{ getWarehouseTypeLabel(row.warehouse_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="address"
          :label="t('warehouse.index.colAddress')"
          min-width="200"
          show-overflow-tooltip
        />
        <el-table-column
          prop="contact_person"
          :label="t('warehouse.index.colContact')"
          width="100"
        />
        <el-table-column prop="phone" :label="t('warehouse.index.colPhone')" width="130" />
        <el-table-column
          prop="capacity"
          :label="t('warehouse.index.colCapacity')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            {{ row.capacity ? `${row.capacity} m³` : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="is_default" :label="t('warehouse.index.colDefault')" width="80">
          <template #default="{ row }">
            <el-tag v-if="row.is_default" type="success" size="small">{{
              t('warehouse.index.defaultYes')
            }}</el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="t('warehouse.index.colStatus')" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{
                row.status === 'active'
                  ? t('warehouse.index.statusActive')
                  : t('warehouse.index.statusInactive')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('warehouse.index.colOperation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button
              v-permission="PERMISSIONS.WAREHOUSE_UPDATE"
              type="primary"
              link
              size="small"
              @click="handleEdit(row as Warehouse)"
              >{{ t('warehouse.index.buttonEdit') }}</el-button
            >
            <el-button
              v-permission="PERMISSIONS.WAREHOUSE_DELETE"
              type="danger"
              link
              size="small"
              @click="handleDelete(row as Warehouse)"
              >{{ t('warehouse.index.buttonDelete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('warehouse.index.ariaPagination')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <!-- 新增/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="600px"
      :close-on-click-modal="false"
      :aria-label="t('warehouse.index.ariaEditDialog')"
      @close="resetForm"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="100px"
        :aria-label="t('warehouse.index.ariaForm')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('warehouse.index.colCode')" prop="warehouse_code">
              <el-input
                v-model="formData.warehouse_code"
                :placeholder="t('warehouse.index.placeholderCode')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('warehouse.index.colName')" prop="warehouse_name">
              <el-input
                v-model="formData.warehouse_name"
                :placeholder="t('warehouse.index.placeholderName')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('warehouse.index.filterType')" prop="warehouse_type">
              <el-select
                v-model="formData.warehouse_type"
                :placeholder="t('warehouse.index.placeholderSelectType')"
                style="width: 100%"
              >
                <el-option :label="t('warehouse.index.optionRaw')" value="raw" />
                <el-option :label="t('warehouse.index.optionFinished')" value="finished" />
                <el-option :label="t('warehouse.index.optionSemi')" value="semi" />
                <el-option :label="t('warehouse.index.optionReturn')" value="return" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('warehouse.index.labelCapacityM3')" prop="capacity">
              <el-input-number v-model="formData.capacity" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('warehouse.index.colAddress')" prop="address">
          <el-input
            v-model="formData.address"
            :placeholder="t('warehouse.index.placeholderAddress')"
          />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('warehouse.index.colContact')" prop="contact_person">
              <el-input
                v-model="formData.contact_person"
                :placeholder="t('warehouse.index.placeholderContact')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('warehouse.index.colPhone')" prop="phone">
              <el-input
                v-model="formData.phone"
                :placeholder="t('warehouse.index.placeholderPhone')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('warehouse.index.labelDescription')" prop="description">
          <el-input
            v-model="formData.description"
            type="textarea"
            :rows="3"
            :placeholder="t('warehouse.index.placeholderDescription')"
          />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('warehouse.index.labelSetDefault')" prop="is_default">
              <el-switch v-model="formData.is_default" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('warehouse.index.filterStatus')" prop="status">
              <el-radio-group v-model="formData.status">
                <el-radio value="active">{{ t('warehouse.index.optionActive') }}</el-radio>
                <el-radio value="inactive">{{ t('warehouse.index.optionInactive') }}</el-radio>
              </el-radio-group>
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          t('warehouse.index.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          t('warehouse.index.buttonSave')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { Plus, Download, Printer } from '@element-plus/icons-vue';
import { deleteWarehouse, updateWarehouse, createWarehouse, type Warehouse } from '@/api/warehouse';
// V15 P0-S12 修复（Batch 475c）：导出改用后端带水印 xlsx 接口
// 后端 GET /warehouses/export 已就绪（含异步审计日志 + 水印）
// 注意：后端 WarehouseListQuery 使用 search 字段而非 keyword，前端需做映射
import { exportFromBackend } from '@/utils/export';
import { printData } from '@/utils/print';
import { useTableApi } from '@/composables/useTableApi';
// Batch 462 P0-S24：引入权限码常量，与后端 warehouses 资源对齐
import { PERMISSIONS } from '@/constants/permissions';

const { t } = useI18n({ useScope: 'global' });

const submitLoading = ref(false);
const dialogVisible = ref(false);
const isEdit = ref(false);
const formRef = ref<FormInstance>();

const queryParams = reactive({
  keyword: '',
  warehouse_type: '',
  status: '',
});

// 批次 275：接入 useTableApi，消除手写 warehouses/total/loading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: warehouses,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<Warehouse>({
  url: '/warehouses',
  onError: (err: unknown) =>
    ElMessage.error(
      (err instanceof Error ? err.message : String(err)) || t('warehouse.index.messageFetchFailed')
    ),
});

// 批次 275：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined);
  setQueryParam('warehouse_type', queryParams.warehouse_type || undefined);
  setQueryParam('status', queryParams.status || undefined);
};

const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  fetchData();
};

const handleReset = () => {
  queryParams.keyword = '';
  queryParams.warehouse_type = '';
  queryParams.status = '';
  syncQueryParams();
  page.value = 1;
  fetchData();
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

const handleSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

const formData = reactive({
  id: undefined as number | undefined,
  warehouse_code: '',
  warehouse_name: '',
  warehouse_type: 'finished',
  address: '',
  contact_person: '',
  phone: '',
  capacity: undefined as number | undefined,
  description: '',
  is_default: false,
  status: 'active',
});

const formRules: FormRules = {
  warehouse_code: [
    { required: true, message: t('warehouse.index.ruleCodeRequired'), trigger: 'blur' },
  ],
  warehouse_name: [
    { required: true, message: t('warehouse.index.ruleNameRequired'), trigger: 'blur' },
  ],
  warehouse_type: [
    { required: true, message: t('warehouse.index.ruleTypeRequired'), trigger: 'change' },
  ],
};

const dialogTitle = computed(() =>
  isEdit.value ? t('warehouse.index.titleEdit') : t('warehouse.index.titleCreate')
);

const getWarehouseTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    raw: t('warehouse.index.optionRaw'),
    finished: t('warehouse.index.optionFinished'),
    semi: t('warehouse.index.optionSemi'),
    return: t('warehouse.index.optionReturn'),
  };
  return labels[type] || type;
};

const getWarehouseTypeTag = (type: string) => {
  const tags: Record<string, string> = {
    raw: 'warning',
    finished: 'success',
    semi: 'info',
    return: 'danger',
  };
  return tags[type] || '';
};

const resetForm = () => {
  formData.id = undefined;
  formData.warehouse_code = '';
  formData.warehouse_name = '';
  formData.warehouse_type = 'finished';
  formData.address = '';
  formData.contact_person = '';
  formData.phone = '';
  formData.capacity = undefined;
  formData.description = '';
  formData.is_default = false;
  formData.status = 'active';
  formRef.value?.clearValidate();
};

const handleCreate = () => {
  resetForm();
  isEdit.value = false;
  dialogVisible.value = true;
};

const handleEdit = (row: Warehouse) => {
  resetForm();
  Object.assign(formData, row);
  isEdit.value = true;
  dialogVisible.value = true;
};

const handleDelete = async (row: Warehouse) => {
  try {
    await ElMessageBox.confirm(
      t('warehouse.index.messageConfirmDelete', { name: row.warehouse_name }),
      t('warehouse.index.titleDeleteConfirm'),
      { type: 'warning' }
    );
    await deleteWarehouse(row.id);
    ElMessage.success(t('warehouse.index.messageDeleteSuccess'));
    fetchData();
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') {
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('warehouse.index.messageDeleteFailed')
      );
    }
  }
};

const handleSubmit = async () => {
  if (!formRef.value) return;

  await formRef.value.validate(async valid => {
    if (!valid) return;

    submitLoading.value = true;
    try {
      if (isEdit.value) {
        await updateWarehouse(formData.id!, formData);
        ElMessage.success(t('warehouse.index.messageUpdateSuccess'));
      } else {
        await createWarehouse(formData);
        ElMessage.success(t('warehouse.index.messageCreateSuccess'));
      }
      dialogVisible.value = false;
      fetchData();
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('warehouse.index.messageOperationFailed')
      );
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleExport = async () => {
  // V15 P0-S12 修复（Batch 475c）：导出改用后端带水印 xlsx 接口
  // 调用后端 GET /warehouses/export，传入当前列表筛选条件（status/search），
  // 保证导出数据与列表筛选一致；后端注入水印 + 异步审计日志
  // 注意：后端 WarehouseListQuery 用 search 字段（前端 queryParams.keyword 需映射）
  const params: Record<string, unknown> = {
    status: queryParams.status || undefined,
    search: queryParams.keyword || undefined,
  };
  await exportFromBackend('/warehouses/export', params, 'warehouses_export');
};

const handlePrint = () => {
  printData({
    title: t('warehouse.index.printTitle'),
    resourceType: 'warehouse',
    columns: [
      { key: 'warehouse_code', title: t('warehouse.index.colCode'), width: '100px' },
      { key: 'warehouse_name', title: t('warehouse.index.colName') },
      {
        key: 'warehouse_type',
        title: t('warehouse.index.colType'),
        width: '80px',
        formatter: v => getWarehouseTypeLabel(String(v)),
      },
      { key: 'contact_person', title: t('warehouse.index.colContact'), width: '80px' },
      { key: 'phone', title: t('warehouse.index.colPhone'), width: '120px' },
      {
        key: 'status',
        title: t('warehouse.index.colStatus'),
        width: '60px',
        formatter: v =>
          v === 'active' ? t('warehouse.index.statusActive') : t('warehouse.index.statusInactive'),
      },
    ],
    data: warehouses.value as unknown as Record<string, unknown>[],
  });
};

// 批次 275：useTableApi 构造时自动初始加载，无需 onMounted 调用 fetchData
</script>

<style scoped>
.warehouse-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.header-actions {
  display: flex;
  gap: 12px;
}
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
