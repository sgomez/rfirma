# rFirma no es un lector de PDF: la firma empieza por un verbo

La firma empieza casi siempre **fuera** de rFirma: hay un PDF en una carpeta y alguien quiere
firmarlo. La forma barata de recoger eso en un escritorio de Linux es declarar
`MimeType=application/pdf` en el lanzador, y es la forma equivocada. Esa línea no dice «esto se
puede firmar», dice **«esto lo abre rFirma»**: aparece «Abrir con rFirma» junto a Okular y a Evince,
rFirma entra en la lista de candidatas a lector predeterminado y, si alguien la elige, cada PDF del
sistema termina en una aplicación que no sabe pasar de página. La etiqueta miente sobre lo que va a
pasar al pulsarla.

rFirma **no declara ningún tipo de documento en ningún lanzador**. Lo que instala es un **verbo** en el
menú contextual del gestor de ficheros: **«Firmar con rFirma»**, al primer nivel, sobre un PDF. El
nombre lleva el «con rFirma» a propósito: compite con «Abrir con» y «Comprimir», y «Firmar» a secas
no dice quién firma ni deja sitio a un futuro «Firmar como …».

**Lo que sí declara el lanzador es el esquema `afirma://`**, y no es una excepción a lo anterior:
`MimeType=x-scheme-handler/afirma;` no registra a rFirma como candidata a abrir ningún fichero, sólo
dice quién atiende las URL de ese esquema, que es lo que el ID-234 necesita para que la sede
electrónica pueda invocarla. La regla, dicha con precisión, es **ningún tipo de documento** en un
lanzador `Type=Application` —ni `application/pdf` ni ningún otro—, y de esquemas exactamente uno, el
de la sede.

## Consecuencias

Son tres, y las tres están medidas antes de escribir esto.

**Dos ficheros, porque no hay verbo común.** GNOME y KDE no comparten mecanismo: KDE lo lee de un
`.desktop` con `Type=Service` en su directorio de *servicemenus*
(`packaging/kde/rfirma-sign.desktop`, instalado en `/usr/share/kio/servicemenus/`), y GNOME de una
extensión de `nautilus-python`, cuyo paquete se llama distinto en cada familia y viaja como
recomendación. En el `.desktop` de KDE sí hay un `MimeType=application/pdf`, y no contradice nada: en
un `Type=Service` esa clave **no registra a nadie como candidato a abrir el fichero**, sólo filtra en
qué menú contextual aparece el verbo. Sin ella el verbo no aparecería en ninguno.

**El flatpak se queda fuera, a propósito.** No puede escribir en los directorios del anfitrión, y
aunque se le diera permiso, `%f` con reenvío de ficheros le entrega una **ruta del portal**, que es
justo la que el ADR-0011 prohíbe enseñar. El verbo es del `.deb` y del `.rpm`; en el flatpak se abre
rFirma y se elige el documento desde dentro. La asimetría es visible para quien instala por flatpak,
y se prefiere a enseñar una ruta que no existe.

**Se renuncia a arrastrar un PDF sobre el icono del *dock*.** Es el mismo `MimeType` el que da las
dos cosas, así que no hay manera de quedarse sólo con la buena. Sigue funcionando arrastrar el
documento **sobre la ventana** de rFirma, que es el gesto equivalente y no exige registrar nada.

## Considered Options

**Declarar `application/pdf` en el lanzador.** Descartada: da el verbo por accidente y el papel de
lector por defecto por contrato. El precio —una entrada «Abrir con» que miente y una candidatura a
predeterminada que nadie quiere— es permanente; el que se paga aquí, no poder arrastrar sobre el
icono, tiene sustituto.

**Un solo mecanismo para los dos escritorios.** No existe: no hay un formato de verbo contextual
común a Nautilus y a Dolphin. Elegir uno solo habría dejado a la mitad de las personas usuarias sin
nada.

**Llevar el verbo también al flatpak.** Descartada por la ruta del portal, no por el sandbox: lo que
llegaría a la ventana es un enlace `/run/user/…/doc/…` que el ADR-0011 no deja mostrar como origen
del documento.

## Cómo se vigila

`packaging/check-version.py` (`just check-version`, en el CI) comprueba las cuatro cosas que este ADR
fija: que ningún lanzador de tipo `Application` declara un `MimeType` de documento, y que el único
esquema que admite es `x-scheme-handler/afirma` —y ahí entra, sobre todo, la
plantilla `packaging/rfirma.desktop.hbs`, que es el lanzador que de verdad instalan el `.deb` y el
`.rpm`; el `.hbs` del nombre la dejaba fuera de un barrido por `*.desktop`, así que la puerta lo
recorre por su sufijo y además exige que el `desktopTemplate` declarado en `tauri.conf.json` para
cada paquete sea uno de los ficheros que inspecciona—, que el *servicemenu* de KDE
filtra por `application/pdf`, sale al primer nivel (`X-KDE-Priority=TopLevel`) y **desaparece con más
de un fichero seleccionado** (`X-KDE-RequiredNumberOfUrls=1`), que el verbo se llama exactamente
«Firmar con rFirma», y que el manifiesto del flatpak no lo instala.
