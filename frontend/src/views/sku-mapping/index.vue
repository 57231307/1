<!--
  SKU 对照表维护页
  - 列表展示 + 筛选
  - 新建/编辑对话框（色号 el-select-v2 虚拟滚动）
  - 批量导入
  - 按产品过滤后懒加载其色号
-->
<template>
  <div class="app-container" :aria-label="t('skuMapping.pageAria')">
    <!-- 顶部操作栏 -->
    <div class="page-header">
      <h2>{{ t('skuMapping.title') }}</h2>
      <div class="header-actions">
        <el-button
          v-permission="PERMISSIONS.SKU_MAPPING_CREATE"
          type="primary"
          @click="dialog.openCreate()"
        >
          {{ t('skuMapping.create') }}
        </el-button>
        <el-button
          v-permission="PERMISSIONS.SKU_MAPPING_CREATE"
          @click="importProc.openImportDialog()"
        >
          {{ t('skuMapping.import') }}
        </el-button>
      </div>
    </div>

    <!-- 筛选栏 -->
    <el-form :inline="true" class="filter-form">
      <el-form-item :label="t('skuMapping.filter.product')">
        <el-select
          v-model="list.queryParams.product_id"
          clearable
          filterable
          :placeholder="t('skuMapping.placeholders.product')"
          style="width: 220px"
          @change="list.handleQuery()"
        >
          <el-option
            v-for="p in list.products.value"
            :key="p.id"
            :label="`${p.product_code} - ${p.product_name}`"
            :value="p.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('skuMapping.filter.supplier')">
        <el-select
          v-model="list.queryParams.supplier_id"
          clearable
          filterable
          :placeholder="t('skuMapping.placeholders.supplier')"
          style="width: 200px"
          @change="list.handleQuery()"
        >
          <el-option
            v-for="s in list.suppliers.value"
            :key="s.id"
            :label="s.supplier_name"
            :value="s.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button @click="list.handleReset()">{{ t('common.reset') }}</el-button>
      </el-form-item>
    </el-form>

    <!-- 数据表格 -->
    <el-table
      v-loading="list.loading.value"
      :data="list.list.value"
      border
      stripe
      style="width: 100%"
    >
      <el-table-column
        prop="product_code"
        :label="t('skuMapping.columns.productCode')"
        width="130"
      />
      <el-table-column
        prop="color_no"
        :label="t('skuMapping.columns.ourColorNo')"
        width="120"
      />
      <el-table-column prop="supplier_name" :label="t('skuMapping.columns.supplier')" width="140" />
      <el-table-column
        prop="supplier_product_code"
        :label="t('skuMapping.columns.supplierProductCode')"
        width="150"
      />
      <el-table-column
        prop="supplier_color_no"
        :label="t('skuMapping.columns.supplierColorNo')"
        width="130"
      />
      <el-table-column prop="supplier_price" :label="t('skuMapping.columns.price')" width="100" />
      <el-table-column :label="t('skuMapping.columns.isPrimary')" width="90" align="center">
        <template #default="{ row }">
          <el-tag v-if="row.is_primary" type="success" size="small">{{ t('common.yes') }}</el-tag>
          <el-tag v-else type="info" size="small">{{ t('common.no') }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('skuMapping.columns.isEnabled')" width="90" align="center">
        <template #default="{ row }">
          <el-tag v-if="row.is_enabled" type="success" size="small">{{ t('common.yes') }}</el-tag>
          <el-tag v-else type="danger" size="small">{{ t('common.no') }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.operation')" width="150" fixed="right">
        <template #default="{ row }">
          <el-button
            v-permission="PERMISSIONS.SKU_MAPPING_UPDATE"
            link
            type="primary"
            size="small"
            @click="dialog.openEdit(row)"
          >
            {{ t('common.edit') }}
          </el-button>
          <el-button
            v-permission="PERMISSIONS.SKU_MAPPING_DELETE"
            link
            type="danger"
            size="small"
            @click="confirmDelete(row.id)"
          >
            {{ t('common.delete') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 分页 -->
    <el-pagination
      v-model:current-page="list.queryParams.page"
      v-model:page-size="list.queryParams.page_size"
      :total="list.total.value"
      :page-sizes="[20, 50, 100]"
      layout="total, sizes, prev, pager, next, jumper"
      class="pagination"
      @size-change="list.handleSizeChange"
      @current-change="list.handlePageChange"
    />

    <!-- 新建/编辑对话框 -->
    <SkuMappingFormDialog
      :proc="dialog"
      :products="list.products.value"
      :suppliers="list.suppliers.value"
    />

    <!-- 导入对话框 -->
    <SkuMappingImportDialog :proc="importProc" />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ElMessageBox } from 'element-plus';
import { PERMISSIONS } from '@/constants/permissions';
import { isDialogDismissal } from '@/utils/monitor';
import { useSkuMappingList } from './composables/useSkuMappingList';
import { useSkuMappingDialog } from './composables/useSkuMappingDialog';
import { useSkuMappingImport } from './composables/useSkuMappingImport';
import SkuMappingFormDialog from './components/SkuMappingFormDialog.vue';
import SkuMappingImportDialog from './components/SkuMappingImportDialog.vue';

const { t } = useI18n({ useScope: 'global' });

const list = useSkuMappingList();
// 传 reactive 代理本身（非快照），proc 对象内的 ref 属性保持双向响应
const dialog = useSkuMappingDialog(list.loadData);
const importProc = useSkuMappingImport(list.loadData);

const confirmDelete = async (id: number) => {
  try {
    await ElMessageBox.confirm(t('skuMapping.deleteConfirm'), t('common.confirmTitle'), {
      type: 'warning',
    });
    list.handleDelete(id);
  } catch (e) {
    if (!isDialogDismissal(e)) {
      console.error('[sku-mapping] delete confirm error:', e);
    }
  }
};
</script>

<style scoped>
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 18px;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.filter-form {
  margin-bottom: 16px;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}
</style>
