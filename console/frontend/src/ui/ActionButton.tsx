import type { ButtonHTMLAttributes } from 'react';
export function ActionButton({ variant = 'primary', className = '', ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: 'primary' | 'secondary' | 'stop' }) {
  return <button {...props} className={`button ${variant} ${className}`} />;
}
