import ky from "ky";

export const API_BASE_URL = "http://localhost:8080/api";

export const deviceAPI = ky.create({
  prefixUrl: API_BASE_URL,
  headers: {
    "Content-Type": "application/json",
    accept: "application/json",
  },
});
