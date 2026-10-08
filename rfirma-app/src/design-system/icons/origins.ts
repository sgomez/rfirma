//! La familia de origen y la licencia de cada icono de la interfaz.

interface IconOrigin {
  family: string;
  license: string;
}

const HEROICONS: IconOrigin = { family: "Heroicons", license: "MIT" };
const OWN: IconOrigin = { family: "rFirma", license: "EUPL-1.2" };

export const ICON_ORIGINS: Record<string, IconOrigin> = {
  AlertIcon: HEROICONS,
  CheckCircleIcon: HEROICONS,
  CheckingIcon: HEROICONS,
  CrossCircleIcon: HEROICONS,
  IdentificationIcon: HEROICONS,
  NotApplicableIcon: HEROICONS,
  CheckIcon: OWN,
  ChevronDownIcon: OWN,
  ChevronLeftIcon: OWN,
  ChevronRightIcon: OWN,
  ChevronsLeftIcon: OWN,
  ChevronsRightIcon: OWN,
  ClockIcon: OWN,
  CloseIcon: OWN,
  CopyIcon: OWN,
  ExternalLinkIcon: OWN,
  FileIcon: OWN,
  FitIcon: OWN,
  FitPageIcon: OWN,
  FolderIcon: OWN,
  InfoIcon: OWN,
  MenuIcon: OWN,
  MinusIcon: OWN,
  MoveIcon: OWN,
  NewVersionIcon: OWN,
  PersonIcon: OWN,
  PlusIcon: OWN,
  RevokedIcon: OWN,
  RubricIcon: OWN,
  SearchIcon: OWN,
  SignedMarkIcon: OWN,
  SpinnerIcon: OWN,
  UploadIcon: OWN,
  UpToDateIcon: OWN,
};
