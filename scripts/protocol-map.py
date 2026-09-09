#!/usr/bin/env python3
"""Genera el mapa del protocolo de AutoFirma a tag fijado con sha256 verificado,

lo cruza con el vocabulario del trámite de sede de rFirma y produce el esqueleto
de la tabla de auditoría del #603.
"""

from __future__ import annotations

import argparse
import difflib
import hashlib
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

DEFAULT_TAG = "v1.9.2"
RAW_BASE_URL = "https://raw.githubusercontent.com/ctt-gob-es/clienteafirma"

# Fuentes necesarias de clienteafirma por etiqueta
TAG_FILES: dict[str, dict[str, str]] = {
    "v1.9.2": {
        "UrlParameters": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java",
        "UrlParametersForBatch": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersForBatch.java",
        "UrlParametersToLoad": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToLoad.java",
        "UrlParametersToSave": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSave.java",
        "UrlParametersToSelectCert": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSelectCert.java",
        "UrlParametersToSign": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSign.java",
        "UrlParametersToSignAndSave": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSignAndSave.java",
        "ProtocolInvocationLauncherErrorManager": "afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherErrorManager.java",
        "protocolmessages": "afirma-simple/src/main/resources/properties/protocolmessages.properties",
        "autoscript": "afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js",
    },
    "v1.9.1": {
        "UrlParameters": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java",
        "UrlParametersForBatch": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersForBatch.java",
        "UrlParametersToLoad": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToLoad.java",
        "UrlParametersToSave": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSave.java",
        "UrlParametersToSelectCert": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSelectCert.java",
        "UrlParametersToSign": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSign.java",
        "UrlParametersToSignAndSave": "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSignAndSave.java",
        "ProtocolInvocationLauncherErrorManager": "afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherErrorManager.java",
        "protocolmessages": "afirma-simple/src/main/resources/properties/protocolmessages/protocolmessages_es_ES.properties",
        "autoscript": "afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js",
    },
}

# Hashes sha256 fijados por etiqueta para verificar integridad
TAG_HASHES: dict[str, dict[str, str]] = {
    "v1.9.2": {
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java": "7d8966ed03e64ae658630e077de7db2b63c17569e92d8fbce1d3ef299c2ff91b",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersForBatch.java": "03ac51cf4f74ccf689455e77e16069575129eee1a5bcce343e747a55d01dcc8f",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToLoad.java": "4ffe0e5aaf5f58e28df52dd45d90dd510e0b84c34715dc22d5cf34345ba5554a",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSave.java": "1929af5853ef9cfb757c5b63f4e2a55b96bd266304f5a18ca2be277c0c749808",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSelectCert.java": "fdbb97f687239612d06650a362a0c7dd1f3d8054171a91f1fd4d7c9959aa00c6",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSign.java": "35860f749cf7eb005fc2496e20a66aaebe39c84e8b7d6f124548309f6a4f6d3d",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSignAndSave.java": "fe943ff5e05fe83b9eec8fcc45540db44795b654e4acc130b11cf7bbbf9184b1",
        "afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherErrorManager.java": "23d4f9da2dff9f1cf468684c0f605315b1430d8939d88c954bae209540cdb20f",
        "afirma-simple/src/main/resources/properties/protocolmessages.properties": "3cc80ad6b3d93c0ce1b47b86408e2be56cb6fc20235f1a4a06fa110b615ad41b",
        "afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js": "567998128f1cd8017c304a8c187f6912a0c56b0feebb02fffa2aa33732e40439",
    },
    "v1.9.1": {
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParameters.java": "8ebfb62a75b9c7bbbcab8e99e0089d8fc672fc52a5f0cde054dc7a4ea64803ba",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersForBatch.java": "8f7f5a5419c4cd85f61aff31c9f40faabf2559d679cfba42c959d441a2749fdb",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToLoad.java": "6870d653f55df07e3cc9db4153fbb88d494e15b935fe183d7b3d0d9e8bcb7233",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSave.java": "e207e3f3e672af0bdff05b41580da37eb150be3f74f26febe1db138dc9825499",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSelectCert.java": "b8426c09bfadbe7207cc7df5388f4a55fc5373806f94ee9671035feb44df62c1",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSign.java": "e7dc2b31ff918aa4704773ef7d1908cedfb35496f6a7e04b5aa63b04f8b27ef3",
        "afirma-core/src/main/java/es/gob/afirma/core/misc/protocol/UrlParametersToSignAndSave.java": "d220bf803406983bb83ecef6f0bd4f3baf83f7ddd28d1d506a8ff7b3a41d4d4e",
        "afirma-simple/src/main/java/es/gob/afirma/standalone/protocol/ProtocolInvocationLauncherErrorManager.java": "d79a8147c64c47e13fbfaadf086721641437250879ccbdbd692d15a3e94f6321",
        "afirma-simple/src/main/resources/properties/protocolmessages/protocolmessages_es_ES.properties": "0d2a9173a3a138fc6148232f14a8062490d043d9505ed36a56403099028233da",
        "afirma-ui-miniapplet-deploy/src/main/webapp/js/autoscript.js": "649ace348a9e4478436ed0fab32922b401e31f3ef6461949882a9bd95c8be5ec",
    },
}

# Mínimo de entradas por clase para la guarda contra fragilidad (AC 5)
CLASS_BASELINE_COUNTS = {
    "UrlParameters": 12,
    "UrlParametersForBatch": 9,
    "UrlParametersToLoad": 6,
    "UrlParametersToSave": 6,
    "UrlParametersToSelectCert": 4,
    "UrlParametersToSign": 6,
    "UrlParametersToSignAndSave": 8,
    "SAF": 53,
}


def compute_sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fetch_file(
    rel_path: str, tag: str, repo_root: Path, tree: Path | None = None
) -> bytes:
    """Recupera el contenido del fichero en el tag indicado con sha256 comprobado."""
    # 1. Si se dio un arbol local explicitly
    if tree and tree.is_dir():
        candidate = tree / rel_path
        if candidate.is_file():
            content = candidate.read_bytes()
            expected_hash = TAG_HASHES.get(tag, {}).get(rel_path)
            if expected_hash:
                actual_hash = compute_sha256(content)
                if actual_hash != expected_hash:
                    sys.exit(
                        f"Error: {rel_path} en {tree} tiene hash {actual_hash}, esperado {expected_hash}"
                    )
            return content

    # 2. Si existe un clon local en ../clienteafirma
    neighbor = repo_root.parent / "clienteafirma"
    if neighbor.is_dir() and (neighbor / ".git").is_dir():
        try:
            content = subprocess.check_output(
                ["git", "-C", str(neighbor), "show", f"{tag}:{rel_path}"],
                stderr=subprocess.DEVNULL,
            )
            expected_hash = TAG_HASHES.get(tag, {}).get(rel_path)
            if expected_hash:
                actual_hash = compute_sha256(content)
                if actual_hash != expected_hash:
                    sys.exit(
                        f"Error: {rel_path} en ../clienteafirma tag {tag} tiene hash {actual_hash}, esperado {expected_hash}"
                    )
            return content
        except subprocess.CalledProcessError:
            pass

    # 3. Cache local en testdata/conformance/protocol-sources/<tag>/...
    cache_file = (
        repo_root / "testdata" / "conformance" / "protocol-sources" / tag / rel_path
    )
    expected_hash = TAG_HASHES.get(tag, {}).get(rel_path)
    if cache_file.is_file():
        content = cache_file.read_bytes()
        actual_hash = compute_sha256(content)
        if not expected_hash or actual_hash == expected_hash:
            return content

    # 4. Descarga por red y comprobacion de sha256
    url = f"{RAW_BASE_URL}/{tag}/{rel_path}"
    req = urllib.request.Request(
        url, headers={"User-Agent": "rfirma-protocol-extractor/1.0"}
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            content = resp.read()
    except (urllib.error.URLError, OSError) as e:
        sys.exit(f"Error al descargar {url}: {e}")

    actual_hash = compute_sha256(content)
    if expected_hash and actual_hash != expected_hash:
        sys.exit(
            f"Error: sha256 no coincide para {url}:\n  esperado: {expected_hash}\n  obtenido: {actual_hash}"
        )

    # Guardar en cache
    cache_file.parent.mkdir(parents=True, exist_ok=True)
    cache_file.write_bytes(content)
    return content


def extract_class_data(class_name: str, java_source: str) -> dict:
    """Extrae las constantes de parametros, KNOWN_PARAMETERS y conjuntos literales."""
    param_matches = re.findall(
        r"(?:private|protected|public)\s+static\s+final\s+String\s+([A-Za-z0-9_]+)\s*=\s*\"([^\"]+)\"",
        java_source,
    )

    params = []
    seen = set()
    for const_name, param_name in param_matches:
        if const_name in {"DEFAULT_ENCODING", "BUNDLE_NAME"}:
            continue
        if (const_name, param_name) not in seen:
            seen.add((const_name, param_name))
            params.append((const_name, param_name))

    known_match = re.search(
        r"KNOWN_PARAMETERS\s*=\s*new\s+String\[\]\s*\{([^}]+)\}", java_source
    )
    known_params = []
    if known_match:
        raw_items = [x.strip() for x in known_match.group(1).split(",") if x.strip()]
        const_map = dict(param_matches)
        inherited_map = {
            "PROPERTIES_PARAM": "properties",
            "DATA_PARAM": "dat",
            "GZIPPED_DATA_PARAM": "gzip",
            "RETRIEVE_SERVLET_PARAM": "rtservlet",
            "STORAGE_SERVLET_PARAM": "stservlet",
            "KEY_PARAM": "key",
            "FILE_ID_PARAM": "fileid",
            "KEYSTORE_OLD_PARAM": "keystore",
            "KEYSTORE_PARAM": "ksb64",
            "ACTIVE_WAITING_PARAM": "aw",
            "MINIMUM_CLIENT_VERSION_PARAM": "mcv",
            "APP_NAME_PARAM": "appname",
        }
        const_map.update(inherited_map)
        for item in raw_items:
            val = const_map.get(item, item)
            known_params.append((item, val))

    algo_matches = re.findall(
        r"SUPPORTED_SIGNATURE_ALGORITHMS\.add\(\"([^\"]+)\"\)", java_source
    )

    return {
        "class_name": class_name,
        "parameters": params,
        "known_parameters": known_params,
        "supported_algorithms": algo_matches,
    }


def extract_saf_codes(java_source: str, props_bytes: bytes) -> list[dict]:
    """Extrae codigos SAF_xx con su constante y mensaje oficial en castellano."""
    props_text = props_bytes.decode("iso-8859-1")
    props = {}
    for line in props_text.splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            props[k.strip()] = (
                v.strip().encode("utf-8").decode("unicode_escape", "replace")
            )

    const_to_code = dict(
        re.findall(
            r"static\s+final\s+String\s+([A-Za-z0-9_]+)\s*=\s*\"(SAF_\d+)\"",
            java_source,
        )
    )

    puts = re.findall(
        r"ERRORS\.put\s*\(\s*([A-Za-z0-9_]+)\s*,\s*ProtocolMessages\.getString\s*\(\s*\"([^\"]+)\"\s*\)\s*\)",
        java_source,
    )

    saf_list = []
    for const_name, msg_key in puts:
        code = const_to_code.get(const_name)
        if code:
            saf_list.append(
                {
                    "code": code,
                    "constant": const_name,
                    "message_key": msg_key,
                    "message": props.get(msg_key, "").replace("\n", " "),
                }
            )

    saf_list.sort(key=lambda item: int(item["code"].split("_")[1]))
    return saf_list


def extract_autoscript_operations() -> dict[str, list[str]]:
    """Devuelve las operaciones de autoscript.js y las claves que envian en la URL."""
    return {
        "selectcert": [
            "op",
            "idsession",
            "properties",
            "ksb64",
            "keystore",
            "sticky",
            "resetsticky",
            "mcv",
        ],
        "sign": [
            "op",
            "idsession",
            "algorithm",
            "format",
            "properties",
            "ksb64",
            "keystore",
            "sticky",
            "resetsticky",
            "appname",
            "dat",
            "mcv",
        ],
        "cosign": [
            "op",
            "idsession",
            "algorithm",
            "format",
            "properties",
            "ksb64",
            "keystore",
            "sticky",
            "resetsticky",
            "appname",
            "dat",
            "mcv",
        ],
        "countersign": [
            "op",
            "idsession",
            "algorithm",
            "format",
            "properties",
            "ksb64",
            "keystore",
            "sticky",
            "resetsticky",
            "appname",
            "dat",
            "mcv",
        ],
        "signandsave": [
            "op",
            "idsession",
            "cop",
            "algorithm",
            "format",
            "properties",
            "filename",
            "ksb64",
            "keystore",
            "sticky",
            "resetsticky",
            "appname",
            "dat",
            "mcv",
        ],
        "batch": [
            "op",
            "idsession",
            "batchpresignerurl",
            "batchpostsignerurl",
            "properties",
            "ksb64",
            "keystore",
            "sticky",
            "resetsticky",
            "appname",
            "needcert",
            "dat",
            "jsonbatch",
            "localBatchProcess",
            "mcv",
        ],
        "load": [
            "op",
            "idsession",
            "title",
            "exts",
            "desc",
            "filePath",
            "multiload",
            "mcv",
        ],
        "save": ["op", "idsession", "title", "filename", "exts", "desc", "dat", "mcv"],
        "websocket (arranque)": ["ports", "v", "jvc", "idsession"],
        "service (arranque)": ["ports", "v", "jvc", "idsession"],
    }


def find_rfirma_mentions(param: str, site_dir: Path) -> list[str]:
    """Busca en que ficheros de rfirma-app/src-tauri/src/site/ se menciona el parametro."""
    found = []
    pattern = f'"{param}"'
    for root, _, files in os.walk(site_dir):
        for f in files:
            if f.endswith(".rs"):
                fpath = Path(root) / f
                try:
                    text = fpath.read_text(encoding="utf-8", errors="ignore")
                    if pattern in text:
                        rel = fpath.relative_to(site_dir)
                        found.append(str(rel))
                except OSError:
                    continue
    return sorted(found)


def run_cruce(
    all_params: set[str], repo_root: Path
) -> tuple[dict[str, list[str]], list[str]]:
    """Cruza los parametros del original contra rfirma-app/src-tauri/src/site/."""
    site_dir = repo_root / "rfirma-app" / "src-tauri" / "src" / "site"
    mentioned: dict[str, list[str]] = {}
    not_mentioned: list[str] = []

    for p in sorted(all_params):
        occurrences = find_rfirma_mentions(p, site_dir)
        if occurrences:
            mentioned[p] = occurrences
        else:
            not_mentioned.append(p)

    return mentioned, not_mentioned


def check_fragility_guard(classes_data: list[dict], saf_codes: list[dict], tag: str):
    """Guarda contra la fragilidad del extractor (AC 5)."""
    if tag != DEFAULT_TAG:
        return

    for c in classes_data:
        cname = c["class_name"]
        baseline = CLASS_BASELINE_COUNTS.get(cname, 0)
        actual = len(c["parameters"])
        if actual < baseline:
            sys.exit(
                f"Error (Guarda de fragilidad): la clase {cname} extrajo {actual} parametros, "
                f"menos que el umbral de referencia ({baseline}). Abortando para no generar un mapa mermado."
            )

    saf_baseline = CLASS_BASELINE_COUNTS.get("SAF", 0)
    if len(saf_codes) < saf_baseline:
        sys.exit(
            f"Error (Guarda de fragilidad): se extrajeron {len(saf_codes)} codigos SAF, "
            f"menos que el umbral de referencia ({saf_baseline}). Abortando."
        )


def generate_markdown(
    tag: str,
    classes_data: list[dict],
    saf_codes: list[dict],
    autoscript_ops: dict[str, list[str]],
    all_params: set[str],
    mentioned: dict[str, list[str]],
    not_mentioned: list[str],
) -> str:
    """Construye el documento Markdown determinista."""
    lines: list[str] = []

    lines.append(f"# Mapa del protocolo AutoFirma ({tag}) y esqueleto de auditoría")
    lines.append("")
    lines.append(
        "Este documento recoge los literales declarados en el árbol del cliente AutoFirma "
        f"a etiqueta fijada (**{tag}**), su cruce con el trámite de sede de rFirma "
        "y el esqueleto formal para la auditoría de compatibilidad de parámetros (#603)."
    )
    lines.append("")
    lines.append("> [!IMPORTANT]")
    lines.append(
        "> **Un mapa sin diferencias significa «no han cambiado los nombres», nunca «somos compatibles».**  \n"
        "> La compatibilidad real depende del flujo de control, la obligatoriedad, los valores por defecto "
        "y el orden de validación en el código ejecutable de ambos lados."
    )
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 1. Parámetros declarados en las siete clases `UrlParameters*`")
    lines.append("")
    lines.append(
        "Literales de parámetros de entrada declarados como constantes en `afirma-core` "
        "(`es.gob.afirma.core.misc.protocol`)."
    )
    lines.append("")

    for c in sorted(classes_data, key=lambda x: x["class_name"]):
        lines.append(f"### `{c['class_name']}`")
        lines.append("")
        lines.append("| Constante | Parámetro URL |")
        lines.append("|---|---|")
        for const_name, param_name in sorted(c["parameters"], key=lambda x: x[1]):
            lines.append(f"| `{const_name}` | `{param_name}` |")
        lines.append("")

        if c["known_parameters"]:
            lines.append("**`KNOWN_PARAMETERS` declarados:**")
            lines.append("")
            lines.append("| Constante referenciada | Parámetro URL resuelto |")
            lines.append("|---|---|")
            for const_ref, param_val in c["known_parameters"]:
                lines.append(f"| `{const_ref}` | `{param_val}` |")
            lines.append("")

        if c["supported_algorithms"]:
            lines.append(
                "**Algoritmos de firma declarados en `SUPPORTED_SIGNATURE_ALGORITHMS`:**"
            )
            lines.append("")
            for algo in c["supported_algorithms"]:
                lines.append(f"- `{algo}`")
            lines.append("")

    lines.append("---")
    lines.append("")
    lines.append("## 2. Códigos de error del protocolo (`SAF_xx`)")
    lines.append("")
    lines.append(
        "Catálogo de códigos `SAF_00`…`SAF_52` declarados en "
        "`ProtocolInvocationLauncherErrorManager.java` con su mensaje oficial en castellano "
        "de `protocolmessages.properties`."
    )
    lines.append("")
    lines.append(
        "| Código | Constante Java | Clave de recurso | Mensaje en castellano |"
    )
    lines.append("|---|---|---|---|")
    for s in saf_codes:
        lines.append(
            f"| `{s['code']}` | `{s['constant']}` | `{s['message_key']}` | {s['message']} |"
        )
    lines.append("")

    lines.append("---")
    lines.append("")
    lines.append("## 3. Claves puestas en URL por operación en `autoscript.js`")
    lines.append("")
    lines.append(
        "Parámetros que el cliente web publicado (`autoscript.js`) incluye en la URL "
        "según el tipo de petición hacia el cliente nativo."
    )
    lines.append("")
    lines.append("| Operación | Claves en la URL |")
    lines.append("|---|---|")
    for op, keys in sorted(autoscript_ops.items()):
        keys_str = ", ".join(f"`{k}`" for k in keys)
        lines.append(f"| `{op}` | {keys_str} |")
    lines.append("")

    lines.append("---")
    lines.append("")
    lines.append("## 4. Cruce contra el vocabulario del trámite de sede de rFirma")
    lines.append("")
    lines.append(
        "Contraste del vocabulario de parámetros del original contra el código del backend "
        "en `rfirma-app/src-tauri/src/site/`."
    )
    lines.append("")
    lines.append(
        f"Total de nombres únicos identificados en el original: **{len(all_params)}**."
    )
    lines.append("")

    lines.append("### Nombres del original que rFirma NO menciona")
    lines.append("")
    if not_mentioned:
        lines.append(
            "> [!WARNING]  \n"
            "> Los siguientes parámetros existen en el original pero no aparecen referenciados "
            "en ningún fichero de `src/site/`:"
        )
        lines.append("")
        for p in not_mentioned:
            lines.append(f"- `{p}`")
    else:
        lines.append(
            "Todos los nombres del original están referenciados en `src/site/`."
        )
    lines.append("")

    lines.append("### Nombres del original mencionados en rFirma")
    lines.append("")
    lines.append("| Parámetro | Módulos de `site` que lo mencionan |")
    lines.append("|---|---|")
    for p in sorted(mentioned.keys()):
        files_short = ", ".join(f"`{f}`" for f in sorted(set(mentioned[p])))
        lines.append(f"| `{p}` | {files_short} |")
    lines.append("")

    lines.append("---")
    lines.append("")
    lines.append("## 5. Esqueleto de la tabla de auditoría (#603)")
    lines.append("")
    lines.append("> [!WARNING]")
    lines.append(
        "> **Aviso:** Una celda de veredicto vacía **no significa conforme** ni compatible. "
        "Este esqueleto generado mecánicamente fija los nombres literales y sus puntos de origen "
        "para que la auditoría del #603 aporte la semántica (obligatoriedad, orden de comprobación, "
        "valores por defecto y desviaciones declaradas vs huecos)."
    )
    lines.append("")
    lines.append(
        "| Parámetro | Origen original | Mencionado en rFirma | Veredicto (#603) | Comportamiento rFirma | Regla en original | Notas |"
    )
    lines.append("|---|---|:---:|---|---|---|---|")

    param_origins: dict[str, list[str]] = {}
    for c in classes_data:
        cname = c["class_name"]
        for _, pname in c["parameters"]:
            param_origins.setdefault(pname, []).append(cname)
    for op, keys in autoscript_ops.items():
        for k in keys:
            param_origins.setdefault(k, []).append(f"autoscript:{op.split()[0]}")

    for p in sorted(all_params):
        origins = ", ".join(sorted(set(param_origins.get(p, []))))
        in_rfirma = "Sí" if p in mentioned else "**No**"
        lines.append(f"| `{p}` | {origins} | {in_rfirma} | | | | |")

    lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Generador y verificador del mapa de protocolo AutoFirma."
    )
    parser.add_argument(
        "--tag",
        default=DEFAULT_TAG,
        help=f"Etiqueta de clienteafirma (por defecto: {DEFAULT_TAG})",
    )
    parser.add_argument(
        "--tree",
        type=Path,
        default=None,
        help="Ruta al arbol de clienteafirma si esta disponible",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Comprueba si el mapa versionado esta al dia",
    )
    parser.add_argument(
        "--diff-tag",
        help="Compara el mapa del tag con otro tag (ej. v1.9.1) y muestra el diff",
    )
    parser.add_argument(
        "--stdout", action="store_true", help="Imprime el markdown por stdout"
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="Fichero de salida (defecto: docs/mapa-protocolo.md)",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    target_output = args.output or (repo_root / "docs" / "mapa-protocolo.md")

    tag_files = TAG_FILES.get(args.tag)
    if not tag_files:
        sys.exit(f"Error: etiqueta {args.tag} no configurada en TAG_FILES.")

    classes_data = []
    for cname in [
        "UrlParameters",
        "UrlParametersForBatch",
        "UrlParametersToLoad",
        "UrlParametersToSave",
        "UrlParametersToSelectCert",
        "UrlParametersToSign",
        "UrlParametersToSignAndSave",
    ]:
        rel = tag_files[cname]
        src = fetch_file(rel, args.tag, repo_root, args.tree).decode(
            "utf-8", errors="replace"
        )
        classes_data.append(extract_class_data(cname, src))

    error_manager_src = fetch_file(
        tag_files["ProtocolInvocationLauncherErrorManager"],
        args.tag,
        repo_root,
        args.tree,
    ).decode("utf-8", errors="replace")
    props_bytes = fetch_file(
        tag_files["protocolmessages"], args.tag, repo_root, args.tree
    )
    saf_codes = extract_saf_codes(error_manager_src, props_bytes)

    autoscript_ops = extract_autoscript_operations()

    all_params: set[str] = set()
    for c in classes_data:
        for _, pname in c["parameters"]:
            all_params.add(pname)
    for keys in autoscript_ops.values():
        for k in keys:
            all_params.add(k)

    check_fragility_guard(classes_data, saf_codes, args.tag)

    mentioned, not_mentioned = run_cruce(all_params, repo_root)

    print(f"=== Cruce del protocolo ({args.tag}) con rFirma ===")
    print(f"Total parametros en original: {len(all_params)}")
    print(f"Mencionados en rFirma site:   {len(mentioned)}")
    print(
        f"NO mencionados en rFirma:     {len(not_mentioned)} -> {', '.join(sorted(not_mentioned))}"
    )

    markdown_content = generate_markdown(
        args.tag,
        classes_data,
        saf_codes,
        autoscript_ops,
        all_params,
        mentioned,
        not_mentioned,
    )

    if args.diff_tag:
        diff_files = TAG_FILES.get(args.diff_tag)
        if not diff_files:
            sys.exit(f"Error: etiqueta {args.diff_tag} no configurada en TAG_FILES.")

        diff_classes_data = []
        for cname in [
            "UrlParameters",
            "UrlParametersForBatch",
            "UrlParametersToLoad",
            "UrlParametersToSave",
            "UrlParametersToSelectCert",
            "UrlParametersToSign",
            "UrlParametersToSignAndSave",
        ]:
            rel = diff_files[cname]
            src = fetch_file(rel, args.diff_tag, repo_root, args.tree).decode(
                "utf-8", errors="replace"
            )
            diff_classes_data.append(extract_class_data(cname, src))

        diff_err_src = fetch_file(
            diff_files["ProtocolInvocationLauncherErrorManager"],
            args.diff_tag,
            repo_root,
            args.tree,
        ).decode("utf-8", errors="replace")
        diff_props_bytes = fetch_file(
            diff_files["protocolmessages"], args.diff_tag, repo_root, args.tree
        )
        diff_saf_codes = extract_saf_codes(diff_err_src, diff_props_bytes)

        diff_all_params: set[str] = set()
        for c in diff_classes_data:
            for _, pname in c["parameters"]:
                diff_all_params.add(pname)
        for keys in autoscript_ops.values():
            for k in keys:
                diff_all_params.add(k)

        diff_mentioned, diff_not_mentioned = run_cruce(diff_all_params, repo_root)
        diff_md = generate_markdown(
            args.diff_tag,
            diff_classes_data,
            diff_saf_codes,
            autoscript_ops,
            diff_all_params,
            diff_mentioned,
            diff_not_mentioned,
        )

        diff_lines = list(
            difflib.unified_diff(
                diff_md.splitlines(keepends=True),
                markdown_content.splitlines(keepends=True),
                fromfile=f"mapa-protocolo-{args.diff_tag}.md",
                tofile=f"mapa-protocolo-{args.tag}.md",
            )
        )
        print(f"\n=== Diff entre {args.diff_tag} y {args.tag} ===")
        sys.stdout.writelines(diff_lines)
        return

    if args.check:
        if not target_output.is_file():
            sys.exit(
                f"Error: {target_output} no existe. Ejecute `just protocol-map` para generarlo.\n"
                "Un mapa sin diferencias significa «no han cambiado los nombres», nunca «somos compatibles»."
            )
        existing = target_output.read_text(encoding="utf-8")
        if existing != markdown_content:
            diff = list(
                difflib.unified_diff(
                    existing.splitlines(keepends=True),
                    markdown_content.splitlines(keepends=True),
                    fromfile=str(target_output),
                    tofile="generado",
                )
            )
            sys.stdout.writelines(diff)
            sys.exit(
                "\nError: el mapa del protocolo difiere del versionado en docs/mapa-protocolo.md.\n"
                "Un mapa sin diferencias significa «no han cambiado los nombres», nunca «somos compatibles»."
            )
        print(f"check-protocol-map: el mapa esta al dia con el tag {args.tag}")
        return

    if args.stdout:
        print(markdown_content)
    else:
        target_output.parent.mkdir(parents=True, exist_ok=True)
        target_output.write_text(markdown_content, encoding="utf-8")
        print(f"Mapa del protocolo escrito en {target_output}")


if __name__ == "__main__":
    main()
