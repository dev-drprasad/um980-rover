import { useCallback, useEffect, useState } from "react";
import { deviceAPI } from "../../../core";

export function useMoveDraftToTrack() {
  const [{ data, status }, setState] = useState<{
    data: null;
    status: "pending" | "success" | "error";
  }>({
    data: null,
    status: "pending",
  });
  const [payloadStr, setPayloadStr] = useState("");

  const save = useCallback((payload: { name: string }) => {
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
        await deviceAPI.post(`draft/save`, { body: payloadStr });

        setState({ data: null, status: "success" });
      } catch (error) {
        console.error(error);
        setState({ data: null, status: "error" });
      }
    })();
  }, [payloadStr]);

  return { data, status, save };
}
