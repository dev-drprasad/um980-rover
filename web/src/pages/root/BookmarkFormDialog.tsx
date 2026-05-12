import { FilledButton } from "../../shared/button";
import { FormInput } from "../../shared/formInputText";
import { Dialog } from "../../shared/dialog";
import "./BookmarkFormDialog.css";

export function BookmarkFormDialog({
  onConfirm,
  onClose,
}: {
  onConfirm: (name: string) => void;
  onClose: () => void;
}) {
  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const name = formData.get("name") as string;
    if (name) {
      onConfirm(name);
    }
  };

  return (
    <Dialog onClose={onClose}>
      <form className="bookmark-form" onSubmit={handleSubmit}>
        <FormInput label="Name" name="name" />
        <FilledButton type="submit" tabIndex={2}>
          Confirm
        </FilledButton>
      </form>
    </Dialog>
  );
}
