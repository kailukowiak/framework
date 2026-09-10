import { invoke } from "@tauri-apps/api/core";
import type { DocumentView } from "./types";

export async function exportFrameAs(
  frameId: string,
  path?: string
): Promise<DocumentView | null> {
  return invoke("export_frame_as", {
    frameId,
    ...(path === undefined ? {} : { path }),
  });
}

export async function updateOriginalDelimited(
  frameId: string
): Promise<DocumentView | null> {
  return invoke("update_original_delimited", { frameId });
}
