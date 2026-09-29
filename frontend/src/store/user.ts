import { defineStore } from 'pinia';
import { ref } from 'vue';
import { login as loginApi, logout as logoutApi, getUserInfo } from '@/api/auth';
import type { UserInfo, LoginRequest } from '@/types/api';

// 权限码 localStorage 缓存：整页刷新时优先用缓存权限渲染首屏，完整用户信息异步补全
const PERMS_CACHE_KEY = 'erp_cached_perms';
const PERMS_CACHE_TTL_KEY = 'erp_cached_perms_ts';
const PERMS_CACHE_TTL_MS = 5 * 60 * 1000; // 5 分钟有效期

function readCachedPerms(): readonly string[] | null {
  try {
    const ts = localStorage.getItem(PERMS_CACHE_TTL_KEY);
    if (ts && Date.now() - Number(ts) > PERMS_CACHE_TTL_MS) return null;
    const raw = localStorage.getItem(PERMS_CACHE_KEY);
    return raw ? (Object.freeze(JSON.parse(raw)) as readonly string[]) : null;
  } catch {
    return null;
  }
}

function writeCachedPerms(perms: readonly string[]): void {
  try {
    localStorage.setItem(PERMS_CACHE_KEY, JSON.stringify(perms));
    localStorage.setItem(PERMS_CACHE_TTL_KEY, String(Date.now()));
  } catch {
    /* quota exceeded 等情况静默失败 */
  }
}

function clearCachedPerms(): void {
  localStorage.removeItem(PERMS_CACHE_KEY);
  localStorage.removeItem(PERMS_CACHE_TTL_KEY);
}

export const useUserStore = defineStore('user', () => {
  const token = ref<string | null>(null);
  // 首屏优化：从 localStorage 恢复权限码，用于快速渲染菜单/按钮门控，避免每次刷新都等 API。
  // 缓存仅含权限、不含 id/username 等完整字段：用 id=0、username='' 作"不完整"哨兵，
  // 由路由守卫识别（userInfo 缺有效 id）后异步补全 fetchUserInfo，二者不冲突。
  const _cachedPerms = readCachedPerms();
  const userInfo = ref<UserInfo | null>(
    _cachedPerms ? { id: 0, username: '', permissions: _cachedPerms } : null
  );

  async function login(loginData: LoginRequest) {
    // loginApi 已在 api 层解包 ApiResponse 信封，返回业务数据 LoginResponse
    const res = await loginApi(loginData);
    // Wave B-3：access_token / refresh_token 由后端写入 httpOnly Cookie，前端不再持有 token
    // 后端 LoginResponse 顶层 permissions 优先于 user.permissions（后端按角色聚合的权威权限码）
    // Object.freeze 防止前端组件运行时篡改权限码数组（如 push 注入 admin:write）
    const perms = res.permissions || res.user?.permissions || [];
    const frozenPerms = Object.freeze([...perms]) as readonly string[];
    userInfo.value = {
      ...(res.user || {}),
      permissions: frozenPerms,
    };
    writeCachedPerms(frozenPerms);
    return res;
  }

  async function logout() {
    try {
      await logoutApi();
    } finally {
      // 后端通过 Set-Cookie + max-age=0 自动清除所有登录态 Cookie
      token.value = null;
      userInfo.value = null;
      clearCachedPerms();
    }
  }

  async function fetchUserInfo() {
    // getUserInfo 已在 api 层解包 ApiResponse 信封，返回业务数据 UserInfo
    const info = await getUserInfo();
    // permissions 用 Object.freeze 做运行时深度防御，防止组件篡改权限码数组。
    // permissions 为 readonly 属性，通过解构创建新对象赋值，避免直接赋值类型错误。
    if (info && info.permissions) {
      const frozenPerms = Object.freeze([...info.permissions]) as readonly string[];
      userInfo.value = {
        ...info,
        permissions: frozenPerms,
      };
      writeCachedPerms(frozenPerms);
    } else {
      userInfo.value = info;
    }
    return info;
  }

  function setUserInfo(info: UserInfo) {
    userInfo.value = info;
  }

  return { token, userInfo, login, logout, fetchUserInfo, setUserInfo };
});
