#!/usr/bin/env python3
"""Guía interactiva que mide la detección en caliente de lector y tarjeta por PC/SC y por PKCS#11, y la lectura sin PIN de los certificados.

Uso: python -I -u hotplug_probe.py [--module <opensc-pkcs11>] [--log <fichero>] [--skip-wait]
Empieza con el lector enchufado y la tarjeta dentro. Nunca hace C_Login ni envía un PIN.
Dependencias: pyscard, PyKCS11, cryptography.
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from card_common import child, clock, default_module, safe

HERE = Path(__file__).resolve()


def watch_pcsc(t0):
    from smartcard.scard import (
        SCARD_E_TIMEOUT,
        SCARD_S_SUCCESS,
        SCARD_SCOPE_USER,
        SCARD_STATE_CHANGED,
        SCARD_STATE_EMPTY,
        SCARD_STATE_EXCLUSIVE,
        SCARD_STATE_INUSE,
        SCARD_STATE_MUTE,
        SCARD_STATE_PRESENT,
        SCARD_STATE_UNAVAILABLE,
        SCARD_STATE_UNAWARE,
        SCARD_STATE_UNKNOWN,
        SCardEstablishContext,
        SCardGetErrorMessage,
        SCardGetStatusChange,
        SCardListReaders,
    )

    say = clock(t0, "pcsc")
    pnp = "\\\\?PnP?\\Notification"
    names = {
        SCARD_STATE_PRESENT: "PRESENT",
        SCARD_STATE_EMPTY: "EMPTY",
        SCARD_STATE_MUTE: "MUTE",
        SCARD_STATE_INUSE: "INUSE",
        SCARD_STATE_EXCLUSIVE: "EXCLUSIVE",
        SCARD_STATE_UNAVAILABLE: "UNAVAILABLE",
        SCARD_STATE_UNKNOWN: "UNKNOWN",
    }
    hr, ctx = SCardEstablishContext(SCARD_SCOPE_USER)
    assert hr == SCARD_S_SUCCESS, SCardGetErrorMessage(hr)

    def readers():
        hr, found = SCardListReaders(ctx, [])
        return list(found) if hr == SCARD_S_SUCCESS else []

    found = readers()
    say("lectores:", found)
    states = [(r, SCARD_STATE_UNAWARE) for r in found] + [(pnp, SCARD_STATE_UNAWARE)]
    while True:
        hr, new = SCardGetStatusChange(ctx, 1000, states)
        if hr == SCARD_E_TIMEOUT:
            continue
        if hr != SCARD_S_SUCCESS:
            say("error:", SCardGetErrorMessage(hr))
            time.sleep(1)
            continue
        readers_changed = False
        previous = dict(states)
        for name, event, atr in new:
            if name == pnp:
                readers_changed |= previous.get(name) != SCARD_STATE_UNAWARE and bool(
                    event & SCARD_STATE_CHANGED
                )
            elif event & SCARD_STATE_CHANGED:
                state = "|".join(v for k, v in names.items() if event & k) or hex(event)
                say(name, "->", state, "(ATR presente)" if atr else "")
        states = [(n, ev & ~SCARD_STATE_CHANGED) for n, ev, _ in new]
        if readers_changed:
            found = readers()
            say("PnP: cambio de lectores ->", found)
            known = dict(states)
            states = [(r, known.get(r, SCARD_STATE_UNAWARE)) for r in found] + [
                (pnp, known[pnp])
            ]


def relist_p11(t0, module):
    import PyKCS11

    say = clock(t0, "p11")
    lib = PyKCS11.PyKCS11Lib()
    lib.load(module)

    def certificates(slot):
        start = time.monotonic()
        try:
            session = lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)
            count = len(
                session.findObjects([(PyKCS11.CKA_CLASS, PyKCS11.CKO_CERTIFICATE)])
            )
            session.closeSession()
            return f"{count} certs en {(time.monotonic() - start) * 1000:.0f} ms"
        except PyKCS11.PyKCS11Error as e:
            return f"error {e}"

    last = None
    while True:
        start = time.monotonic()
        try:
            snapshot = (
                list(lib.getSlotList(tokenPresent=False)),
                list(lib.getSlotList(tokenPresent=True)),
            )
        except PyKCS11.PyKCS11Error as e:
            snapshot = ("error", str(e))
        elapsed = (time.monotonic() - start) * 1000
        if snapshot != last:
            extra = (
                ", ".join(f"slot {s}: {certificates(s)}" for s in snapshot[1])
                if snapshot[0] != "error"
                else ""
            )
            say(
                f"slots={snapshot[0]} con_token={snapshot[1]} (C_GetSlotList {elapsed:.0f} ms) {extra}"
            )
            last = snapshot
        time.sleep(1)


def wait_p11(t0, module):
    import PyKCS11

    say = clock(t0, "wait")
    lib = PyKCS11.PyKCS11Lib()
    lib.load(module)
    try:
        say(
            "C_WaitForSlotEvent(DONT_BLOCK) ->",
            lib.waitForSlotEvent(PyKCS11.CKF_DONT_BLOCK),
        )
    except PyKCS11.PyKCS11Error as e:
        say("C_WaitForSlotEvent(DONT_BLOCK) ->", e)
    while True:
        try:
            say("C_WaitForSlotEvent(bloqueante) -> slot", lib.waitForSlotEvent(0))
        except PyKCS11.PyKCS11Error as e:
            say("C_WaitForSlotEvent(bloqueante) ->", e)
            time.sleep(1)


def cold_start(t0, module):
    import PyKCS11
    from cryptography import x509
    from cryptography.x509.oid import NameOID

    say = clock(t0, "frio")
    lib = PyKCS11.PyKCS11Lib()
    start = time.monotonic()
    lib.load(module)
    say(f"C_Initialize: {(time.monotonic() - start) * 1000:.0f} ms")
    for slot in lib.getSlotList(tokenPresent=True):
        info = lib.getTokenInfo(slot)
        say(
            "token:",
            safe(info.label),
            "| modelo:",
            info.model.strip(),
            "| flags:",
            info.flags2text(),
        )
        session = lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)
        start = time.monotonic()
        objects = session.findObjects()
        say(
            f"C_FindObjects sin login: {len(objects)} objetos en {(time.monotonic() - start) * 1000:.0f} ms"
        )
        private_keys = session.findObjects(
            [(PyKCS11.CKA_CLASS, PyKCS11.CKO_PRIVATE_KEY)]
        )
        say("claves privadas visibles sin login:", len(private_keys))
        for handle in session.findObjects(
            [(PyKCS11.CKA_CLASS, PyKCS11.CKO_CERTIFICATE)]
        ):
            label = session.getAttributeValue(handle, [PyKCS11.CKA_LABEL])[0]
            der = bytes(
                session.getAttributeValue(
                    handle, [PyKCS11.CKA_VALUE], allAsBinary=True
                )[0]
            )
            cert = x509.load_der_x509_certificate(der)
            issuer = cert.issuer.get_attributes_for_oid(NameOID.COMMON_NAME)
            usages = []
            try:
                ku = cert.extensions.get_extension_for_class(x509.KeyUsage).value
                usages = [
                    n
                    for n in (
                        "digital_signature",
                        "content_commitment",
                        "key_cert_sign",
                    )
                    if getattr(ku, n)
                ]
            except x509.ExtensionNotFound:
                pass
            try:
                is_ca = cert.extensions.get_extension_for_class(
                    x509.BasicConstraints
                ).value.ca
            except x509.ExtensionNotFound:
                is_ca = None
            expired = cert.not_valid_after_utc.timestamp() < time.time()
            say(
                f"  cert {safe(label)} | emisor CN: {issuer[0].value if issuer else '?'} | usos: {usages} "
                f"| CA: {is_ca} | caducado: {expired}"
            )
        session.closeSession()


def guide(args):
    log = Path(args.log)
    log.write_text("")
    t0 = time.time()
    say = clock(t0, "GUIA")

    def mark(text):
        say(text)
        with log.open("a") as f:
            f.write(f"[GUIA {time.time() - t0:7.2f}s] {text}\n")

    def step(text):
        input(f"\n>>> {text} y pulsa Intro ")
        mark(f"HECHO: {text}")
        time.sleep(4)

    def run_pass(name, mode):
        print(f"\n=== Pasada: {name} ===")
        mark(f"INICIO {name}")
        with log.open("a") as out:
            proc = subprocess.Popen(
                child(
                    str(HERE), "--child", mode, "--t0", str(t0), "--module", args.module
                ),
                stdout=out,
                stderr=subprocess.STDOUT,
            )
        try:
            time.sleep(5)
            step("Desenchufa el lector USB (con la tarjeta dentro)")
            step("Vuelve a enchufar el lector con la tarjeta dentro")
            time.sleep(4)
            step("Saca la tarjeta del lector")
            step("Vuelve a meter la tarjeta")
            time.sleep(6)
        finally:
            proc.kill()
            proc.wait()
        mark(f"FIN {name}")

    print(
        "Empieza con el lector enchufado y la tarjeta dentro. Cierra el navegador si tiene la tarjeta abierta."
    )
    input(">>> Pulsa Intro para empezar ")
    run_pass("PC/SC SCardGetStatusChange", "pcsc")
    run_pass("PKCS#11 re-listado sin reinicializar", "relist")
    if not args.skip_wait:
        run_pass("PKCS#11 C_WaitForSlotEvent bloqueante", "wait")
    print("\n=== Arranque en frío ===")
    step("Saca la tarjeta del lector")
    input(">>> Mete la tarjeta y pulsa Intro AL MISMO TIEMPO ")
    mark("Tarjeta metida, lanzo C_Initialize")
    with log.open("a") as out:
        subprocess.run(
            child(
                str(HERE), "--child", "cold", "--t0", str(t0), "--module", args.module
            ),
            stdout=out,
            stderr=subprocess.STDOUT,
        )
    mark("FIN arranque en frío")
    print(f"\nListo. La salida está en {log}")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--module")
    parser.add_argument("--log", default=str(Path.home() / "tmp" / "card-hotplug.log"))
    parser.add_argument(
        "--skip-wait", action="store_true", help="omite la pasada de C_WaitForSlotEvent"
    )
    parser.add_argument(
        "--child", choices=["pcsc", "relist", "wait", "cold"], help=argparse.SUPPRESS
    )
    parser.add_argument("--t0", type=float, help=argparse.SUPPRESS)
    args = parser.parse_args()
    args.module = args.module or default_module()
    modes = {
        "pcsc": lambda: watch_pcsc(args.t0),
        "relist": lambda: relist_p11(args.t0, args.module),
        "wait": lambda: wait_p11(args.t0, args.module),
        "cold": lambda: cold_start(args.t0, args.module),
    }
    if args.child:
        modes[args.child]()
    else:
        guide(args)


if __name__ == "__main__":
    sys.exit(main())
