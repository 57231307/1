/**
 * WebSocket 客户端（复用 P3-2 设计）
 *
 * 用途：移动端接收服务端实时通知推送
 * 与 frontend/src/utils/websocket.ts 设计一致
 */

import type { NotificationPayload } from '../types/api';

export type WsMessageType = 'notification' | 'ping' | 'pong' | 'error' | 'mark_as_read';

export type WsMessage =
  | { type: 'notification'; data: NotificationPayload }
  | { type: 'ping'; timestamp: number }
  | { type: 'pong'; timestamp: number }
  | { type: 'error'; code: string; message: string }
  | { type: 'mark_as_read'; id: number };

const HEARTBEAT_INTERVAL = 30000;
const MAX_RECONNECT_DELAY = 30000;
const INITIAL_RECONNECT_DELAY = 1000;
const MAX_RECONNECT_ATTEMPTS = 10;

export class WebSocketClient {
  private url: string;
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private isManualClose = false;
  private listeners: Map<WsMessageType, Set<(data: any) => void>> = new Map();

  constructor(baseUrl: string, token: string) {
    this.url = `${baseUrl}?token=${encodeURIComponent(token)}`;
  }

  connect(): void {
    this.isManualClose = false;
    try {
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        this.reconnectAttempts = 0;
        this.startHeartbeat();
      };

      this.ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(String(event.data)) as WsMessage;
          this.dispatch(msg);
        } catch (err) {
          console.error('WebSocket 消息解析失败:', err);
        }
      };

      this.ws.onclose = () => {
        this.stopHeartbeat();
        if (!this.isManualClose) {
          this.scheduleReconnect();
        }
      };
    } catch (err) {
      console.error('WebSocket 连接失败:', err);
      this.scheduleReconnect();
    }
  }

  disconnect(): void {
    this.isManualClose = true;
    this.stopHeartbeat();
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  send(msg: WsMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  on(type: WsMessageType, listener: (data: any) => void): void {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set());
    }
    this.listeners.get(type)!.add(listener);
  }

  off(type: WsMessageType, listener: (data: any) => void): void {
    this.listeners.get(type)?.delete(listener);
  }

  private dispatch(msg: WsMessage): void {
    const listeners = this.listeners.get(msg.type);
    if (listeners) {
      listeners.forEach((l) => l(msg));
    }
  }

  private scheduleReconnect(): void {
    this.reconnectAttempts += 1;
    if (this.reconnectAttempts > MAX_RECONNECT_ATTEMPTS) {
      console.error('WebSocket 重连失败，已达最大次数');
      return;
    }
    const delay = Math.min(
      INITIAL_RECONNECT_DELAY * Math.pow(2, this.reconnectAttempts - 1),
      MAX_RECONNECT_DELAY,
    );
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, delay);
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = setInterval(() => {
      this.send({ type: 'ping', timestamp: Date.now() });
    }, HEARTBEAT_INTERVAL);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  get isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }
}
