# AnyLive admin-web — Vite build + nginx for test deploy.
# Build context: apps/admin-web

FROM node:22-bookworm-slim AS builder
WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY index.html vite.config.ts tsconfig.json tsconfig.app.json tsconfig.node.json ./
COPY public ./public
COPY src ./src

# Browser-facing API origin (host port published by compose).
ARG VITE_API_BASE=http://localhost:8088
ENV VITE_API_BASE=$VITE_API_BASE

# Typecheck is covered by CI; image build uses Vite only for reliability.
RUN pnpm exec vite build

FROM nginx:1.27-alpine AS runtime
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=5 \
  CMD wget -qO- http://127.0.0.1/ >/dev/null || exit 1
