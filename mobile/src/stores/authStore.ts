/**
 * 认证状态 store（Zustand）
 *
 * 功能：
 * - 存储当前 token + 用户信息
 * - login() 持久化到 AsyncStorage
 * - logout() 清理 AsyncStorage
 * - hydrate() 启动时从 AsyncStorage 恢复
 *
 * 复用方式：
 * - LoginPage：调用 login()
 * - HomePage：调用 logout()
 * - App.tsx：调用 hydrate()
 */
import { create } from 'zustand';
import AsyncStorage from '@react-native-async-storage/async-storage';

export interface User {
  id: number;
  username: string;
  tenant_id: number;
}

interface AuthState {
  token: string | null;
  user: User | null;
  login: (token: string, user: User) => Promise<void>;
  logout: () => Promise<void>;
  hydrate: () => Promise<void>;
}

const TOKEN_KEY = 'auth_token';
const USER_KEY = 'auth_user';

export const useAuthStore = create<AuthState>((set) => ({
  token: null,
  user: null,

  login: async (token, user) => {
    await AsyncStorage.setItem(TOKEN_KEY, token);
    await AsyncStorage.setItem(USER_KEY, JSON.stringify(user));
    set({ token, user });
  },

  logout: async () => {
    await AsyncStorage.removeItem(TOKEN_KEY);
    await AsyncStorage.removeItem(USER_KEY);
    set({ token: null, user: null });
  },

  hydrate: async () => {
    const token = await AsyncStorage.getItem(TOKEN_KEY);
    const userJson = await AsyncStorage.getItem(USER_KEY);
    if (token && userJson) {
      try {
        const user: User = JSON.parse(userJson);
        set({ token, user });
      } catch (err) {
        console.error('恢复登录状态失败:', err);
      }
    }
  },
}));
