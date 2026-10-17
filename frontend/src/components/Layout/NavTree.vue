<template>
  <template v-for="node in nodes" :key="node.kind === 'leaf' ? node.path : node.key">
    <el-menu-item v-if="isLeaf(node) && canAccess(node.path)" role="menuitem" :index="node.path">
      <span>{{ t(node.labelKey) }}</span>
    </el-menu-item>
    <el-sub-menu
      v-else-if="!isLeaf(node) && groupVisible(node)"
      :index="node.key"
      role="menuitem"
      aria-haspopup="true"
      :aria-expanded="isOpen ? isOpen(node.key) : false"
    >
      <template #title>
        <el-icon v-if="node.icon"><component :is="icons[node.icon]" /></el-icon>
        <span>{{ t(node.labelKey) }}</span>
      </template>
      <NavTree :nodes="node.children" :can-access="canAccess" :is-open="isOpen" />
    </el-sub-menu>
  </template>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ElIcon, ElMenuItem, ElSubMenu } from 'element-plus';
import {
  Box,
  CircleCheck,
  Cpu,
  MagicStick,
  Money,
  Sell,
  Setting,
  ShoppingCart,
} from '@element-plus/icons-vue';
import type { NavGroup, NavLeaf, NavNode } from '@/config/navigation';

/**
 * 递归渲染 `config/navigation.ts` 的导航树，深度不限（一级域→二级段→三级页面）。
 * 调用方：`MainLayout.vue`。入参：nodes 子树、canAccess 单页可见判据（路由 meta.permission）、
 * isOpen 分组展开态查询（供 aria-expanded）。不自行推断权限，可见性一律回落路由元。
 */
const props = defineProps<{
  nodes: NavNode[];
  canAccess: (path: string) => boolean;
  isOpen?: (key: string) => boolean;
}>();

const { t } = useI18n({ useScope: 'global' });

const icons: Record<string, unknown> = {
  Box,
  CircleCheck,
  Cpu,
  MagicStick,
  Money,
  Sell,
  Setting,
  ShoppingCart,
};

const isLeaf = (node: NavNode): node is NavLeaf => node.kind === 'leaf';

const accessibleLeaves = (node: NavGroup): number =>
  node.children.reduce(
    (acc, child) =>
      acc + (isLeaf(child) ? (props.canAccess(child.path) ? 1 : 0) : accessibleLeaves(child)),
    0
  );

const groupVisible = (node: NavGroup): boolean => accessibleLeaves(node) > 0;
</script>
