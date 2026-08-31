# Especificación de Desarrollo y Plan de Implementación: rfirma

Este documento constituye la especificación de diseño detallada para el nuevo proyecto **rfirma**, una reimplementación nativa, ligera y portable de la herramienta de firma digital **AutoFirma** utilizando **Tauri v2 (Rust + Svelte)** y el motor criptográfico original de Java compilado con **GraalVM Native Image**.

* **Nombre del Proyecto:** `rfirma`
* **Arquitectura:** Híbrida (Tauri Backend en Rust + Webview Frontend en Svelte + FFI a librería nativa GraalVM)
* **Plataforma Objetivo:** Linux, distribuido como **flatpak** sobre `org.gnome.Platform//50` (ver ADR-0004). La distribución del usuario es indiferente.
* **Protocolo de invocación:** `rfirma://` (y compatibilidad opcional con `afirma://`)

Nota: el proyecto original para evaluar como funciona y copiar la funcionalidad esta en /home/sergio/Developer/SideProjects/clienteafirma

---

## 1. Estructura Completa del Repositorio `rfirma`

El repositorio se organizará con la siguiente estructura de archivos:

```text
rfirma/
├── bootstrap.sh                     # Script para instalar dependencias en ~/.m2 automáticamente
│
├── rfirma-native-bridge/            # Módulo Java para el FFI Bridge
│   ├── pom.xml                      # POM que consume las librerías de Autofirma de ~/.m2
│   ├── src/
│   │   └── main/
│   │       ├── java/
│   │       │   └── es/gob/afirma/nativebridge/
│   │       │       └── NativeBridge.java  # Puente FFI
│   │       └── resources/
│   │           └── reflect-config.json    # Reglas de reflexión para GraalVM
│   └── target/
│       └── libautofirma_crypto.so   # Librería nativa generada (.so)
│
└── rfirma-app/                      # Aplicación Tauri (Frontend + Backend Rust)
    ├── package.json                 # Dependencias npm del frontend
    ├── svelte.config.js             # Configuración de Svelte
    ├── tailwind.config.js           # Estilos UI
    ├── src/                         # Frontend en Svelte
    │   ├── main.ts                  # Inicialización frontend
    │   ├── App.svelte               # UI del selector de certificados
    │   └── assets/                  # Iconos y recursos estáticos
    └── src-tauri/                   # Backend en Rust
        ├── Cargo.toml               # Dependencias de Rust
        ├── tauri.conf.json          # Configuración del empaquetador Tauri
        ├── build.rs                 # Script de compilación (Deep Links)
        └── src/
            ├── main.rs              # Punto de entrada y comandos de Tauri
            ├── crypto.rs            # Wrapper FFI y extractor dinámico del .so
            ├── keystores.rs         # Lector PKCS#11 (DNIe / FNMT en Linux)
            └── server.rs            # Servidor HTTPS y WS seguro (Axum)
```

---

## 2. El Núcleo Criptográfico Java (GraalVM FFI Bridge)

El componente `rfirma-native-bridge` expone las funciones Java de prefirma y postfirma como una librería dinámica de C.

### A. Archivo de Configuración de Maven: `rfirma-native-bridge/pom.xml`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>es.gob.afirma</groupId>
    <artifactId>rfirma-native-bridge</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>

    <properties>
        <maven.compiler.source>17</maven.compiler.source>
        <maven.compiler.target>17</maven.compiler.target>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
        <autofirma.version>1.9.1</autofirma.version>
        <graalvm.version>23.0.1</graalvm.version>
    </properties>

    <dependencies>
        <dependency>
            <groupId>es.gob.afirma</groupId>
            <artifactId>afirma-core</artifactId>
            <version>${autofirma.version}</version>
        </dependency>
        <dependency>
            <groupId>es.gob.afirma</groupId>
            <artifactId>afirma-crypto-cades</artifactId>
            <version>${autofirma.version}</version>
        </dependency>
        <dependency>
            <groupId>es.gob.afirma</groupId>
            <artifactId>afirma-crypto-pdf</artifactId>
            <version>${autofirma.version}</version>
        </dependency>
        <dependency>
            <groupId>es.gob.afirma</groupId>
            <artifactId>afirma-crypto-xades</artifactId>
            <version>${autofirma.version}</version>
        </dependency>
        <dependency>
            <groupId>es.gob.afirma</groupId>
            <artifactId>afirma-server-triphase-signer-core</artifactId>
            <version>${autofirma.version}</version>
        </dependency>
        <dependency>
            <groupId>org.graalvm.sdk</groupId>
            <artifactId>graal-sdk</artifactId>
            <version>${graalvm.version}</version>
            <scope>provided</scope>
        </dependency>
        <dependency>
            <groupId>com.fasterxml.jackson.core</groupId>
            <artifactId>jackson-databind</artifactId>
            <version>2.15.2</version>
        </dependency>
    </dependencies>

    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-compiler-plugin</artifactId>
                <version>3.11.0</version>
                <configuration>
                    <source>${maven.compiler.source}</source>
                    <target>${maven.compiler.target}</target>
                </configuration>
            </plugin>
            <plugin>
                <groupId>org.graalvm.buildtools</groupId>
                <artifactId>native-maven-plugin</artifactId>
                <version>0.10.2</version>
                <executions>
                    <execution>
                        <goals>
                            <goal>compile-no-fork</goal>
                        </goals>
                        <phase>package</phase>
                    </execution>
                </executions>
                <configuration>
                    <imageName>libautofirma_crypto</imageName>
                    <buildArgs>
                        <buildArg>--shared</buildArg>
                        <buildArg>-H:Name=libautofirma_crypto</buildArg>
                        <buildArg>--no-fallback</buildArg>
                        <buildArg>-H:ReflectionConfigurationFiles=src/main/resources/reflect-config.json</buildArg>
                    </buildArgs>
                </configuration>
            </plugin>
        </plugins>
    </build>
</project>
```

### B. Código del Puente Java: `NativeBridge.java`
Este archivo gestiona la traducción de tipos nativos C a Java, parsea los requests JSON de Rust y ejecuta la prefirma/postfirma delegando en la suite de Autofirma.

```java
package es.gob.afirma.nativebridge;

import java.io.ByteArrayInputStream;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.util.Base64;
import java.util.Map;
import java.util.Properties;
import java.util.List;
import java.util.ArrayList;

import org.graalvm.nativeimage.IsolateThread;
import org.graalvm.nativeimage.c.function.CEntryPoint;
import org.graalvm.nativeimage.c.function.CFunction;
import org.graalvm.nativeimage.c.function.CLibrary;
import org.graalvm.nativeimage.c.type.CCharPointer;
import org.graalvm.nativeimage.c.type.CTypeConversion;
import org.graalvm.nativeimage.c.type.VoidPointer;
import org.graalvm.nativeimage.UnmanagedMemory;
import org.graalvm.word.WordFactory;

import com.fasterxml.jackson.databind.ObjectMapper;

import es.gob.afirma.core.signers.TriphaseData;
import es.gob.afirma.triphase.signer.processors.PreProcessorFactory;
import es.gob.afirma.triphase.signer.processors.TriPhasePreProcessor;

@CLibrary("c")
public class NativeBridge {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    @CFunction
    public static native void free(VoidPointer ptr);

    @CEntryPoint(name = "autofirma_free_string")
    public static void freeString(IsolateThread thread, VoidPointer pointer) {
        if (!pointer.isNull()) {
            free(pointer);
        }
    }

    @CEntryPoint(name = "autofirma_presign")
    public static CCharPointer preSign(IsolateThread thread, CCharPointer requestJsonChar) {
        try {
            String requestJson = CTypeConversion.toJavaString(requestJsonChar);
            PreSignRequest request = MAPPER.readValue(requestJson, PreSignRequest.class);

            byte[] data = Base64.getDecoder().decode(request.dataBase64);
            X509Certificate[] certChain = parseCertificates(request.certificates);
            
            Properties extraParams = new Properties();
            if (request.extraParams != null) {
                extraParams.putAll(request.extraParams);
            }

            TriPhasePreProcessor preProcessor = PreProcessorFactory.getPreProcessor(request.format);
            TriphaseData triphaseData = preProcessor.preProcessPreSign(
                data, 
                request.algorithm, 
                certChain, 
                extraParams, 
                false
            );

            PreSignResponse response = new PreSignResponse();
            response.status = "OK";
            response.triphaseXml = triphaseData.toString();
            
            String responseJson = MAPPER.writeValueAsString(response);
            return toCStringUnmanaged(responseJson);
        } catch (Exception e) {
            return returnErrorResponse(e);
        }
    }

    @CEntryPoint(name = "autofirma_postsign")
    public static CCharPointer postSign(IsolateThread thread, CCharPointer requestJsonChar) {
        try {
            String requestJson = CTypeConversion.toJavaString(requestJsonChar);
            PostSignRequest request = MAPPER.readValue(requestJson, PostSignRequest.class);

            byte[] data = Base64.getDecoder().decode(request.dataBase64);
            X509Certificate[] certChain = parseCertificates(request.certificates);
            
            Properties extraParams = new Properties();
            if (request.extraParams != null) {
                extraParams.putAll(request.extraParams);
            }

            TriphaseData triphaseData = TriphaseData.parser(request.triphaseXml.getBytes("UTF-8"));

            TriPhasePreProcessor preProcessor = PreProcessorFactory.getPreProcessor(request.format);
            byte[] signedDoc = preProcessor.preProcessPostSign(
                data, 
                request.algorithm, 
                certChain, 
                extraParams, 
                triphaseData
            );

            PostSignResponse response = new PostSignResponse();
            response.status = "OK";
            response.signedDataBase64 = Base64.getEncoder().encodeToString(signedDoc);

            String responseJson = MAPPER.writeValueAsString(response);
            return toCStringUnmanaged(responseJson);
        } catch (Exception e) {
            return returnErrorResponse(e);
        }
    }

    private static CCharPointer returnErrorResponse(Exception e) {
        try {
            ErrorResponse response = new ErrorResponse();
            response.status = "ERROR";
            response.errorMessage = e.getMessage() != null ? e.getMessage() : e.toString();
            
            String responseJson = MAPPER.writeValueAsString(response);
            return toCStringUnmanaged(responseJson);
        } catch (Exception ex) {
            return toCStringUnmanaged("{\"status\":\"ERROR\",\"errorMessage\":\"Internal serialization error\"}");
        }
    }

    private static X509Certificate[] parseCertificates(List<String> b64Certs) throws Exception {
        CertificateFactory cf = CertificateFactory.getInstance("X.509");
        List<X509Certificate> certs = new ArrayList<>();
        for (String b64 : b64Certs) {
            byte[] certBytes = Base64.getDecoder().decode(b64);
            certs.add((X509Certificate) cf.generateCertificate(new ByteArrayInputStream(certBytes)));
        }
        return certs.toArray(new X509Certificate[0]);
    }

    private static CCharPointer toCStringUnmanaged(String javaString) {
        byte[] bytes = javaString.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        CCharPointer pointer = UnmanagedMemory.malloc(bytes.length + 1);
        for (int i = 0; i < bytes.length; i++) {
            pointer.write(i, bytes[i]);
        }
        pointer.write(bytes.length, (byte) 0);
        return pointer;
    }

    public static class PreSignRequest {
        public String format;
        public String dataBase64;
        public String algorithm;
        public List<String> certificates;
        public Map<String, String> extraParams;
    }

    public static class PreSignResponse {
        public String status;
        public String triphaseXml;
    }

    public static class PostSignRequest {
        public String format;
        public String dataBase64;
        public String algorithm;
        public List<String> certificates;
        public Map<String, String> extraParams;
        public String triphaseXml;
    }

    public static class PostSignResponse {
        public String status;
        public String signedDataBase64;
    }

    public static class ErrorResponse {
        public String status;
        public String errorMessage;
    }
}
```

### C. Archivo de Reflexión: `reflect-config.json`
Ubicación: `rfirma-native-bridge/src/main/resources/reflect-config.json`. Contiene el listado de DTOs y preprocesadores de firmas que se cargan dinámicamente. El listado completo se encuentra en el archivo anterior **`tauri_migration_assessment.md`**.

---

## 3. Desarrollo del Backend Tauri (`src-tauri`)

El backend de Tauri se escribe en Rust y gestiona la auto-extracción del binario `.so`, el servidor HTTPS local de loopback, el parseo de la petición de firmas, y la firma de los hashes usando almacenes locales de Linux (PKCS#11).

### A. Incrustador y Wrapper del Motor Criptográfico: `crypto.rs`
Este archivo incrusta `libautofirma_crypto.so` usando `include_bytes!`. Al iniciar el programa, extrae la librería en el directorio de la caché local y la carga usando `libloading`.

```rust
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::fs;
use libloading::Library;

// Incrustar el archivo compilado directamente en el binario de Rust.
const NATIVE_LIB_BYTES: &[u8] = include_bytes!("../../rfirma-native-bridge/target/libautofirma_crypto.so");

pub struct CryptoEngine {
    _lib: Library,
    graal_create_isolate: unsafe extern "C" fn(*const c_void, *mut *mut c_void, *mut *mut c_void) -> c_int,
    graal_tear_down_isolate: unsafe extern "C" fn(*mut c_void) -> c_int,
    autofirma_presign: unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_char,
    autofirma_postsign: unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_char,
    autofirma_free_string: unsafe extern "C" fn(*mut c_void, *mut c_void),
}

impl CryptoEngine {
    // Extrae y carga dinámicamente la librería FFI.
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("rfirma");
        
        fs::create_dir_all(&cache_dir)?;
        let lib_path = cache_dir.join("libautofirma_crypto.so");
        
        // Escribir los bytes incrustados si el fichero no existe o el tamaño difiere.
        if !lib_path.exists() || fs::metadata(&lib_path)?.len() != NATIVE_LIB_BYTES.len() as u64 {
            fs::write(&lib_path, NATIVE_LIB_BYTES)?;
        }
        
        unsafe {
            let lib = Library::new(&lib_path)?;
            let graal_create_isolate = *lib.get(b"graal_create_isolate")?;
            let graal_tear_down_isolate = *lib.get(b"graal_tear_down_isolate")?;
            let autofirma_presign = *lib.get(b"autofirma_presign")?;
            let autofirma_postsign = *lib.get(b"autofirma_postsign")?;
            let autofirma_free_string = *lib.get(b"autofirma_free_string")?;
            
            Ok(Self {
                _lib: lib,
                graal_create_isolate,
                graal_tear_down_isolate,
                autofirma_presign,
                autofirma_postsign,
                autofirma_free_string,
            })
        }
    }

    pub fn execute_presign(&self, request_json: &str) -> Result<String, String> {
        unsafe {
            let mut isolate: *mut c_void = std::ptr::null_mut();
            let mut thread: *mut c_void = std::ptr::null_mut();
            
            if (self.graal_create_isolate)(std::ptr::null(), &mut isolate, &mut thread) != 0 {
                return Err("No se pudo iniciar el Isolate de GraalVM".into());
            }

            let req_c_str = CString::new(request_json).map_err(|e| e.to_string())?;
            let res_ptr = (self.autofirma_presign)(thread, req_c_str.as_ptr());
            
            if res_ptr.is_null() {
                (self.graal_tear_down_isolate)(thread);
                return Err("Puntero nulo retornado por el motor FFI".into());
            }

            let res_str = CStr::from_ptr(res_ptr).to_string_lossy().into_owned();
            (self.autofirma_free_string)(thread, res_ptr as *mut c_void);
            (self.graal_tear_down_isolate)(thread);
            
            Ok(res_str)
        }
    }

    pub fn execute_postsign(&self, request_json: &str) -> Result<String, String> {
        unsafe {
            let mut isolate: *mut c_void = std::ptr::null_mut();
            let mut thread: *mut c_void = std::ptr::null_mut();
            
            if (self.graal_create_isolate)(std::ptr::null(), &mut isolate, &mut thread) != 0 {
                return Err("No se pudo iniciar el Isolate de GraalVM".into());
            }

            let req_c_str = CString::new(request_json).map_err(|e| e.to_string())?;
            let res_ptr = (self.autofirma_postsign)(thread, req_c_str.as_ptr());
            
            if res_ptr.is_null() {
                (self.graal_tear_down_isolate)(thread);
                return Err("Puntero nulo retornado por el motor FFI".into());
            }

            let res_str = CStr::from_ptr(res_ptr).to_string_lossy().into_owned();
            (self.autofirma_free_string)(thread, res_ptr as *mut c_void);
            (self.graal_tear_down_isolate)(thread);
            
            Ok(res_str)
        }
    }
}
```

### B. Módulo de Integración de Certificados PKCS#11 (DNIe): `keystores.rs`
El acceso a tarjetas criptográficas del gobierno español en Linux se realiza cargando la biblioteca compartida `.so` del DNIe instalada en el sistema.

```rust
use cryptoki::context::Pkcs11;
use cryptoki::session::{Session, UserType};
use cryptoki::slot::Slot;
use cryptoki::mechanism::Mechanism;
use std::convert::TryFrom;

const DNIE_LIB_PATHS: &[&str] = &[
    "/usr/lib/libdniepkcs11.so",
    "/usr/lib/x86_64-linux-gnu/libdniepkcs11.so",
    "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so"
];

pub struct Pkcs11Manager {
    pkcs11: Pkcs11,
}

impl Pkcs11Manager {
    pub fn init() -> Result<Self, String> {
        let mut loaded_lib = None;
        for path in DNIE_LIB_PATHS {
            if std::path::Path::new(path).exists() {
                loaded_lib = Some(path);
                break;
            }
        }

        let lib_path = loaded_lib.ok_or_else(|| "No se ha encontrado ninguna librería PKCS#11 instalada en el sistema (instalar dnie-configurador o opensc)".to_string())?;
        
        let pkcs11 = Pkcs11::new(lib_path).map_err(|e| format!("Error cargando PKCS11: {:?}", e))?;
        Ok(Self { pkcs11 })
    }

    // Lista de slots (lectores) que contienen tarjetas inteligentes activas.
    pub fn list_active_slots(&self) -> Result<Vec<Slot>, String> {
        let slots = self.pkcs11.get_slots_with_token().map_err(|e| format!("Error listando slots: {:?}", e))?;
        Ok(slots)
    }

    // Firma el hash extraído en la prefirma usando la clave seleccionada y el PIN suministrado.
    pub fn sign_hash(&self, slot: Slot, pin: &str, hash: &[u8]) -> Result<Vec<u8>, String> {
        let session = self.pkcs11.open_rw_session(slot)
            .map_err(|e| format!("Error abriendo sesión: {:?}", e))?;

        session.login(UserType::User, Some(pin))
            .map_err(|e| format!("Error de autenticación (PIN incorrecto): {:?}", e))?;

        // Buscar el objeto de clave privada del token (DNIe)
        // Usar cryptoki::object para buscar objetos de tipo PrivateKey
        let private_key = self.find_private_key(&session)?;

        // En España el mecanismo común de firma es RSA PKCS (SHA256)
        let mechanism = Mechanism::RsaPkcs;

        let signature = session.sign(&mechanism, private_key, hash)
            .map_err(|e| format!("Error en la firma criptográfica: {:?}", e))?;

        session.logout().ok();
        Ok(signature)
    }

    fn find_private_key(&self, session: &Session) -> Result<cryptoki::object::Object, String> {
        use cryptoki::object::{Attribute, AttributeType, ObjectClass};

        let template = vec![
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Sign(true),
        ];

        let mut objects = session.find_objects(&template)
            .map_err(|e| format!("Error buscando clave privada: {:?}", e))?;

        if objects.is_empty() {
            return Err("No se encontró ninguna clave de firma privada en la tarjeta".into());
        }

        Ok(objects.remove(0))
    }
}
```

### C. Servidor HTTPS Local (Loopback): `server.rs`
El servidor local simula la interfaz del cliente afirma tradicional. Escucha peticiones del navegador y genera el certificado SSL local autofirmado en la primera ejecución.

```rust
use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::State,
    response::IntoResponse,
};
use axum::http::Method;
use tower_http::cors::{Any, CorsLayer};
use std::net::SocketResult;
use std::sync::Arc;
use serde::Serialize;
use rcgen::generate_simple_self_signed;

#[derive(Clone)]
struct AppState {
    // Almacena la referencia a nuestro motor criptográfico cargado por FFI
    crypto: Arc<super::crypto::CryptoEngine>,
}

pub async fn start_local_server(crypto: super::crypto::CryptoEngine) {
    let state = AppState {
        crypto: Arc::new(crypto),
    };

    let cors = CorsLayer::new()
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_origin(Any);

    let app = Router::new()
        .route("/echo", get(handle_echo))
        .route("/presign", post(handle_presign_route))
        .route("/postsign", post(handle_postsign_route))
        .layer(cors)
        .with_state(state);

    // Configurar TLS con certificado autofirmado para localhost
    let tls_config = generate_tls_config().expect("Error al generar certificado TLS para localhost");

    println!("Iniciando servidor local seguro HTTPS en 127.0.0.1:63117...");
    let listener = std::net::TcpListener::bind("127.0.0.1:63117").unwrap();
    
    // Iniciar servidor axum con soporte TLS (usando rustls)
    // ...
}

async fn handle_echo() -> &'static str {
    "OK"
}

async fn handle_presign_route(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>
) -> impl IntoResponse {
    let req_str = payload.to_string();
    match state.crypto.execute_presign(&req_str) {
        Ok(res) => Json(serde_json::from_str::<serde_json::Value>(&res).unwrap()).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
    }
}

async fn handle_postsign_route(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>
) -> impl IntoResponse {
    let req_str = payload.to_string();
    match state.crypto.execute_postsign(&req_str) {
        Ok(res) => Json(serde_json::from_str::<serde_json::Value>(&res).unwrap()).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
    }
}

fn generate_tls_config() -> Result<rustls::ServerConfig, Box<dyn std::error::Error>> {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names)?;
    
    let cert_der = cert.serialize_der()?;
    let key_der = cert.serialize_private_key_der();

    let certs = vec![rustls::Certificate(cert_der)];
    let key = rustls::PrivateKey(key_der);

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(config)
}
```

---

## 4. Frontend de Tauri (`src`) en Svelte + Tailwind

El frontend se encarga de mostrar la lista de certificados (leída a través de un comando `tauri::command` de Rust que accede a los almacenes del DNIe/NSS) y pedir la confirmación de la firma o introducción del código PIN.

### A. Vista Principal: `rfirma-app/src/App.svelte`
```html
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface Certificate {
    alias: string;
    subject: string;
    issuer: string;
    expiryDate: string;
    isHardware: boolean;
  }

  let certificates: Certificate[] = [];
  let selectedCert: Certificate | null = null;
  let pinCode: string = "";
  let loading: boolean = false;
  let showPinModal: boolean = false;
  let statusMessage: string = "Cargando certificados...";

  onMount(async () => {
    try {
      certificates = await invoke<Certificate[]>('get_certificates');
      statusMessage = certificates.length > 0 ? "" : "No se han encontrado certificados válidos.";
    } catch (e) {
      statusMessage = "Error al leer los certificados del sistema.";
      console.error(e);
    }
  });

  async function handleSign() {
    if (!selectedCert) return;

    if (selectedCert.isHardware) {
      showPinModal = true;
      return;
    }
    
    executeSignature();
  }

  async function executeSignature() {
    loading = true;
    showPinModal = false;
    statusMessage = "Firmando documento...";
    
    try {
      const response: string = await invoke('perform_signature', {
        certAlias: selectedCert.alias,
        pin: pinCode
      });
      statusMessage = "¡Firma realizada con éxito!";
      // Enviar resultado de vuelta al servidor HTTPS local para que lo reciba la web solicitante
    } catch (e) {
      statusMessage = `Error de firma: ${e}`;
    } finally {
      loading = false;
    }
  }
</script>

<main class="h-screen w-screen bg-slate-900 text-white flex flex-col p-6">
  <header class="flex items-center justify-between pb-4 border-b border-slate-700">
    <h1 class="text-xl font-bold tracking-wide">rfirma - Firma Electrónica</h1>
    <span class="text-xs px-2 py-1 bg-teal-500/20 text-teal-300 rounded-full font-semibold border border-teal-500/30">Linux Native</span>
  </header>

  <section class="flex-grow overflow-y-auto mt-4 space-y-2">
    {#if statusMessage}
      <div class="p-4 bg-slate-800 rounded border border-slate-700 text-slate-300 text-sm">
        {statusMessage}
      </div>
    {/if}

    <div class="grid grid-cols-1 gap-3">
      {#each certificates as cert}
        <button
          class="flex items-start justify-between p-4 bg-slate-800 hover:bg-slate-700/80 rounded-lg text-left transition border-2 {selectedCert === cert ? 'border-teal-500' : 'border-transparent'}"
          on:click={() => selectedCert = cert}
        >
          <div class="space-y-1">
            <h3 class="font-bold text-sm">{cert.subject}</h3>
            <p class="text-xs text-slate-400">CA: {cert.issuer}</p>
            <p class="text-[10px] text-slate-500">Vence: {cert.expiryDate}</p>
          </div>
          {#if cert.isHardware}
            <span class="text-[10px] px-2 py-0.5 bg-yellow-500/20 text-yellow-300 rounded border border-yellow-500/30">DNIe / Tarjeta</span>
          {/if}
        </button>
      {/each}
    </div>
  </section>

  <!-- Modal para PIN de SmartCards -->
  {#if showPinModal}
    <div class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="bg-slate-800 p-6 rounded-lg w-full max-w-sm border border-slate-700 space-y-4">
        <h3 class="text-md font-bold">Introducir PIN del DNIe</h3>
        <p class="text-xs text-slate-400">Por favor, escribe el PIN de tu documento de identidad para autorizar la firma.</p>
        <input 
          type="password" 
          bind:value={pinCode}
          placeholder="Código PIN"
          class="w-full p-2 bg-slate-900 border border-slate-700 rounded focus:border-teal-500 focus:outline-none text-sm text-center tracking-widest"
        />
        <div class="flex justify-end space-x-2 pt-2">
          <button on:click={() => showPinModal = false} class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded text-xs">Cancelar</button>
          <button on:click={executeSignature} class="px-4 py-2 bg-teal-600 hover:bg-teal-500 rounded text-xs font-bold">Aceptar</button>
        </div>
      </div>
    </div>
  {/if}

  <footer class="mt-4 pt-4 border-t border-slate-700 flex justify-end space-x-2">
    <button class="px-5 py-2 bg-slate-800 hover:bg-slate-700 rounded text-xs transition">Cancelar</button>
    <button 
      class="px-5 py-2 bg-teal-600 hover:bg-teal-500 disabled:opacity-50 disabled:cursor-not-allowed rounded text-xs font-bold transition"
      disabled={!selectedCert || loading}
      on:click={handleSign}
    >
      Firmar Documento
    </button>
  </footer>
</main>
```

---

## 5. Script de Instalación y Registro del Servidor Local (Firefox SSL)

Para que el servidor HTTPS sea accesible desde el navegador, el instalador debe registrar el certificado de `localhost` y configurar Firefox. (Nota de auditoría: el canal es un **flatpak**, no un `.deb`/`.rpm`, y el canal navegador está **fuera del alcance** de v0.1.)

### Script de Instalación del Sistema (`setup.sh`):
```bash
#!/bin/bash
# Evitar que se ejecute si no es root
if [ "$EUID" -ne 0 ]; then
  echo "Por favor, ejecuta como root (sudo ./setup.sh)"
  exit 1
fi

echo "--- Instalando Autoridad SSL de rfirma en el Sistema ---"

# Generar certificado si no existe
# (O dejar que la aplicación Tauri lo cree y lo guarde en /etc/rfirma/local_ca.crt)
mkdir -p /usr/local/share/ca-certificates/rfirma
cp /tmp/rfirma_ca.crt /usr/local/share/ca-certificates/rfirma/rfirma_ca.crt

# Actualizar el almacén de CA de Linux
update-ca-certificates

echo "--- Configurando políticas de Firefox para confiar en las CAs del Sistema ---"
# Esto evita tener que alterar las bases de datos cert9.db individuales de Firefox.
mkdir -p /etc/firefox/policies
cat <<EOF > /etc/firefox/policies/policies.json
{
  "policies": {
    "Certificates": {
      "Install": ["/usr/local/share/ca-certificates/rfirma/rfirma_ca.crt"]
    }
  }
}
EOF

echo "--- Registrando el manejador de protocolo rfirma:// ---"
# Crear el archivo Desktop Entry
cat <<EOF > /usr/share/applications/rfirma.desktop
[Desktop Entry]
Name=rfirma
Exec=/usr/bin/rfirma %u
Type=Application
Terminal=false
MimeType=x-scheme-handler/rfirma;x-scheme-handler/afirma;
Categories=Utility;
EOF

# Actualizar la base de datos MIME
update-desktop-database

echo "Configuración completada con éxito."
```

---

## 6. Procedimiento paso a paso para el siguiente Agente

Cuando un nuevo agente o desarrollador continúe este proyecto, el orden de implementación exacto debe ser:

1. **Obtener e instalar las dependencias Java originales**:
   * Ejecutar el script `bootstrap.sh` en la raíz de `rfirma`. El script se encargará de comprobar si las librerías oficiales de Autofirma ya están en la caché de Maven local; si no están, clonará temporalmente el repositorio original de forma remota, ejecutará `mvn clean install` para registrarlas y limpiará los directorios intermedios.
   * Esto asegura que cualquier persona pueda compilar el proyecto inmediatamente tras clonarlo.
   * No copiar ningún código fuente Java al repositorio de `rfirma`. El proyecto `rfirma` consumirá los artefactos compilados directamente de la caché de Maven local (`~/.m2`).
2. **Construir `rfirma-native-bridge`**:
   * Crear la carpeta `rfirma-native-bridge` con el `pom.xml` anterior.
   * Pegar la implementación de `NativeBridge.java` y el archivo `reflect-config.json`.
   * Ejecutar `mvn clean package` y comprobar que genera `libautofirma_crypto.so` en el target.
3. **Crear la App de Tauri (`rfirma-app`)**:
   * Ejecutar `npm create tauri-app@latest` con plantilla Svelte + TS.
   * Copiar `libautofirma_crypto.so` en el directorio de recursos de Rust para su incrustación (`include_bytes!`).
   * Programar los ficheros `crypto.rs` (Ffi), `keystores.rs` (PKCS11), y `server.rs` (HTTPS/WS Axum) en Rust.
   * Modificar `main.rs` para enlazar los comandos con el frontend Svelte.
   * Diseñar la UI en `App.svelte` de acuerdo con el boceto HTML.
4. **Verificar**:
   * Levantar la app con `cargo tauri dev`.
   * Abrir una web en el navegador, enviar una petición HTTPS POST a `https://localhost:63117/presign` y verificar que devuelve el XML de pre-firma.
