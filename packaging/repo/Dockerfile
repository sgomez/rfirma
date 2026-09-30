# La imagen de rfirma.sgomez.me: Caddy y la landing (ADR-0015). El contexto es la raíz del repositorio.
FROM node:24-alpine AS landing

RUN corepack enable
WORKDIR /landing
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY rfirma-app/src/design-system/bundle ./rfirma-app/src/design-system/bundle
COPY packaging/repo/site ./packaging/repo/site
RUN pnpm install --frozen-lockfile --filter rfirma-landing
WORKDIR /landing/packaging/repo/site
RUN pnpm build

FROM caddy:alpine

RUN addgroup -g 1000 -S caddy && \
    adduser -u 1000 -S -D -H -G caddy caddy && \
    mkdir -p /data /config /srv && \
    chown -R caddy:caddy /data /config /srv

COPY packaging/repo/Caddyfile /etc/caddy/Caddyfile
COPY --from=landing --chown=caddy:caddy /landing/packaging/repo/site/dist/ /srv/

USER caddy

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --quiet --tries=1 --spider http://127.0.0.1:3000/healthz || exit 1
