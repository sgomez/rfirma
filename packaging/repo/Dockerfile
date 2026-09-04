# La imagen de rfirma.sgomez.me: Caddy y la landing, nada más (ADR-0015).
#
# Los tres repositorios de paquetes (ostree, apt, dnf) no viven dentro de la imagen: los
# publica un montaje de directorio del anfitrión aparte, por rsync. Coolify construye esta
# imagen desde `main` cada vez que cambia este directorio.
FROM caddy:alpine

# Crear usuario y grupo caddy sin privilegios y asegurar permisos en directorios de trabajo
RUN addgroup -g 1000 -S caddy && \
    adduser -u 1000 -S -D -H -G caddy caddy && \
    mkdir -p /data /config /srv && \
    chown -R caddy:caddy /data /config /srv

COPY Caddyfile /etc/caddy/Caddyfile
COPY index.html rfirma-main-window.png /srv/

# Ejecutar con el usuario sin privilegios caddy (UID 1000)
USER caddy

# Puerto de servicio sin privilegios (Coolify actúa como frontal y proxy inverso)
EXPOSE 3000

# Healthcheck nativo para Coolify
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --quiet --tries=1 --spider http://127.0.0.1:3000/healthz || exit 1
