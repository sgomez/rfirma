package es.gob.afirma.nativebridge;

import java.util.GregorianCalendar;
import java.util.Properties;

import es.gob.afirma.signers.tsp.pkcs7.CMSTimestamper;
import es.gob.afirma.signers.tsp.pkcs7.TsaParams;
import es.gob.afirma.signers.xades.XAdESTspUtil;

/** El sello de tiempo que pide {@code tsaURL} en CAdES y XAdES; no el de PAdES (ADR-0030). */
final class SignatureTimestamp {

    private static final String PARAM_TSA_URL = "tsaURL";

    private SignatureTimestamp() { }

    /** Falla antes de firmar si la sede pide un sello con una TSA que no se puede usar. */
    static void requireAUsableTsa(final Properties extraParams) throws TimestampFailedException {
        tsaOf(extraParams);
    }

    /** El CMS con el sello de tiempo en cada firmante, o tal cual si no se pidio sello. */
    static byte[] stampCms(final byte[] cms, final Properties extraParams)
            throws TimestampFailedException {
        final TsaParams tsa = tsaOf(extraParams);
        if (tsa == null) {
            return cms;
        }
        try {
            return new CMSTimestamper(tsa)
                    .addTimestamp(cms, tsa.getTsaHashAlgorithm(), new GregorianCalendar());
        } catch (final Exception e) {
            throw unstamped(tsa, e);
        }
    }

    /** El XML con el sello de tiempo en su primera firma, o tal cual si no se pidio sello. */
    static byte[] stampXades(final byte[] xml, final Properties extraParams)
            throws TimestampFailedException {
        final TsaParams tsa = tsaOf(extraParams);
        if (tsa == null) {
            return xml;
        }
        try {
            return XAdESTspUtil.timestampXAdES(xml, extraParams);
        } catch (final Exception e) {
            throw unstamped(tsa, e);
        }
    }

    private static TsaParams tsaOf(final Properties extraParams) throws TimestampFailedException {
        final String url = extraParams == null ? null : extraParams.getProperty(PARAM_TSA_URL);
        if (url == null || url.isBlank()) {
            return null;
        }
        try {
            return new TsaParams(extraParams);
        } catch (final RuntimeException e) {
            throw new TimestampFailedException(
                    "la sede pide sello de tiempo con una configuracion de TSA que no se puede"
                            + " usar (" + PARAM_TSA_URL + "=«" + url + "»): " + e.getMessage(),
                    e);
        }
    }

    private static TimestampFailedException unstamped(final TsaParams tsa, final Exception e) {
        return new TimestampFailedException("no se ha podido obtener el sello de tiempo de la TSA "
                + tsa.getTsaUrl() + ", y la firma no sale sin el sello que se pidio: " + e, e);
    }
}
