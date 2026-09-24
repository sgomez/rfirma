package es.gob.afirma.nativebridge;

import java.io.IOException;
import java.math.BigInteger;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.cert.X509Certificate;
import java.util.Date;
import java.util.List;
import java.util.concurrent.atomic.AtomicLong;

import org.bouncycastle.asn1.ASN1ObjectIdentifier;
import org.bouncycastle.asn1.oiw.OIWObjectIdentifiers;
import org.bouncycastle.asn1.x500.X500Name;
import org.bouncycastle.asn1.x509.AlgorithmIdentifier;
import org.bouncycastle.asn1.x509.ExtendedKeyUsage;
import org.bouncycastle.asn1.x509.Extension;
import org.bouncycastle.asn1.x509.KeyPurposeId;
import org.bouncycastle.cert.jcajce.JcaCertStore;
import org.bouncycastle.cert.jcajce.JcaX509CertificateConverter;
import org.bouncycastle.cert.jcajce.JcaX509v3CertificateBuilder;
import org.bouncycastle.cms.jcajce.JcaSimpleSignerInfoGeneratorBuilder;
import org.bouncycastle.operator.jcajce.JcaContentSignerBuilder;
import org.bouncycastle.operator.jcajce.JcaDigestCalculatorProviderBuilder;
import org.bouncycastle.tsp.TSPAlgorithms;
import org.bouncycastle.tsp.TimeStampRequest;
import org.bouncycastle.tsp.TimeStampResponseGenerator;
import org.bouncycastle.tsp.TimeStampTokenGenerator;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;

/** Una TSA RFC 3161 por HTTP en el bucle local, con un certificado propio de un solo uso. */
final class FakeTsa implements AutoCloseable {

    private static final String SIGNATURE_ALGORITHM = "SHA256withRSA";
    private static final ASN1ObjectIdentifier FALLBACK_POLICY = new ASN1ObjectIdentifier("1.2.3.4");

    private final HttpServer server;
    private final KeyPair keys;
    private final X509Certificate certificate;
    private final AtomicLong serial = new AtomicLong(1);

    private FakeTsa() throws Exception {
        final KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(2048);
        this.keys = generator.generateKeyPair();
        this.certificate = timestampingCertificate(this.keys);
        this.server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        this.server.createContext("/tsa", this::answer);
        this.server.start();
    }

    static FakeTsa start() throws Exception {
        return new FakeTsa();
    }

    String url() {
        return "http://127.0.0.1:" + this.server.getAddress().getPort() + "/tsa";
    }

    /** Una URL de TSA en un puerto donde no escucha nadie. */
    static String unreachableUrl() throws IOException {
        try (ServerSocket probe = new ServerSocket(0)) {
            return "http://127.0.0.1:" + probe.getLocalPort() + "/tsa";
        }
    }

    @Override
    public void close() {
        this.server.stop(0);
    }

    private void answer(final HttpExchange exchange) throws IOException {
        try (exchange) {
            final byte[] reply = replyTo(new TimeStampRequest(exchange.getRequestBody().readAllBytes()));
            exchange.getResponseHeaders().add("Content-Type", "application/timestamp-reply");
            exchange.sendResponseHeaders(200, reply.length);
            exchange.getResponseBody().write(reply);
        } catch (final Exception e) {
            throw new IOException(e);
        }
    }

    private byte[] replyTo(final TimeStampRequest request) throws Exception {
        final TimeStampTokenGenerator tokens = new TimeStampTokenGenerator(
                new JcaSimpleSignerInfoGeneratorBuilder()
                        .build(SIGNATURE_ALGORITHM, this.keys.getPrivate(), this.certificate),
                new JcaDigestCalculatorProviderBuilder().build()
                        .get(new AlgorithmIdentifier(OIWObjectIdentifiers.idSHA1)),
                request.getReqPolicy() != null ? request.getReqPolicy() : FALLBACK_POLICY);
        tokens.addCertificates(new JcaCertStore(List.of(this.certificate)));
        return new TimeStampResponseGenerator(tokens, TSPAlgorithms.ALLOWED)
                .generate(request, BigInteger.valueOf(this.serial.getAndIncrement()), new Date())
                .getEncoded();
    }

    private static X509Certificate timestampingCertificate(final KeyPair keys) throws Exception {
        final X500Name name = new X500Name("CN=rfirma fake TSA");
        final long now = System.currentTimeMillis();
        final JcaX509v3CertificateBuilder builder = new JcaX509v3CertificateBuilder(name,
                BigInteger.ONE, new Date(now - 60_000), new Date(now + 3_600_000), name,
                keys.getPublic());
        builder.addExtension(Extension.extendedKeyUsage, true,
                new ExtendedKeyUsage(KeyPurposeId.id_kp_timeStamping));
        return new JcaX509CertificateConverter().getCertificate(builder
                .build(new JcaContentSignerBuilder(SIGNATURE_ALGORITHM).build(keys.getPrivate())));
    }
}
