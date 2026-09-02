package es.gob.afirma.nativebridge;

import java.io.ByteArrayOutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.Signature;
import java.security.cert.X509Certificate;
import java.util.Base64;
import java.util.Properties;
import java.util.TreeSet;

import org.junit.jupiter.api.Test;

import com.aowagie.text.Document;
import com.aowagie.text.Paragraph;
import com.aowagie.text.pdf.PdfWriter;

/** Sondeo del #150: que hace el puente con signaturePages. Temporal. */
class SignaturePagesProbeTest {

    private static final String ALGORITHM = "SHA256withRSA";

    private static byte[] pdf(final int pages) throws Exception {
        final ByteArrayOutputStream out = new ByteArrayOutputStream();
        final Document document = new Document();
        PdfWriter.getInstance(document, out);
        document.open();
        for (int i = 1; i <= pages; i++) {
            if (i > 1) {
                document.newPage();
            }
            document.add(new Paragraph("Pagina " + i));
        }
        document.close();
        return out.toByteArray();
    }

    private static Properties visible(final String pagesValue) {
        final Properties p = new Properties();
        p.setProperty("signaturePositionOnPageLowerLeftX", "100");
        p.setProperty("signaturePositionOnPageLowerLeftY", "100");
        p.setProperty("signaturePositionOnPageUpperRightX", "300");
        p.setProperty("signaturePositionOnPageUpperRightY", "200");
        if (pagesValue != null) {
            p.setProperty("signaturePages", pagesValue);
        }
        return p;
    }

    private static void dump(final String label, final int pages, final String value) {
        System.out.println("### PROBE " + label + " pdf=" + pages + "p signaturePages=" + value);
        try {
            final byte[] doc = pdf(pages);
            final X509Certificate[] chain = TestFixtures.certificateChain();
            final Properties sent = visible(value);
            final PadesBridge.PreSignResult pre =
                    PadesBridge.preSign(doc, ALGORITHM, chain, sent);
            System.out.println("### PROBE " + label + " sent-after-call="
                    + new TreeSet<>(sent.stringPropertyNames())
                    + " signaturePages=" + sent.getProperty("signaturePages"));
            final SessionStamp stamp = SessionStamp.decode(pre.stamp());
            final Properties eff = stamp.extraParams();
            System.out.println("### PROBE " + label + " sealed keys="
                    + new TreeSet<>(eff.stringPropertyNames()));
            System.out.println("### PROBE " + label + " sealed signaturePages="
                    + eff.getProperty("signaturePages"));

            final Signature signature = Signature.getInstance(ALGORITHM);
            signature.initSign(TestFixtures.privateKey());
            signature.update(Base64.getDecoder().decode(pre.preSignB64()));
            final String pkcs1 = Base64.getEncoder().encodeToString(signature.sign());
            final byte[] signed =
                    PadesBridge.postSign(doc, chain, pre.stamp(), pre.session(), pkcs1);
            final Path out = Path.of("target", "probe-" + label + ".pdf");
            Files.createDirectories(out.getParent());
            Files.write(out, signed);
            System.out.println("### PROBE " + label + " OK -> " + out);
            System.out.println("### PROBE " + label + " widgets" + widgetPages(signed));
        }
        catch (final Throwable e) {
            System.out.println("### PROBE " + label + " THROWS "
                    + e.getClass().getName() + ": " + e.getMessage());
            Throwable c = e.getCause();
            while (c != null) {
                System.out.println("### PROBE " + label + "   caused by "
                        + c.getClass().getName() + ": " + c.getMessage());
                c = c.getCause();
            }
        }
    }

    /** Paginas del PDF firmado y cuantas anotaciones lleva cada una. */
    private static String widgetPages(final byte[] signed) throws Exception {
        final com.aowagie.text.pdf.PdfReader reader =
                new com.aowagie.text.pdf.PdfReader(signed);
        final StringBuilder sb = new StringBuilder();
        sb.append(" paginas=").append(reader.getNumberOfPages()).append(" annots=[");
        for (int i = 1; i <= reader.getNumberOfPages(); i++) {
            final com.aowagie.text.pdf.PdfArray annots =
                    reader.getPageN(i).getAsArray(com.aowagie.text.pdf.PdfName.ANNOTS);
            sb.append(i).append(":").append(annots == null ? 0 : annots.size())
                    .append("@").append(reader.getPageSize(i)).append(" ");
        }
        sb.append("] campos=")
                .append(reader.getAcroFields().getSignatureNames());
        for (final String name : reader.getAcroFields().getSignatureNames()) {
            sb.append(" rect(").append(name).append(")=")
                    .append(java.util.Arrays.toString(reader.getAcroFields().getFieldPositions(name)));
        }
        reader.close();
        return sb.toString();
    }

    @Test
    void probe() throws Exception {
        dump("sin-parametro", 3, null);
        dump("all", 3, "all");
        dump("dos", 3, "2");
        dump("rango", 3, "1-3,-3--1");
        dump("ultima", 3, "-1");
        dump("append", 3, "append");
        dump("fuera-de-rango", 3, "99");
        dump("fuera-de-rango-rango", 3, "2-99");
        dump("basura", 3, "pepe");
        dump("cero", 3, "0");
        dumpSingular("singular-1", 3, "1", null);
        dumpSingular("singular-99", 3, "99", null);
        dumpSingular("singular-0", 3, "0", null);
        dumpSingular("singular-menos1", 3, "-1", null);
        dumpSingular("singular-basura", 3, "pepe", null);
        dumpSingular("ambos-1-y-3", 3, "1", "3");
        dumpSingular("ambos-1-y-all", 3, "1", "all");
        sessionOf("sesion-99", 3, "99");
        sessionOf("sesion-2", 3, "2");
        mixed("mixta-all", "all");
        mixed("mixta-solo-pequena", "2");
    }

    /** PDF con pagina 1 A4 y pagina 2 diminuta: el recuadro solo cabe en la 1. */
    private static void mixed(final String label, final String plural) {
        System.out.println("### PROBE " + label + " signaturePages=" + plural);
        try {
            final ByteArrayOutputStream out = new ByteArrayOutputStream();
            final Document document = new Document(com.aowagie.text.PageSize.A4);
            PdfWriter.getInstance(document, out);
            document.open();
            document.add(new Paragraph("Pagina A4"));
            document.setPageSize(new com.aowagie.text.Rectangle(200, 200));
            document.newPage();
            document.add(new Paragraph("Pagina pequena"));
            document.close();
            final byte[] doc = out.toByteArray();

            final X509Certificate[] chain = TestFixtures.certificateChain();
            final PadesBridge.PreSignResult pre =
                    PadesBridge.preSign(doc, ALGORITHM, chain, visible(plural));
            final Signature signature = Signature.getInstance(ALGORITHM);
            signature.initSign(TestFixtures.privateKey());
            signature.update(Base64.getDecoder().decode(pre.preSignB64()));
            final String pkcs1 = Base64.getEncoder().encodeToString(signature.sign());
            final byte[] signed =
                    PadesBridge.postSign(doc, chain, pre.stamp(), pre.session(), pkcs1);
            System.out.println("### PROBE " + label + " widgets" + widgetPages(signed));
        }
        catch (final Throwable e) {
            System.out.println("### PROBE " + label + " THROWS "
                    + e.getClass().getName() + ": " + e.getMessage());
        }
    }

    /** Que devuelve la prefirma en la sesion: hay o no rastro de la pagina. */
    private static void sessionOf(final String label, final int pages, final String plural)
            throws Exception {
        final byte[] doc = pdf(pages);
        final PadesBridge.PreSignResult pre = PadesBridge.preSign(doc, ALGORITHM,
                TestFixtures.certificateChain(), visible(plural));
        final String session = pre.session().replaceAll("(?s)<param n=\"PRE\">.*?</param>",
                "<param n=\"PRE\">…</param>");
        System.out.println("### PROBE " + label + " session=" + session.replace("\n", " "));
    }

    /** Con {@code signaturePage} (singular, el que envia rfirma hoy). */
    private static void dumpSingular(final String label, final int pages,
            final String singular, final String plural) {
        System.out.println("### PROBE " + label + " pdf=" + pages
                + "p signaturePage=" + singular + " signaturePages=" + plural);
        try {
            final byte[] doc = pdf(pages);
            final X509Certificate[] chain = TestFixtures.certificateChain();
            final Properties sent = visible(plural);
            sent.setProperty("signaturePage", singular);
            final PadesBridge.PreSignResult pre =
                    PadesBridge.preSign(doc, ALGORITHM, chain, sent);
            final SessionStamp stamp = SessionStamp.decode(pre.stamp());
            System.out.println("### PROBE " + label + " sealed keys="
                    + new TreeSet<>(stamp.extraParams().stringPropertyNames()));
            final Signature signature = Signature.getInstance(ALGORITHM);
            signature.initSign(TestFixtures.privateKey());
            signature.update(Base64.getDecoder().decode(pre.preSignB64()));
            final String pkcs1 = Base64.getEncoder().encodeToString(signature.sign());
            final byte[] signed =
                    PadesBridge.postSign(doc, chain, pre.stamp(), pre.session(), pkcs1);
            System.out.println("### PROBE " + label + " widgets" + widgetPages(signed));
        }
        catch (final Throwable e) {
            System.out.println("### PROBE " + label + " THROWS "
                    + e.getClass().getName() + ": " + e.getMessage());
        }
    }
}
