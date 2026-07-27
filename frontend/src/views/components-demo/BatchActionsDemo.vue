<template>
  <div class="batch-actions-demo">
    <el-card>
      <template #header>{{ t('componentsDemo.batchActions.title') }}</template>

      <BatchActions
        :selected-rows="selectedRows"
        @clear="selectedRows = []"
        @complete="handleComplete"
      />

      <el-table
        :data="tableData"
        row-key="id"
        border
        :aria-label="t('componentsDemo.batchActions.ariaLabel')"
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" />
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="name" :label="t('componentsDemo.batchActions.colName')" />
        <el-table-column
          prop="status"
          :label="t('componentsDemo.batchActions.colStatus')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 'pending' ? 'warning' : 'success'">
              {{
                row.status === 'pending'
                  ? t('componentsDemo.batchActions.statusPending')
                  : t('componentsDemo.batchActions.statusApproved')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="date"
          :label="t('componentsDemo.batchActions.colDate')"
          width="150"
        />
      </el-table>

      <el-alert
        v-if="selectedRows.length === 0"
        :title="t('componentsDemo.batchActions.alertSelectRows')"
        type="info"
        :closable="false"
        show-icon
        style="margin-top: 16px"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BatchActions from '@/components/BatchActions.vue';

const { t } = useI18n({ useScope: 'global' });

// Demo 演示用的订单行数据结构
interface DemoOrder {
  id: number;
  name: string;
  status: string;
  date: string;
}

const selectedRows = ref<DemoOrder[]>([]);

const buildOrderName = (id: number): string =>
  t('componentsDemo.batchActions.orderName', { id: String(id) });

const tableData = ref<DemoOrder[]>([
  { id: 1, name: buildOrderName(1001), status: 'pending', date: '2026-01-15' },
  { id: 2, name: buildOrderName(1002), status: 'pending', date: '2026-01-16' },
  { id: 3, name: buildOrderName(1003), status: 'approved', date: '2026-01-17' },
  { id: 4, name: buildOrderName(1004), status: 'pending', date: '2026-01-18' },
  { id: 5, name: buildOrderName(1005), status: 'pending', date: '2026-01-19' },
  { id: 6, name: buildOrderName(1006), status: 'approved', date: '2026-01-20' },
]);

const handleSelectionChange = (selection: DemoOrder[]) => {
  selectedRows.value = selection;
};

const handleComplete = (_key: string, success: boolean) => {
  if (success) {
    // 操作完成处理
  }
};
</script>

<style scoped>
.batch-actions-demo {
  padding: 10px;
}
</style>
