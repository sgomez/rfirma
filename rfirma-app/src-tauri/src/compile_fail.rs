//! Lo que ya no compila: el cebo de cada tipo que sustituyó a una guarda que leía el código como texto (#439).

/// ```
/// use rfirma_lib::signing::adapters::ffi::parse_presign;
/// use rfirma_lib::signing::domain::bridge::PostSignRequest;
/// use rfirma_lib::signing::domain::TokenSignature;
/// use rfirma_lib::site::domain::protocol::SafCode;
///
/// let presigned = parse_presign(r#"{"ok":true,"session":"<xml/>","pre":"MTIz","stamp":"c2VsbG8="}"#)
///     .expect("es el JSON del contrato");
/// let sealed = presigned
///     .sealed_with(&TokenSignature::invented(), presigned.stamp())
///     .expect("el sello es el mismo");
/// let request = PostSignRequest {
///     pdf_b64: "",
///     certificate_chain_b64: "",
///     sealed: &sealed,
/// };
/// assert_eq!(request.sealed.session(), "<xml/>");
/// assert_eq!(sealed.completed_with(b"%PDF-".to_vec()).pdf(), b"%PDF-");
/// assert_eq!(SafCode::ALL[0].as_str(), "SAF_00");
/// ```
pub struct TheDoorThatIsOpenSoTheClosedOnesBelowAreNotATypo;

/// ```compile_fail,E0451
/// use rfirma_lib::signing::domain::{SealedPreSignature, SessionSeal};
///
/// let _ = SealedPreSignature {
///     session: String::new(),
///     pkcs1_b64: String::new(),
///     stamp: SessionSeal::from_bridge(""),
/// };
/// ```
pub struct TheSealedPreSignatureIsOnlyMadeFromAPreSignatureAndItsSeal;

/// ```compile_fail,E0560
/// use rfirma_lib::signing::domain::bridge::PostSignRequest;
///
/// let _ = PostSignRequest {
///     pdf_b64: "",
///     certificate_chain_b64: "",
///     session: "<xml/>",
///     pkcs1_b64: "una firma en claro",
/// };
/// ```
pub struct ThePostsignTakesNoSignatureInTheClear;

/// ```compile_fail,E0451
/// use rfirma_lib::signing::domain::CompletedCycle;
///
/// let _ = CompletedCycle { pdf: Vec::new() };
/// ```
pub struct TheCompletedCycleIsOnlyMadeByThePostsign;

/// ```compile_fail,E0277
/// use rfirma_lib::site::domain::protocol::SafCode;
///
/// let _ = SafCode::from("SAF_48");
/// ```
pub struct NoSafCodeIsMadeFromAString;

/// ```compile_fail,E0277
/// use rfirma_lib::site::domain::protocol::SafCode;
///
/// let _: SafCode = "SAF_48".parse().unwrap();
/// ```
pub struct NoSafCodeIsParsedFromAString;
