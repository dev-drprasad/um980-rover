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

interface TrackPayload {
  name: string;
  type: "track";
  coordinates: [number, number][];
}
interface BookmarkPayload {
  name: string;
  type: "bookmark";
  coordinates: [number, number];
}

interface DraftPayload {
  name: string;
  type: "draft";
  coordinates: [number, number][];
}

type SavePayload = TrackPayload | BookmarkPayload | DraftPayload;

export function useSaveTrack() {
  const [{ data, status }, setState] = useState<QueryData<SerialDevice[]>>({
    data: null,
    status: "pending",
  });
  const [payloadStr, setPayloadStr] = useState("");

  const save = useCallback((payload: SavePayload) => {
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
      const payload = JSON.parse(payloadStr) as SavePayload;
      try {
        await deviceAPI.post<SerialDevice[]>(
          `track/${payload.type === "draft" ? "draft" : randomUUID()}`,
          {
            body: payloadStr,
          },
        );

        setState({ data: null, status: "success" });
      } catch (error) {
        console.error(error);
        setState({ data: null, status: "error" });
      }
    })();
  }, [payloadStr]);

  return { data: Object.values(data || {}), status, save };
}
