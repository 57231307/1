/**
 * WebSocketClient - WebSocket 客户端封装
 *
 * P3-2 关键路径 demo：通知模块 WebSocket 客户端
 *
 * 功能：
 * 1. 自动重连（指数退避：1s → 2s → 4s → 8s → 16s → 30s 上限）
 * 2. 心跳（30s ping）
 * 3. 事件分发（EventTarget）
 * 4. 一次性票据鉴权（v12 P1-4：替代 URL query JWT）
 * 5. TypeScript 严格类型
 */

// ==================== 类型定义 ====================

/** WebSocket 消息类型 */
export type WsMessageType =
  | 'notification'
  | 'ping'
  | 'pong'
  | 'error'
  | 'mark_as_read';

/** 通知数据载荷 */
export interface NotificationPayload {
  id: number;
  title: string;
  content: string;
  category: string;
  priority: number;
  created_at: string;
}

/** WebSocket 消息 */
export type WsMessage =
  | { type: 'notification'; data: NotificationPayload }
  | { type: 'ping'; timestamp: number }
  | { type: 'pong'; timestamp: number }
  | { type: 'error'; code: string; message: string }
  | { type: 'mark_as_read'; id: number };

/** WebSocket 事件 Map（类型安全） */
export interface WebSocketEventMap {
  open: CustomEvent<void>;
  close: CustomEvent<void>;
  error: CustomEvent<Event | { message: string }>;
  reconnecting: CustomEvent<{ delay: number; attempt: number }>;
  max_reconnect_failed: CustomEvent<void>;
  notification: CustomEvent<{ type: 'notification'; data: NotificationPayload }>;
  pong: CustomEvent<{ type: 'pong'; timestamp: number }>;
  ping: CustomEvent<{ type: 'ping'; timestamp: number }>;
  ws_error: CustomEvent<{ type: 'error'; code: string; message: string }>;
  mark_as_read: CustomEvent<{ type: 'mark_as_read'; id: number }>;
}

/** 票据获取函数类型 */
export type TicketFetcher = () => Promise<string>;

// ==================== 常量 ====================

/** 心跳间隔（毫秒） */
const HEARTBEAT_INTERVAL = 30000;

/** 最大重连延迟（毫秒） */
const MAX_RECONNECT_DELAY = 30000;

/** 初始重连延迟（毫秒） */
const INITIAL_RECONNECT_DELAY = 1000;

/** 最大重连次数 */
const MAX_RECONNECT_ATTEMPTS = 10;

// ==================== WebSocketClient 类 ====================

/**
 * WebSocket 客户端
 *
 * v12 P1-4 修复：改用一次性短时票据鉴权，不再通过 URL query 传递 JWT。
 * 浏览器 WebSocket API 不支持自定义 header，原方案 `?token=<JWT>` 会导致
 * JWT 泄露到浏览器历史、服务器 access log。新方案在连接前通过 HTTP POST
 * 获取一次性票据（30 秒有效），票据消费后即失效，即使泄露也无法复用。
 *
 * 用法：
 * ```typescript
 * import { fetchWsTicket } from '@/api/notification';
 * const ws = new WebSocketClient('/api/v1/erp/ws/notifications', fetchWsTicket);
 * ws.connect();
 * ws.addEventListener('notification', (event) => {
 *   console.log('收到通知:', event.detail.data);
 * });
 * ```
 */
export class WebSocketClient extends EventTarget {
  private baseUrl: string;
  private ticketFetcher: TicketFetcher;
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private heartbeatTimer: number | null = null;
  private reconnectTimer: number | null = null;
  private isManualClose = false;
  private isConnecting = false;

  /**
   * 构造 WebSocket 客户端
   * @param baseUrl WebSocket 服务端地址（如 /api/v1/erp/ws/notifications）
   * @param ticketFetcher 票据获取函数（返回一次性短时票据字符串）
   */
  constructor(baseUrl: string, ticketFetcher: TicketFetcher) {
    super();
    this.baseUrl = baseUrl;
    this.ticketFetcher = ticketFetcher;
  }

  /**
   * 建立 WebSocket 连接
   *
   * 先调用 ticketFetcher 获取一次性票据，再用票据建立 WebSocket 连接。
   * 票据 30 秒过期，每次连接（含重连）都重新获取新票据。
   */
  async connect(): Promise<void> {
    if (this.isConnecting || (this.ws && this.ws.readyState === WebSocket.OPEN)) {
      return;
    }
    this.isManualClose = false;
    this.isConnecting = true;

    try {
      // v12 P1-4：获取一次性票据
      const ticket = await this.ticketFetcher();
      const url = `${this.baseUrl}?ticket=${encodeURIComponent(ticket)}`;

      this.ws = new WebSocket(url);

      this.ws.onopen = () => {
        this.isConnecting = false;
        this.reconnectAttempts = 0;
        this.startHeartbeat();
        this.dispatchEvent(new CustomEvent('open'));
      };

      this.ws.onmessage = (event: MessageEvent) => {
        try {
          const msg = JSON.parse(event.data) as WsMessage;
          this.handleMessage(msg);
        } catch (err) {
          console.error('WebSocket 消息解析失败:', err);
        }
      };

      this.ws.onerror = (event: Event) => {
        this.isConnecting = false;
        this.dispatchEvent(new CustomEvent('error', { detail: event }));
      };

      this.ws.onclose = () => {
        this.isConnecting = false;
        this.stopHeartbeat();
        this.dispatchEvent(new CustomEvent('close'));
        if (!this.isManualClose) {
          this.scheduleReconnect();
        }
      };
    } catch (err) {
      this.isConnecting = false;
      console.error('WebSocket 连接失败:', err);
      this.scheduleReconnect();
    }
  }

  /**
   * 主动关闭连接（不触发自动重连）
   */
  disconnect(): void {
    this.isManualClose = true;
    this.stopHeartbeat();
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  /**
   * 发送消息
   */
  send(msg: WsMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    } else {
      console.warn('WebSocket 未连接，消息已丢弃:', msg);
    }
  }

  /**
   * 类型安全的事件监听（重写 EventTarget 方法）
   */
  addEventListener<K extends keyof WebSocketEventMap>(
    type: K,
    listener: (event: WebSocketEventMap[K]) => void,
  ): void;
  addEventListener(type: string, listener: EventListenerOrEventListenerObject): void;
  addEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
    super.addEventListener(type, listener);
  }

  // ==================== 私有方法 ====================

  /**
   * 处理收到的消息
   */
  private handleMessage(msg: WsMessage): void {
    switch (msg.type) {
      case 'notification':
        this.dispatchEvent(
          new CustomEvent('notification', { detail: msg }),
        );
        break;
      case 'pong':
        this.dispatchEvent(
          new CustomEvent('pong', { detail: msg }),
        );
        break;
      case 'error':
        this.dispatchEvent(
          new CustomEvent('ws_error', { detail: msg }),
        );
        break;
      case 'ping':
      case 'mark_as_read':
        this.dispatchEvent(
          new CustomEvent(msg.type, { detail: msg }),
        );
        break;
    }
  }

  /**
   * 指数退避重连
   */
  private scheduleReconnect(): void {
    this.reconnectAttempts += 1;
    if (this.reconnectAttempts > MAX_RECONNECT_ATTEMPTS) {
      this.dispatchEvent(new CustomEvent('max_reconnect_failed'));
      return;
    }

    const delay = Math.min(
      INITIAL_RECONNECT_DELAY * Math.pow(2, this.reconnectAttempts - 1),
      MAX_RECONNECT_DELAY,
    );

    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null;
      void this.connect();
    }, delay);

    this.dispatchEvent(
      new CustomEvent('reconnecting', {
        detail: { delay, attempt: this.reconnectAttempts },
      }),
    );
  }

  /**
   * 启动心跳
   */
  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = window.setInterval(() => {
      this.send({ type: 'ping', timestamp: Date.now() });
    }, HEARTBEAT_INTERVAL);
  }

  /**
   * 停止心跳
   */
  private stopHeartbeat(): void {
    if (this.heartbeatTimer !== null) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  // ==================== 状态查询 ====================

  /** 是否已连接 */
  get isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }

  /** 重连尝试次数 */
  get attempts(): number {
    return this.reconnectAttempts;
  }
}

// ==================== 默认导出 ====================

export default WebSocketClient;
