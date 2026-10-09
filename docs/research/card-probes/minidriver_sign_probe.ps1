<#
Firma una vez con el DNIe por el minidriver, fuera de rFirma, para separar H1 de H2 del 0x8010001F (#1817, ID-485).

Uso: powershell -NoProfile -ExecutionPolicy Bypass -File minidriver_sign_probe.ps1 [-Path Ksp|Capi] [-Certificate Signing|Authentication]
-Path Ksp (por defecto) firma como rFirma: la clave abierta solo por CNG y NCryptSignHash con PKCS#1 y pszAlgId SHA256.
-Path Capi es la segunda firma, solo si la primera sale H1: la clave por el proveedor CAPI de tarjeta y CryptSignHash.
Cada ejecución gasta una firma y como mucho un PIN, que pide el diálogo de Windows; no reintenta nunca.
Ejecútala con rFirma y AutoFirma cerrados y nada más usando la tarjeta. No imprime certificados, nombres ni números de serie.
#>
param(
    [ValidateSet('Ksp', 'Capi')] [string] $Path = 'Ksp',
    [ValidateSet('Signing', 'Authentication')] [string] $Certificate = 'Signing'
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public sealed class ProbeOutcome {
    public string Stage;
    public int Code;
    public byte[] Signature;
}

public static class MinidriverProbe {
    const uint CRYPT_ACQUIRE_ONLY_NCRYPT_KEY_FLAG = 0x00040000;
    const uint NCRYPT_PAD_PKCS1_FLAG = 2;
    const uint CERT_KEY_PROV_INFO_PROP_ID = 2;
    const uint PROV_RSA_FULL = 1;
    const uint CALG_SHA_256 = 0x0000800C;
    const uint HP_HASHVAL = 2;
    const uint AT_KEYEXCHANGE = 1;
    const uint AT_SIGNATURE = 2;
    const string SMART_CARD_CSP = "Microsoft Base Smart Card Crypto Provider";

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct BCRYPT_PKCS1_PADDING_INFO { public string pszAlgId; }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct CRYPT_KEY_PROV_INFO {
        public string pwszContainerName;
        public string pwszProvName;
        public uint dwProvType;
        public uint dwFlags;
        public uint cProvParam;
        public IntPtr rgProvParam;
        public uint dwKeySpec;
    }

    [DllImport("crypt32.dll", SetLastError = true)]
    static extern bool CryptAcquireCertificatePrivateKey(IntPtr cert, uint flags, IntPtr parameters, out IntPtr key, out uint keySpec, out bool callerFrees);

    [DllImport("crypt32.dll", SetLastError = true)]
    static extern bool CertGetCertificateContextProperty(IntPtr cert, uint propId, IntPtr data, ref uint size);

    [DllImport("ncrypt.dll")]
    static extern int NCryptSignHash(IntPtr key, ref BCRYPT_PKCS1_PADDING_INFO padding, byte[] hash, uint hashSize, byte[] signature, uint signatureSize, out uint written, uint flags);

    [DllImport("ncrypt.dll")]
    static extern int NCryptFreeObject(IntPtr handle);

    [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern bool CryptAcquireContextW(out IntPtr provider, string container, string providerName, uint providerType, uint flags);

    [DllImport("advapi32.dll", SetLastError = true)]
    static extern bool CryptGetUserKey(IntPtr provider, uint keySpec, out IntPtr key);

    [DllImport("advapi32.dll", SetLastError = true)]
    static extern bool CryptDestroyKey(IntPtr key);

    [DllImport("advapi32.dll", SetLastError = true)]
    static extern bool CryptCreateHash(IntPtr provider, uint algorithm, IntPtr key, uint flags, out IntPtr hash);

    [DllImport("advapi32.dll", SetLastError = true)]
    static extern bool CryptSetHashParam(IntPtr hash, uint param, byte[] data, uint flags);

    [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern bool CryptSignHashW(IntPtr hash, uint keySpec, string description, uint flags, byte[] signature, ref uint size);

    [DllImport("advapi32.dll", SetLastError = true)]
    static extern bool CryptDestroyHash(IntPtr hash);

    [DllImport("advapi32.dll", SetLastError = true)]
    static extern bool CryptReleaseContext(IntPtr provider, uint flags);

    public static string ProviderOf(IntPtr cert) {
        CRYPT_KEY_PROV_INFO info;
        if (!KeyProvInfo(cert, out info)) return "sin CERT_KEY_PROV_INFO";
        return info.pwszProvName + " (tipo " + info.dwProvType + ", keySpec " + info.dwKeySpec + ")";
    }

    public static ProbeOutcome SignWithKsp(IntPtr cert, byte[] digest) {
        IntPtr key;
        uint keySpec;
        bool callerFrees;
        if (!CryptAcquireCertificatePrivateKey(cert, CRYPT_ACQUIRE_ONLY_NCRYPT_KEY_FLAG, IntPtr.Zero, out key, out keySpec, out callerFrees))
            return Failed("abrir la clave por CNG", Marshal.GetLastWin32Error());
        try {
            BCRYPT_PKCS1_PADDING_INFO padding = new BCRYPT_PKCS1_PADDING_INFO { pszAlgId = "SHA256" };
            uint size;
            int status = NCryptSignHash(key, ref padding, digest, (uint)digest.Length, null, 0, out size, NCRYPT_PAD_PKCS1_FLAG);
            if (status != 0) return Failed("medir la firma", status);
            byte[] signature = new byte[size];
            status = NCryptSignHash(key, ref padding, digest, (uint)digest.Length, signature, size, out size, NCRYPT_PAD_PKCS1_FLAG);
            if (status != 0) return Failed("firmar el resumen", status);
            Array.Resize(ref signature, (int)size);
            return Signed(signature);
        } finally {
            if (callerFrees) NCryptFreeObject(key);
        }
    }

    public static ProbeOutcome SignWithCapi(IntPtr cert, byte[] digest) {
        IntPtr provider;
        uint keySpec;
        bool callerFrees;
        if (!CryptAcquireCertificatePrivateKey(cert, 0, IntPtr.Zero, out provider, out keySpec, out callerFrees)) {
            ProbeOutcome opened = OpenInSmartCardCsp(cert, out provider, out keySpec);
            if (opened != null) return opened;
            callerFrees = true;
        }
        try {
            IntPtr hash;
            if (!CryptCreateHash(provider, CALG_SHA_256, IntPtr.Zero, 0, out hash))
                return Failed("crear el resumen SHA-256 en el CSP", Marshal.GetLastWin32Error());
            try {
                if (!CryptSetHashParam(hash, HP_HASHVAL, digest, 0))
                    return Failed("cargar el resumen", Marshal.GetLastWin32Error());
                uint size = 0;
                if (!CryptSignHashW(hash, keySpec, null, 0, null, ref size))
                    return Failed("medir la firma", Marshal.GetLastWin32Error());
                byte[] signature = new byte[size];
                if (!CryptSignHashW(hash, keySpec, null, 0, signature, ref size))
                    return Failed("firmar el resumen", Marshal.GetLastWin32Error());
                Array.Resize(ref signature, (int)size);
                Array.Reverse(signature);
                return Signed(signature);
            } finally {
                CryptDestroyHash(hash);
            }
        } finally {
            if (callerFrees) CryptReleaseContext(provider, 0);
        }
    }

    static ProbeOutcome OpenInSmartCardCsp(IntPtr cert, out IntPtr provider, out uint keySpec) {
        provider = IntPtr.Zero;
        keySpec = 0;
        CRYPT_KEY_PROV_INFO info;
        if (!KeyProvInfo(cert, out info))
            return Failed("leer el contenedor de la clave", Marshal.GetLastWin32Error());
        if (!CryptAcquireContextW(out provider, info.pwszContainerName, SMART_CARD_CSP, PROV_RSA_FULL, 0))
            return Failed("abrir el contenedor en el CSP de tarjeta", Marshal.GetLastWin32Error());
        foreach (uint spec in new uint[] { AT_KEYEXCHANGE, AT_SIGNATURE }) {
            IntPtr key;
            if (CryptGetUserKey(provider, spec, out key)) {
                CryptDestroyKey(key);
                keySpec = spec;
                return null;
            }
        }
        int code = Marshal.GetLastWin32Error();
        CryptReleaseContext(provider, 0);
        return Failed("encontrar la clave en el contenedor", code);
    }

    static bool KeyProvInfo(IntPtr cert, out CRYPT_KEY_PROV_INFO info) {
        info = new CRYPT_KEY_PROV_INFO();
        uint size = 0;
        if (!CertGetCertificateContextProperty(cert, CERT_KEY_PROV_INFO_PROP_ID, IntPtr.Zero, ref size)) return false;
        IntPtr buffer = Marshal.AllocHGlobal((int)size);
        try {
            if (!CertGetCertificateContextProperty(cert, CERT_KEY_PROV_INFO_PROP_ID, buffer, ref size)) return false;
            info = (CRYPT_KEY_PROV_INFO)Marshal.PtrToStructure(buffer, typeof(CRYPT_KEY_PROV_INFO));
            return true;
        } finally {
            Marshal.FreeHGlobal(buffer);
        }
    }

    static ProbeOutcome Failed(string stage, int code) {
        return new ProbeOutcome { Stage = stage, Code = code };
    }

    static ProbeOutcome Signed(byte[] signature) {
        return new ProbeOutcome { Stage = "firmado", Signature = signature };
    }
}
'@

$KnownCodes = @{
    0x8010001F = 'SCARD_E_UNEXPECTED'
    0x8010006B = 'SCARD_W_WRONG_CHV (PIN incorrecto)'
    0x8010006C = 'SCARD_W_CHV_BLOCKED (PIN bloqueado)'
    0x8010006E = 'SCARD_W_CANCELLED_BY_USER'
    0x80100002 = 'SCARD_E_CANCELLED'
    0x800704C7 = 'ERROR_CANCELLED'
    0x80090036 = 'NTE_USER_CANCELLED'
    0x8010000C = 'SCARD_E_NO_SMARTCARD'
    0x80100069 = 'SCARD_W_REMOVED_CARD'
    0x80090008 = 'NTE_BAD_ALGID'
    0x80090009 = 'NTE_BAD_FLAGS'
    0x80090029 = 'NTE_NOT_SUPPORTED'
    0x8009000D = 'NTE_NO_KEY'
    0x80090016 = 'NTE_BAD_KEYSET'
    0x80090014 = 'NTE_BAD_PROV_TYPE'
    0x80092004 = 'CRYPT_E_NOT_FOUND'
}

function Say([string] $text) {
    Write-Host "[$(Get-Date -Format HH:mm:ss)] $text"
}

function Describe([int] $code) {
    $name = $KnownCodes[$code]
    if ($name) { '0x{0:X8} {1}' -f $code, $name } else { '0x{0:X8}' -f $code }
}

function Test-Usage($certificate, [string] $usage) {
    $extension = $certificate.Extensions | Where-Object { $_ -is [System.Security.Cryptography.X509Certificates.X509KeyUsageExtension] }
    if (-not $extension) { return $false }
    $nonRepudiation = ($extension.KeyUsages -band [System.Security.Cryptography.X509Certificates.X509KeyUsageFlags]::NonRepudiation) -ne 0
    if ($usage -eq 'Signing') { $nonRepudiation } else { -not $nonRepudiation }
}

$running = Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match '^(rfirma|autofirma)' }
if ($running) {
    Say "Cierra antes estos procesos: $(($running.ProcessName | Sort-Object -Unique) -join ', ')"
    exit 2
}

$candidates = @(Get-ChildItem Cert:\CurrentUser\My |
    Where-Object { $_.HasPrivateKey -and $_.Issuer -match 'CN=AC DNIE' -and (Test-Usage $_ $Certificate) })
Say "Certificados del DNIe con clave para '$Certificate' en CurrentUser\MY: $($candidates.Count)"
if ($candidates.Count -ne 1) {
    Say 'Hace falta exactamente uno: inserta el DNIe, espera a que Windows lo propague y vuelve a ejecutar.'
    exit 2
}
$certificate = $candidates[0]
Say "Proveedor de la clave: $([MinidriverProbe]::ProviderOf($certificate.Handle))"
Say "Vía: $Path"

$digest = [System.Security.Cryptography.SHA256]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes('rfirma-sonda-1817'))

Say 'Se va a gastar una firma. Windows pedirá el PIN una vez: escríbelo solo si estás seguro de que es el correcto; si dudas, cancela.'
if ((Read-Host 'Escribe FIRMAR para seguir') -cne 'FIRMAR') {
    Say 'Cancelado sin tocar la tarjeta.'
    exit 0
}

$outcome = if ($Path -eq 'Ksp') {
    [MinidriverProbe]::SignWithKsp($certificate.Handle, $digest)
} else {
    [MinidriverProbe]::SignWithCapi($certificate.Handle, $digest)
}

if ($outcome.Signature) {
    $rsa = [System.Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPublicKey($certificate)
    $valid = $rsa.VerifyHash($digest, $outcome.Signature,
        [System.Security.Cryptography.HashAlgorithmName]::SHA256,
        [System.Security.Cryptography.RSASignaturePadding]::Pkcs1)
    Say "Firma de $($outcome.Signature.Length) bytes; verifica con la clave pública: $valid"
} else {
    Say "Falla al $($outcome.Stage): $(Describe $outcome.Code)"
}

$foreign = [System.Diagnostics.Process]::GetCurrentProcess().Modules |
    Where-Object { $_.FileVersionInfo.CompanyName -notmatch 'Microsoft' } |
    ForEach-Object { $_.ModuleName } | Sort-Object -Unique
Say "Módulos de terceros en el proceso: $(if ($foreign) { $foreign -join ', ' } else { 'ninguno' })"

$wrongPin = $outcome.Code -eq 0x8010006B -or $outcome.Code -eq 0x8010006C
$cancelled = @(0x8010006E, 0x80100002, 0x800704C7, 0x80090036) -contains $outcome.Code
if ($wrongPin) {
    Say 'Veredicto: ninguno. El PIN no se aceptó; no vuelvas a ejecutar la sonda y anótalo en el ticket.'
} elseif ($cancelled) {
    Say 'Veredicto: ninguno. Se canceló el diálogo del PIN; no se gastó la firma.'
} elseif ($Path -eq 'Ksp' -and $outcome.Signature) {
    Say 'Veredicto: H2. Fuera de rFirma el KSP firma: el fallo es del proceso de rFirma.'
} elseif ($Path -eq 'Ksp' -and $outcome.Code -eq 0x8010001F) {
    Say 'Veredicto: H1. El minidriver no acepta la firma PKCS#1 por el KSP. La segunda firma, si se gasta: -Path Capi.'
} elseif ($Path -eq 'Capi' -and $outcome.Signature) {
    Say 'Veredicto: H1 con arreglo. La vía CAPI de tarjeta firma.'
} else {
    Say 'Veredicto: ninguno. Copia la salida en el ticket tal cual.'
}
