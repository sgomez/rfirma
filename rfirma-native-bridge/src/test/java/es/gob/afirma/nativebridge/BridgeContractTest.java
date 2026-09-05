package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.InputStream;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.TreeSet;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import org.graalvm.nativeimage.c.function.CEntryPoint;
import org.junit.jupiter.api.Test;

import es.gob.afirma.keystores.filters.CertFilterManager;

/**
 * Grada A: las invariantes del puente que no son comportamiento y que un
 * cambio bienintencionado rompe sin que falle nada mas.
 *
 * <p>Son caras justamente porque su modo de fallo no es una excepcion: un
 * {@code toCString} en un valor de retorno corrompe memoria bajo carga, y
 * reintroducir {@code afirma-ui-utils} hace que un JPEG con perfil ICC aborte el
 * proceso entero. Ninguna de las dos cosas se nota probando una firma.
 */
class BridgeContractTest {

    private static final Path SOURCES = Path.of("src", "main", "java");

    @Test
    void exposes_exactly_the_entry_points_rust_links_against() {
        final Set<String> names = new TreeSet<>();
        for (final Method method : NativeBridge.class.getDeclaredMethods()) {
            final CEntryPoint entryPoint = method.getAnnotation(CEntryPoint.class);
            if (entryPoint != null) {
                names.add(entryPoint.name());
            }
        }

        assertEquals(
                Set.of("autofirma_expand_extra_params", "autofirma_filter_certificates",
                        "autofirma_free_string", "autofirma_pades_postsign",
                        "autofirma_pades_presign"),
                names,
                "cambiar un nombre aqui rompe el enlace de Rust en tiempo de ejecucion, no de compilacion");
    }

    @Test
    void never_returns_a_string_allocated_by_ctypeconversion() throws Exception {
        // ADR-0003 / ID-11: GraalVM libera la memoria de CTypeConversion.toCString
        // al salir del bloque, asi que devolverla a Rust es un puntero colgante y
        // un doble free. Se reserva a mano con UnmanagedMemory.malloc y libera
        // Rust. Como el fallo es silencioso, la regla se comprueba sobre el texto.
        try (Stream<Path> sources = Files.walk(SOURCES)) {
            final List<Path> offenders = sources
                    .filter(p -> p.toString().endsWith(".java"))
                    .filter(BridgeContractTest::usesToCString)
                    .collect(Collectors.toList());
            assertTrue(offenders.isEmpty(),
                    "CTypeConversion.toCString no puede aparecer en el puente: " + offenders);
        }
    }

    private static boolean usesToCString(final Path source) {
        try {
            return Files.readString(source).contains("toCString(");
        }
        catch (final Exception e) {
            throw new IllegalStateException("no se pudo leer " + source, e);
        }
    }

    @Test
    void does_not_carry_afirma_ui_utils_on_the_classpath() {
        // ADR-0012 / ID-08: la exclusion del pom es lo que deja la imagen en un
        // solo fichero. Si vuelve, PdfPreProcessor.getImage:304 encuentra la clase
        // por reflexion, javax.imageio vuelve a ser alcanzable y con ella los
        // cinco auxiliares de AWT.
        assertThrows(ClassNotFoundException.class,
                () -> Class.forName("es.gob.afirma.ui.utils.ImageUtils"),
                "afirma-ui-utils ha vuelto al classpath: revisa las exclusiones del pom.xml");
    }

    /**
     * TD-56, primera mitad: {@code afirma-keystores-filters} entra al puente
     * (ID-252) <b>sin</b> traerse con el la fabrica de gestores de almacen.
     *
     * <p>{@code AOKeyStoreManagerFactory} era falsa alarma (ID-262): quien lo
     * nombra en el modulo original son <b>dos de sus pruebas</b>, no su
     * {@code src/main}. Como la clase si esta en el classpath —viene de
     * {@code afirma-core-keystores}—, un {@code Class.forName} no diria nada;
     * lo que se comprueba es que ninguna clase del jar de los filtros la nombre
     * en su pool de constantes, que es lo que la haria alcanzable dentro de la
     * imagen nativa y arrastraria con ella el arbol de almacenes.
     */
    @Test
    void the_borrowed_filter_engine_does_not_reach_the_keystore_manager_factory() throws Exception {
        final Path jar = Path.of(CertFilterManager.class.getProtectionDomain()
                .getCodeSource().getLocation().toURI());
        assertTrue(jar.getFileName().toString().startsWith("afirma-keystores-filters"),
                "se esperaba el jar de los filtros y no " + jar);

        final List<String> offenders = new ArrayList<>();
        try (JarFile classes = new JarFile(jar.toFile())) {
            for (final JarEntry entry : classes.stream().toList()) {
                if (!entry.getName().endsWith(".class")) {
                    continue;
                }
                try (InputStream in = classes.getInputStream(entry)) {
                    if (new String(in.readAllBytes(), StandardCharsets.ISO_8859_1)
                            .contains("AOKeyStoreManagerFactory")) {
                        offenders.add(entry.getName());
                    }
                }
            }
        }

        assertTrue(offenders.isEmpty(),
                "el motor de filtros ha empezado a nombrar AOKeyStoreManagerFactory: " + offenders);
    }

    /**
     * TD-56, segunda mitad: {@code libawt.so} no vuelve al paquete.
     *
     * <p>La invariante del ADR-0012 la hace cumplir
     * {@code packaging/verifica-contenido.sh} sobre los tres formatos, y lo que
     * esta prueba impide es que esa comprobacion desaparezca en silencio: si el
     * guion deja de buscar {@code libawt.so}, el paquete pasaria a verde con los
     * auxiliares dentro y un JPEG con perfil ICC abortaria el proceso entero.
     */
    @Test
    void the_packaging_check_still_refuses_libawt() throws Exception {
        final Path script = Path.of("..", "packaging", "verifica-contenido.sh");
        final String contents = Files.readString(script);

        assertTrue(contents.contains("-name 'libawt.so'"),
                script + " ha dejado de buscar libawt.so: la invariante del ADR-0012 se ha quedado "
                        + "sin quien la haga cumplir");
        assertTrue(contents.contains("-name 'librfirma_crypto.so'"),
                script + " ha dejado de contar los librfirma_crypto.so");
    }

    @Test
    void ships_the_native_image_metadata_on_the_classpath() throws Exception {
        // ID-06: los metadatos van VERSIONADOS y native-image los recoge del
        // classpath sin bandera. Si desaparecen, la imagen se sigue construyendo
        // —con otras opciones— y el fallo aparece en tiempo de ejecucion.
        final String resource =
                "META-INF/native-image/es.gob.afirma/rfirma-native-bridge/native-image.properties";
        try (InputStream in = NativeBridge.class.getClassLoader().getResourceAsStream(resource)) {
            assertNotNull(in, "faltan los metadatos de native-image en " + resource);
            final String content = new String(in.readAllBytes(), StandardCharsets.UTF_8);
            assertTrue(content.contains("-H:Name=librfirma_crypto"),
                    "el nombre de la libreria tiene que salir de aqui, no del justfile");
            assertTrue(content.contains("--no-fallback"),
                    "sin --no-fallback native-image produce una imagen de reserva que arranca una JVM");
            assertTrue(content.contains("com/lowagie/text/pdf/fonts/"),
                    "sin los .afm de iText la rubrica revienta con «Courier not found as resource»");
        }
    }
}
