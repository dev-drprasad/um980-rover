import { useEffect, type SubmitEvent } from "react";
import { FilledButton } from "../../../shared/button";
import { FormInput } from "../../../shared/formInputText";
import { useNTRIPSettings } from "../hooks/useNTRIPSettings";
import { useSaveNTRIPSetttings } from "../hooks/useSaveNTRIPSetttings";
import type { NTRIPSettings } from "../../../entities/types/NTRIPSettings";
import "./NTRIPSettingsPage.css";

export function NTRIPSettingsPage() {
  const { data, refetch } = useNTRIPSettings();
  const { save, status: saveStatus } = useSaveNTRIPSetttings();

  useEffect(() => {
    if (saveStatus === "success") {
      refetch();
    }
  }, [refetch, saveStatus]);

  if (!data) {
    return "loading...";
  }
  return (
    <NTRIPSettingsForm
      settings={data}
      onSubmit={save}
      isSubmitting={saveStatus === "fetching"}
    />
  );
}
function NTRIPSettingsForm({
  settings,
  onSubmit,
  isSubmitting,
}: {
  isSubmitting: boolean;
  onSubmit?: (settings: NTRIPSettings) => void;
  settings: NTRIPSettings;
}) {
  const handleSubmit = (e: SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const host = formData.get("host") as string;
    const mountpoint = formData.get("mountpoint") as string;
    const username = formData.get("username") as string;
    const password = formData.get("password") as string;
    const settings = { host, mountpoint, username, password };
    console.log("NTRIP Settings Submitted:", settings);
    onSubmit?.(settings);
  };
  return (
    <form className="ntripSettingsForm" onSubmit={handleSubmit}>
      <FormInput
        label="Host"
        name="host"
        defaultValue={settings.host}
        required
      />
      <FormInput
        label="Mountpoint"
        name="mountpoint"
        defaultValue={settings.mountpoint}
        required
      />
      <FormInput
        label="Username"
        name="username"
        defaultValue={settings.username}
        required
      />
      <FormInput
        label="Password"
        name="password"
        type="password"
        defaultValue={settings.password}
      />
      <FilledButton type="submit" tabIndex={2} disabled={isSubmitting}>
        Confirm
      </FilledButton>
    </form>
  );
}
