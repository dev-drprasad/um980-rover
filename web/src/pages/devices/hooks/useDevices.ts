import { useEffect, useState } from "react";
import { deviceAPI } from "../../../core";
import type { QueryData } from "../../../shared/utils/types/QueryData";

interface SerialDevice {
  id: string;
  name: string;
  vendor: string;
  product: string;
}

export function useDevices() {
  const [{ data, status }, setState] = useState<QueryData<SerialDevice[]>>({
    data: null,
    status: "pending",
  });

  useEffect(() => {
    (async () => {
      setState({ data: null, status: "pending" });
      try {
        const devices = await deviceAPI.get<SerialDevice[]>("devices").json();
        console.log("devices :>> ", devices);
        setState({ data: devices, status: "success" });
      } catch (error) {
        console.error(error);
        setState({ data: null, status: "error" });
      }
    })();
  }, []);

  return { data: Object.values(data || {}), status };
}
