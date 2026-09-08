#!/usr/bin/env bash
# Regenera testdata/reference/ con los firmadores monofasicos del original
# 1.9.2 (AOCAdESSigner, AOXAdESSigner, AOFacturaESigner), consumidos desde
# Maven local (ADR-0002). Cada formato lo firma el original, no rfirma: es lo
# que la firma trifasica de rfirma tiene que igualar.
#
# Determinista salvo la fecha de firma: challenge.bin, document.xml e
# invoice.xml son fijos, pero AOCAdESSigner/AOXAdESSigner incrustan el
# instante de firma (signingTime/SigningTime) en cada regeneracion.
#
# Uso: make-reference-signatures.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SIGNER="$ROOT/rfirma-native-bridge/testbench/reference-signer"
REF="$ROOT/testdata/reference"
CERT="$ROOT/testdata/fnmt/active-rsa.p12"
PIN="1234"

mkdir -p "$REF"

echo "== Classpath (Maven local, ADR-0002)"
mvn -q -B -f "$SIGNER/pom.xml" dependency:build-classpath \
    -Dmdep.outputFile="$SIGNER/target/cp.txt" -Dmdep.includeScope=compile
mkdir -p "$SIGNER/target/classes"
javac -cp "$(cat "$SIGNER/target/cp.txt")" -d "$SIGNER/target/classes" \
    "$SIGNER/ReferenceSigner.java"
RUN_CP="$SIGNER/target/classes:$(cat "$SIGNER/target/cp.txt")"

sign() {
    echo "-- $*"
    java -cp "$RUN_CP" ReferenceSigner "$@"
}

# --- Entradas fijas ----------------------------------------------------------
python3 - "$REF/challenge.bin" <<'PY'
import sys
open(sys.argv[1], "wb").write(bytes(range(64)))
PY

cat > "$REF/document.xml" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<documento xmlns="urn:rfirma:testbench:document">
  <titulo>Documento de prueba rfirma</titulo>
  <cuerpo>Contenido determinista para el banco de referencia XAdES.</cuerpo>
</documento>
EOF

cat > "$REF/invoice.xml" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<fe:Facturae xmlns:ds="http://www.w3.org/2000/09/xmldsig#" xmlns:fe="http://www.facturae.es/Facturae/2009/v3.2/Facturae">
<FileHeader>
 <SchemaVersion>3.2</SchemaVersion>
 <Modality>I</Modality>
 <InvoiceIssuerType>EM</InvoiceIssuerType>
 <Batch>
  <BatchIdentifier>RFIRMA-0001</BatchIdentifier>
  <InvoicesCount>1</InvoicesCount>
  <TotalInvoicesAmount><TotalAmount>121.00</TotalAmount></TotalInvoicesAmount>
  <TotalOutstandingAmount><TotalAmount>121.00</TotalAmount></TotalOutstandingAmount>
  <TotalExecutableAmount><TotalAmount>121.00</TotalAmount></TotalExecutableAmount>
  <InvoiceCurrencyCode>EUR</InvoiceCurrencyCode>
 </Batch>
</FileHeader>
<Parties>
 <SellerParty>
  <TaxIdentification>
   <PersonTypeCode>J</PersonTypeCode>
   <ResidenceTypeCode>R</ResidenceTypeCode>
   <TaxIdentificationNumber>A00000000</TaxIdentificationNumber>
  </TaxIdentification>
  <LegalEntity>
   <CorporateName>rfirma pruebas SL</CorporateName>
   <TradeName>rfirma pruebas</TradeName>
   <RegistrationData>
    <Book>1</Book>
    <RegisterOfCompaniesLocation>Madrid</RegisterOfCompaniesLocation>
    <Sheet>1</Sheet>
    <Folio>1</Folio>
    <Section>1</Section>
    <Volume>1</Volume>
    <AdditionalRegistrationData>Sin datos</AdditionalRegistrationData>
   </RegistrationData>
   <AddressInSpain>
    <Address>Calle de Prueba 1</Address>
    <PostCode>28001</PostCode>
    <Town>Madrid</Town>
    <Province>Madrid</Province>
    <CountryCode>ESP</CountryCode>
   </AddressInSpain>
   <ContactDetails>
    <Telephone>910000000</Telephone>
    <ElectronicMail>pruebas@rfirma.example</ElectronicMail>
   </ContactDetails>
  </LegalEntity>
 </SellerParty>
 <BuyerParty>
  <TaxIdentification>
   <PersonTypeCode>J</PersonTypeCode>
   <ResidenceTypeCode>R</ResidenceTypeCode>
   <TaxIdentificationNumber>B00000000</TaxIdentificationNumber>
  </TaxIdentification>
  <LegalEntity>
   <CorporateName>cliente pruebas SL</CorporateName>
   <TradeName>cliente pruebas</TradeName>
   <RegistrationData>
    <Book>1</Book>
    <RegisterOfCompaniesLocation>Madrid</RegisterOfCompaniesLocation>
    <Sheet>1</Sheet>
    <Folio>1</Folio>
    <Section>1</Section>
    <Volume>1</Volume>
    <AdditionalRegistrationData>Sin datos</AdditionalRegistrationData>
   </RegistrationData>
   <AddressInSpain>
    <Address>Calle de Prueba 2</Address>
    <PostCode>28002</PostCode>
    <Town>Madrid</Town>
    <Province>Madrid</Province>
    <CountryCode>ESP</CountryCode>
   </AddressInSpain>
   <ContactDetails>
    <Telephone>910000001</Telephone>
    <ElectronicMail>cliente@rfirma.example</ElectronicMail>
   </ContactDetails>
  </LegalEntity>
 </BuyerParty>
</Parties>
<Invoices>
 <Invoice>
  <InvoiceHeader>
   <InvoiceNumber>1</InvoiceNumber>
   <InvoiceSeriesCode>RF</InvoiceSeriesCode>
   <InvoiceDocumentType>FC</InvoiceDocumentType>
   <InvoiceClass>OO</InvoiceClass>
  </InvoiceHeader>
  <InvoiceIssueData>
   <IssueDate>2026-01-01</IssueDate>
   <InvoiceCurrencyCode>EUR</InvoiceCurrencyCode>
   <TaxCurrencyCode>EUR</TaxCurrencyCode>
   <LanguageName>es</LanguageName>
  </InvoiceIssueData>
  <TaxesOutputs>
   <Tax>
    <TaxTypeCode>01</TaxTypeCode>
    <TaxRate>21.00</TaxRate>
    <TaxableBase><TotalAmount>100.00</TotalAmount></TaxableBase>
    <TaxAmount><TotalAmount>21.00</TotalAmount></TaxAmount>
   </Tax>
  </TaxesOutputs>
  <InvoiceTotals>
   <TotalGrossAmount>100.00</TotalGrossAmount>
   <TotalGeneralDiscounts>0.00</TotalGeneralDiscounts>
   <TotalGeneralSurcharges>0.00</TotalGeneralSurcharges>
   <TotalGrossAmountBeforeTaxes>100.00</TotalGrossAmountBeforeTaxes>
   <TotalTaxOutputs>21.00</TotalTaxOutputs>
   <TotalTaxesWithheld>0.00</TotalTaxesWithheld>
   <InvoiceTotal>121.00</InvoiceTotal>
   <TotalOutstandingAmount>121.00</TotalOutstandingAmount>
   <TotalExecutableAmount>121.00</TotalExecutableAmount>
  </InvoiceTotals>
  <Items>
   <InvoiceLine>
    <ItemDescription>Servicios de prueba rfirma</ItemDescription>
    <Quantity>1.00</Quantity>
    <UnitOfMeasure>01</UnitOfMeasure>
    <UnitPriceWithoutTax>100.000000</UnitPriceWithoutTax>
    <TotalCost>100.000000</TotalCost>
    <GrossAmount>100.000000</GrossAmount>
    <TaxesOutputs>
     <Tax>
      <TaxTypeCode>01</TaxTypeCode>
      <TaxRate>21.00</TaxRate>
      <TaxableBase><TotalAmount>100.00</TotalAmount></TaxableBase>
      <TaxAmount><TotalAmount>21.00</TotalAmount></TaxAmount>
     </Tax>
    </TaxesOutputs>
   </InvoiceLine>
  </Items>
 </Invoice>
</Invoices>
</fe:Facturae>
EOF

# --- CAdES -------------------------------------------------------------------
sign cades implicit "$REF/challenge.bin" "$CERT" "$PIN" "$REF/cades-implicit.p7s"
sign cades explicit "$REF/challenge.bin" "$CERT" "$PIN" "$REF/cades-explicit.p7s"
sign cosign cades "$REF/cades-implicit.p7s" "$CERT" "$PIN" \
    "$REF/cades-implicit.cosign.p7s"
sign countersign cades tree "$REF/cades-implicit.p7s" "$CERT" "$PIN" \
    "$REF/cades-implicit.countersign-tree.p7s"
sign countersign cades leafs "$REF/cades-implicit.p7s" "$CERT" "$PIN" \
    "$REF/cades-implicit.countersign-leafs.p7s"

# --- XAdES ---------------------------------------------------------------
sign xades detached "$REF/document.xml" "$CERT" "$PIN" "$REF/xades-detached.xml"
sign xades enveloping "$REF/document.xml" "$CERT" "$PIN" "$REF/xades-enveloping.xml"
sign xades enveloped "$REF/document.xml" "$CERT" "$PIN" "$REF/xades-enveloped.xml"
sign cosign xades "$REF/xades-enveloping.xml" "$CERT" "$PIN" \
    "$REF/xades-enveloping.cosign.xml"
sign countersign xades tree "$REF/xades-enveloping.xml" "$CERT" "$PIN" \
    "$REF/xades-enveloping.countersign-tree.xml"
sign countersign xades leafs "$REF/xades-enveloping.xml" "$CERT" "$PIN" \
    "$REF/xades-enveloping.countersign-leafs.xml"

# --- FacturaE ------------------------------------------------------------
sign facturae "$REF/invoice.xml" "$CERT" "$PIN" "$REF/facturae.xsig"

ls -la "$REF"
