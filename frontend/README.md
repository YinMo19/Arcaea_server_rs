# Arcaea Admin Frontend

独立管理台前端，使用 React + TypeScript + Tailwind CSS + shadcn/ui 风格基础组件，依赖通过 pnpm 管理。

```sh
pnpm install
pnpm dev
```

开发服务器会把 `/web/api/*` 代理到 `http://127.0.0.1:8090`，因此本地测试时后端建议用：

```sh
ROCKET_PORT=8090 cargo run
```

生产构建：

```sh
pnpm build
```
