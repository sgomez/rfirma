import java.io.ByteArrayOutputStream;
import java.io.FileInputStream;
import java.io.InputStream;
import java.math.BigInteger;
import java.nio.file.Files;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.KeyStore;
import java.security.MessageDigest;
import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.time.Duration;
import java.time.Instant;
import java.util.Arrays;
import java.util.Date;
import java.util.GregorianCalendar;
import java.util.HexFormat;
import java.util.List;
import java.util.Properties;
import java.util.function.UnaryOperator;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import org.bouncycastle.asn1.ASN1EncodableVector;
import org.bouncycastle.asn1.ASN1ObjectIdentifier;
import org.bouncycastle.asn1.DERSet;
import org.bouncycastle.asn1.cms.Attribute;
import org.bouncycastle.asn1.cms.AttributeTable;
import org.bouncycastle.asn1.oiw.OIWObjectIdentifiers;
import org.bouncycastle.asn1.pkcs.PKCSObjectIdentifiers;
import org.bouncycastle.asn1.x500.X500Name;
import org.bouncycastle.asn1.x509.AlgorithmIdentifier;
import org.bouncycastle.asn1.x509.ExtendedKeyUsage;
import org.bouncycastle.asn1.x509.Extension;
import org.bouncycastle.asn1.x509.KeyPurposeId;
import org.bouncycastle.cert.jcajce.JcaCertStore;
import org.bouncycastle.cert.jcajce.JcaX509CertificateConverter;
import org.bouncycastle.cert.jcajce.JcaX509v3CertificateBuilder;
import org.bouncycastle.cms.CMSSignedData;
import org.bouncycastle.cms.SignerInformation;
import org.bouncycastle.cms.SignerInformationStore;
import org.bouncycastle.cms.jcajce.JcaSimpleSignerInfoGeneratorBuilder;
import org.bouncycastle.operator.jcajce.JcaContentSignerBuilder;
import org.bouncycastle.operator.jcajce.JcaDigestCalculatorProviderBuilder;
import org.bouncycastle.tsp.TSPAlgorithms;
import org.bouncycastle.tsp.TimeStampRequestGenerator;
import org.bouncycastle.tsp.TimeStampToken;
import org.bouncycastle.tsp.TimeStampTokenGenerator;
import org.spongycastle.asn1.ASN1InputStream;

import com.aowagie.text.Document;
import com.aowagie.text.Paragraph;
import com.aowagie.text.Rectangle;
import com.aowagie.text.pdf.PdfAnnotation;
import com.aowagie.text.pdf.PdfFormField;
import com.aowagie.text.pdf.PdfName;
import com.aowagie.text.pdf.PdfWriter;

import es.gob.afirma.core.signers.AOSignConstants;
import es.gob.afirma.core.signers.CounterSignTarget;
import es.gob.afirma.signers.cades.AOCAdESSigner;
import es.gob.afirma.signers.pades.AOPDFSigner;
import es.gob.afirma.signers.tsp.pkcs7.CMSTimestamper;
import es.gob.afirma.signers.tsp.pkcs7.TsaParams;
import es.gob.afirma.signers.xades.AOFacturaESigner;
import es.gob.afirma.signers.xades.AOXAdESSigner;

/** Firma con los firmadores monofasicos del 1.9.2 para producir testdata/reference/ y testdata/previous-signatures/. */
public final class ReferenceSigner {

    private static final String ALGORITHM = "SHA256withRSA";

    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            usageAndExit();
        }
        switch (args[0]) {
            case "cades" -> cades(args);
            case "xades" -> xades(args);
            case "xades-extra-certificate" -> xadesExtraCertificate(args);
            case "xades-foreign-key" -> xadesForeignKey(args);
            case "facturae" -> facturae(args);
            case "pdf" -> pdf(args);
            case "pades" -> pades(args);
            case "pades-timestamped" -> padesTimestamped(args);
            case "pades-stamped-at" -> padesStampedAt(args);
            case "cosign" -> cosign(args);
            case "countersign" -> countersign(args);
            default -> usageAndExit();
        }
    }

    private static void usageAndExit() {
        System.err.println("""
                Uso:
                  ReferenceSigner cades <implicit|explicit> <entrada> <p12> <pin> <salida>
                  ReferenceSigner xades <detached|enveloping|enveloped> <entrada.xml> <p12> <pin> <salida>
                  ReferenceSigner xades-extra-certificate <entrada.xml> <p12> <pin> <cert.pem> <salida>
                  ReferenceSigner xades-foreign-key <entrada.xml> <p12> <pin> <cert.pem> <salida>
                  ReferenceSigner facturae <invoice.xml> <p12> <pin> <salida>
                  ReferenceSigner pdf <salida> [campo de firma vacio]
                  ReferenceSigner pades <entrada.pdf> <p12> <pin> <salida> [clave=valor ...]
                  ReferenceSigner pades-timestamped <entrada.pdf> <p12> <pin> <tsaURL> <salida>
                  ReferenceSigner pades-stamped-at <entrada.pdf> <p12> <pin> <instante ISO-8601> <salida>
                  ReferenceSigner cosign <cades|xades> <firma-origen> <p12> <pin> <salida>
                  ReferenceSigner countersign <cades|xades> <tree|leafs> <firma-origen> <p12> <pin> <salida>
                """);
        System.exit(2);
    }

    private static void cades(String[] args) throws Exception {
        Properties extraParams = new Properties();
        extraParams.setProperty("mode", args[1]);
        byte[] data = Files.readAllBytes(Path.of(args[2]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[3], args[4]);
        byte[] result = new AOCAdESSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        Files.write(Path.of(args[5]), result);
    }

    private static void xades(String[] args) throws Exception {
        Properties extraParams = new Properties();
        extraParams.setProperty("format", xadesFormat(args[1]));
        byte[] data = Files.readAllBytes(Path.of(args[2]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[3], args[4]);
        byte[] result = new AOXAdESSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        Files.write(Path.of(args[5]), result);
    }

    /** XAdES Enveloping con un certificado mas al final de la cadena, que acaba en el KeyInfo. */
    private static void xadesExtraCertificate(String[] args) throws Exception {
        Properties extraParams = new Properties();
        extraParams.setProperty("format", AOSignConstants.SIGN_FORMAT_XADES_ENVELOPING);
        byte[] data = Files.readAllBytes(Path.of(args[1]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[2], args[3]);
        Certificate extra;
        try (InputStream in = new FileInputStream(args[4])) {
            extra = CertificateFactory.getInstance("X.509").generateCertificate(in);
        }
        Certificate[] chain = pke.getCertificateChain();
        Certificate[] longer = Arrays.copyOf(chain, chain.length + 1);
        longer[chain.length] = extra;
        byte[] result = new AOXAdESSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), longer, extraParams);
        Files.write(Path.of(args[5]), result);
    }

    // Una XAdES firmada con la clave del p12, con su KeyValue delante del KeyInfo y el certificado
    // de <cert.pem> en X509Data: el KeyValueKeySelector del original comprueba con el KeyValue y da
    // la firma por buena, pero enseña como firmante al titular de <cert.pem>.
    private static void xadesForeignKey(String[] args) throws Exception {
        Properties extraParams = new Properties();
        extraParams.setProperty("format", AOSignConstants.SIGN_FORMAT_XADES_ENVELOPING);
        extraParams.setProperty("addKeyInfoKeyValue", "true");
        extraParams.setProperty("keepKeyInfoUnsigned", "true");
        byte[] data = Files.readAllBytes(Path.of(args[1]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[2], args[3]);
        byte[] signed = new AOXAdESSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);

        Certificate foreign;
        try (InputStream in = new FileInputStream(args[4])) {
            foreign = CertificateFactory.getInstance("X.509").generateCertificate(in);
        }

        javax.xml.parsers.DocumentBuilderFactory factory =
                javax.xml.parsers.DocumentBuilderFactory.newInstance();
        factory.setNamespaceAware(true);
        org.w3c.dom.Document xml = factory.newDocumentBuilder()
                .parse(new java.io.ByteArrayInputStream(signed));
        String ns = "http://www.w3.org/2000/09/xmldsig#";
        org.w3c.dom.Element signature =
                (org.w3c.dom.Element) xml.getElementsByTagNameNS(ns, "Signature").item(0);
        org.w3c.dom.Element keyInfo = firstChild(signature, ns, "KeyInfo");
        org.w3c.dom.Element x509Data = firstChild(keyInfo, ns, "X509Data");
        keyInfo.insertBefore(firstChild(keyInfo, ns, "KeyValue"), x509Data);
        org.w3c.dom.Element certificate =
                (org.w3c.dom.Element) x509Data.getElementsByTagNameNS(ns, "X509Certificate").item(0);
        certificate.setTextContent(java.util.Base64.getEncoder().encodeToString(foreign.getEncoded()));

        org.w3c.dom.ls.DOMImplementationLS ls =
                (org.w3c.dom.ls.DOMImplementationLS) xml.getImplementation().getFeature("LS", "3.0");
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        org.w3c.dom.ls.LSOutput output = ls.createLSOutput();
        output.setByteStream(out);
        output.setEncoding("UTF-8");
        ls.createLSSerializer().write(xml, output);
        Files.write(Path.of(args[5]), out.toByteArray());
    }

    private static org.w3c.dom.Element firstChild(
            org.w3c.dom.Element parent, String ns, String localName) {
        org.w3c.dom.NodeList children = parent.getChildNodes();
        for (int i = 0; i < children.getLength(); i++) {
            org.w3c.dom.Node child = children.item(i);
            if (child instanceof org.w3c.dom.Element element
                    && ns.equals(element.getNamespaceURI())
                    && localName.equals(element.getLocalName())) {
                return element;
            }
        }
        return null;
    }

    private static String xadesFormat(String mode) {
        return switch (mode) {
            case "detached" -> AOSignConstants.SIGN_FORMAT_XADES_DETACHED;
            case "enveloping" -> AOSignConstants.SIGN_FORMAT_XADES_ENVELOPING;
            case "enveloped" -> AOSignConstants.SIGN_FORMAT_XADES_ENVELOPED;
            default -> throw new IllegalArgumentException("formato XAdES desconocido: " + mode);
        };
    }

    private static void facturae(String[] args) throws Exception {
        byte[] data = Files.readAllBytes(Path.of(args[1]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[2], args[3]);
        byte[] result = new AOFacturaESigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), new Properties());
        Files.write(Path.of(args[4]), result);
    }

    private static void pdf(String[] args) throws Exception {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        Document document = new Document();
        PdfWriter writer = PdfWriter.getInstance(document, out);
        document.open();
        document.add(new Paragraph("Documento de prueba de rfirma."));
        if (args.length > 2) {
            PdfFormField field = PdfFormField.createSignature(writer);
            field.setWidget(new Rectangle(100, 600, 300, 700), new PdfName("I"));
            field.setFieldName(args[2]);
            field.setFlags(PdfAnnotation.FLAGS_PRINT);
            field.setPage(1);
            writer.addAnnotation(field);
        }
        document.close();
        Files.write(Path.of(args[1]), out.toByteArray());
    }

    private static void padesTimestamped(String[] args) throws Exception {
        byte[] data = Files.readAllBytes(Path.of(args[1]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[2], args[3]);
        Properties extraParams = new Properties();
        extraParams.setProperty("headless", "true");
        byte[] signed = new AOPDFSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        Properties tsaParams = new Properties();
        tsaParams.setProperty("tsaURL", args[4]);
        tsaParams.setProperty("tsaPolicy", "0.4.0.2023.1.1");
        tsaParams.setProperty("tsaHashAlgorithm", "SHA-256");
        Files.write(Path.of(args[5]), withSignatureTimestamp(signed, new TsaParams(tsaParams)));
    }

    private static void pades(String[] args) throws Exception {
        byte[] data = Files.readAllBytes(Path.of(args[1]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[2], args[3]);
        Properties extraParams = new Properties();
        extraParams.setProperty("headless", "true");
        for (int i = 5; i < args.length; i++) {
            String[] pair = args[i].split("=", 2);
            extraParams.setProperty(pair[0], pair[1]);
        }
        Files.write(Path.of(args[4]), new AOPDFSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams));
    }

    private static void padesStampedAt(String[] args) throws Exception {
        byte[] data = Files.readAllBytes(Path.of(args[1]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[2], args[3]);
        Properties extraParams = new Properties();
        extraParams.setProperty("headless", "true");
        byte[] signed = new AOPDFSigner().sign(
                data, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
        Date genTime = Date.from(Instant.parse(args[4]));
        Files.write(Path.of(args[5]),
                withContainerReplaced(signed, cms -> stampedAt(cms, genTime)));
    }

    // Un sello con la fecha que se le pida, firmado por una TSA de un solo uso en vigor en esa fecha.
    private static byte[] stampedAt(byte[] cms, Date genTime) {
        try {
            CMSSignedData signed = new CMSSignedData(cms);
            SignerInformation signer = signed.getSignerInfos().getSigners().iterator().next();
            byte[] imprint = MessageDigest.getInstance("SHA-256").digest(signer.getSignature());
            KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
            generator.initialize(2048);
            KeyPair keys = generator.generateKeyPair();
            X509Certificate authority = timestampingCertificate(keys, genTime);
            TimeStampTokenGenerator tokens = new TimeStampTokenGenerator(
                    new JcaSimpleSignerInfoGeneratorBuilder()
                            .build(ALGORITHM, keys.getPrivate(), authority),
                    new JcaDigestCalculatorProviderBuilder().build()
                            .get(new AlgorithmIdentifier(OIWObjectIdentifiers.idSHA1)),
                    new ASN1ObjectIdentifier("0.4.0.2023.1.1"));
            tokens.addCertificates(new JcaCertStore(List.of(authority)));
            TimeStampRequestGenerator request = new TimeStampRequestGenerator();
            request.setCertReq(true);
            TimeStampToken token = tokens.generate(
                    request.generate(TSPAlgorithms.SHA256, imprint), BigInteger.ONE, genTime);
            ASN1EncodableVector unsigned = new ASN1EncodableVector();
            unsigned.add(new Attribute(PKCSObjectIdentifiers.id_aa_signatureTimeStampToken,
                    new DERSet(token.toCMSSignedData().toASN1Structure())));
            SignerInformation stamped =
                    SignerInformation.replaceUnsignedAttributes(signer, new AttributeTable(unsigned));
            return CMSSignedData.replaceSigners(signed, new SignerInformationStore(stamped))
                    .getEncoded("DER");
        }
        catch (Exception e) {
            throw new IllegalStateException(e);
        }
    }

    private static X509Certificate timestampingCertificate(KeyPair keys, Date genTime)
            throws Exception {
        X500Name name = new X500Name("CN=rfirma backdated TSA");
        Instant at = genTime.toInstant();
        JcaX509v3CertificateBuilder builder = new JcaX509v3CertificateBuilder(name, BigInteger.ONE,
                Date.from(at.minus(Duration.ofDays(1))), Date.from(at.plus(Duration.ofDays(3650))),
                name, keys.getPublic());
        builder.addExtension(Extension.extendedKeyUsage, true,
                new ExtendedKeyUsage(KeyPurposeId.id_kp_timeStamping));
        return new JcaX509CertificateConverter().getCertificate(
                builder.build(new JcaContentSignerBuilder(ALGORITHM).build(keys.getPrivate())));
    }

    // El PdfTimestamper del 1.9.2 devuelve la firma sin sello: se sella el CMS en su
    // hueco de /Contents, fuera del ByteRange.
    private static byte[] withSignatureTimestamp(byte[] pdf, TsaParams tsa) throws Exception {
        return withContainerReplaced(pdf, cms -> {
            try {
                return new CMSTimestamper(tsa)
                        .addTimestamp(cms, tsa.getTsaHashAlgorithm(), new GregorianCalendar());
            }
            catch (Exception e) {
                throw new IllegalStateException(e);
            }
        });
    }

    private static byte[] withContainerReplaced(byte[] pdf, UnaryOperator<byte[]> replacement)
            throws Exception {
        String text = new String(pdf, StandardCharsets.ISO_8859_1);
        Matcher byteRange = Pattern
                .compile("/ByteRange\\s*\\[\\s*(\\d+)\\s+(\\d+)\\s+(\\d+)\\s+(\\d+)\\s*\\]")
                .matcher(text);
        if (!byteRange.find()) {
            throw new IllegalStateException("el PDF firmado no trae ByteRange");
        }
        int open = Integer.parseInt(byteRange.group(2));
        int close = Integer.parseInt(byteRange.group(3)) - 1;
        byte[] container = HexFormat.of().parseHex(text.substring(open + 1, close));
        byte[] cms;
        try (ASN1InputStream in = new ASN1InputStream(container)) {
            cms = in.readObject().getEncoded("DER");
        }
        byte[] stamped = replacement.apply(cms);
        String hex = HexFormat.of().formatHex(stamped);
        int room = close - open - 1;
        if (hex.length() > room) {
            throw new IllegalStateException(
                    "el sello no cabe en /Contents: " + hex.length() + " > " + room);
        }
        byte[] result = pdf.clone();
        byte[] padded = (hex + "0".repeat(room - hex.length())).getBytes(StandardCharsets.US_ASCII);
        System.arraycopy(padded, 0, result, open + 1, room);
        return result;
    }

    private static void cosign(String[] args) throws Exception {
        byte[] sign = Files.readAllBytes(Path.of(args[2]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[3], args[4]);
        Properties extraParams = new Properties();
        byte[] result = switch (args[1]) {
            case "cades" -> new AOCAdESSigner().cosign(
                    sign, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
            case "xades" -> new AOXAdESSigner().cosign(
                    sign, ALGORITHM, pke.getPrivateKey(), pke.getCertificateChain(), extraParams);
            default -> throw new IllegalArgumentException("formato desconocido: " + args[1]);
        };
        Files.write(Path.of(args[5]), result);
    }

    private static void countersign(String[] args) throws Exception {
        CounterSignTarget target =
                "tree".equals(args[2]) ? CounterSignTarget.TREE : CounterSignTarget.LEAFS;
        byte[] sign = Files.readAllBytes(Path.of(args[3]));
        KeyStore.PrivateKeyEntry pke = loadKey(args[4], args[5]);
        Properties extraParams = new Properties();
        byte[] result = switch (args[1]) {
            case "cades" -> new AOCAdESSigner().countersign(
                    sign, ALGORITHM, target, null, pke.getPrivateKey(), pke.getCertificateChain(),
                    extraParams);
            case "xades" -> new AOXAdESSigner().countersign(
                    sign, ALGORITHM, target, null, pke.getPrivateKey(), pke.getCertificateChain(),
                    extraParams);
            default -> throw new IllegalArgumentException("formato desconocido: " + args[1]);
        };
        Files.write(Path.of(args[6]), result);
    }

    private static KeyStore.PrivateKeyEntry loadKey(String p12Path, String pin) throws Exception {
        KeyStore ks = KeyStore.getInstance("PKCS12");
        try (InputStream in = new FileInputStream(p12Path)) {
            ks.load(in, pin.toCharArray());
        }
        String alias = ks.aliases().nextElement();
        return (KeyStore.PrivateKeyEntry) ks.getEntry(
                alias, new KeyStore.PasswordProtection(pin.toCharArray()));
    }
}
