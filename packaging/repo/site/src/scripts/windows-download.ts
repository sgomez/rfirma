const FEED = "/windows/latest.json";

interface Feed {
  platforms?: Record<string, { url?: string }>;
}

/** Apunta el enlace al instalador que anuncia el servidor; si no lo consigue, se queda en la Release. */
export async function mountWindowsDownload(link: HTMLAnchorElement): Promise<void> {
  try {
    const response = await fetch(FEED);
    if (!response.ok) {
      return;
    }
    const feed = (await response.json()) as Feed;
    const url = feed.platforms?.["windows-x86_64"]?.url;
    if (url?.startsWith(`${location.origin}/windows/`)) {
      link.href = url;
    }
  } catch {
    // Sin el feed el enlace conserva su destino: la última Release.
  }
}
