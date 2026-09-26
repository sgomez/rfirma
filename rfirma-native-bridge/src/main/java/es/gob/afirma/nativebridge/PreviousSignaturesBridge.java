package es.gob.afirma.nativebridge;

import java.security.cert.X509Certificate;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Map;

import javax.security.auth.x500.X500Principal;

import es.gob.afirma.core.signers.AOSimpleSignInfo;
import es.gob.afirma.core.util.tree.AOTreeModel;
import es.gob.afirma.core.util.tree.AOTreeNode;
import es.gob.afirma.signers.pades.AOPDFSigner;

/**
 * Las firmas que ya trae un PDF, vistas por el mismo recorrido de firmantes
 * que usa el escritorio de AutoFirma 1.9.2.
 *
 * <p>Reutiliza {@link AOPDFSigner#getSignersStructure(byte[], boolean)}: no se
 * escanea el PDF a mano. Ese recorrido nunca lanza para un PDF sin
 * firmas, cifrado o con una firma corrupta: en los tres casos devuelve el
 * árbol vacío o salta la firma que no se pudo leer, así que esta clase tampoco
 * necesita distinguirlos.
 */
final class PreviousSignaturesBridge {

    private static final Map<String, String> READABLE_KEYWORDS = Map.of(
            "2.5.4.5", "SERIALNUMBER",
            "2.5.4.97", "organizationIdentifier");

    private PreviousSignaturesBridge() { }

    /** Titular, emisor, número de serie del certificado y fecha de una firma previa. */
    record Signature(String subject, String issuer, String serialNumber, String signingTime) { }

    static List<Signature> read(final byte[] pdf) {
        final AOTreeModel structure = new AOPDFSigner().getSignersStructure(pdf, true);
        final List<AOSimpleSignInfo> infos = new ArrayList<>();
        collect((AOTreeNode) structure.getRoot(), infos);
        infos.sort(Comparator.comparing(
                AOSimpleSignInfo::getSigningTime, Comparator.nullsLast(Comparator.naturalOrder())));

        final List<Signature> signatures = new ArrayList<>(infos.size());
        for (final AOSimpleSignInfo info : infos) {
            final X509Certificate[] certs = info.getCerts();
            if (certs == null || certs.length == 0) {
                continue;
            }
            final X509Certificate signer = certs[0];
            signatures.add(new Signature(
                    readable(signer.getSubjectX500Principal()),
                    readable(signer.getIssuerX500Principal()),
                    signer.getSerialNumber().toString(),
                    info.getSigningTime() == null
                            ? null
                            : DateTimeFormatter.ISO_INSTANT.format(info.getSigningTime().toInstant())));
        }
        return signatures;
    }

    static String readable(final X500Principal name) {
        return name.getName(X500Principal.RFC2253, READABLE_KEYWORDS);
    }

    private static void collect(final AOTreeNode node, final List<AOSimpleSignInfo> infos) {
        if (node.getUserObject() instanceof AOSimpleSignInfo info) {
            infos.add(info);
        }
        for (int i = 0; i < node.getChildCount(); i++) {
            collect(node.getChildAt(i), infos);
        }
    }
}
