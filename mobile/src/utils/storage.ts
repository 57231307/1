/**
 * AsyncStorage 工具封装
 *
 * 简化版 KV 存储（仅用于 demo）
 * 生产环境建议：
 * - 敏感信息用 react-native-keychain
 * - 大数据用 SQLite（react-native-sqlite-storage）
 */
import AsyncStorage from '@react-native-async-storage/async-storage';

export const Storage = {
  /** 读取字符串 */
  async getString(key: string): Promise<string | null> {
    return AsyncStorage.getItem(key);
  },

  /** 写入字符串 */
  async setString(key: string, value: string): Promise<void> {
    return AsyncStorage.setItem(key, value);
  },

  /** 读取 JSON */
  async getJson<T = unknown>(key: string): Promise<T | null> {
    const json = await AsyncStorage.getItem(key);
    if (!json) return null;
    try {
      return JSON.parse(json) as T;
    } catch {
      return null;
    }
  },

  /** 写入 JSON */
  async setJson<T>(key: string, value: T): Promise<void> {
    return AsyncStorage.setItem(key, JSON.stringify(value));
  },

  /** 删除键 */
  async remove(key: string): Promise<void> {
    return AsyncStorage.removeItem(key);
  },

  /** 清空（慎用） */
  async clear(): Promise<void> {
    return AsyncStorage.clear();
  },
};
