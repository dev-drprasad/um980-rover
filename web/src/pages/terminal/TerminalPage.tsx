import { useNMEAMessages } from "./hooks/useNMEAMessages";
import "./TerminalPage.css";

export function TerminalPage() {
  const messages = useNMEAMessages();
  return (
    <div className="terminal-page">
      <ul>
        {messages.map((message, index) => (
          <li key={message.timestamp + index}>
            {message.timestamp} | <span className="msg">{message.message}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
