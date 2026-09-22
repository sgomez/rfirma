/** Las notificaciones del escritorio, si el navegador las tiene y la persona las permite. */
export function askToNotify(): void {
  if (typeof Notification === "undefined" || Notification.permission !== "default") return;
  void Notification.requestPermission();
}

export function notify(body: string): void {
  if (typeof Notification === "undefined" || Notification.permission !== "granted") return;
  new Notification("Suite de conformidad", { body });
}
