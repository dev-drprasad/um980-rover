import { useCallback, useEffect, useState } from "react";
import type { QueryData } from "../utils/types/QueryData";

export function useAPI<T>({ queryFn }: { queryFn: () => Promise<T> }) {
  const [{ data, status }, setState] = useState<QueryData<T>>({
    data: null,
    status: "pending",
  });

  const refetch = useCallback(() => {
    setState((state) => ({ ...state, status: "pending" }));
  }, []);

  useEffect(() => {
    if (status !== "pending") return;
    (async () => {
      setState({ data: null, status: "fetching" });
      try {
        const result = await queryFn();
        setState({
          data: result,
          status: "success",
        });
      } catch (error) {
        console.error(error);
        setState({ data: null, status: "error" });
      }
    })();
  }, [queryFn, status]);

  return { data: data, status, refetch };
}
