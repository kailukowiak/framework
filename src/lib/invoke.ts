import {
  invoke as tauriInvoke,
  type InvokeArgs,
  type InvokeOptions,
} from "@tauri-apps/api/core";
import { reportCommandFailure } from "./errorReporting";

/**
 * The one place every `#[tauri::command]` call in `src/lib/api.ts` passes
 * through. A rejected command is otherwise silent unless whatever called it
 * happens to render the error -- see `src/lib/errorReporting.ts` for where
 * the report actually goes. The rejection itself is rethrown unchanged: this
 * only adds a side-effect, never changes what the caller sees.
 *
 * Argument *values* are never logged -- they routinely carry document
 * content -- only the argument key names, so a failure can be traced back to
 * which parameters a call was shaped with.
 */
export async function invoke<T>(
  command: string,
  args?: InvokeArgs,
  options?: InvokeOptions
): Promise<T> {
  try {
    return await tauriInvoke<T>(command, args, options);
  } catch (error) {
    reportCommandFailure(command, error, argKeyNames(args));
    throw error;
  }
}

/** `InvokeArgs` also allows a raw byte buffer; only a plain args record has
 * key names worth reporting. */
function argKeyNames(args: InvokeArgs | undefined): string[] | undefined {
  if (!args || Array.isArray(args) || args instanceof ArrayBuffer || args instanceof Uint8Array) {
    return undefined;
  }
  return Object.keys(args);
}
