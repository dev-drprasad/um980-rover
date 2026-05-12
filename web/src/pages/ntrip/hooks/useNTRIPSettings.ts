import { deviceAPI } from "../../../core";
import type { NTRIPSettingsAPI } from "../../../entities/types/NTRIPSettings";
import { useAPI } from "../../../shared/hooks/useAPI";

export function useNTRIPSettings() {
  const { data, status, refetch } = useAPI({
    queryFn: () =>
      deviceAPI.get("ntrip-settings").json<NTRIPSettingsAPI | null>(),
  });

  return {
    data: data,
    status,
    refetch,
  };
}
