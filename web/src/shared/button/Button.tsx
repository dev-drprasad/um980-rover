import type { ButtonHTMLAttributes } from "react";
import "./Button.css";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: React.ReactNode;
  onClick?: () => void;
  className?: string;
  disabled?: boolean;
  selected?: boolean;
  tabIndex?: number;
}

export function OutlinedButton({
  children,
  className,
  selected,
  ...restProps
}: ButtonProps) {
  return (
    <button
      className={`button outlined-button ${className || ""} ${selected ? "selected" : ""}`}
      {...restProps}
    >
      {children}
    </button>
  );
}

export function IconButton({ children, className, ...restProps }: ButtonProps) {
  return (
    <OutlinedButton className={`icon-button ${className || ""}`} {...restProps}>
      {children}
    </OutlinedButton>
  );
}

export function FilledIconButton({
  children,
  onClick,
  className,
  disabled,
  selected,
}: ButtonProps) {
  return (
    <FilledButton
      className={`icon-button ${className || ""}`}
      onClick={onClick}
      disabled={disabled}
      selected={selected}
    >
      {children}
    </FilledButton>
  );
}

export function FilledButton({
  children,
  className,
  ...restProps
}: ButtonProps) {
  return (
    <OutlinedButton
      className={`filled-button ${className || ""}`}
      {...restProps}
    >
      {children}
    </OutlinedButton>
  );
}
