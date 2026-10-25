interface IconProps {
  name: string;
  size?: number;
  className?: string;
}

export function Icon({ name, size, className }: IconProps) {
  return (
    <svg
      className={`icon${className ? ` ${className}` : ""}`}
      aria-hidden="true"
      style={size ? { width: size, height: size } : undefined}
    >
      <use href={`/icons/lucide.svg#${name}`} />
    </svg>
  );
}
