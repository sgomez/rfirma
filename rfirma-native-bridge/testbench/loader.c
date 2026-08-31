/* Carga la libreria con dlopen (como hara libloading en Rust, ADR-0004),
   crea el isolate y ejecuta una prefirma PAdES real. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

typedef int  (*create_isolate_fn)(void *, void **, void **);
typedef char *(*presign_fn)(void *, const char *, const char *, const char *, const char *);
typedef void (*free_fn)(void *, void *);

static char *slurp(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) { fprintf(stderr, "no puedo abrir %s\n", path); exit(2); }
    fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
    char *b = malloc(n + 1);
    if (fread(b, 1, n, f) != (size_t)n) { exit(2); }
    b[n] = 0;
    while (n > 0 && (b[n-1] == '\n' || b[n-1] == '\r')) { b[--n] = 0; }
    fclose(f);
    return b;
}

int main(int argc, char **argv) {
    if (argc < 4) { fprintf(stderr, "uso: loader <lib.so> <pdf.b64> <cert.b64>\n"); return 2; }

    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "DLOPEN FALLO: %s\n", dlerror()); return 1; }
    printf("dlopen: OK\n");

    create_isolate_fn create = (create_isolate_fn) dlsym(h, "graal_create_isolate");
    presign_fn presign      = (presign_fn)       dlsym(h, "rfirma_pades_presign");
    free_fn freestr         = (free_fn)          dlsym(h, "rfirma_free_string");
    if (!create || !presign || !freestr) {
        fprintf(stderr, "DLSYM FALLO: %s\n", dlerror()); return 1;
    }
    printf("dlsym: OK\n");

    void *isolate = NULL, *thread = NULL;
    int rc = create(NULL, &isolate, &thread);
    if (rc != 0) { fprintf(stderr, "CREATE_ISOLATE FALLO rc=%d\n", rc); return 1; }
    printf("graal_create_isolate: OK\n");

    char *pdf  = slurp(argv[2]);
    char *cert = slurp(argv[3]);

    char *extra = (argc > 4) ? slurp(argv[4]) : (char*)"";
    char *out = presign(thread, pdf, "SHA256withRSA", cert, extra);
    if (!out) { fprintf(stderr, "PRESIGN devolvio NULL\n"); return 1; }

    if (strncmp(out, "ERROR:", 6) == 0) {
        printf("PRESIGN ERROR\n%s\n", out);
        freestr(thread, out);
        return 3;
    }
    printf("PRESIGN OK (%zu bytes)\n", strlen(out));
    printf("---8<--- TriphaseData ---8<---\n%.1200s\n---8<---\n", out);
    freestr(thread, out);
    return 0;
}
