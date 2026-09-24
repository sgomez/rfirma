//! Pruebas de integración del sello de tiempo de CAdES y XAdES contra una TSA de OpenSSL en el bucle local (ADR-0030).

#[path = "native_cycle/support.rs"]
mod support;

mod fake_tsa {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::path::Path;
    use std::process::Command;

    /// Una TSA RFC 3161 por HTTP: cada petición la contesta `openssl ts -reply`.
    pub(crate) struct FakeTsa {
        url: String,
    }

    impl FakeTsa {
        pub(crate) fn start(name: &str) -> Self {
            let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
            std::fs::create_dir_all(&folder).expect("debería poder crearse la carpeta de la TSA");
            write_the_authority(&folder);
            let listener = TcpListener::bind("127.0.0.1:0").expect("debería abrirse un puerto");
            let url = format!(
                "http://{}/tsa",
                listener.local_addr().expect("el puerto tiene dirección")
            );
            std::thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    answer(stream, &folder);
                }
            });
            Self { url }
        }

        pub(crate) fn url(&self) -> &str {
            &self.url
        }
    }

    /// Una URL de TSA en un puerto donde no escucha nadie.
    pub(crate) fn unreachable_url() -> String {
        let probe = TcpListener::bind("127.0.0.1:0").expect("debería abrirse un puerto");
        let address = probe.local_addr().expect("el puerto tiene dirección");
        drop(probe);
        format!("http://{address}/tsa")
    }

    fn write_the_authority(folder: &Path) {
        let key = folder.join("tsa.key");
        let certificate = folder.join("tsa.pem");
        let made = Command::new("openssl")
            .args([
                "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
            ])
            .args(["-subj", "/CN=rfirma fake TSA"])
            .args(["-addext", "extendedKeyUsage=critical,timeStamping"])
            .arg("-keyout")
            .arg(&key)
            .arg("-out")
            .arg(&certificate)
            .output()
            .expect("debería ejecutarse openssl req");
        assert!(
            made.status.success(),
            "openssl no creó el certificado de la TSA: {}",
            String::from_utf8_lossy(&made.stderr)
        );
        std::fs::write(folder.join("serial"), "01\n").expect("debería escribirse el serial");
        std::fs::write(
            folder.join("tsa.cnf"),
            format!(
                "[ tsa ]\ndefault_tsa = fake\n[ fake ]\nserial = {}\nsigner_digest = sha256\n\
                 default_policy = 0.4.0.2023.1.1\ndigests = sha1, sha256, sha384, sha512\n\
                 ess_cert_id_alg = sha256\n",
                folder.join("serial").display()
            ),
        )
        .expect("debería escribirse la configuración de la TSA");
    }

    fn answer(mut stream: TcpStream, folder: &Path) {
        let query = folder.join("query.tsq");
        std::fs::write(&query, request_body(&stream)).expect("debería guardarse la petición");
        let reply = reply_to(&query, folder);
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/timestamp-reply\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n",
            reply.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&reply);
    }

    fn request_body(stream: &TcpStream) -> Vec<u8> {
        let mut reader = BufReader::new(stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .expect("debería leerse la cabecera");
            if line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().expect("Content-Length es un número");
                }
            }
        }
        let mut body = vec![0; length];
        reader
            .read_exact(&mut body)
            .expect("debería leerse la petición");
        body
    }

    fn reply_to(query: &Path, folder: &Path) -> Vec<u8> {
        let reply = folder.join("reply.tsr");
        let made = Command::new("openssl")
            .args(["ts", "-reply", "-config"])
            .arg(folder.join("tsa.cnf"))
            .arg("-queryfile")
            .arg(query)
            .arg("-signer")
            .arg(folder.join("tsa.pem"))
            .arg("-inkey")
            .arg(folder.join("tsa.key"))
            .arg("-out")
            .arg(&reply)
            .output()
            .expect("debería ejecutarse openssl ts");
        assert!(
            made.status.success(),
            "openssl no selló la petición: {}",
            String::from_utf8_lossy(&made.stderr)
        );
        std::fs::read(reply).expect("debería leerse la respuesta de la TSA")
    }
}

mod timestamp {
    use rfirma_lib::signing::application::cycle;
    use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation, XadesVariant};

    use super::fake_tsa::{unreachable_url, FakeTsa};
    use super::support::{
        a_cycle_of, a_cycle_that_may_fail, openssl_cms_verify, signing_certificate,
        write_to_target, CHALLENGE, PIN,
    };

    /// `id-aa-signatureTimeStampToken` (1.2.840.113549.1.9.16.2.14), codificado en DER.
    const SIGNATURE_TIMESTAMP_OID: &[u8] = &[
        0x06, 0x0b, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x02, 0x0e,
    ];
    const XADES_TIMESTAMP: &[u8] = b"SignatureTimeStamp>";
    const REFERENCE_XML: &[u8] = b"<?xml version=\"1.0\"?><root><item>dato</item></root>";

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn a_signature_failure(format: Format, data: &[u8], tsa_url: &str) -> String {
        a_cycle_that_may_fail(
            &signing_certificate(),
            PIN,
            format,
            cycle::ALGORITHM,
            data,
            SignatureOperation::Sign,
            &[("tsaURL", tsa_url)],
        )
        .expect_err("una firma que no se puede sellar no debería salir")
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_cades_signature_that_asks_for_a_timestamp_carries_it_and_openssl_verifies_it() {
        let tsa = FakeTsa::start("fake-tsa-cades");

        let signed = a_cycle_of(
            Format::Cades,
            cycle::ALGORITHM,
            CHALLENGE,
            SignatureOperation::Sign,
            &[("mode", "implicit"), ("tsaURL", tsa.url())],
        );

        assert!(
            contains(&signed, SIGNATURE_TIMESTAMP_OID),
            "la firma CAdES no lleva el sello que se pidió"
        );
        let signature = write_to_target("cades-timestamped.p7s", &signed);
        assert_eq!(openssl_cms_verify(&signature, None), CHALLENGE);
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_xades_signature_that_asks_for_a_timestamp_carries_it() {
        let tsa = FakeTsa::start("fake-tsa-xades");

        let signed = a_cycle_of(
            Format::Xades(XadesVariant::Enveloping),
            cycle::ALGORITHM,
            REFERENCE_XML,
            SignatureOperation::Sign,
            &[("tsaURL", tsa.url())],
        );

        assert!(
            contains(&signed, XADES_TIMESTAMP),
            "la firma XAdES no lleva el sello que se pidió"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_cades_signature_with_an_invalid_tsa_url_fails_instead_of_leaving_unstamped() {
        let failure = a_signature_failure(Format::Cades, CHALLENGE, "http://tsa invalida");

        assert!(
            failure.contains("TimestampFailedException"),
            "el fallo tiene que ser el del sello: {failure}"
        );
    }

    #[test]
    #[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
    fn a_xades_signature_whose_tsa_does_not_answer_fails_instead_of_leaving_unstamped() {
        let failure = a_signature_failure(
            Format::Xades(XadesVariant::Enveloping),
            REFERENCE_XML,
            &unreachable_url(),
        );

        assert!(
            failure.contains("TimestampFailedException"),
            "el fallo tiene que ser el del sello: {failure}"
        );
    }
}
