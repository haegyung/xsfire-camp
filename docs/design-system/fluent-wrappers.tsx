import * as React from 'react';

export type MsThemeMode = 'light' | 'dark' | 'highContrast';

type ThemeContextValue = {
  mode: MsThemeMode;
};

const ThemeContext = React.createContext<ThemeContextValue>({ mode: 'light' });

type ThemeTokenSet = {
  theme: MsThemeMode;
  children: React.ReactNode;
  className?: string;
};

export function MsFluentTheme({
  theme,
  children,
  className,
}: ThemeTokenSet) {
  return (
    <ThemeContext.Provider value={{ mode: theme }}>
      <div
        data-ms-theme={theme}
        className={`ms-fluent-root${className ? ` ${className}` : ''}`}
      >
        {children}
      </div>
    </ThemeContext.Provider>
  );
}

type ButtonState = 'rest' | 'hover' | 'active' | 'focus' | 'disabled' | 'selected';

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  children: React.ReactNode;
  state?: ButtonState;
  fullWidth?: boolean;
};

export function MsButton({
  children,
  state,
  fullWidth,
  className,
  ...props
}: ButtonProps) {
  const classes = ['ms-fluent-button'];
  if (state) classes.push(`ms-fluent-button-${state}`);
  if (fullWidth) classes.push('ms-fluent-button-full');
  if (className) classes.push(className);

  return (
    <button className={classes.join(' ')} {...props}>
      {children}
    </button>
  );
}

type InputProps = React.InputHTMLAttributes<HTMLInputElement> & {
  label?: string;
  helperText?: string;
  state?: 'rest' | 'hover' | 'focus' | 'invalid';
};

export function MsInput({ label, helperText, className, state, ...props }: InputProps) {
  const inputClass = ['ms-fluent-input'];
  if (state) inputClass.push(`ms-fluent-input-${state}`);
  if (className) inputClass.push(className);

  return (
    <label style={{ display: 'block', width: '100%' }}>
      {label && (
        <span style={{ marginBottom: 4, display: 'inline-block', fontSize: '14px' }}>{label}</span>
      )}
      <input className={inputClass.join(' ')} {...props} />
      {helperText && (
        <small style={{ color: 'var(--ms-color-foreground-muted)' }}>{helperText}</small>
      )}
    </label>
  );
}

type CardProps = {
  title?: string;
  children: React.ReactNode;
  className?: string;
};

export function MsCard({ title, children, className }: CardProps) {
  return (
    <section className={`ms-fluent-card${className ? ` ${className}` : ''}`}>
      {title && <h3>{title}</h3>}
      {children}
    </section>
  );
}

type DialogProps = {
  open: boolean;
  title?: React.ReactNode;
  children: React.ReactNode;
  onClose: () => void;
  className?: string;
};

export function MsDialog({ open, title, children, onClose, className }: DialogProps) {
  if (!open) return null;

  return (
    <div className="ms-fluent-dialog-backdrop">
      <div
        role="dialog"
        aria-modal="true"
        className={`ms-fluent-dialog${className ? ` ${className}` : ''}`}
        style={{ margin: '8vh auto', maxWidth: 560, width: 'min(92vw, 560px)', padding: 'var(--ms-spacing-16)' }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          {title && <strong>{title}</strong>}
          <button className="ms-fluent-button" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </div>
        <div style={{ marginTop: 'var(--ms-spacing-16)' }}>{children}</div>
      </div>
    </div>
  );
}

export function useMsTheme(): MsThemeMode {
  const ctx = React.useContext(ThemeContext);
  return ctx.mode;
}
