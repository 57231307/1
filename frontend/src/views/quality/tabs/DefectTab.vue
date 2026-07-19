<!--
  DefectTab.vue - 缺陷管理 Tab
  来源：原 quality/index.vue 中 缺陷管理 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="defect-tab">
    <div class="page-header">
      <h2 class="page-title">质量缺陷管理</h2>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="defects" stripe aria-label="质量缺陷列表">
        <el-table-column prop="defect_type" label="缺陷类型" width="140" />
        <el-table-column prop="defect_description" label="缺陷描述" min-width="200" />
        <el-table-column prop="severity" label="严重程度" width="100" align="center">
          <template #default="{ row }">
            <el-tag
              :type="
                row.severity === 'critical'
                  ? 'danger'
                  : row.severity === 'major'
                    ? 'warning'
                    : 'info'
              "
              size="small"
            >
              {{
                row.severity === 'critical' ? '严重' : row.severity === 'major' ? '重大' : '轻微'
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="quantity" label="数量" width="80" align="right" />
        <el-table-column prop="processed" label="是否处理" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.processed ? 'success' : 'info'" size="small">
              {{ row.processed ? '已处理' : '未处理' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="!row.processed"
              type="primary"
              link
              size="small"
              @click="processDefect(row)"
              >处理</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { processDefect as processDefectApi, type Defect } from '@/api/quality'
import { logger } from '@/utils/logger'

const defects = ref<Defect[]>([])
const loading = ref(false)

const fetchDefects = async () => {
  loading.value = true
  try {
    const { listDefects } = await import('@/api/quality')
    const res = await listDefects()
    defects.value = (res.data as Defect[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取缺陷列表失败', err.message)
  } finally {
    loading.value = false
  }
}

const processDefect = async (row: Defect) => {
  try {
    const { value } = await ElMessageBox.prompt('请输入处理备注', '处理缺陷')
    await processDefectApi(row.id, { remark: value })
    ElMessage.success('处理成功')
    fetchDefects()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

onMounted(() => {
  fetchDefects()
})

defineExpose({ fetchDefects })
</script>
