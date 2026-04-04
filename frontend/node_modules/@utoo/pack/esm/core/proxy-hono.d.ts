/**
 * HTTP proxy middleware for Hono dev server.
 * Matches ProxyRule by path, applies pathRewrite, and forwards via Hono proxy().
 * No generic WS proxying in this module.
 */
import type { DevServerProxy, PathRewrite, ProxyRule } from "@utoo/pack-shared";
import type { Context, Next } from "hono";
/** Path prefix or ^-prefixed regex match (regex tests full path). */
export declare function doesContextMatchUrl(context: string, pathOrUrl: string): boolean;
export declare function ruleMatchesUrl(rule: ProxyRule, pathOrUrl: string): boolean;
/**
 * Apply pathRewrite (http-proxy-middleware style).
 * Object: first matching regex → replace, then stop. Function: (path) => new path.
 */
export declare function applyPathRewrite(path: string, pathRewrite?: PathRewrite): string;
export interface PreparedProxyRequest {
    url: URL;
    headers: Headers;
}
/**
 * Build target URL and headers for the proxy request from current context and rule.
 */
export declare function prepareProxyRequest(c: Context, rule: ProxyRule): PreparedProxyRequest;
/**
 * Hono middleware: on match, proxy to rule.target with pathRewrite and changeOrigin;
 * on error return 502; otherwise next().
 * Caller (e.g. dev.ts) must pass a non-empty array; empty array means no rule matches.
 */
export declare function createHttpProxyMiddleware(rules: DevServerProxy): (c: Context, next: Next) => Promise<void | Response>;
