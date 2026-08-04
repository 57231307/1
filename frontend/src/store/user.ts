import { defineStore } from 'pinia';
import { ref } from 'vue';
import { login as loginApi, logout as logoutApi, getUserInfo } from '@/api/auth';
import type { UserInfo, LoginRequest } from '@/types/api';

// V15 P2 20.11-D：权限码 localStorage 缓存，减少页面刷新时的 API 调用
const PERMS_CACHE_KEY = 'erp_cached_perms';
const PERMS_CACHE_TTL_KEY = 'erp_cached_perms_ts';
const PERMS_CACHE_TTL_MS = 5 * 60 * 1000; // 5 分钟有效期

function readCachedPerms(): readonly string[] | null {
  try {
    const ts = localStorage.getItem(PERMS_CACHE_TTL_KEY);
    if (ts && Date.now() - Number(ts) > PERMS_CACHE_TTL_MS) return null;
    const raw = localStorage.getItem(PERMS_CACHE_KEY);
    return raw ? Object.freeze(JSON.parse(raw)) as readonly string[] : null;
  } catch { return null; }
}

function writeCachedPerms(perms: readonly string[]): void {
  try {
    localStorage.setItem(PERMS_CACHE_KEY, JSON.stringify(perms));
    localStorage.setItem(PERMS_CACHE_TTL_KEY, String(Date.now()));
  } catch { /* quota exceeded 等情况静默失败 */ }
}

function clearCachedPerms(): void {
  localStorage.removeItem(PERMS_CACHE_KEY);
  localStorage.removeItem(PERMS_CACHE_TTL_KEY);
}

export const useUserStore = defineStore('user', () => {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const token = ref<string | null>(null);
  // 20.11-D：初始化时尝试从 localStorage 恢复权限，避免每次刷新都调 API
  const _cachedPerms = readCachedPerms();
  const userInfo = ref<UserInfo | null>(_cachedPerms ? { permissions: _cachedPerms } as UserInfo : null);

  async function login(loginData: LoginRequest) {
    const res = await loginApi(loginData);
    // Wave B-3：access_token / refresh_token 由后端写入 httpOnly Cookie，前端不再持有 token
    // FE-P-2/FE-P-3 修复：后端 LoginResponse 顶层 permissions 优先于 user.permissions
    // 批次 22 v5 P0-5：Object.freeze 防止前端组件恶意修改权限码数组
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
    const info = await getUserInfo();
    // 批次 22 v5 P0-5 修复：对 permissions 字段添加 Object.freeze 运行时保护，
    // 防止前端组件恶意修改权限码数组（如 push 注入 admin:write）。
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
