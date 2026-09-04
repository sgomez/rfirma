# La landing de rfirma.sgomez.me

La imagen que sirve `https://rfirma.sgomez.me` es **sólo `nginx` con esta landing, y nada
más** ([ADR-0015](../../docs/adr/0015-canal-de-distribucion-propio.md)). Los tres
repositorios de paquetes (ostree, apt, dnf) no van dentro de la imagen: los publica un
montaje de directorio del anfitrión aparte, fuera de este árbol.

| Fichero | Qué es |
|---|---|
| `index.html` | La landing, escrita a mano, sin generador y sin paso de construcción |
| `Dockerfile` | `nginx:alpine` más la landing |

## Coolify

Coolify construye esta imagen **desde `main`**, con este `Dockerfile` como raíz de
construcción (*Build Pack*: Dockerfile; *Base Directory*: `packaging/repo/`). No hace falta
ningún paso de construcción adicional: no hay `package.json`, ni `pnpm`, ni assets que
compilar. Un cambio en `index.html` o en `Dockerfile` en `main` es lo único que dispara un
redespliegue.

El resto de la infraestructura —dominio, certificado TLS, el volumen `/srv/rfirma-repo` con
los tres repositorios de paquetes— es aprovisionamiento humano, fuera de este repositorio.
