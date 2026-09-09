# La imagen de rfirma.sgomez.me: Caddy y la landing, nada más (ADR-0015).
#
# EL CONTEXTO DE CONSTRUCCIÓN ES LA RAÍZ DEL REPOSITORIO, no este directorio: la landing
# toma el sistema de diseño de `rfirma-app/src/design-system/bundle/`, que es su única
# fuente, y esa carpeta tiene que entrar en el contexto. Cómo se configura en Coolify, en
# el README de al lado.
#
# Los tres repositorios de paquetes (ostree, apt, dnf) no viven dentro de la imagen: los
# publica un montaje de directorio del anfitrión aparte, por rsync. Coolify construye esta
# imagen desde `main` cada vez que cambia este directorio.
FROM node:24-alpine AS landing

RUN corepack enable
WORKDIR /landing
COPY rfirma-app/src/design-system/bundle ./rfirma-app/src/design-system/bundle
COPY packaging/repo/site ./packaging/repo/site
WORKDIR /landing/packaging/repo/site
RUN pnpm install --frozen-lockfile
RUN pnpm build

FROM caddy:alpine

# Crear usuario y grupo caddy sin privilegios y asegurar permisos en directorios de trabajo
RUN addgroup -g 1000 -S caddy && \
    adduser -u 1000 -S -D -H -G caddy caddy && \
    mkdir -p /data /config /srv && \
    chown -R caddy:caddy /data /config /srv

COPY packaging/repo/Caddyfile /etc/caddy/Caddyfile
COPY --from=landing --chown=caddy:caddy /landing/packaging/repo/site/dist/ /srv/

# Ejecutar con el usuario sin privilegios caddy (UID 1000)
USER caddy

# Puerto de servicio sin privilegios (Coolify actúa como frontal y proxy inverso)
EXPOSE 3000

# Healthcheck nativo para Coolify
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --quiet --tries=1 --spider http://127.0.0.1:3000/healthz || exit 1
