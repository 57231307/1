<!--
  编辑报价单页
  - 复用 create.vue（已支持编辑模式：检测 route.params.id 并自动加载）
  - 本页是 create.vue 的包装
-->
<template>
  <QuotationCreate />
</template>

<script setup lang="ts">
// 编辑报价单 - 复用 create.vue
// create.vue 在 onMounted 时检测到 route.params.id 会自动进入编辑模式
import QuotationCreate from './create.vue'
</script>
