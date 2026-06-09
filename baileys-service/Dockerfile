# Stage 1: Install dependencies
# node:26-alpine ships npm 11+ which patches the bundled-npm CVEs.
# Global npm/npx/corepack are removed from the runtime stage below.
FROM node:26-alpine AS deps
WORKDIR /app
RUN npm install -g npm@latest
COPY package.json package-lock.json* ./
RUN npm ci --omit=dev

# Stage 2: Build TypeScript
FROM node:26-alpine AS build
WORKDIR /app
COPY package.json package-lock.json* tsconfig.json ./
RUN npm ci
COPY src/ ./src/
RUN npm run build

# Stage 3: Production image
FROM node:26-alpine AS runtime
WORKDIR /app

RUN addgroup -S appgroup && adduser -S appuser -G appgroup \
 && rm -rf /usr/local/lib/node_modules /usr/local/bin/npm /usr/local/bin/npx /usr/local/bin/corepack

COPY --from=deps /app/node_modules ./node_modules
COPY --from=build /app/dist ./dist
COPY package.json ./

ENV NODE_ENV=production
ENV PORT=3100

EXPOSE 3100

USER appuser

CMD ["node", "dist/index.js"]
