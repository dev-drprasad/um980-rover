export interface QueryData<T> {
  data: T | null;
  status: "pending" | "fetching" | "success" | "error";
}
