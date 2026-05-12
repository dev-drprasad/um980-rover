import "./FormInput.css";

interface FormInputProps {
  label: string;
  name: string;
  defaultValue?: string;
  placeholder?: string;
  autoComplete?: string;
  required?: boolean;
  type?: "text" | "number" | "password" | "file";
  accept?: string;
}

export function FormInput({
  label,
  name,
  defaultValue,
  placeholder,
  autoComplete = "off",
  type = "text",
  required = false,
  ...restProps
}: FormInputProps) {
  return (
    <label className="label">
      <span className="label-text">{label}:</span>
      <input
        tabIndex={0}
        name={name}
        type={type}
        autoComplete={autoComplete}
        autoFocus
        defaultValue={defaultValue}
        placeholder={placeholder}
        required={required}
        {...restProps}
      ></input>
    </label>
  );
}
