#!/usr/bin/env python3
"""
benchmark_startup.py - Medidor de tiempo de apertura de AutoFirma vs rFirma.

Detecta con precisión de milisegundos el instante exacto en que la ventana
aparece en el servidor gráfico (compatible con Wayland y XWayland mediante AT-SPI).
Soporta purga de caché del kernel para medir en frío (Cold Start) o en caliente (Warm Start).
"""

import argparse
import os
import subprocess
import sys
import time

try:
    import gi
    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi
except ImportError:
    print("Error: Se requiere python3-gi y at-spi2 (habitual en Ubuntu/GNOME).")
    print("Instala con: sudo apt install python3-gi gir1.2-atspi-2.0")
    sys.exit(1)


# Colores y códigos ANSI
BOLD = "\033[1m"
GREEN = "\033[32m"
BLUE = "\033[34m"
CYAN = "\033[36m"
YELLOW = "\033[33m"
RED = "\033[31m"
RESET = "\033[0m"
CLEAR_LINE = "\033[2K\r"


def drop_os_caches():
    """Limpia la caché de páginas (pagecache, dentries e inodes) del kernel Linux."""
    try:
        subprocess.run(["sync"], check=True)
        # Escribir 3 en drop_caches limpia pagecache, dentries e inodes
        res = subprocess.run(
            ["sudo", "sh", "-c", "echo 3 > /proc/sys/vm/drop_caches"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        if res.returncode == 0:
            return True
    except Exception:
        pass
    return False


def get_desktop_windows():
    """Devuelve el conjunto de (aplicación, título_de_ventana) activas en el escritorio."""
    d = Atspi.get_desktop(0)
    windows = set()
    count = d.get_child_count()
    for i in range(count):
        app = d.get_child_at_index(i)
        app_name = app.get_name() or ""
        win_count = app.get_child_count()
        for w in range(win_count):
            win = app.get_child_at_index(w)
            wname = win.get_name() or ""
            if wname:
                windows.add((app_name, wname))
    return windows


def is_target_window(app_name: str, win_name: str, target_id: str) -> bool:
    app_lower = app_name.lower()
    win_lower = win_name.lower()
    if target_id == "rfirma":
        return "rfirma" in app_lower or "rfirma" in win_lower
    elif target_id == "autofirma":
        return "autofirma" in win_lower or "autofirma" in app_lower or "simpleafirma" in win_lower
    return True


def countdown(seconds: int, app_name: str, cold_start: bool):
    if cold_start:
        print(f"  ❄️  {CYAN}Limpiando caché del SO (drop_caches para arranque en frío)...{RESET}", flush=True)
        if not drop_os_caches():
            print(f"  {YELLOW}Aviso: No se pudo limpiar la caché (se requiere permisos de sudo).{RESET}")
        time.sleep(0.5)

    if seconds <= 0:
        return
    print(f"\n{BOLD}▶ Preparando lanzamiento de {CYAN}{app_name}{RESET}...")
    for s in range(seconds, 0, -1):
        print(f"{CLEAR_LINE}  Iniciando en {YELLOW}{s}{RESET}...", end="", flush=True)
        time.sleep(1.0)
    print(f"{CLEAR_LINE}  🚀 ¡Lanzando {BOLD}{app_name}{RESET}!", flush=True)


def measure_single(cmd: list[str], target_id: str, label: str, auto_close: bool, timeout: float = 25.0) -> float | None:
    # 1. Foto fija previa de ventanas abiertas en el escritorio
    before_windows = get_desktop_windows()

    # 2. Lanzar proceso midiendo tiempo exacto
    t0 = time.perf_counter()
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        preexec_fn=os.setsid,  # Para matar todo el árbol de procesos (especialmente JVM)
    )

    detected_window = None
    elapsed = 0.0

    # 3. Bucle de sondeo a alta frecuencia (~10ms)
    while elapsed < timeout:
        time.sleep(0.01)
        elapsed = time.perf_counter() - t0
        print(f"{CLEAR_LINE}  ⏱️  Tiempo transcurrido: {YELLOW}{elapsed:.3f} s{RESET}", end="", flush=True)

        current_windows = get_desktop_windows()
        new_windows = current_windows - before_windows

        for app_name, win_name in new_windows:
            if is_target_window(app_name, win_name, target_id):
                detected_window = (app_name, win_name)
                break

        if detected_window:
            break

    print(f"{CLEAR_LINE}", end="", flush=True)

    if detected_window:
        print(f"  {GREEN}✔ Ventana en pantalla:{RESET} {BOLD}{detected_window[1]}{RESET} ({CYAN}{elapsed:.3f} s{RESET})")
    else:
        print(f"  {RED}✘ Tiempo agotado ({timeout}s) sin detectar la ventana.{RESET}")
        elapsed = None

    # 4. Cerrar la aplicación si corresponde
    if auto_close:
        print(f"  🛑 Cerrando {label}...", end="", flush=True)
        try:
            os.killpg(os.getpgid(proc.pid), 15)  # SIGTERM
            proc.wait(timeout=3)
        except Exception:
            try:
                os.killpg(os.getpgid(proc.pid), 9)  # SIGKILL
            except Exception:
                pass
        print(f"{CLEAR_LINE}  ✔ {label} cerrado.")
        time.sleep(1.0)  # Pausa para estabilizar el compositor gráfico
    else:
        print(f"  ℹ️  La ventana permanece abierta.")

    return elapsed


def main():
    parser = argparse.ArgumentParser(
        description="Mide el tiempo exacto que tarda en aparecer en pantalla la ventana de AutoFirma vs rFirma.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--app",
        choices=["both", "rfirma", "autofirma"],
        default="both",
        help="Aplicación a medir (both, rfirma, o autofirma). Por defecto: both.",
    )
    parser.add_argument(
        "--cold",
        action="store_true",
        help="Purga la caché del sistema operativo (drop_caches) antes de cada lanzamiento (Cold Start).",
    )
    parser.add_argument(
        "--delay",
        type=int,
        default=3,
        help="Segundos de cuenta atrás antes de lanzar la app (para dar tiempo a la grabación). Por defecto: 3.",
    )
    parser.add_argument(
        "--no-close",
        action="store_true",
        help="No cerrar la ventana automáticamente tras ser detectada.",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=25.0,
        help="Tiempo máximo en segundos de espera por ventana. Por defecto: 25.0.",
    )

    args = parser.parse_args()
    auto_close = not args.no_close

    mode_label = f"{CYAN}❄️  COLD START (Caché purgada){RESET}" if args.cold else f"{YELLOW}🔥 WARM START (En memoria){RESET}"

    print(f"\n{BOLD}════════════════════════════════════════════════════════════════════{RESET}")
    print(f"{BOLD}  Benchmark de tiempo de apertura de ventana (TTI / First Frame)  {RESET}")
    print(f"  Modo: {mode_label}")
    print(f"{BOLD}════════════════════════════════════════════════════════════════════{RESET}")

    results = {}

    if args.app in ("both", "autofirma"):
        countdown(args.delay, "AutoFirma (Java/Swing)", cold_start=args.cold)
        t_af = measure_single(["autofirma"], "autofirma", "AutoFirma", auto_close, args.timeout)
        results["AutoFirma"] = t_af
        if not auto_close and args.app == "both":
            input(f"\n{YELLOW}Presiona [ENTER] cuando hayas cerrado AutoFirma para continuar con rFirma...{RESET}")

    if args.app in ("both", "rfirma"):
        countdown(args.delay, "rFirma (Rust/Tauri)", cold_start=args.cold)
        t_rf = measure_single(["rfirma"], "rfirma", "rFirma", auto_close, args.timeout)
        results["rFirma"] = t_rf

    # Resumen final
    print(f"\n{BOLD}──────────────────────── RESUMEN FINAL ({'COLD' if args.cold else 'WARM'}) ────────────────────────{RESET}")
    for name, elapsed in results.items():
        if elapsed is not None:
            ms = elapsed * 1000.0
            print(f"  • {BOLD}{name:12}{RESET}: {GREEN}{elapsed:.3f} s{RESET} ({ms:6.1f} ms)")
        else:
            print(f"  • {BOLD}{name:12}{RESET}: {RED}Error / Timeout{RESET}")

    if "AutoFirma" in results and "rFirma" in results:
        t_af = results["AutoFirma"]
        t_rf = results["rFirma"]
        if t_af and t_rf and t_rf > 0:
            speedup = t_af / t_rf
            diff_s = t_af - t_rf
            print(f"\n  🚀 {BOLD}Resultado:{RESET} {CYAN}rFirma{RESET} es {BOLD}{GREEN}{speedup:.1f}x más rápida{RESET}")
            print(f"     (Aparece {diff_s:.3f} segundos antes que AutoFirma)")
    print(f"{BOLD}───────────────────────────────────────────────────────────────────{RESET}\n")


if __name__ == "__main__":
    main()
