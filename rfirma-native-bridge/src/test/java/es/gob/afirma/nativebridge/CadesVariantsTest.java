package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayInputStream;
import java.util.Arrays;
import java.util.List;
import java.util.Properties;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

import org.junit.jupiter.api.Test;

import es.gob.afirma.core.signers.asic.ASiCUtil;
import es.gob.afirma.signers.cades.AOCAdESSigner;
import es.gob.afirma.signvalidation.SignValidity;

/**
 * El ciclo trifasico completo de {@code CAdES-ASiC-S}: el contenedor que sale,
 * lo que lleva dentro y las dos multifirmas que el procesador del original no
 * atiende.
 */
class CadesVariantsTest {

    private static final byte[] ZIP_LOCAL_FILE_HEADER = {'P', 'K', 3, 4};
    private static final String ASIC_MIME_TYPE_ENTRY = "mimetype";
    private static final String ASIC_META_INF = "META-INF/";

    @Test
    void signs_asic_s_into_a_zip_container_with_the_cades_signature_of_the_original()
            throws Exception {
        final byte[] container = CadesCycle.sign(TestFixtures.challenge(), asicS(), "sign");

        assertArrayEquals(ZIP_LOCAL_FILE_HEADER, Arrays.copyOf(container, 4),
                "un ASiC-S es un ZIP y empieza por su firma de fichero local");
        assertTrue(new AOCAdESSigner().isSign(ASiCUtil.getASiCSBinarySignature(container)),
                "el firmador CAdES del original no reconoce la firma que viaja en el contenedor");
    }

    @Test
    void the_container_carries_the_document_and_the_cades_signature_that_covers_it()
            throws Exception {
        final byte[] document = TestFixtures.challenge();

        final byte[] container = CadesCycle.sign(document, asicS(), "sign");

        assertArrayEquals(document, packagedDataOf(container),
                "el ASiC-S lleva el documento tal cual dentro del contenedor");
        final List<SignValidity> verdicts =
                CadesCycle.validate(ASiCUtil.getASiCSBinarySignature(container));
        assertTrue(CadesCycle.isValid(verdicts),
                "el validador del original no da por valida la firma del contenedor: " + verdicts);
    }

    @Test
    void seals_the_variant_that_was_asked_for() throws Exception {
        final SessionStamp stamp = SessionStamp
                .decode(CadesCycle.preSign(TestFixtures.challenge(), asicS(), "sign").stamp());

        assertEquals("CAdES-ASiC-S", stamp.extraParams().getProperty("format"),
                "la variante que se firmo se sella, no se deduce en la postfirma");
    }

    @Test
    void without_a_format_the_signature_is_the_plain_cades_of_before() throws Exception {
        final byte[] signed = CadesCycle.sign(TestFixtures.challenge(), new Properties(), "sign");

        assertTrue(new AOCAdESSigner().isSign(signed),
                "sin format sigue saliendo un CMS CAdES, no un contenedor");
    }

    @Test
    void reads_the_container_format_ignoring_the_blanks_around_it() throws Exception {
        final byte[] container =
                CadesCycle.sign(TestFixtures.challenge(), variant("  CAdES-ASiC-S  "), "sign");

        assertArrayEquals(ZIP_LOCAL_FILE_HEADER, Arrays.copyOf(container, 4),
                "un format con espacios alrededor sigue pidiendo el contenedor");
    }

    @Test
    void refuses_to_cosign_an_asic_s_container_with_the_code_of_the_original() {
        assertThrows(UnsupportedOperationException.class,
                () -> CadesCycle.preSign(TestFixtures.challenge(), asicS(), "cosign"));
    }

    @Test
    void refuses_to_countersign_an_asic_s_container_with_the_code_of_the_original() {
        assertThrows(UnsupportedOperationException.class,
                () -> CadesCycle.preSign(TestFixtures.challenge(), asicS(), "countersign"));
    }

    /** Lo unico que el ASiC-S guarda fuera de {@code META-INF/} y del {@code mimetype}. */
    private static byte[] packagedDataOf(final byte[] container) throws Exception {
        try (ZipInputStream zip = new ZipInputStream(new ByteArrayInputStream(container))) {
            ZipEntry entry;
            while ((entry = zip.getNextEntry()) != null) {
                if (!entry.getName().startsWith(ASIC_META_INF)
                        && !ASIC_MIME_TYPE_ENTRY.equals(entry.getName())) {
                    return zip.readAllBytes();
                }
            }
        }
        return null;
    }

    private static Properties asicS() {
        return variant("CAdES-ASiC-S");
    }

    private static Properties variant(final String format) {
        final Properties params = new Properties();
        params.setProperty("format", format);
        return params;
    }
}
