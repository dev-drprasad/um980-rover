import { useDevices } from "../hooks/useDevices";

export function DevicesPage() {
  const { data } = useDevices();

  if (!data) {
    return <div>Loading...</div>;
  }

  return (
    <div>
      <ul>
        {data.map((device) => (
          <li key={device.id}>
            <div>{device.name}</div>
            <div>
              {device.vendor}: {device.product}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
