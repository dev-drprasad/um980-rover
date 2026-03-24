import { useEffect, useState } from "react";
import { API_HOST } from "../../../core";

export function useNMEAMessages() {
  const [messages, setMessages] = useState<
    { timestamp: string; message: string }[]
  >([]);

  useEffect(() => {
    const handleEvents = (event: MessageEvent<unknown>) => {
      if (typeof event.data !== "string") {
        return;
      }
      const data = event.data;
      let parsedData: unknown | null = null;
      try {
        parsedData = JSON.parse(data);
      } catch (error) {
        console.error("Error parsing WebSocket message:", error);
      }
      if (typeof parsedData !== "object" || parsedData === null) return;
      if (
        !("event" in parsedData) ||
        typeof parsedData.event !== "string" ||
        !("data" in parsedData)
      )
        return;
      switch (parsedData.event) {
        case "messages":
          setMessages((prev) => [
            ...(parsedData.data as string[]).map((msg) => ({
              timestamp: new Date().toISOString().slice(11, -1),
              message: msg,
            })),
            ...prev.slice(-99), // Keep only the last 99
          ]);
          break;
        default:
          console.warn("Unknown event type:", parsedData.event);
      }
    };
    const socket = new WebSocket(`ws://${API_HOST}/nmea-ws`);
    socket.addEventListener("message", handleEvents);
    return () => {
      socket.removeEventListener("message", handleEvents);
      socket.close();
    };
  }, []);

  return messages;
}
