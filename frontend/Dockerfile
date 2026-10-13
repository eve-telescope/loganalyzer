# The SvelteKit node server (adapter-node build).

FROM node:26-bookworm-slim AS builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

# The runtime stage carries no node_modules: adapter-node bundles every
# devDependency into build/, and packages listed under dependencies
# would be left external, so the app keeps all of them under
# devDependencies.
FROM node:26-bookworm-slim AS runtime
WORKDIR /app
# The analysis page renders log times and numbers on the server: UTC and
# en-US, like the Laravel box, so the link previews of shared logs (the
# OpenGraph description) read the same.
ENV NODE_ENV=production \
    TZ=UTC \
    LANG=en_US.UTF-8
COPY --from=builder /app/build build
COPY --from=builder /app/package.json ./
USER node
EXPOSE 3000
CMD ["node", "build"]
