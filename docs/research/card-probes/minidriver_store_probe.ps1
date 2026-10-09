<#
Guía interactiva que mide cómo refleja Windows el DNIe del minidriver: su almacén, PC/SC por winscard.dll y el KSP de tarjeta (#1820, ID-486).

Uso: powershell -NoProfile -ExecutionPolicy Bypass -File minidriver_store_probe.ps1 [-Window <s>] [-Log <fichero>]
Empieza con el lector conectado y el DNIe dentro; guía por sacar y meter la tarjeta y desconectar y conectar el lector. Enter salta al paso siguiente.
-Window son los segundos que se vigilan los almacenes tras cada evento del lector (45 por defecto).
Nunca firma ni pide PIN: no abre la clave por el almacén, y lo que habla con el KSP lleva NCRYPT_SILENT_FLAG.
Ejecútala con rFirma y AutoFirma cerrados. No imprime certificados, nombres, números de serie, huellas, ATR ni contenedores.
#>
param(
    [int] $Window = 45,
    [string] $Log
)

$ErrorActionPreference = 'Stop'

trap {
    Write-Host "Error interno en la línea $($_.InvocationInfo.ScriptLineNumber): $($_.FullyQualifiedErrorId)"
    exit 3
}

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using System.Threading;

public static class Codes {
    static readonly Dictionary<uint, string> Known = new Dictionary<uint, string> {
        { 0x8010000A, "SCARD_E_TIMEOUT" },
        { 0x80100003, "SCARD_E_INVALID_HANDLE" },
        { 0x80100009, "SCARD_E_UNKNOWN_READER" },
        { 0x8010000C, "SCARD_E_NO_SMARTCARD" },
        { 0x80100017, "SCARD_E_READER_UNAVAILABLE" },
        { 0x8010001D, "SCARD_E_NO_SERVICE" },
        { 0x8010001E, "SCARD_E_SERVICE_STOPPED" },
        { 0x8010001F, "SCARD_E_UNEXPECTED" },
        { 0x8010002E, "SCARD_E_NO_READERS_AVAILABLE" },
        { 0x80100069, "SCARD_W_REMOVED_CARD" },
        { 0x8009002A, "NTE_NO_MORE_ITEMS" },
        { 0x80090022, "NTE_SILENT_CONTEXT" },
        { 0x80090016, "NTE_BAD_KEYSET" },
        { 0x8009000D, "NTE_NO_KEY" },
        { 0x80090029, "NTE_NOT_SUPPORTED" },
        { 0x80090026, "NTE_INVALID_HANDLE" },
    };

    public static string Describe(int code) {
        string name;
        return Known.TryGetValue(unchecked((uint)code), out name)
            ? string.Format("0x{0:X8} {1}", code, name)
            : string.Format("0x{0:X8}", code);
    }
}

public sealed class PcscWatch {
    const uint SCARD_SCOPE_USER = 0;
    const uint SCARD_STATE_UNAWARE = 0;
    const uint SCARD_STATE_CHANGED = 0x2;
    const uint SCARD_STATE_PRESENT = 0x20;
    const int SCARD_E_TIMEOUT = unchecked((int)0x8010000A);
    const int SCARD_E_NO_READERS_AVAILABLE = unchecked((int)0x8010002E);
    const string PNP = "\\\\?PnP?\\Notification";

    static readonly KeyValuePair<uint, string>[] StateNames = {
        new KeyValuePair<uint, string>(0x4, "UNKNOWN"),
        new KeyValuePair<uint, string>(0x8, "UNAVAILABLE"),
        new KeyValuePair<uint, string>(0x10, "EMPTY"),
        new KeyValuePair<uint, string>(0x20, "PRESENT"),
        new KeyValuePair<uint, string>(0x40, "ATRMATCH"),
        new KeyValuePair<uint, string>(0x80, "EXCLUSIVE"),
        new KeyValuePair<uint, string>(0x100, "INUSE"),
        new KeyValuePair<uint, string>(0x200, "MUTE"),
        new KeyValuePair<uint, string>(0x400, "UNPOWERED"),
    };

    static readonly KeyValuePair<uint, string>[] Providers = {
        new KeyValuePair<uint, string>(0x80000001, "minidriver"),
        new KeyValuePair<uint, string>(3, "KSP"),
        new KeyValuePair<uint, string>(2, "CSP"),
    };

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct SCARD_READERSTATE {
        public string szReader;
        public IntPtr pvUserData;
        public uint dwCurrentState;
        public uint dwEventState;
        public uint cbAtr;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 36)] public byte[] rgbAtr;
    }

    [DllImport("winscard.dll")]
    static extern int SCardEstablishContext(uint scope, IntPtr reserved1, IntPtr reserved2, out IntPtr context);

    [DllImport("winscard.dll")]
    static extern int SCardReleaseContext(IntPtr context);

    [DllImport("winscard.dll", CharSet = CharSet.Unicode)]
    static extern int SCardListReadersW(IntPtr context, string groups, char[] readers, ref uint length);

    [DllImport("winscard.dll", CharSet = CharSet.Unicode)]
    static extern int SCardGetStatusChangeW(IntPtr context, uint timeout, [In, Out] SCARD_READERSTATE[] states, uint count);

    [DllImport("winscard.dll", CharSet = CharSet.Unicode)]
    static extern int SCardListCardsW(IntPtr context, byte[] atr, IntPtr interfaces, uint interfaceCount, char[] cards, ref uint length);

    [DllImport("winscard.dll", CharSet = CharSet.Unicode)]
    static extern int SCardGetCardTypeProviderNameW(IntPtr context, string card, uint providerId, char[] provider, ref uint length);

    IntPtr context = IntPtr.Zero;
    SCARD_READERSTATE[] states = new SCARD_READERSTATE[0];
    List<string> readers = new List<string>();
    int lastError;

    public int ReaderCount { get { return readers.Count; } }

    public bool CardPresent {
        get {
            foreach (SCARD_READERSTATE state in states)
                if (state.szReader != PNP && (state.dwCurrentState & SCARD_STATE_PRESENT) != 0) return true;
            return false;
        }
    }

    public List<string> Poll(uint timeout) {
        List<string> events = new List<string>();
        if (context == IntPtr.Zero && !Establish(events)) {
            Thread.Sleep((int)timeout);
            return events;
        }
        int status = SCardGetStatusChangeW(context, timeout, states, (uint)states.Length);
        if (status == SCARD_E_TIMEOUT) return events;
        if (status != 0) {
            Fail("SCardGetStatusChange", status, events);
            return events;
        }
        bool readersChanged = false;
        for (int i = 0; i < states.Length; i++) {
            uint before = states[i].dwCurrentState;
            uint after = states[i].dwEventState & ~SCARD_STATE_CHANGED;
            if ((states[i].dwEventState & SCARD_STATE_CHANGED) != 0) {
                if (states[i].szReader == PNP) readersChanged = true;
                else events.Add(DescribeChange(states[i], before, after));
            }
            states[i].dwCurrentState = after;
        }
        if (readersChanged) Rebuild(events);
        return events;
    }

    bool Establish(List<string> events) {
        IntPtr opened;
        int status = SCardEstablishContext(SCARD_SCOPE_USER, IntPtr.Zero, IntPtr.Zero, out opened);
        if (status != 0) {
            if (status != lastError) events.Add("SCardEstablishContext: " + Codes.Describe(status));
            lastError = status;
            return false;
        }
        context = opened;
        lastError = 0;
        events.Add("contexto PC/SC abierto");
        Rebuild(events);
        return context != IntPtr.Zero;
    }

    void Fail(string call, int status, List<string> events) {
        if (status != lastError) events.Add(call + ": " + Codes.Describe(status) + "; se cierra el contexto");
        lastError = status;
        SCardReleaseContext(context);
        context = IntPtr.Zero;
        states = new SCARD_READERSTATE[0];
        Announce(new List<string>(), events);
    }

    void Rebuild(List<string> events) {
        List<string> found;
        int status = ListReaders(out found);
        if (status != 0) {
            Fail("SCardListReaders", status, events);
            return;
        }
        Dictionary<string, uint> known = new Dictionary<string, uint>();
        foreach (SCARD_READERSTATE state in states) known[state.szReader] = state.dwCurrentState;
        List<SCARD_READERSTATE> rebuilt = new List<SCARD_READERSTATE>();
        foreach (string name in found) rebuilt.Add(NewState(name, known));
        rebuilt.Add(NewState(PNP, known));
        states = rebuilt.ToArray();
        Announce(found, events);
    }

    void Announce(List<string> found, List<string> events) {
        if (string.Join("|", found) == string.Join("|", readers)) return;
        readers = found;
        events.Add("lectores: [" + string.Join(", ", found) + "]");
    }

    static SCARD_READERSTATE NewState(string name, Dictionary<string, uint> known) {
        uint current;
        if (!known.TryGetValue(name, out current)) current = SCARD_STATE_UNAWARE;
        return new SCARD_READERSTATE { szReader = name, dwCurrentState = current, rgbAtr = new byte[36] };
    }

    int ListReaders(out List<string> found) {
        found = new List<string>();
        uint length = 0;
        int status = SCardListReadersW(context, null, null, ref length);
        if (status == SCARD_E_NO_READERS_AVAILABLE) return 0;
        if (status != 0) return status;
        char[] buffer = new char[length];
        status = SCardListReadersW(context, null, buffer, ref length);
        if (status == SCARD_E_NO_READERS_AVAILABLE) return 0;
        if (status != 0) return status;
        found = MultiString(buffer);
        return 0;
    }

    string DescribeChange(SCARD_READERSTATE state, uint before, uint after) {
        List<string> flags = new List<string>();
        foreach (KeyValuePair<uint, string> pair in StateNames)
            if ((after & pair.Key) != 0) flags.Add(pair.Value);
        string text = state.szReader + " -> " + (flags.Count > 0 ? string.Join("|", flags) : string.Format("0x{0:X}", after));
        bool inserted = (after & SCARD_STATE_PRESENT) != 0 && (before & SCARD_STATE_PRESENT) == 0;
        if (inserted && state.cbAtr > 0) text += "; " + CardType(state);
        return text;
    }

    string CardType(SCARD_READERSTATE state) {
        byte[] atr = new byte[state.cbAtr];
        Array.Copy(state.rgbAtr, atr, (int)state.cbAtr);
        uint length = 0;
        int status = SCardListCardsW(context, atr, IntPtr.Zero, 0, null, ref length);
        if (status != 0) return "tipo de tarjeta: " + Codes.Describe(status);
        char[] buffer = new char[length];
        status = SCardListCardsW(context, atr, IntPtr.Zero, 0, buffer, ref length);
        if (status != 0) return "tipo de tarjeta: " + Codes.Describe(status);
        List<string> cards = MultiString(buffer);
        if (cards.Count == 0) return "tipo de tarjeta: ninguno registrado para su ATR";
        List<string> parts = new List<string>();
        foreach (string card in cards) {
            List<string> providers = new List<string>();
            foreach (KeyValuePair<uint, string> pair in Providers)
                providers.Add(pair.Value + " " + ProviderName(card, pair.Key));
            parts.Add("'" + card + "' (" + string.Join(", ", providers) + ")");
        }
        return "tipo de tarjeta: " + string.Join("; ", parts);
    }

    string ProviderName(string card, uint providerId) {
        uint length = 0;
        int status = SCardGetCardTypeProviderNameW(context, card, providerId, null, ref length);
        if (status != 0) return Codes.Describe(status);
        char[] buffer = new char[length];
        status = SCardGetCardTypeProviderNameW(context, card, providerId, buffer, ref length);
        if (status != 0) return Codes.Describe(status);
        return new string(buffer).TrimEnd('\0');
    }

    static List<string> MultiString(char[] buffer) {
        List<string> items = new List<string>();
        foreach (string item in new string(buffer).Split('\0'))
            if (item.Length > 0) items.Add(item);
        return items;
    }
}

public sealed class KspKey {
    public string Name;
    public string Algorithm;
    public uint LegacySpec;
    public byte[] Certificate;
    public int CertificateStatus;
}

public sealed class KspListing {
    public int Status;
    public List<KspKey> Keys = new List<KspKey>();
}

public static class SmartCardKsp {
    const uint NCRYPT_SILENT_FLAG = 0x40;
    const int NTE_NO_MORE_ITEMS = unchecked((int)0x8009002A);
    const string PROVIDER = "Microsoft Smart Card Key Storage Provider";
    const string CERTIFICATE_PROPERTY = "SmartCardKeyCertificate";

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct NCryptKeyName {
        public string pszName;
        public string pszAlgid;
        public uint dwLegacyKeySpec;
        public uint dwFlags;
    }

    [DllImport("ncrypt.dll", CharSet = CharSet.Unicode)]
    static extern int NCryptOpenStorageProvider(out IntPtr provider, string name, uint flags);

    [DllImport("ncrypt.dll", CharSet = CharSet.Unicode)]
    static extern int NCryptEnumKeys(IntPtr provider, string scope, out IntPtr keyName, ref IntPtr enumState, uint flags);

    [DllImport("ncrypt.dll", CharSet = CharSet.Unicode)]
    static extern int NCryptOpenKey(IntPtr provider, out IntPtr key, string name, uint legacySpec, uint flags);

    [DllImport("ncrypt.dll", CharSet = CharSet.Unicode)]
    static extern int NCryptGetProperty(IntPtr handle, string property, byte[] output, uint size, out uint written, uint flags);

    [DllImport("ncrypt.dll")]
    static extern int NCryptFreeBuffer(IntPtr buffer);

    [DllImport("ncrypt.dll")]
    static extern int NCryptFreeObject(IntPtr handle);

    public static KspListing List() {
        KspListing listing = new KspListing();
        IntPtr provider;
        listing.Status = NCryptOpenStorageProvider(out provider, PROVIDER, 0);
        if (listing.Status != 0) return listing;
        IntPtr enumState = IntPtr.Zero;
        try {
            while (true) {
                IntPtr found;
                int status = NCryptEnumKeys(provider, null, out found, ref enumState, NCRYPT_SILENT_FLAG);
                if (status == NTE_NO_MORE_ITEMS) return listing;
                if (status != 0) {
                    listing.Status = status;
                    return listing;
                }
                NCryptKeyName name = (NCryptKeyName)Marshal.PtrToStructure(found, typeof(NCryptKeyName));
                NCryptFreeBuffer(found);
                listing.Keys.Add(Describe(provider, name));
            }
        } finally {
            if (enumState != IntPtr.Zero) NCryptFreeBuffer(enumState);
            NCryptFreeObject(provider);
        }
    }

    static KspKey Describe(IntPtr provider, NCryptKeyName name) {
        KspKey key = new KspKey { Name = name.pszName, Algorithm = name.pszAlgid, LegacySpec = name.dwLegacyKeySpec };
        IntPtr handle;
        key.CertificateStatus = NCryptOpenKey(provider, out handle, name.pszName, name.dwLegacyKeySpec, NCRYPT_SILENT_FLAG);
        if (key.CertificateStatus != 0) return key;
        try {
            uint size;
            key.CertificateStatus = NCryptGetProperty(handle, CERTIFICATE_PROPERTY, null, 0, out size, NCRYPT_SILENT_FLAG);
            if (key.CertificateStatus != 0) return key;
            byte[] certificate = new byte[size];
            key.CertificateStatus = NCryptGetProperty(handle, CERTIFICATE_PROPERTY, certificate, size, out size, NCRYPT_SILENT_FLAG);
            if (key.CertificateStatus != 0) return key;
            Array.Resize(ref certificate, (int)size);
            key.Certificate = certificate;
            return key;
        } finally {
            NCryptFreeObject(handle);
        }
    }
}

public sealed class OpenStore {
    const uint CERT_SYSTEM_STORE_CURRENT_USER = 0x00010000;
    const uint CERT_STORE_READONLY_FLAG = 0x00008000;
    const uint CERT_STORE_OPEN_EXISTING_FLAG = 0x00004000;
    const uint CERT_KEY_PROV_INFO_PROP_ID = 2;
    static readonly IntPtr CERT_STORE_PROV_SYSTEM_W = new IntPtr(10);

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
    static extern IntPtr CertOpenStore(IntPtr provider, uint encoding, IntPtr cryptProvider, uint flags, [MarshalAs(UnmanagedType.LPWStr)] string name);

    [DllImport("crypt32.dll", SetLastError = true)]
    static extern IntPtr CertEnumCertificatesInStore(IntPtr store, IntPtr previous);

    [DllImport("crypt32.dll", SetLastError = true)]
    static extern bool CertGetCertificateContextProperty(IntPtr cert, uint propId, IntPtr data, ref uint size);

    readonly IntPtr handle;

    public OpenStore(string name) {
        handle = CertOpenStore(CERT_STORE_PROV_SYSTEM_W, 0, IntPtr.Zero,
            CERT_SYSTEM_STORE_CURRENT_USER | CERT_STORE_READONLY_FLAG | CERT_STORE_OPEN_EXISTING_FLAG, name);
        if (handle == IntPtr.Zero) throw new InvalidOperationException("CertOpenStore " + Codes.Describe(Marshal.GetHRForLastWin32Error()));
    }

    public List<X509Certificate2> Certificates() {
        List<X509Certificate2> found = new List<X509Certificate2>();
        IntPtr context = IntPtr.Zero;
        while ((context = CertEnumCertificatesInStore(handle, context)) != IntPtr.Zero)
            found.Add(new X509Certificate2(context));
        return found;
    }

    public static string ProviderOf(IntPtr cert) {
        uint size = 0;
        if (!CertGetCertificateContextProperty(cert, CERT_KEY_PROV_INFO_PROP_ID, IntPtr.Zero, ref size)) return null;
        IntPtr buffer = Marshal.AllocHGlobal((int)size);
        try {
            if (!CertGetCertificateContextProperty(cert, CERT_KEY_PROV_INFO_PROP_ID, buffer, ref size)) return null;
            CRYPT_KEY_PROV_INFO info = (CRYPT_KEY_PROV_INFO)Marshal.PtrToStructure(buffer, typeof(CRYPT_KEY_PROV_INFO));
            return info.pwszProvName + " (tipo " + info.dwProvType + ", keySpec " + info.dwKeySpec + ")";
        } finally {
            Marshal.FreeHGlobal(buffer);
        }
    }
}
'@

$X509 = 'System.Security.Cryptography.X509Certificates'
$Clock = [System.Diagnostics.Stopwatch]::StartNew()
$Watch = New-Object PcscWatch
$LongOpen = New-Object OpenStore 'MY'
$script:Fresh = @{}
$script:Open = @{}
$script:LastChange = [TimeSpan]::Zero

function Say([string] $text) {
    $line = '[{0,8:N2} s] {1}' -f $Clock.Elapsed.TotalSeconds, $text
    Write-Host $line
    if ($Log) { Add-Content -Path $Log -Value $line -Encoding UTF8 }
}

function Get-CommonName([string] $distinguishedName) {
    if ($distinguishedName -match 'CN=([^,]+)') { $Matches[1].Trim() } else { '<sin CN>' }
}

function Test-Dnie($cert) {
    (Get-CommonName $cert.Issuer) -match '^AC (RAIZ )?DNIE'
}

function Get-Role($cert) {
    $constraints = $cert.Extensions | Where-Object { $_ -is [System.Security.Cryptography.X509Certificates.X509BasicConstraintsExtension] }
    if ($constraints -and $constraints.CertificateAuthority) { return 'CA' }
    $usage = $cert.Extensions | Where-Object { $_ -is [System.Security.Cryptography.X509Certificates.X509KeyUsageExtension] }
    if ($usage -and ($usage.KeyUsages -band [System.Security.Cryptography.X509Certificates.X509KeyUsageFlags]::NonRepudiation)) { 'firma' } else { 'autenticación' }
}

function Describe-Certificate($cert) {
    $role = Get-Role $cert
    if ($role -eq 'CA') { return "CA '$(Get-CommonName $cert.Subject)' emitida por '$(Get-CommonName $cert.Issuer)'" }
    $provider = if ($cert.HasPrivateKey) { [OpenStore]::ProviderOf($cert.Handle) } else { $null }
    $key = if ($provider) { "clave en $provider" } else { 'sin clave' }
    $expired = if ($cert.NotAfter -lt (Get-Date)) { ', caducado' } else { '' }
    "$role de '$(Get-CommonName $cert.Issuer)', $key$expired"
}

function ConvertTo-Entries([string] $store, $certificates) {
    $entries = @{}
    foreach ($cert in $certificates) {
        if (-not (Test-Dnie $cert)) { continue }
        $text = Describe-Certificate $cert
        $entries["$store|$($cert.Thumbprint)"] = [pscustomobject]@{ Text = "$store`: $text"; OnCard = $text -match 'Smart Card' }
    }
    $entries
}

function Read-FreshStores {
    $entries = @{}
    foreach ($name in 'My', 'CA', 'Root') {
        $store = New-Object "$X509.X509Store" $name, 'CurrentUser'
        $store.Open('ReadOnly, OpenExistingOnly')
        try {
            $found = ConvertTo-Entries $name $store.Certificates
            foreach ($key in $found.Keys) { $entries[$key] = $found[$key] }
        } finally {
            $store.Close()
        }
    }
    $entries
}

function Read-OpenStore {
    ConvertTo-Entries 'My' $LongOpen.Certificates()
}

function Count-OnCard($entries) {
    @($entries.Values | Where-Object { $_.OnCard }).Count
}

function Compare-Entries([string] $label, $old, $new, $eventAt) {
    $since = if ($eventAt) { ' ({0:N2} s tras el evento)' -f ($Clock.Elapsed - $eventAt).TotalSeconds } else { '' }
    $changed = $false
    foreach ($key in $new.Keys) {
        if (-not $old.ContainsKey($key)) { Say "$label + $($new[$key].Text)$since"; $changed = $true }
    }
    foreach ($key in $old.Keys) {
        if (-not $new.ContainsKey($key)) { Say "$label - $($old[$key].Text)$since"; $changed = $true }
    }
    $changed
}

function Show-Entries([string] $label, $entries) {
    Say "$label`: $($entries.Count) certificados del DNIe, $(Count-OnCard $entries) con la clave en la tarjeta"
    foreach ($entry in ($entries.Values | Sort-Object Text)) { Say "  $($entry.Text)" }
}

function Update-Stores($eventAt) {
    $fresh = Read-FreshStores
    $open = Read-OpenStore
    $changedFresh = Compare-Entries 'almacén reabierto' $script:Fresh $fresh $eventAt
    $changedOpen = Compare-Entries 'almacén abierto desde el inicio' $script:Open $open $eventAt
    if ($changedFresh -or $changedOpen) { $script:LastChange = $Clock.Elapsed }
    $script:Fresh = $fresh
    $script:Open = $open
}

function Get-SafeKeyName([string] $name) {
    $leaf = ($name -split '\\')[-1]
    if ($leaf -match '^(Cert|Kpriv|Kpub)[A-Za-z]*$') { $leaf } else { '<redactado>' }
}

function Show-Ksp([string] $label) {
    $started = $Clock.Elapsed
    $listing = [SmartCardKsp]::List()
    $took = ($Clock.Elapsed - $started).TotalMilliseconds
    $status = if ($listing.Status -eq 0) { 'bien' } else { [Codes]::Describe($listing.Status) }
    Say ("KSP de tarjeta ({0}): {1} claves en {2:N0} ms, estado {3}" -f $label, $listing.Keys.Count, $took, $status)
    foreach ($key in $listing.Keys) {
        $certificate = if ($key.Certificate) {
            $cert = New-Object "$X509.X509Certificate2" (, $key.Certificate)
            $inStore = if ($script:Fresh.ContainsKey("My|$($cert.Thumbprint)")) { 'está' } else { 'no está' }
            "$(Get-Role $cert) de '$(Get-CommonName $cert.Issuer)', $inStore en My"
        } else {
            "sin certificado: $([Codes]::Describe($key.CertificateStatus))"
        }
        Say "  $(Get-SafeKeyName $key.Name), $($key.Algorithm), keySpec $($key.LegacySpec): $certificate"
    }
    $listing.Keys.Count
}

function Wait-KspKeys($eventAt) {
    $limit = [TimeSpan]::FromSeconds(30)
    $lastStatus = $null
    while (($Clock.Elapsed - $eventAt) -lt $limit) {
        $listing = [SmartCardKsp]::List()
        if ($listing.Keys.Count -gt 0) {
            Say ('KSP de tarjeta: {0} claves a los {1:N2} s del evento' -f $listing.Keys.Count, ($Clock.Elapsed - $eventAt).TotalSeconds)
            return
        }
        if ($listing.Status -ne $lastStatus) {
            Say "KSP de tarjeta: sin claves todavía, estado $([Codes]::Describe($listing.Status))"
            $lastStatus = $listing.Status
        }
        Start-Sleep -Milliseconds 250
    }
    Say 'KSP de tarjeta: sin claves 30 s después del evento'
}

function Test-EnterPressed {
    try {
        while ([Console]::KeyAvailable) {
            if ([Console]::ReadKey($true).Key -eq 'Enter') { return $true }
        }
    } catch [System.InvalidOperationException] { }
    $false
}

function Poll-Readers([uint32] $timeout) {
    foreach ($line in $Watch.Poll($timeout)) { Say "PC/SC: $line" }
}

function Watch-Step([string] $title, [string] $instruction, [scriptblock] $reached, [bool] $cardExpected, [switch] $KspOnEvent) {
    Say "=== $title"
    Say "$instruction (Enter salta el paso)"
    $startedAt = $Clock.Elapsed
    $eventAt = $null
    while ($true) {
        Poll-Readers 250
        if (-not $eventAt -and (& $reached)) {
            $eventAt = $Clock.Elapsed
            Say "Evento del lector a los $('{0:N2}' -f ($eventAt - $startedAt).TotalSeconds) s de la instrucción; se vigilan los almacenes $Window s"
            if ($KspOnEvent) { Wait-KspKeys $eventAt }
        }
        Update-Stores $eventAt
        if (Test-EnterPressed) { Say 'Paso saltado con Enter'; break }
        if (-not $eventAt) {
            if (($Clock.Elapsed - $startedAt).TotalSeconds -gt 120) { Say 'Sin evento del lector en 120 s; se sigue'; break }
            continue
        }
        $settled = (((Count-OnCard $script:Fresh) -gt 0) -eq $cardExpected) -and
            (((Count-OnCard $script:Open) -gt 0) -eq $cardExpected) -and
            ($Clock.Elapsed - $script:LastChange).TotalSeconds -gt 10 -and
            ($Clock.Elapsed - $eventAt).TotalSeconds -gt 10
        if ($settled) { Say 'Los dos almacenes muestran lo esperado y llevan 10 s quietos'; break }
        if (($Clock.Elapsed - $eventAt).TotalSeconds -gt $Window) { Say "Fin de los $Window s de vigilancia"; break }
    }
    Show-Entries 'Almacén reabierto' $script:Fresh
    Show-Entries 'Almacén abierto desde el inicio' $script:Open
}

$running = Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match '^(rfirma|autofirma)' }
if ($running) {
    Say "Cierra antes estos procesos: $(($running.ProcessName | Sort-Object -Unique) -join ', ')"
    exit 2
}

Say "Windows $([Environment]::OSVersion.Version)"
foreach ($service in 'SCardSvr', 'CertPropSvc', 'ScDeviceEnum') {
    $found = Get-Service -Name $service -ErrorAction SilentlyContinue
    if ($found) { Say "Servicio $service`: $($found.Status), inicio $($found.StartType)" } else { Say "Servicio $service`: no existe" }
}
$policy = Get-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\CertProp' -ErrorAction SilentlyContinue
if ($policy) {
    $values = $policy.PSObject.Properties | Where-Object { $_.Name -notmatch '^PS' } | ForEach-Object { "$($_.Name)=$($_.Value)" }
    Say "Directiva CertProp: $($values -join ', ')"
} else {
    Say 'Directiva CertProp: sin configurar'
}

Poll-Readers 0
Poll-Readers 0
if (-not $Watch.CardPresent) {
    Say 'Conecta el lector y mete el DNIe; la sonda espera.'
    while (-not $Watch.CardPresent) { Poll-Readers 1000 }
    Say 'Tarjeta dentro: espera a que Windows termine de leerla y pulsa Enter.'
    while (-not (Test-EnterPressed)) { Poll-Readers 250 }
}

Say '=== Punto de partida'
$script:Fresh = Read-FreshStores
$script:Open = Read-OpenStore
Show-Entries 'Almacén reabierto' $script:Fresh
Show-Entries 'Almacén abierto desde el inicio' $script:Open
$null = Show-Ksp 'con la tarjeta dentro'

Watch-Step 'Sacar la tarjeta' 'Saca el DNIe del lector y déjalo fuera' { -not $Watch.CardPresent } $false
$null = Show-Ksp 'sin tarjeta'

Watch-Step 'Meter la tarjeta' 'Mete el DNIe' { $Watch.CardPresent } $true
$null = Show-Ksp 'tras meterla'

Watch-Step 'Sacarla otra vez' 'Saca el DNIe otra vez' { -not $Watch.CardPresent } $false
Watch-Step 'Meterla listando por el KSP' 'Mete el DNIe; esta vez se lista por el KSP en cuanto PC/SC la ve' { $Watch.CardPresent } $true -KspOnEvent

Watch-Step 'Desconectar el lector' 'Desconecta el lector con el DNIe dentro; si va integrado, pulsa Enter' { $Watch.ReaderCount -eq 0 } $false
Watch-Step 'Conectar el lector' 'Vuelve a conectar el lector con el DNIe dentro' { $Watch.CardPresent } $true
$null = Show-Ksp 'al final'

Say 'Fin. Copia la salida en el ticket tal cual.'
