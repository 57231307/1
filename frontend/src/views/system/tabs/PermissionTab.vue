<!--
  PermissionTab.vue - 权限管理 Tab
  来源：原 system/index.vue 中 权限管理 tab 内容
  拆分日期：2026-06-05
  说明：本文件由 system/index.vue 拆分骨架，待前端工程师按 UserTab 模板完成实际数据加载与表单逻辑
  关联 API：listPermissions
-->
<template>
  <div class="permissiontab-tab">
    <div class="page-header">
      <h2 class="page-title">权限管理</h2>
    </div>
    <el-card shadow="hover">
      <el-alert
        title="待迁移"
        type="info"
        :closable="false"
        description="本 Tab 已从 system/index.vue 拆分至此独立子组件。\n下一步：参照 UserTab.vue 实现完整数据加载与表单逻辑。"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
// TODO: 从原 system/index.vue 抽取 权限管理 相关 setup 逻辑
// 模板可参考同目录下 UserTab.vue
</script>
