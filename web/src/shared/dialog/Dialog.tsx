import { useRef } from "react";
import "./Dialog.css";
import { IconButton } from "../button";
import { CloseIcon } from "../icons";
import { createPortal } from "react-dom";

export function Dialog({
  children,
  onClose,
}: {
  children: React.ReactNode;
  onClose: () => void;
}) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  const handleDialogClose = () => {
    const dialog = dialogRef.current;
    if (dialog) {
      dialog.close();
    }
    onClose();
  };
  return createPortal(
    <div className="dialog-mask">
      <dialog ref={dialogRef} onClose={handleDialogClose} open>
        {children}
        <IconButton
          className="close-dialog-button"
          onClick={handleDialogClose}
          tabIndex={4}
        >
          <CloseIcon />
        </IconButton>
      </dialog>
    </div>,
    document.body,
  );
}
