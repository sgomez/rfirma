/* Arnes del sondeo #314: llama autofirma_filter_certificates por dlopen. */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void *graal_isolate_t;
typedef void *graal_isolatethread_t;

static char *slurp(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) { perror(path); exit(1); }
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *b = malloc(n + 1);
    if (fread(b, 1, n, f) != (size_t) n) { exit(1); }
    b[n] = 0;
    while (n > 0 && (b[n - 1] == '\n' || b[n - 1] == '\r')) { b[--n] = 0; }
    fclose(f);
    return b;
}

int main(int argc, char **argv) {
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }

    int (*create)(void *, graal_isolate_t **, graal_isolatethread_t **) =
        dlsym(h, "graal_create_isolate");
    char *(*filter)(graal_isolatethread_t *, const char *, const char *) =
        dlsym(h, "autofirma_filter_certificates");
    void (*freestr)(graal_isolatethread_t *, void *) = dlsym(h, "autofirma_free_string");
    if (!create || !filter || !freestr) {
        fprintf(stderr, "falta un simbolo: create=%p filter=%p free=%p\n",
                (void *) create, (void *) filter, (void *) freestr);
        return 1;
    }

    graal_isolate_t *iso = NULL;
    graal_isolatethread_t *th = NULL;
    if (create(NULL, &iso, &th) != 0) { fprintf(stderr, "isolate ko\n"); return 1; }

    char *a = slurp(argv[2]);
    char *b = slurp(argv[3]);
    char *chain = malloc(strlen(a) + strlen(b) + 2);
    sprintf(chain, "%s;%s", a, b);

    for (int i = 4; i < argc; i++) {
        char *out = filter(th, chain, argv[i]);
        printf("filters=%-40s -> %s\n", argv[i], out);
        freestr(th, out);
    }
    return 0;
}
