# Build stage
FROM node:20-slim as builder
WORKDIR /app

# Copy package.json and package-lock.json
COPY package*.json ./

# Install dependencies
RUN npm install

# Copy the rest of the application
COPY . .

# Create .env.production from example if not exists
RUN cp .env.production.example .env.production || true

# Build the frontend application
RUN npm run build

# Runtime stage
FROM nginx:alpine

# Copy the build output
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy the custom nginx configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

# 使用 nginx 用户运行主进程（避免 root 权限）
# 批次 21 修复：nginx:alpine 镜像内置 nginx 用户，默认主进程以 root 启动用于绑定 80 端口
# TODO(tech-debt): 若 nginx.conf 监听 80 端口，非 root 用户无法绑定特权端口，
# 需改用非特权端口（如 8080）或授予 CAP_NET_BIND_SERVICE，当前先以 USER nginx 落地
USER nginx

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
