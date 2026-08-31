/* Carga la libreria con dlopen (como hara libloading en Rust, ADR-0004),
   crea el isolate y ejecuta una prefirma o una postfirma PAdES reales.

   uso: loader <lib.so> presign  <pdf.b64> <cert.b64> [extra.properties]
        loader <lib.so> postsign <pdf.b64> <cert.b64> <triphase.xml> [extra.properties]

   presign  imprime el XML del TriphaseData por stdout y lo deja en presign.xml
   postsign deja el PDF firmado en postsign.pdf                                  */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

typedef int  (*create_isolate_fn)(void *, void **, void **);
typedef char *(*presign_fn)(void *, const char *, const char *, const char *, const char *);
typedef char *(*postsign_fn)(void *, const char *, const char *, const char *, const char *, const char *);
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

static void spit(const char *path, const char *data, size_t n) {
    FILE *f = fopen(path, "wb");
    if (!f) { fprintf(stderr, "no puedo escribir %s\n", path); exit(2); }
    fwrite(data, 1, n, f);
    fclose(f);
}

/* Decodificador Base64 minimo: el PDF firmado vuelve de Java en Base64. */
static size_t b64decode(const char *in, unsigned char *out) {
    static int t[256]; static int init = 0;
    if (!init) {
        const char *A = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for (int i = 0; i < 256; i++) { t[i] = -1; }
        for (int i = 0; i < 64; i++) { t[(unsigned char)A[i]] = i; }
        init = 1;
    }
    size_t o = 0; int val = 0, bits = -8;
    for (const char *p = in; *p; p++) {
        int c = t[(unsigned char)*p];
        if (c < 0) { continue; }
        val = (val << 6) | c; bits += 6;
        if (bits >= 0) { out[o++] = (unsigned char)((val >> bits) & 0xFF); bits -= 8; }
    }
    return o;
}

int main(int argc, char **argv) {
    if (argc < 5) {
        fprintf(stderr, "uso: loader <lib.so> presign|postsign <pdf.b64> <cert.b64> [...]\n");
        return 2;
    }
    const char *mode = argv[2];
    int is_post = strcmp(mode, "postsign") == 0;
    if (is_post && argc < 6) { fprintf(stderr, "postsign necesita <triphase.xml>\n"); return 2; }

    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "DLOPEN FALLO: %s\n", dlerror()); return 1; }
    printf("dlopen: OK\n");

    create_isolate_fn create = (create_isolate_fn) dlsym(h, "graal_create_isolate");
    free_fn freestr          = (free_fn)           dlsym(h, "rfirma_free_string");
    presign_fn presign       = (presign_fn)        dlsym(h, "rfirma_pades_presign");
    postsign_fn postsign     = (postsign_fn)       dlsym(h, "rfirma_pades_postsign");
    if (!create || !freestr || (is_post ? (void *) postsign == NULL : (void *) presign == NULL)) {
        fprintf(stderr, "DLSYM FALLO: %s\n", dlerror()); return 1;
    }
    printf("dlsym: OK\n");

    void *isolate = NULL, *thread = NULL;
    int rc = create(NULL, &isolate, &thread);
    if (rc != 0) { fprintf(stderr, "CREATE_ISOLATE FALLO rc=%d\n", rc); return 1; }
    printf("graal_create_isolate: OK\n");

    char *pdf  = slurp(argv[3]);
    char *cert = slurp(argv[4]);
    char *out;
    const char *label;

    if (is_post) {
        char *xml   = slurp(argv[5]);
        char *extra = (argc > 6) ? slurp(argv[6]) : (char *) "";
        label = "POSTSIGN";
        out = postsign(thread, pdf, "SHA256withRSA", cert, extra, xml);
    }
    else {
        char *extra = (argc > 5) ? slurp(argv[5]) : (char *) "";
        label = "PRESIGN";
        out = presign(thread, pdf, "SHA256withRSA", cert, extra);
    }

    if (!out) { fprintf(stderr, "%s devolvio NULL\n", label); return 1; }
    if (strncmp(out, "ERROR:", 6) == 0) {
        printf("%s ERROR\n%s\n", label, out);
        freestr(thread, out);
        return 3;
    }

    if (is_post) {
        size_t n = strlen(out);
        unsigned char *raw = malloc(n);
        size_t m = b64decode(out, raw);
        spit("postsign.pdf", (const char *) raw, m);
        printf("POSTSIGN OK (%zu bytes de PDF) -> postsign.pdf\n", m);
        printf("cabecera: %.8s\n", raw);
    }
    else {
        spit("presign.xml", out, strlen(out));
        printf("PRESIGN OK (%zu bytes) -> presign.xml\n", strlen(out));
        printf("---8<--- TriphaseData ---8<---\n%.1200s\n---8<---\n", out);
    }
    freestr(thread, out);
    return 0;
}
