# El servidor local usa HTTPS con una CA propia instalada en el almacén del sistema

Las sedes electrónicas invocan al cliente de firma desde una página servida por
HTTPS. Un servidor local en texto plano sería bloqueado por el navegador como
contenido mixto, así que `rfirma` levanta un servidor **HTTPS** en
`127.0.0.1:63117` con un certificado autofirmado generado en la primera
ejecución. Para que el navegador confíe en él, el instalador registra la CA
local en el **almacén de CA del sistema** en lugar de manipular las bases de
datos `cert9.db` de cada perfil de Firefox.

## Consequences

- Instalar `rfirma` requiere privilegios de root una única vez. A cambio, la
  confianza se establece para todos los navegadores y todos los perfiles de
  usuario a la vez, y no se rompe al crear un perfil nuevo.
- Estamos añadiendo una CA al almacén del sistema: su clave privada solo puede
  vivir en la máquina del usuario, con permisos restrictivos, y debe emitir
  únicamente para `localhost`. Nunca debe distribuirse una CA precompilada
  compartida entre instalaciones.
- El puerto y el protocolo son parte del contrato con las sedes electrónicas
  existentes: cambiarlos rompe la compatibilidad con AutoFirma.
