#!/usr/bin/env python3
"""Mide qué dice el módulo PKCS#11 del estado del PIN de un DNIe, con AutoFirma y rFirma como referencia.

Uso: python -I -u pin_probe.py [--module <opensc-pkcs11>]           solo lectura: no envía ningún PIN
     python -I -u pin_probe.py [--module <opensc-pkcs11>] --lock    BLOQUEA el DNIe con PINs erróneos a propósito
Sin --lock se mide el estado sin gastar intentos. Con --lock, el DNIe queda bloqueado hasta un punto de actualización.
--lock envía un PIN erróneo en un proceso nuevo y, con la tarjeta ya bloqueada, prueba cómo se recupera la sesión:
tras un C_Login fallido, OpenSC rechaza el siguiente del mismo proceso sin llegar a la tarjeta.
--recover gasta un intento con un PIN erróneo y prueba con tu PIN real, de menos a más, qué recupera la sesión.
--interference mide si otro uso de la tarjeta entre dos C_Login rompe la sesión; cada fallo se recupera con tu PIN real.
--sign-interference firma datos de prueba con tu PIN real y mide si otro uso de la tarjeta entre el login y la firma la rompe.
Dependencias: PyKCS11. Opcional: pkcs15-tool de OpenSC en el PATH.
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from card_common import DNIE_TOKEN, child, default_module, pkcs15_pin_lines

WRONG_PIN = "ZZZZ9999zzzz"
ACCEPTED = 3


def say(*parts):
    print(f"[{time.strftime('%H:%M:%S')}]", *parts, flush=True)


def pause(text):
    input(f"\n>>> {text}\n    Pulsa Intro para seguir ")


def autofirma_running():
    if sys.platform == "win32":
        listing = subprocess.run(["tasklist"], capture_output=True, text=True).stdout
        return "autofirma" in listing.lower()
    return (
        subprocess.run(
            ["pgrep", "-fi", r"autofirma\.(jar|app)"], capture_output=True
        ).returncode
        == 0
    )


def autofirma_reference(text):
    pause(f"REFERENCIA (AutoFirma): {text} Después CIERRA AutoFirma del todo.")
    while autofirma_running():
        pause(
            "AutoFirma sigue abierto y comparte la tarjeta con esta sonda: ciérralo del todo."
        )
    say("AutoFirma cerrado.")


def wrong_login(module):
    import PyKCS11

    lib = PyKCS11.PyKCS11Lib()
    lib.load(module)
    slot = lib.getSlotList(tokenPresent=True)[0]
    session = lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)
    start = time.monotonic()
    try:
        session.login(WRONG_PIN)
        session.logout()
        return ACCEPTED
    except PyKCS11.PyKCS11Error as e:
        print(
            f"C_Login -> {e} ({(time.monotonic() - start) * 1000:.0f} ms) | flags después, mismo proceso:",
            lib.getTokenInfo(slot).flags2text(),
        )


def recovery(module):
    import PyKCS11

    def attempt(lib, session, step):
        start = time.monotonic()
        try:
            session.login(WRONG_PIN)
            session.logout()
            print(f"{step}: C_Login ACEPTADO")
            sys.exit(ACCEPTED)
        except PyKCS11.PyKCS11Error as e:
            print(
                f"{step}: C_Login -> {e} ({(time.monotonic() - start) * 1000:.0f} ms)",
                flush=True,
            )

    lib = PyKCS11.PyKCS11Lib()
    lib.load(module)
    slot = lib.getSlotList(tokenPresent=True)[0]
    session = lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)
    attempt(lib, session, "R1 sesión nueva")
    attempt(lib, session, "R2 misma sesión")
    session.closeSession()
    attempt(
        lib,
        lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION),
        "R3 otra sesión, mismo C_Initialize",
    )
    lib.closeAllSessions(slot)
    attempt(
        lib,
        lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION),
        "R4 tras C_CloseAllSessions",
    )
    lib.unload()
    lib = PyKCS11.PyKCS11Lib()
    lib.load(module)
    slot = lib.getSlotList(tokenPresent=True)[0]
    attempt(
        lib,
        lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION),
        "R5 tras C_Finalize y C_Initialize",
    )
    print("flags al final de la recuperación:", lib.getTokenInfo(slot).flags2text())


def verify_pin_first(lib, slot, pin):
    import PyKCS11

    session = lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)
    start = time.monotonic()
    try:
        session.login(pin)
        session.logout()
        say(
            f"Comprobación previa: tu PIN es correcto ({(time.monotonic() - start) * 1000:.0f} ms)."
        )
    except PyKCS11.PyKCS11Error as e:
        sys.exit(
            f"Comprobación previa: C_Login -> {e}. Paro sin enviar ningún PIN erróneo."
        )
    finally:
        session.closeSession()


def recover(lib, slot, module):
    import getpass

    import PyKCS11

    def login(session, pin, step):
        start = time.monotonic()
        try:
            session.login(pin)
            say(
                f"{step}: C_Login ACEPTADO ({(time.monotonic() - start) * 1000:.0f} ms)"
            )
            return "ok"
        except PyKCS11.PyKCS11Error as e:
            say(f"{step}: C_Login -> {e} ({(time.monotonic() - start) * 1000:.0f} ms)")
            return "incorrect" if "CKR_PIN_INCORRECT" in str(e) else "other"

    answer = input(
        "\nTe pedirá tu PIN real dos veces y después GASTARÁ UN INTENTO con un PIN erróneo. Escribe SEGUIR: "
    )
    if answer.strip() != "SEGUIR":
        sys.exit("Cancelado: no se ha enviado nada.")
    while True:
        pin = getpass.getpass("\nTeclea tu PIN real (no se ve ni se guarda): ")
        if pin == getpass.getpass("Repítelo: "):
            break
        print(
            "No coinciden; vuelve a teclearlo. Aún no se ha enviado nada a la tarjeta."
        )
    verify_pin_first(lib, slot, pin)
    session = lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)
    if login(session, WRONG_PIN, "R1 PIN erróneo") != "incorrect":
        sys.exit(
            "El PIN erróneo no devolvió CKR_PIN_INCORRECT; paro sin enviar tu PIN."
        )
    say("  flags tras el fallo:", lib.getTokenInfo(slot).flags2text())

    def other_session():
        session.closeSession()
        return lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)

    def after_close_all():
        lib.closeAllSessions(slot)
        return lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)

    def after_reinit():
        nonlocal lib
        lib.unload()
        lib = PyKCS11.PyKCS11Lib()
        lib.load(module)
        return lib.openSession(
            lib.getSlotList(tokenPresent=True)[0], PyKCS11.CKF_SERIAL_SESSION
        )

    steps = [
        ("R2 misma sesión", lambda: session),
        ("R3 otra sesión, mismo C_Initialize", other_session),
        ("R4 tras C_CloseAllSessions", after_close_all),
        ("R5 tras C_Finalize y C_Initialize", after_reinit),
    ]
    for step, next_session in steps:
        session = next_session()
        outcome = login(session, pin, step)
        if outcome == "ok":
            session.logout()
            break
        if outcome == "incorrect":
            say(
                "La tarjeta ha rechazado el PIN tecleado: se ha gastado otro intento. Paro."
            )
            break
    else:
        say("Ningún paso aceptó el PIN.")
    del pin
    lib.closeAllSessions(lib.getSlotList(tokenPresent=True)[0])
    say("Fin. Si algún paso lo aceptó, el contador vuelve a estar a 3.")


def interference(lib, slot, module):
    import getpass

    import PyKCS11

    ctx = {"lib": lib, "slot": slot}

    def login(session, pin, step):
        start = time.monotonic()
        try:
            session.login(pin)
            say(
                f"  {step}: C_Login ACEPTADO ({(time.monotonic() - start) * 1000:.0f} ms)"
            )
            session.logout()
            return "ok"
        except PyKCS11.PyKCS11Error as e:
            say(
                f"  {step}: C_Login -> {e} ({(time.monotonic() - start) * 1000:.0f} ms)"
            )
            return "incorrect" if "CKR_PIN_INCORRECT" in str(e) else "other"

    def open_session():
        return ctx["lib"].openSession(ctx["slot"], PyKCS11.CKF_SERIAL_SESSION)

    def other_process():
        subprocess.run(
            child(__file__, "--child-flags", "--module", module), capture_output=True
        )

    def pkcs15_tool():
        pkcs15_pin_lines()

    def same_process_queries():
        ctx["lib"].getTokenInfo(ctx["slot"])
        session = open_session()
        session.findObjects([(PyKCS11.CKA_CLASS, PyKCS11.CKO_CERTIFICATE)])
        session.closeSession()

    def reinit():
        ctx["lib"].unload()
        ctx["lib"] = PyKCS11.PyKCS11Lib()
        ctx["lib"].load(module)
        ctx["slot"] = ctx["lib"].getSlotList(tokenPresent=True)[0]

    def recovery_ladder(session, pin):
        session.closeSession()
        if (outcome := login(open_session(), pin, "otra sesión")) != "other":
            return outcome
        ctx["lib"].closeAllSessions(ctx["slot"])
        if (
            outcome := login(open_session(), pin, "tras C_CloseAllSessions")
        ) != "other":
            return outcome
        reinit()
        return login(open_session(), pin, "tras C_Finalize y C_Initialize")

    trials = [
        ("T1 sin fallo previo, otro proceso con OpenSC en medio", False, other_process),
        ("T2 fallo previo, otro proceso con OpenSC en medio", True, other_process),
        ("T3 fallo previo, pkcs15-tool en medio", True, pkcs15_tool),
        (
            "T4 fallo previo, consultas en el mismo proceso en medio",
            True,
            same_process_queries,
        ),
    ]
    answer = input(
        "\nTe pedirá tu PIN real dos veces. Después, T2, T3 y T4 gastan un intento cada uno, "
        "y cada uno lo recupera con tu PIN antes del siguiente. Escribe SEGUIR: "
    )
    if answer.strip() != "SEGUIR":
        sys.exit("Cancelado: no se ha enviado nada.")
    while True:
        pin = getpass.getpass("\nTeclea tu PIN real (no se ve ni se guarda): ")
        if pin == getpass.getpass("Repítelo: "):
            break
        print(
            "No coinciden; vuelve a teclearlo. Aún no se ha enviado nada a la tarjeta."
        )
    verify_pin_first(ctx["lib"], ctx["slot"], pin)
    for name, with_failure, interferer in trials:
        say(name)
        session = open_session()
        if with_failure and login(session, WRONG_PIN, "PIN erróneo") != "incorrect":
            sys.exit("El PIN erróneo no devolvió CKR_PIN_INCORRECT; paro.")
        interferer()
        outcome = login(session, pin, "misma sesión")
        if outcome == "other":
            outcome = recovery_ladder(session, pin)
        else:
            session.closeSession()
        if outcome == "incorrect":
            sys.exit(
                "La tarjeta ha rechazado el PIN tecleado. Paro: firma una vez con el PIN correcto para volver a 3."
            )
        if outcome != "ok":
            sys.exit(
                "Nada recuperó la sesión. Paro: firma una vez con el PIN correcto para volver a 3."
            )
    del pin
    say("Fin: cada fallo se recuperó con tu PIN, el contador está a 3.")


def sign_interference(lib, slot, module):
    import getpass

    import PyKCS11

    data = b"rfirma: datos de prueba, no es un documento"
    mechanism = PyKCS11.Mechanism(PyKCS11.CKM_SHA256_RSA_PKCS, None)

    def other_process():
        subprocess.run(
            child(__file__, "--child-flags", "--module", module), capture_output=True
        )

    def other_process_later():
        delayed = "import subprocess,sys,time;time.sleep(3);subprocess.run(sys.argv[1:],capture_output=True)"
        subprocess.Popen(
            [
                sys.executable,
                "-I",
                "-c",
                delayed,
                *child(__file__, "--child-flags", "--module", module),
            ]
        )

    def signing_key(session):
        for key in session.findObjects([(PyKCS11.CKA_CLASS, PyKCS11.CKO_PRIVATE_KEY)]):
            if (
                session.getAttributeValue(key, [PyKCS11.CKA_LABEL])[0].strip()
                == "KprivFirmaDigital"
            ):
                return key
        sys.exit("No encuentro la clave KprivFirmaDigital tras el login; paro.")

    def sign(session, step):
        start = time.monotonic()
        try:
            signature = session.sign(signing_key(session), data, mechanism)
            say(
                f"  {step}: C_Sign CORRECTO, {len(signature)} bytes ({(time.monotonic() - start) * 1000:.0f} ms)"
            )
            return True
        except PyKCS11.PyKCS11Error as e:
            say(f"  {step}: C_Sign -> {e} ({(time.monotonic() - start) * 1000:.0f} ms)")
            return False

    def logged_in_session():
        session = lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)
        start = time.monotonic()
        try:
            session.login(pin)
        except PyKCS11.PyKCS11Error as e:
            sys.exit(f"C_Login -> {e}; paro.")
        say(f"  C_Login ACEPTADO ({(time.monotonic() - start) * 1000:.0f} ms)")
        return session

    trials = [
        (
            "S0 control: nada entre el login y la firma",
            None,
            "Acepta la ventana de confirmación de OpenSC en cuanto salga.",
        ),
        (
            "S1 otro proceso con OpenSC entre el login y la firma",
            other_process,
            "Acepta la ventana de confirmación de OpenSC en cuanto salga.",
        ),
        (
            "S2 pkcs15-tool entre el login y la firma",
            pkcs15_pin_lines,
            "Acepta la ventana de confirmación de OpenSC en cuanto salga.",
        ),
        (
            "S3 otro proceso con OpenSC MIENTRAS sale la confirmación",
            other_process_later,
            "Cuando salga la ventana de confirmación, ESPERA unos 6 segundos y luego acéptala.",
        ),
    ]
    answer = input(
        "\nTe pedirá tu PIN real dos veces y firmará cuatro veces unos datos de prueba "
        "(no es ningún documento). No envía ningún PIN erróneo. Escribe SEGUIR: "
    )
    if answer.strip() != "SEGUIR":
        sys.exit("Cancelado: no se ha enviado nada.")
    while True:
        pin = getpass.getpass("\nTeclea tu PIN real (no se ve ni se guarda): ")
        if pin == getpass.getpass("Repítelo: "):
            break
        print(
            "No coinciden; vuelve a teclearlo. Aún no se ha enviado nada a la tarjeta."
        )
    verify_pin_first(lib, slot, pin)
    for name, interferer, instruction in trials:
        say(name)
        input(f"    {instruction} Pulsa Intro para empezar ")
        session = logged_in_session()
        if interferer is not None:
            interferer()
        signed = sign(session, "misma sesión")
        session.closeSession()
        if not signed and interferer is None:
            sys.exit(
                "La firma de control falla sin nada en medio; el resto no tendría sentido. Paro."
            )
        if not signed:
            retry = logged_in_session()
            sign(retry, "otra sesión, nuevo login")
            retry.closeSession()
    del pin
    lib.closeAllSessions(slot)
    say("Fin. No se ha enviado ningún PIN erróneo: el contador sigue a 3.")


def token_flags(module):
    import PyKCS11

    lib = PyKCS11.PyKCS11Lib()
    lib.load(module)
    for slot in lib.getSlotList(tokenPresent=True):
        print(slot, lib.getTokenInfo(slot).flags2text())


def snapshot(lib, slot, module):
    import PyKCS11

    session = lib.openSession(slot, PyKCS11.CKF_SERIAL_SESSION)
    certs = len(session.findObjects([(PyKCS11.CKA_CLASS, PyKCS11.CKO_CERTIFICATE)]))
    session.closeSession()
    say(
        "  flags en este proceso:",
        lib.getTokenInfo(slot).flags2text(),
        "| certificados sin login:",
        certs,
    )
    fresh = subprocess.run(
        child(__file__, "--child-flags", "--module", module),
        capture_output=True,
        text=True,
    )
    say("  flags en un proceso nuevo:", fresh.stdout.strip() or fresh.stderr.strip())
    say("  pkcs15-tool --list-pins:", pkcs15_pin_lines())


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--module")
    parser.add_argument(
        "--lock", action="store_true", help="envía PINs erróneos hasta bloquear el DNIe"
    )
    parser.add_argument(
        "--recover",
        action="store_true",
        help="gasta un intento y prueba con tu PIN real qué recupera la sesión",
    )
    parser.add_argument(
        "--interference",
        action="store_true",
        help="mide si otro uso de la tarjeta entre dos C_Login rompe la sesión",
    )
    parser.add_argument(
        "--sign-interference",
        action="store_true",
        help="mide si otro uso de la tarjeta entre el login y la firma rompe la firma",
    )
    parser.add_argument("--child-flags", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--child-login", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--child-recovery", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    module = args.module or default_module()
    if args.child_flags:
        return token_flags(module)
    if args.child_login:
        return wrong_login(module)
    if args.child_recovery:
        return recovery(module)

    import PyKCS11

    lib = PyKCS11.PyKCS11Lib()
    lib.load(module)
    slots = lib.getSlotList(tokenPresent=True)
    if len(slots) != 1:
        sys.exit(f"Esperaba un solo token y hay {len(slots)}")
    slot = slots[0]
    if not DNIE_TOKEN.match(lib.getTokenInfo(slot).label.strip()):
        sys.exit("El token no es un DNIe")

    say("Estado inicial:")
    snapshot(lib, slot, module)
    if args.recover:
        return recover(lib, slot, module)
    if args.interference:
        return interference(lib, slot, module)
    if args.sign_interference:
        return sign_interference(lib, slot, module)
    autofirma_reference(
        "abre AutoFirma, «Continuar con DNIe», intenta firmar un fichero cualquiera "
        "y APUNTA qué dice el diálogo del PIN (¿intentos restantes?). Cancela SIN teclear nada."
    )
    if not args.lock:
        say("Fin de la medición de solo lectura: no se ha enviado ningún PIN.")
        return

    answer = input(
        "\nEsto BLOQUEA tu DNIe hasta ir a un punto de actualización. Escribe BLOQUEAR para seguir: "
    )
    if answer.strip() != "BLOQUEAR":
        sys.exit("Cancelado: no se ha enviado nada.")
    login = subprocess.run(
        child(__file__, "--child-login", "--module", module),
        capture_output=True,
        text=True,
    )
    say("INTENTO (proceso nuevo):", login.stdout.strip() or login.stderr.strip())
    if login.returncode == ACCEPTED:
        sys.exit("El PIN erróneo se aceptó; paro.")
    snapshot(lib, slot, module)
    say("RECUPERACIÓN DE LA SESIÓN, con la tarjeta ya bloqueada (proceso nuevo):")
    rec = subprocess.run(
        child(__file__, "--child-recovery", "--module", module),
        capture_output=True,
        text=True,
    )
    for line in (rec.stdout + rec.stderr).splitlines():
        say("  " + line)
    if rec.returncode == ACCEPTED:
        sys.exit("El PIN erróneo se aceptó; paro.")
    snapshot(lib, slot, module)
    say("Fin de los intentos: la tarjeta debería estar bloqueada.")
    autofirma_reference(
        "repite la prueba con la tarjeta bloqueada y APUNTA si avisa antes de pedir el PIN."
    )
    pause(
        "REFERENCIA (rFirma): intenta firmar con el certificado de firma, teclea cualquier cosa y APUNTA el mensaje."
    )


if __name__ == "__main__":
    sys.exit(main())
