<template>
  <el-container class="main-layout" :aria-label="t('layout.main.pageAriaLabel')">
    <!-- V15 P1-20-2 响应式侧边栏：桌面 el-aside 固定 / 移动 el-drawer 抽屉化 -->
    <component
      :is="isMobile ? ElDrawer : ElAside"
      v-bind="sidebarAttrs"
      :class="isMobile ? 'mobile-sidebar' : 'aside'"
    >
      <div class="logo">
        <img class="logo-mark" :src="brandLogo" alt="" />
        <h2>{{ brandName }}</h2>
      </div>
      <el-menu
        :default-active="activeMenu"
        class="menu"
        background-color="#304156"
        text-color="#bfcbd9"
        active-text-color="#409eff"
        router
        role="menubar"
        :aria-label="t('layout.menuAriaLabel')"
        @open="handleMenuOpen"
        @close="handleMenuClose"
      >
        <NavTree :nodes="navTree" :can-access="canAccessMenu" :is-open="isMenuOpen" />
      </el-menu>
    </component>

    <el-container>
      <el-header class="header">
        <div class="header-left">
          <!-- V15 P1-20-2 移动端汉堡按钮（触屏尺寸 ≥ 44px，WCAG 2.5.5） -->
          <el-button
            v-if="isMobile"
            class="hamburger-btn"
            size="large"
            :aria-label="t('layout.main.toggleSidebarAriaLabel')"
            :title="t('layout.main.toggleSidebar')"
            text
            @click="openDrawer"
          >
            <el-icon :size="22"><Expand /></el-icon>
          </el-button>
          <el-breadcrumb separator="/" :aria-label="t('layout.main.breadcrumbAriaLabel')">
            <el-breadcrumb-item :to="{ path: '/' }">{{
              t('layout.breadcrumb.home')
            }}</el-breadcrumb-item>
            <el-breadcrumb-item>{{ currentTitle }}</el-breadcrumb-item>
          </el-breadcrumb>
          <!-- 全局搜索：真实接入后端 /search 端点（批次 104 SearchClient）；移动端隐藏防 header 溢出 -->
          <GlobalSearch v-if="!isMobile" class="header-search" />
        </div>
        <div class="header-right">
          <!-- V15 P1-20-16 暗黑模式切换按钮（触屏尺寸 ≥ 44px，WCAG 2.5.5） -->
          <!-- 通知实时推送：未读数徽标（WS 断连时自动回退轮询间隔由 unread poller 承担） -->
          <el-tooltip :content="t('layout.main.notifications')" placement="bottom">
            <el-badge
              :value="unreadCount"
              :hidden="unreadCount === 0"
              :max="99"
              class="notification-badge"
            >
              <el-button
                size="large"
                text
                :aria-label="t('layout.main.notifications')"
                :title="t('layout.main.notifications')"
                @click="$router.push('/notification')"
              >
                <el-icon :size="18"><Bell /></el-icon>
              </el-button>
            </el-badge>
          </el-tooltip>
          <el-button
            class="theme-toggle-btn"
            size="large"
            text
            :aria-label="isDark ? t('layout.main.lightMode') : t('layout.main.darkMode')"
            :title="isDark ? t('layout.main.lightMode') : t('layout.main.darkMode')"
            @click="toggleTheme"
          >
            <el-icon :size="18">
              <Sunny v-if="isDark" />
              <Moon v-else />
            </el-icon>
          </el-button>
          <el-dropdown :aria-label="t('layout.main.userMenuAriaLabel')">
            <span class="user-info" role="button" tabindex="0">
              {{ userStore.userInfo?.username || t('layout.user.defaultName') }}
              <el-icon><ArrowDown /></el-icon>
            </span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="$router.push('/system/profile')">{{
                  t('layout.user.profile')
                }}</el-dropdown-item>
                <el-dropdown-item divided @click="handleLogout">{{
                  t('layout.user.logout')
                }}</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <el-main class="main-content">
        <router-view v-slot="{ Component, route }">
          <ErrorBoundary :key="route.path">
            <keep-alive :include="cachedViewNames">
              <component :is="Component" :key="route.path" />
            </keep-alive>
          </ErrorBoundary>
        </router-view>
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import {
  onNewNotification,
  startNotificationWs,
  stopNotificationWs,
} from '@/composables/useNotificationWs';
import { getUnreadCount } from '@/api/notification';
import { logger } from '@/utils/logger';
import { ref as vueRef, onBeforeUnmount } from 'vue';
import { Bell } from '@element-plus/icons-vue';

import GlobalSearch from './GlobalSearch.vue';
import NavTree from './NavTree.vue';
import { NAV_TREE } from '@/config/navigation';
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
// V15 P1-20-2 响应式侧边栏：动态组件切换需要显式导入 ElDrawer/ElAside
import { ElDrawer, ElAside } from 'element-plus';
import { ArrowDown, Expand, Sunny, Moon } from '@element-plus/icons-vue';
import { useUserStore } from '@/store/user';
import ErrorBoundary from '@/components/ErrorBoundary.vue';
// 批次 6 修复（2026-06-28）：MainLayout 菜单按 permission 过滤（审计 #8 完整修复）
// 复用 router 守卫同款宽松匹配函数，保证菜单可见性与路由可达性一致。
import { hasRoutePermission } from '@/router';
// V15 P1-20-2 响应式断点 composable（md 以下视为移动端，侧边栏抽屉化）
import { useBreakpoint } from '@/composables/useBreakpoint';
// V15 P1-20-16 暗黑模式切换 composable（持久化用户偏好到 localStorage）
import { useTheme } from '@/composables/useTheme';
// V15 P1-20-8 键盘导航焦点管理（Tab/Shift+Tab/Esc 焦点陷阱 + 路由切换后重置焦点）
import { useKeyboardNavigation } from '@/composables/useKeyboardNavigation';
// 公司档案服务端化：头部品牌名读共享档案（空/未加载回退 i18n 字面量）
import { useCompanyProfile } from '@/composables/useCompanyProfile';
// 品牌图形资产（已抠掉白底的透明版，深色侧栏上不会带出白方块）
import brandLogo from '@/assets/logo.png';

const { t } = useI18n({ useScope: 'global' });
const route = useRoute();
const router = useRouter();
const userStore = useUserStore();

// V15 P1-20-2 响应式断点：md 以下（< 992px）视为移动端，侧边栏改用 el-drawer 抽屉化
const { isMobile } = useBreakpoint();
// V15 P1-20-16 暗黑模式：isDark 派生当前主题，toggleTheme 切换亮/暗
const { isDark, toggleTheme } = useTheme();
// V15 P1-20-8 键盘导航：路由切换后重置焦点到主内容区，模态框内 Tab 焦点陷阱
useKeyboardNavigation();
// 公司档案服务端化：读取模块级共享的对外硬投影，头部品牌优先显示公司名。
const { company, load: loadCompanyProfile } = useCompanyProfile();
// 展示层回退（非兜底掩盖）：档案为空或未加载时显示 i18n 品牌字面量；
// 档案加载失败由 composable 显式 logger.error 留痕，此处仅按 null 走展示回退。
const brandName = computed(() => company.value?.name || t('layout.brand'));
// 移动端抽屉可见性状态
const drawerVisible = ref(false);

const activeMenu = computed(() => route.path);
const currentTitle = computed(() => (route.meta.title as string) || '');

// V15 P1-20-14 keep-alive 状态保留：缓存高频切换页面（列表页 + 工作台）
// 仅缓存需要保留状态（搜索条件/分页/滚动位置）的页面，详情/编辑页不缓存
const cachedViewNames = computed<string[]>(() => [
  'Dashboard',
  'InventoryList',
  'SalesList',
  'PurchaseList',
  'CustomerList',
  'SupplierList',
  'FinanceOverview',
]);

// 批次 6：用户权限与角色响应式派生
// 批次 22 v5 P0-5：permissions 为 readonly string[]，computed 类型同步
const userPermissions = computed<readonly string[]>(() => userStore.userInfo?.permissions || []);
// P2 1-12 修复：删除 isAdmin computed（role_name === 'admin' 硬编码），统一走 *:* 通配权限

// P0 4-3 修复：维护子菜单展开状态供 aria-expanded 使用（WCAG 无障碍）
const openedMenus = ref<string[]>([]);
const handleMenuOpen = (index: string) => {
  if (!openedMenus.value.includes(index)) {
    openedMenus.value.push(index);
  }
};
const handleMenuClose = (index: string) => {
  openedMenus.value = openedMenus.value.filter(i => i !== index);
};

// 导航树可见性：单项取路由 meta.permission，分组取子树内任一可达单项；不再维护第二份清单
const navTree = NAV_TREE;
const isMenuOpen = (key: string) => openedMenus.value.includes(key);

/**
 * V15 P1-20-2 侧边栏动态属性
 *
 * 桌面端（md 及以上）：使用 ElAside 固定侧边栏，width=220px
 * 移动端（md 以下）：使用 ElDrawer 抽屉化，direction=ltr，size=260px，
 *   withHeader=false（logo 在内容中渲染保持品牌一致），modal=true（点击遮罩关闭），
 *   appendToBody=true（避免 z-index 层级问题）
 */
const sidebarAttrs = computed<Record<string, unknown>>(() => {
  if (isMobile.value) {
    return {
      modelValue: drawerVisible.value,
      'onUpdate:modelValue': (v: boolean) => {
        drawerVisible.value = v;
      },
      title: t('layout.main.sidebarDrawerTitle'),
      direction: 'ltr' as const,
      size: '260px',
      withHeader: false,
      modal: true,
      appendToBody: true,
      destroyOnClose: false,
      closeOnPressEscape: true,
    };
  }
  return {
    width: '220px',
  };
});

// V15 P1-20-2 打开移动端抽屉
const openDrawer = () => {
  drawerVisible.value = true;
};

// V15 P1-20-2 路由切换时关闭移动端抽屉（点击菜单项导航后自动收起）
watch(
  () => route.path,
  () => {
    if (isMobile.value && drawerVisible.value) {
      drawerVisible.value = false;
    }
  }
);

// V15 P1-20-2 切换到桌面端时关闭抽屉（避免从移动端放大后抽屉残留）
watch(isMobile, mobile => {
  if (!mobile && drawerVisible.value) {
    drawerVisible.value = false;
  }
});

/**
 * 批次 6（2026-06-28）：菜单项可见性判定
 *
 * 批次 22 v5 P0-7 修复：与守卫 P0-6 严格化保持一致，移除"空权限放行"。
 * - admin 角色直接通过
 * - 路由 meta.permission 不存在 → 放行（菜单 path 未配置 permission）
 * - 通过 hasRoutePermission 匹配（支持通配符、read/view 等价）
 * - 空权限码用户不再放行，与 router.beforeEach 守卫行为一致
 *
 * @param menuItemPath 菜单项 index（即路由 path，如 '/inventory'）
 * @returns 是否在菜单中显示
 */

// ===== 通知实时推送（WS）与未读数 =====
const unreadCount = vueRef(0);

async function refreshUnread() {
  try {
    const res = await getUnreadCount();
    const data = res as unknown as { data?: number };
    unreadCount.value = Number(data?.data ?? 0);
  } catch (error) {
    logger.error(t('layout.main.unreadCountFailed'), error);
  }
}

onNewNotification(() => {
  refreshUnread();
});

if (userStore.userInfo) {
  refreshUnread();
  startNotificationWs();
}

onMounted(() => {
  // MainLayout 挂载于已鉴权外壳内，此时登录态成立：拉取共享公司档案一次并缓存。
  void loadCompanyProfile();
});

onBeforeUnmount(() => {
  stopNotificationWs();
});

function canAccessMenu(menuItemPath: string): boolean {
  // 通过 router.resolve 找到匹配的叶子路由 record
  const resolved = router.resolve(menuItemPath);
  const leafRecord = resolved.matched[resolved.matched.length - 1];
  // P1 4-1 修复（批次 64）：路由不存在 → 保守隐藏（return false）
  // 原实现 return true，菜单 path 配置错误或路由未注册时放行，菜单可见性泄露
  if (!leafRecord) return false;
  // P0 4-2 修复：hidden 路由不在菜单显示（详情/编辑/创建等子页面）
  // 必须在 admin 判断之前，否则 admin 仍会看到 hidden 路由
  if (leafRecord.meta?.hidden) return false;
  // 以下保持原权限校验逻辑
  // P2 1-12 修复：删除 isAdmin 硬编码绕过，统一走 hasRoutePermission
  // 后端为 system 角色注入 *:* 通配权限，hasRoutePermission 自动放行
  const required = leafRecord.meta?.permission as string | string[] | undefined;
  return hasRoutePermission(required, userPermissions.value);
}

async function handleLogout() {
  await userStore.logout();
  router.push('/login');
}
</script>

<style scoped>
/* V15 P1-20-15 使用 CSS 变量替代硬编码颜色，支持主题切换与暗黑模式 */
.main-layout {
  --layout-aside-bg: #304156;
  --layout-aside-logo-bg: #263445;
  --layout-header-bg: #ffffff;
  --layout-content-bg: #f0f2f5;
  --layout-header-shadow: rgba(0, 21, 41, 0.08);
  --layout-text-light: #ffffff;
  --layout-shadow-blur: 4px;
  /* dvh 跟随可视区收展动态取值，沉浸式布局下 vh 会把底部顶出屏幕，vh 行在前，
     留给不认识 dvh 的旧内核兜底，安全区变量的口径见 theme.css */
  height: 100vh;
  height: 100dvh;
}
.aside {
  background-color: var(--layout-aside-bg);
  /* 侧栏滚到底的最后一项要避开系统导航条 */
  padding-bottom: var(--bx-safe-bottom);
}
.logo {
  /* 高度与安全区相加而不是覆盖，沉浸布局下 logo 文字才不被状态栏压住 */
  height: calc(60px + var(--bx-safe-top));
  padding-top: var(--bx-safe-top);
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--layout-aside-logo-bg);
}
.logo h2 {
  color: var(--layout-text-light);
  font-size: 18px;
  margin: 0;
}
.logo-mark {
  /* 图形按行高取方，flex:none 保证窄屏文字省略时不被压扁 */
  height: 34px;
  width: 34px;
  flex: none;
  margin-right: 8px;
}
.menu {
  border-right: none;
}
.header {
  /* 顶栏总高为内容高 60 像素加安全区，避让量落在 padding 上，
     沉浸布局下面包屑与按钮整体下移到状态栏之下仍可点，桌面取 0 无变化 */
  --el-header-height: calc(60px + var(--bx-safe-top));
  padding-top: var(--bx-safe-top);
  padding-left: calc(20px + var(--bx-safe-left));
  padding-right: calc(20px + var(--bx-safe-right));
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--layout-header-bg);
  box-shadow: 0 1px var(--layout-shadow-blur) var(--layout-header-shadow);
}
.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.header-right .user-info {
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
}
.main-content {
  background: var(--layout-content-bg);
  padding: 20px;
  /* 底部与左右避让系统条带与切口，中间内容滚动到底不被导航条盖住 */
  padding-left: calc(20px + var(--bx-safe-left));
  padding-right: calc(20px + var(--bx-safe-right));
  padding-bottom: calc(20px + var(--bx-safe-bottom));
}

/* V15 P1-20-2 移动端汉堡按钮：触屏目标尺寸 ≥ 44x44px（WCAG 2.5.5 Target Size） */
.hamburger-btn {
  min-height: 44px;
  min-width: 44px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* V15 P1-20-2 移动端抽屉内部样式：与桌面 aside 背景色保持一致 */
/* 抽屉是铺满整屏的固定层，左右与底部要避让切口与导航条，顶部避让由抽屉内 logo 条自带 */
.mobile-sidebar :deep(.el-drawer__body) {
  padding: 0 var(--bx-safe-left) var(--bx-safe-bottom) var(--bx-safe-right);
  background-color: var(--layout-aside-bg);
  overflow-y: auto;
}

/* V15 P1-20-2 移动端响应式适配（max-width: 991px，与 md 断点一致） */
@media (max-width: 991px) {
  /* 窄屏顶栏降档，52 像素内容高加安全区，触控目标仍靠下方 44 像素规则保证 */
  .header {
    --el-header-height: calc(52px + var(--bx-safe-top));
    padding-left: calc(8px + var(--bx-safe-left));
    padding-right: calc(8px + var(--bx-safe-right));
  }

  /* 窄屏 logo 条同步降档，品牌字从 18 像素收到 15 像素，与 14 像素面包屑保持一档层级差 */
  .logo {
    height: calc(52px + var(--bx-safe-top));
  }
  .logo h2 {
    font-size: 15px;
  }
  .logo-mark {
    height: 26px;
    width: 26px;
    margin-right: 6px;
  }

  /* 移动端主内容区减小内边距，释放更多内容空间，底部与左右仍要加安全区 */
  .main-content {
    padding: 12px calc(12px + var(--bx-safe-right)) calc(12px + var(--bx-safe-bottom))
      calc(12px + var(--bx-safe-left));
  }

  /* 移动端头部用户菜单触屏尺寸 ≥ 44px */
  .header-right .user-info {
    min-height: 44px;
    padding: 8px 12px;
  }

  /* 移动端面包屑简化（减少水平占用） */
  .header-left {
    gap: 4px;
  }
}

/* V15 P1-20-2 触屏按钮全局最小尺寸（移动端所有 el-button 至少 44x44px） */
@media (max-width: 991px) {
  .header :deep(.el-button) {
    min-height: 44px;
    min-width: 44px;
  }
}

/* V15 P1-20-16 暗黑模式切换按钮：触屏目标尺寸 ≥ 44x44px */
.theme-toggle-btn {
  min-height: 44px;
  min-width: 44px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
</style>
