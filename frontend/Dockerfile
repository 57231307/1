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
# v18 批次 48 修复 P0-6/7：nginx.conf 已改为 listen 8080（非特权端口），
# 与 USER nginx 兼容；原 listen 80 因非 root 无法绑定特权端口导致容器无法启动。
USER nginx

EXPOSE 8080

CMD ["nginx", "-g", "daemon off;"]
