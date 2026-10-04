//! Si el `/ByteRange` de cada firma o sello de un PDF cubre su revisión: cuatro enteros, desde el byte 0 y con el hueco justo en `/Contents`; en local, además, hasta el final de una revisión (EN 319 142-1 §6.3 k).
package es.gob.afirma.nativebridge;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import com.aowagie.text.pdf.AcroFields;
import com.aowagie.text.pdf.PdfArray;
import com.aowagie.text.pdf.PdfDictionary;
import com.aowagie.text.pdf.PdfName;
import com.aowagie.text.pdf.PdfNumber;
import com.aowagie.text.pdf.PdfReader;
import com.aowagie.text.pdf.PdfString;

/** El alcance del {@code /ByteRange} de las firmas de un PDF, que el validador del original no mira. */
final class ByteRanges {

    private ByteRanges() { }

    private static final byte[] EOF_MARKER = "%%EOF".getBytes(StandardCharsets.US_ASCII);

    /** Lo que se exige: en el protocolo, solo lo que ningún PDF legítimo incumple (ADR-0044). */
    enum Strictness {
        PROTOCOL,
        LOCAL
    }

    /** Los campos de firma o de sello cuyo rango no cubre su revisión; ninguno si el PDF no se lee. */
    static List<String> uncoveredIn(final byte[] pdf, final Strictness strictness) {
        final AcroFields fields;
        try {
            fields = new PdfReader(pdf).getAcroFields();
        }
        catch (final Exception e) {
            return List.of();
        }
        final List<String> uncovered = new ArrayList<>();
        for (final String name : fields.getSignatureNames()) {
            if (!coversItsRevision(pdf, fields.getSignatureDictionary(name), strictness)) {
                uncovered.add(name);
            }
        }
        return uncovered;
    }

    static boolean coversItsRevision(final byte[] pdf, final PdfDictionary signature,
            final Strictness strictness) {
        final long[] range = offsetsWithin(pdf.length, signature.getAsArray(PdfName.BYTERANGE));
        final PdfString contents = signature.getAsString(PdfName.CONTENTS);
        if (range == null || contents == null || range[0] != 0) {
            return false;
        }
        final long gapStart = range[1];
        final long gapEnd = range[2];
        if (gapStart >= gapEnd || range[3] > pdf.length - gapEnd) {
            return false;
        }
        final long end = gapEnd + range[3];
        return isExactlyTheHexString(pdf, (int) gapStart, (int) gapEnd, contents.getOriginalBytes())
                && (strictness == Strictness.PROTOCOL || endsARevision(pdf, (int) end));
    }

    /** Los cuatro enteros del rango, o ninguno si alguno no es un offset entre 0 y {@code length}. */
    private static long[] offsetsWithin(final int length, final PdfArray range) {
        if (range == null || range.size() != 4) {
            return null;
        }
        final long[] integers = new long[4];
        for (int i = 0; i < integers.length; i++) {
            final PdfNumber number = range.getAsNumber(i);
            if (number == null || number.doubleValue() < 0 || number.doubleValue() > length
                    || number.doubleValue() != Math.rint(number.doubleValue())) {
                return null;
            }
            integers[i] = (long) number.doubleValue();
        }
        return integers;
    }

    /** Un {@code <...>} de punta a punta del hueco, cuyas cifras son el valor de {@code /Contents}. */
    private static boolean isExactlyTheHexString(final byte[] pdf, final int from, final int to,
            final byte[] contents) {
        if (to - from < 2 || pdf[from] != '<' || pdf[to - 1] != '>') {
            return false;
        }
        final StringBuilder digits = new StringBuilder();
        for (int i = from + 1; i < to - 1; i++) {
            final char c = (char) (pdf[i] & 0xff);
            if (Character.digit(c, 16) >= 0) {
                digits.append(c);
            }
            else if (!isPdfWhitespace(c)) {
                return false;
            }
        }
        if (digits.length() % 2 != 0) {
            digits.append('0');
        }
        final byte[] decoded = new byte[digits.length() / 2];
        for (int i = 0; i < decoded.length; i++) {
            decoded[i] = (byte) Integer.parseInt(digits.substring(2 * i, 2 * i + 2), 16);
        }
        return Arrays.equals(decoded, contents);
    }

    private static boolean isPdfWhitespace(final char c) {
        return c == ' ' || c == '\n' || c == '\r' || c == '\t' || c == '\f' || c == 0;
    }

    /** Justo tras un {@code %%EOF}, con su fin de línea o sin él. */
    private static boolean endsARevision(final byte[] pdf, final int end) {
        int marker = end;
        for (int eol = 0; eol < 2 && marker > 0 && isEndOfLine(pdf[marker - 1]); eol++) {
            marker--;
        }
        final int from = marker - EOF_MARKER.length;
        return from >= 0 && Arrays.equals(pdf, from, marker, EOF_MARKER, 0, EOF_MARKER.length);
    }

    private static boolean isEndOfLine(final byte b) {
        return b == '\n' || b == '\r';
    }
}
