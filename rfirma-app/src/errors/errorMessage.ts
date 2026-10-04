//! El mensaje que cuenta cada situación de error: el backend nombra muchas situaciones y la persona lee pocos mensajes, uno por lo que puede hacer.

import type { TFunction } from "i18next";

type ErrorMessage =
  | "retry"
  | "otherCertificate"
  | "certificateExpired"
  | "checkToken"
  | "otherCertificateFile"
  | "otherImage"
  | "otherFolder"
  | "reopenDocument"
  | "moveVisibleSignature"
  | "handlerNotAvailable"
  | "incorrectPin"
  | "pinLocked"
  | "moduleNotFound"
  | "incorrectPkcs12Password"
  | "removalNotSupported"
  | "notAPdf"
  | "documentEncrypted"
  | "documentCertified"
  | "droppedSomeDiscarded"
  | "renderFailed"
  | "noFreeName"
  | "noKeyring"
  | "keyringPinMissing";

/** Cada situación que puede mandar el backend, con el mensaje que la cuenta. */
export const MESSAGE_OF = {
  unknown: "retry",
  expiredSession: "retry",
  sealMismatch: "retry",
  bridgeFailed: "retry",
  settingNotSaved: "retry",
  activityNotForgotten: "retry",
  handlerListUnreadable: "retry",
  handlerListUnwritable: "retry",
  certificateNotYetValid: "otherCertificate",
  certificateRevoked: "otherCertificate",
  certificateUnreadable: "otherCertificate",
  keyKindUnsupported: "otherCertificate",
  mechanismNotOffered: "otherCertificate",
  certificateExpired: "certificateExpired",
  tokenAbsent: "checkToken",
  certificateNotFound: "checkToken",
  pkcs12Unreadable: "otherCertificateFile",
  pkcs12NoPrivateKey: "otherCertificateFile",
  notAnAcceptedImage: "otherImage",
  damagedImage: "otherImage",
  imageTooLarge: "otherImage",
  sourceUnreadable: "otherImage",
  storeUnwritable: "otherImage",
  storeUnreadable: "otherImage",
  folderMissing: "otherFolder",
  notAFolder: "otherFolder",
  folderUnreadable: "otherFolder",
  folderUnwritable: "otherFolder",
  documentUnreadable: "reopenDocument",
  droppedFileUnreadable: "reopenDocument",
  boxOutOfPage: "moveVisibleSignature",
  pageOutOfDocument: "moveVisibleSignature",
  handlerNotAvailable: "handlerNotAvailable",
  incorrectPin: "incorrectPin",
  pinLocked: "pinLocked",
  moduleNotFound: "moduleNotFound",
  incorrectPkcs12Password: "incorrectPkcs12Password",
  removalNotSupported: "removalNotSupported",
  notAPdf: "notAPdf",
  documentEncrypted: "documentEncrypted",
  documentCertified: "documentCertified",
  droppedSomeDiscarded: "droppedSomeDiscarded",
  renderFailed: "renderFailed",
  noFreeName: "noFreeName",
  noKeyring: "noKeyring",
  keyringPinMissing: "keyringPinMissing",
} as const satisfies Record<string, ErrorMessage>;

/** Lo que el backend sabe nombrar de un fallo. */
export type ErrorSituation = keyof typeof MESSAGE_OF;

/** Un mensaje de error traducido; sin `body`, el título lo dice todo. */
interface ErrorText {
  title: string;
  body?: string;
}

/** El mensaje traducido de una situación. */
export function errorText(situation: ErrorSituation, t: TFunction): ErrorText {
  switch (MESSAGE_OF[situation]) {
    case "retry":
      return { title: t("errors.messages.retry.title"), body: t("errors.messages.retry.body") };
    case "otherCertificate":
      return { title: t("errors.messages.otherCertificate.title") };
    case "certificateExpired":
      return {
        title: t("errors.messages.certificateExpired.title"),
        body: t("errors.messages.certificateExpired.body"),
      };
    case "checkToken":
      return {
        title: t("errors.messages.checkToken.title"),
        body: t("errors.messages.checkToken.body"),
      };
    case "otherCertificateFile":
      return {
        title: t("errors.messages.otherCertificateFile.title"),
        body: t("errors.messages.otherCertificateFile.body"),
      };
    case "otherImage":
      return {
        title: t("errors.messages.otherImage.title"),
        body: t("errors.messages.otherImage.body"),
      };
    case "otherFolder":
      return {
        title: t("errors.messages.otherFolder.title"),
        body: t("errors.messages.otherFolder.body"),
      };
    case "reopenDocument":
      return {
        title: t("errors.messages.reopenDocument.title"),
        body: t("errors.messages.reopenDocument.body"),
      };
    case "moveVisibleSignature":
      return { title: t("errors.messages.moveVisibleSignature.title") };
    case "handlerNotAvailable":
      return {
        title: t("errors.messages.handlerNotAvailable.title"),
        body: t("errors.messages.handlerNotAvailable.body"),
      };
    case "incorrectPin":
      return {
        title: t("errors.messages.incorrectPin.title"),
        body: t("errors.messages.incorrectPin.body"),
      };
    case "pinLocked":
      return {
        title: t("errors.messages.pinLocked.title"),
        body: t("errors.messages.pinLocked.body"),
      };
    case "moduleNotFound":
      return {
        title: t("errors.messages.moduleNotFound.title"),
        body: t("errors.messages.moduleNotFound.body"),
      };
    case "incorrectPkcs12Password":
      return { title: t("errors.messages.incorrectPkcs12Password.title") };
    case "removalNotSupported":
      return {
        title: t("errors.messages.removalNotSupported.title"),
        body: t("errors.messages.removalNotSupported.body"),
      };
    case "notAPdf":
      return { title: t("errors.messages.notAPdf.title") };
    case "documentEncrypted":
      return {
        title: t("errors.messages.documentEncrypted.title"),
        body: t("errors.messages.documentEncrypted.body"),
      };
    case "documentCertified":
      return {
        title: t("errors.messages.documentCertified.title"),
        body: t("errors.messages.documentCertified.body"),
      };
    case "droppedSomeDiscarded":
      return {
        title: t("errors.messages.droppedSomeDiscarded.title"),
        body: t("errors.messages.droppedSomeDiscarded.body"),
      };
    case "renderFailed":
      return {
        title: t("errors.messages.renderFailed.title"),
        body: t("errors.messages.renderFailed.body"),
      };
    case "noFreeName":
      return {
        title: t("errors.messages.noFreeName.title"),
        body: t("errors.messages.noFreeName.body"),
      };
    case "noKeyring":
      return {
        title: t("errors.messages.noKeyring.title"),
        body: t("errors.messages.noKeyring.body"),
      };
    case "keyringPinMissing":
      return {
        title: t("errors.messages.keyringPinMissing.title"),
        body: t("errors.messages.keyringPinMissing.body"),
      };
  }
}
