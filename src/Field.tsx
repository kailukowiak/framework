/** A compact commit-on-blur field shared by inspectors and side panels. */
export function Field({
  label,
  initial,
  help,
  onCommit,
}: {
  label: string;
  initial: string;
  help?: string;
  onCommit: (value: string) => void;
}) {
  return (
    <label className="inspector-field">
      {label}
      <input
        defaultValue={initial}
        key={initial}
        onBlur={(event) => {
          if (event.target.value !== initial) onCommit(event.target.value);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
      />
      {help && <small>{help}</small>}
    </label>
  );
}
