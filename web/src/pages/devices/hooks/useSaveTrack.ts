import { useCallback, useEffect, useState } from "react";
import { deviceAPI } from "../../../core";
import type { QueryData } from "../../../shared/utils/types/QueryData";
import { randomUUID } from "../../../shared/utils/randomUUID";

interface SerialDevice {
  id: string;
  name: string;
  vendor: string;
  product: string;
}

export function useSaveTrack() {
  const [{ data, status }, setState] = useState<QueryData<SerialDevice[]>>({
    data: null,
    status: "pending",
  });
  const [payloadStr, setPayloadStr] = useState("");

  const save = useCallback((payload: unknown) => {
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
        await deviceAPI.post<SerialDevice[]>(`track/${randomUUID()}`, {
          body: payloadStr,
        });

        setState({ data: null, status: "success" });
      } catch (error) {
        console.error(error);
        setState({ data: null, status: "error" });
      }
    })();
  }, [payloadStr]);

  return { data: Object.values(data || {}), status, save };
}
