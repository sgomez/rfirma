# La imagen de rfirma.sgomez.me: nginx y la landing, nada más (ADR-0015).
#
# Los tres repositorios de paquetes (ostree, apt, dnf) no viven dentro de la imagen: los
# publica un montaje de directorio del anfitrión aparte, por rsync. Coolify construye esta
# imagen desde `main` cada vez que cambia este directorio; una publicación de paquetes no
# reconstruye ni redespliega nada de esto.
FROM nginx:alpine

COPY index.html /usr/share/nginx/html/index.html
