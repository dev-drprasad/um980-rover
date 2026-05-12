import { useCallback, useEffect, useState } from "react";
import { deviceAPI } from "../../../core";
import type { QueryData } from "../../../shared/utils/types/QueryData";
import type { NTRIPSettings } from "../../../entities/types/NTRIPSettings";

export function useSaveNTRIPSetttings() {
  const [{ data, status }, setState] = useState<QueryData<NTRIPSettings>>({
    data: null,
    status: "pending",
  });
  const [payloadStr, setPayloadStr] = useState("");

  const save = useCallback((payload: NTRIPSettings) => {
    try {
      setPayloadStr(JSON.stringify(payload));
    } catch (error) {
      console.error("Failed to stringify payload:", error);
    }
  }, []);

  useEffect(() => {
    (async () => {
      if (!payloadStr) return;
      setState({ data: null, status: "pending" });
      try {
        await deviceAPI.post<NTRIPSettings>("ntrip-settings", {
          body: payloadStr,
        });

        setState({ data: null, status: "success" });
      } catch (error) {
        console.error(error);
        setState({ data: null, status: "error" });
      }
    })();
  }, [payloadStr]);

  return { data, status, save };
}
