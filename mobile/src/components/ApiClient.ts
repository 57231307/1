/**
 * ApiClient - API 客户端（关键路径 demo）
 *
 * 功能：
 * 1. 统一 axios 实例 + 拦截器
 * 2. JWT token 自动注入（从 AsyncStorage 读取）
 * 3. 错误统一处理（业务错误 + 网络错误）
 * 4. 业务模块按 namespace 组织（auth / inventory）
 *
 * 与主项目 backend/ 的 REST API 兼容：
 * - 基础 URL：https://api.bingxi-erp.com/api/v1/erp
 * - 鉴权：Bearer JWT
 * - 响应：{ code, message, data }
 */
import axios, { AxiosInstance, AxiosError, AxiosResponse } from 'axios';
import AsyncStorage from '@react-native-async-storage/async-storage';

const API_BASE_URL =
  (typeof process !== 'undefined' && process.env?.EXPO_PUBLIC_API_BASE_URL) ||
  'https://api.bingxi-erp.com/api/v1/erp';

/** 业务响应统一格式 */
export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
}

/** 登录请求 */
export interface LoginRequest {
  username: string;
  password: string;
}

/** 登录响应 */
export interface LoginResponse {
  token: string;
  user: {
    id: number;
    username: string;
    tenant_id: number;
  };
}

class ApiClientImpl {
  private axios: AxiosInstance;

  constructor(baseURL: string) {
    this.axios = axios.create({ baseURL, timeout: 10000 });
    this.setupInterceptors();
  }

  private setupInterceptors(): void {
    // 请求拦截器：自动注入 JWT
    this.axios.interceptors.request.use(async (config) => {
      const token = await AsyncStorage.getItem('token');
      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    });

    // 响应拦截器：统一错误处理
    this.axios.interceptors.response.use(
      (response: AxiosResponse<ApiResponse>) => response.data,
      (error: AxiosError<ApiResponse>) => Promise.reject(this.normalizeError(error)),
    );
  }

  /** 认证模块 */
  auth = {
    /** 登录 */
    login: (data: LoginRequest): Promise<LoginResponse> =>
      this.axios
        .post<ApiResponse<LoginResponse>, AxiosResponse<ApiResponse<LoginResponse>>>(
          '/auth/login',
          data,
        )
        .then((res) => res.data.data as LoginResponse),

    /** 登出 */
    logout: (): Promise<void> =>
      this.axios
        .post<ApiResponse<null>>('/auth/logout')
        .then(() => undefined),
  };

  /** 库存模块（示例） */
  inventory = {
    /** 库存列表 */
    list: (params: { page?: number; size?: number } = {}): Promise<unknown> =>
      this.axios
        .get<ApiResponse<unknown>>('/inventory', { params })
        .then((res) => res.data.data),
  };

  private normalizeError(error: AxiosError<ApiResponse>): Error {
    if (error.response) {
      const data = error.response.data;
      if (data?.message) {
        return new Error(data.message);
      }
      return new Error(`请求失败 (${error.response.status})`);
    }
    if (error.request) {
      return new Error('网络连接失败，请检查网络');
    }
    return new Error(error.message || '未知错误');
  }
}

export const ApiClient = new ApiClientImpl(API_BASE_URL);
export default ApiClient;
