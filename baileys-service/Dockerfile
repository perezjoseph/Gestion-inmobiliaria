# Stage 1: Install dependencies
# node:24-alpine ships npm 11 which patches the bundled-npm CVEs
# (tar, minimatch, glob, cross-spawn, brace-expansion, ip-address)
# that ship inside node:20-alpine's older npm 10.x.
# We also upgrade npm to latest to pick up patched bundled deps.
FROM node:24-alpine AS deps
WORKDIR /app
RUN npm install -g npm@latest
COPY package.json package-lock.json* ./
RUN npm ci --omit=dev

# Stage 2: Build TypeScript
FROM node:24-alpine AS build
WORKDIR /app
COPY package.json package-lock.json* tsconfig.json ./
RUN npm ci
COPY src/ ./src/
RUN npm run build

# Stage 3: Production image
FROM node:24-alpine AS runtime
WORKDIR /app

RUN addgroup -S appgroup && adduser -S appuser -G appgroup

COPY --from=deps /app/node_modules ./node_modules
COPY --from=build /app/dist ./dist
COPY package.json ./

ENV NODE_ENV=production
ENV PORT=3100

EXPOSE 3100

USER appuser

CMD ["node", "dist/index.js"]
