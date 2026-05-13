import { useState } from "react";
import type { Bluetooth } from "../types/Bluetooth";

declare global {
  interface Navigator {
    bluetooth: Bluetooth;
  }
}

const SERVICE_UUID = "12345678-1234-5678-1234-56789abcdef0";
const CHAR_UUID = "abcdef01-1234-5678-1234-56789abcdef0";

export function RoverSetupPage() {
  const [ipAddress, setIpAddress] = useState<string>("");
  const [status, setStatus] = useState<string>("Disconnected");
  const [deviceName, setDeviceName] = useState<string>("");

  const connectBluetooth = async () => {
    try {
      setStatus("Requesting Bluetooth device...");

      const device = await navigator.bluetooth.requestDevice({
        filters: [
          {
            name: "RadxaZero3W",
          },
        ],
        optionalServices: [SERVICE_UUID],
      });

      setDeviceName(device.name || "Unknown Device");

      setStatus("Connecting...");

      const server = await device.gatt?.connect();

      if (!server) {
        throw new Error("Failed to connect to GATT server");
      }

      setStatus("Connected");

      // Optional reconnect handler
      device.addEventListener("gattserverdisconnected", () => {
        setStatus("Disconnected");
      });

      const service = await server.getPrimaryService(SERVICE_UUID);

      const characteristic = await service.getCharacteristic(CHAR_UUID);

      const value = await characteristic.readValue();

      const decoder = new TextDecoder("utf-8");

      const ip = decoder.decode(value);

      setIpAddress(ip);

      console.log("IP Address:", ip);
    } catch (error) {
      console.error(error);

      if (error instanceof Error) {
        setStatus(`Error: ${error.message}`);
      } else {
        setStatus("Unknown error");
      }
    }
  };

  return (
    <div
      style={{
        fontFamily: "Arial",
        padding: "2rem",
      }}
    >
      <h1>Radxa BLE IP Reader</h1>

      <button
        onClick={connectBluetooth}
        style={{
          padding: "12px 20px",
          fontSize: "16px",
          cursor: "pointer",
        }}
      >
        Connect to Radxa
      </button>

      <div style={{ marginTop: "2rem" }}>
        <p>
          <strong>Status:</strong> {status}
        </p>

        <p>
          <strong>Device:</strong> {deviceName}
        </p>

        <p>
          <strong>IP Address:</strong> {ipAddress || "Not Read"}
        </p>
      </div>
    </div>
  );
}
