export interface QueryData<T> {
  data: T | null;
  status: "pending" | "success" | "error";
}
