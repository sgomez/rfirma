#!/usr/bin/env python3
import datetime, glob, os, re, sys

root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
os.chdir(root)
version = sys.argv[1]


def numero_issue(p):
    nombre = os.path.splitext(os.path.basename(p))[0]
    try:
        return int(nombre)
    except ValueError:
        sys.exit(
            f"{p}: el nombre debe ser el numero de issue (ej. 252.md), "
            f"segun changelog.d/README.md."
        )


fragments = sorted(
    (p for p in glob.glob("changelog.d/*.md") if os.path.basename(p) != "README.md"),
    key=numero_issue,
)
if not fragments:
    sys.exit("changelog.d/ no tiene fragmentos que reunir.")

# Orden canonico de Keep a Changelog. Cada fragmento agrupa sus lineas bajo
# uno o varios encabezados "### <categoria>"; aqui se acumulan por categoria
# (conservando el orden por numero de issue dentro de cada una) para que la
# seccion publicada no repita encabezados sueltos.
categorias_canonicas = [
    "Added", "Changed", "Deprecated", "Removed", "Fixed", "Security",
]
lineas_por_categoria = {c: [] for c in categorias_canonicas}
encabezado = re.compile(r"^### (\w+)\s*$", re.MULTILINE)

for f in fragments:
    contenido = open(f, encoding="utf-8").read().strip()
    coincidencias = list(encabezado.finditer(contenido))
    if not coincidencias:
        sys.exit(f"{f}: no tiene ningun encabezado '### <categoria>'.")
    for i, m in enumerate(coincidencias):
        categoria = m.group(1)
        if categoria not in lineas_por_categoria:
            sys.exit(f"{f}: categoria desconocida '### {categoria}'.")
        inicio = m.end()
        fin = coincidencias[i + 1].start() if i + 1 < len(coincidencias) else len(contenido)
        lineas_por_categoria[categoria].append(contenido[inicio:fin].strip())

body = "\n\n".join(
    f"### {categoria}\n{chr(10).join(lineas_por_categoria[categoria])}"
    for categoria in categorias_canonicas
    if lineas_por_categoria[categoria]
)
today = datetime.date.today().isoformat()

changelog = open("CHANGELOG.md", encoding="utf-8").read()
placeholder = re.compile(
    r"^## \[" + re.escape(version) + r"\] - sin publicar$", re.MULTILINE
)
if placeholder.search(changelog):
    seccion = f"## [{version}] - {today}\n\n{body}"
    changelog = placeholder.sub(lambda _m: seccion, changelog, count=1)
else:
    first_heading = re.search(r"^## \[", changelog, re.MULTILINE)
    section = f"## [{version}] - {today}\n\n{body}\n\n"
    if first_heading:
        pos = first_heading.start()
        changelog = changelog[:pos] + section + changelog[pos:]
    else:
        changelog = changelog.rstrip("\n") + "\n\n" + section

open("CHANGELOG.md", "w", encoding="utf-8").write(changelog)

for f in fragments:
    os.remove(f)

print(f"CHANGELOG.md: version {version} publicada con {len(fragments)} fragmento(s).")
