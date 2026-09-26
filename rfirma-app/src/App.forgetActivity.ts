/** Olvida los ajustes y los documentos, aunque el primero falle, y relanza el primer fallo. */
export async function forgetActivity(
  forgetPreferences: () => Promise<void>,
  forgetDocuments: () => Promise<void>,
): Promise<void> {
  // El centinela envuelve el valor: un rechazo con `null` no se perdería.
  let failure: { thrown: unknown } | null = null;
  for (const forget of [forgetPreferences, forgetDocuments]) {
    try {
      await forget();
    } catch (thrown) {
      failure ??= { thrown };
    }
  }
  if (failure !== null) throw failure.thrown;
}
