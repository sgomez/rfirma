package es.gob.afirma.nativebridge;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Properties;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

import es.gob.afirma.core.AOFormatFileException;
import es.gob.afirma.core.AOInvalidFormatException;
import es.gob.afirma.signers.pades.InvalidPdfException;

/** Grada A: cada rechazo de los datos viaja con la clase que el lanzador del original traduce a su codigo. */
class DataRejectionKindTest {

    private static byte[] reference(final String name) throws Exception {
        return Files.readAllBytes(Path.of("..", "testdata", "reference", name));
    }

    private static Properties withFormat(final String format) {
        final Properties params = new Properties();
        params.setProperty("format", format);
        return params;
    }

    private static String kindOfFailure(final Executable operation) {
        return NativeBridge.kindOf(assertThrows(Exception.class, operation));
    }

    @Test
    void an_invoice_that_is_already_signed_travels_as_facturae_already_signed() {
        assertEquals("facturaeAlreadySigned", kindOfFailure(
                () -> XadesCycle.preSign(reference("facturae.xsig"), withFormat("FacturaE"), "sign")));
    }

    @Test
    void xml_that_is_no_invoice_travels_as_invalid_facturae() {
        assertEquals("invalidFacturae", kindOfFailure(
                () -> XadesCycle.preSign(reference("document.xml"), withFormat("FacturaE"), "sign")));
    }

    @Test
    void cosigning_an_explicit_signature_without_a_matching_digest_travels_as_sign_without_data() {
        assertEquals("signWithoutData", kindOfFailure(() -> CadesBridge.preSign(
                reference("cades-explicit.p7s"), "SHA512withRSA",
                TestFixtures.certificateChain(), new Properties(), "cosign")));
    }

    @Test
    void cosigning_data_that_is_no_xades_signature_travels_as_no_sign_data() {
        assertEquals("noSignData", kindOfFailure(
                () -> XadesCycle.preSign(reference("challenge.bin"), new Properties(), "cosign")));
    }

    @Test
    void countersigning_data_that_is_no_xades_signature_travels_as_no_sign_data() {
        assertEquals("noSignData", kindOfFailure(
                () -> XadesCycle.preSign(reference("challenge.bin"), new Properties(), "countersign")));
    }

    @Test
    void an_enveloped_xades_over_data_that_is_no_xml_travels_as_invalid_xml() {
        assertEquals("invalidXml", kindOfFailure(() -> XadesCycle.preSign(
                reference("challenge.bin"), withFormat("XAdES Enveloped"), "sign")));
    }

    @Test
    void a_file_format_violation_travels_as_invalid_data() {
        assertEquals("invalidData",
                NativeBridge.kindOf(new AOFormatFileException("los datos no casan con el formato")));
    }

    @Test
    void any_other_invalid_format_travels_as_no_sign_data() {
        assertEquals("noSignData",
                NativeBridge.kindOf(new AOInvalidFormatException("no es una firma")));
    }

    @Test
    void a_pdf_the_signer_cannot_read_travels_as_invalid_pdf_and_not_as_invalid_data() {
        assertEquals("invalidPdf", NativeBridge.kindOf(new InvalidPdfException("no es un PDF")));
    }

    @Test
    void the_rejection_is_found_through_the_causes() {
        assertEquals("invalidData", NativeBridge.kindOf(new IllegalStateException(
                "no se ha podido prefirmar", new AOFormatFileException("formato"))));
    }
}
