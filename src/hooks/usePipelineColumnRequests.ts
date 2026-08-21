import { useCallback, useRef, useState } from "react";
import type { Column, FrameObject, Selection } from "../lib/types";

export type AddCalculatedColumnRequest = {
  frameId: string;
  token: number;
  afterColumnId?: string;
  anchorRowIndex?: number;
} | null;
export type TransformColumnRequest = {
  frameId: string;
  columnId: string;
  formula: string;
  focus?: boolean;
  editExisting?: boolean;
  anchorRowIndex?: number;
  orderByColumnId?: string;
  focusAtEnd?: boolean;
  token: number;
} | null;
export type FilterColumnRequest = {
  frameId: string;
  columnId: string;
  token: number;
} | null;
export type HidePipelineColumnRequest = {
  frameId: string;
  columnId: string;
  token: number;
} | null;
export type RearrangeColumnsRequest = {
  frameId: string;
  columnIds: string[];
  token: number;
} | null;

// The editors that receive a request already know which frame they are
// editing, so the request they are handed drops the frameId that scoped it.
export type AddCalculatedColumnEditorRequest = Omit<
  NonNullable<AddCalculatedColumnRequest>,
  "frameId"
>;
export type TransformColumnEditorRequest = Omit<
  NonNullable<TransformColumnRequest>,
  "frameId"
>;
export type FilterColumnEditorRequest = Omit<
  NonNullable<FilterColumnRequest>,
  "frameId"
>;
export type HidePipelineColumnEditorRequest = Omit<
  NonNullable<HidePipelineColumnRequest>,
  "frameId"
>;
export type RearrangeColumnsEditorRequest = Omit<
  NonNullable<RearrangeColumnsRequest>,
  "frameId"
>;

/**
 * The pipeline-column-editing gestures — add a calculated column,
 * transform or fill one, filter one, hide one, rearrange them — all funnel
 * through the same shape: close whatever context menu was open, select the
 * column being edited, switch the inspector to Wrangle, and hand
 * PipelineEditor a freshly-tokened request. The token is what lets it tell
 * "the user asked again" apart from "this document re-rendered" — a
 * request object's identity alone isn't a safe signal once it has been
 * threaded through props and reshaped along the way.
 */
export function usePipelineColumnRequests({
  setContextMenu,
  setSelection,
  setInspectorSection,
}: {
  setContextMenu: (value: null) => void;
  setSelection: (value: Selection) => void;
  setInspectorSection: (value: "wrangle") => void;
}) {
  const [addCalculatedColumnRequest, setAddCalculatedColumnRequest] =
    useState<AddCalculatedColumnRequest>(null);
  const addCalculatedColumnToken = useRef(0);
  const [transformColumnRequest, setTransformColumnRequest] =
    useState<TransformColumnRequest>(null);
  const transformColumnToken = useRef(0);
  const [filterColumnRequest, setFilterColumnRequest] =
    useState<FilterColumnRequest>(null);
  const filterColumnToken = useRef(0);
  const [hidePipelineColumnRequest, setHidePipelineColumnRequest] =
    useState<HidePipelineColumnRequest>(null);
  const hidePipelineColumnToken = useRef(0);
  const [rearrangeColumnsRequest, setRearrangeColumnsRequest] =
    useState<RearrangeColumnsRequest>(null);
  const rearrangeColumnsToken = useRef(0);

  const requestAddCalculatedColumn = useCallback(
    (
      frameId: string,
      afterColumnId: string | undefined,
      anchorRowIndex: number | undefined,
      viewId?: string
    ) => {
      setContextMenu(null);
      setSelection({ objectId: frameId, viewId, columnId: afterColumnId });
      setInspectorSection("wrangle");
      addCalculatedColumnToken.current += 1;
      setAddCalculatedColumnRequest({
        frameId,
        token: addCalculatedColumnToken.current,
        afterColumnId,
        anchorRowIndex,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );

  const requestColumnTransformation = useCallback(
    (
      frame: FrameObject,
      column: Column,
      formula: string,
      focus = false,
      viewId?: string,
      orderByColumnId?: string
    ) => {
      setContextMenu(null);
      setSelection({ objectId: frame.id, viewId, columnId: column.id });
      setInspectorSection("wrangle");
      transformColumnToken.current += 1;
      setTransformColumnRequest({
        frameId: frame.id,
        columnId: column.id,
        formula,
        focus,
        orderByColumnId,
        token: transformColumnToken.current,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );

  const requestColumnFill = useCallback(
    (
      frame: FrameObject,
      column: Column,
      formula: string,
      _rowIndex?: number,
      viewId?: string
    ) => requestColumnTransformation(frame, column, formula, false, viewId, column.id),
    [requestColumnTransformation]
  );

  const requestColumnFilter = useCallback(
    (frame: FrameObject, column: Column, viewId?: string) => {
      setContextMenu(null);
      setSelection({ objectId: frame.id, viewId, columnId: column.id });
      setInspectorSection("wrangle");
      filterColumnToken.current += 1;
      setFilterColumnRequest({
        frameId: frame.id,
        columnId: column.id,
        token: filterColumnToken.current,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );

  // Deliberately does not close a context menu: this is also reached
  // straight from the formula bar, where there is none open to close.
  const requestCalculatedColumnEdit = useCallback(
    (frame: FrameObject, column: Column, rowIndex?: number, viewId?: string) => {
      setSelection({ objectId: frame.id, viewId, columnId: column.id });
      setInspectorSection("wrangle");
      transformColumnToken.current += 1;
      setTransformColumnRequest({
        frameId: frame.id,
        columnId: column.id,
        formula: "",
        focus: true,
        editExisting: true,
        anchorRowIndex: rowIndex,
        token: transformColumnToken.current,
      });
    },
    [setSelection, setInspectorSection]
  );

  const requestHidePipelineColumn = useCallback(
    (frameId: string, columnId: string, viewId?: string) => {
      setContextMenu(null);
      setSelection({ objectId: frameId, viewId, columnId });
      setInspectorSection("wrangle");
      hidePipelineColumnToken.current += 1;
      setHidePipelineColumnRequest({
        frameId,
        columnId,
        token: hidePipelineColumnToken.current,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );

  const requestRearrangeColumns = useCallback(
    (frameId: string, columnIds: string[], viewId?: string) => {
      setSelection({ objectId: frameId, viewId });
      setInspectorSection("wrangle");
      rearrangeColumnsToken.current += 1;
      setRearrangeColumnsRequest({
        frameId,
        columnIds,
        token: rearrangeColumnsToken.current,
      });
    },
    [setSelection, setInspectorSection]
  );

  const clearAddCalculatedColumnRequest = useCallback(
    () => setAddCalculatedColumnRequest(null),
    []
  );
  const clearTransformColumnRequest = useCallback(
    () => setTransformColumnRequest(null),
    []
  );
  const clearFilterColumnRequest = useCallback(() => setFilterColumnRequest(null), []);
  const clearHidePipelineColumnRequest = useCallback(
    () => setHidePipelineColumnRequest(null),
    []
  );
  const clearRearrangeColumnsRequest = useCallback(
    () => setRearrangeColumnsRequest(null),
    []
  );

  return {
    addCalculatedColumnRequest,
    transformColumnRequest,
    filterColumnRequest,
    hidePipelineColumnRequest,
    rearrangeColumnsRequest,
    clearAddCalculatedColumnRequest,
    clearTransformColumnRequest,
    clearFilterColumnRequest,
    clearHidePipelineColumnRequest,
    clearRearrangeColumnsRequest,
    requestAddCalculatedColumn,
    requestColumnTransformation,
    requestColumnFill,
    requestColumnFilter,
    requestCalculatedColumnEdit,
    requestHidePipelineColumn,
    requestRearrangeColumns,
    // Exposed raw, in addition to the triggers above, because the keyboard
    // formula gesture (handleGridFormulaKey in GridFormulaKeyboard.ts) mints
    // its own differently-shaped request and needs to manage the token
    // itself rather than go through requestColumnTransformation's signature.
    transformColumnToken,
    setTransformColumnRequest,
  };
}

/**
 * One request narrowed to the object it targets, in the shape its editor
 * prop expects: present only while it names this `frameId`, and with the
 * `frameId` itself dropped since the editor already knows which frame it
 * is editing.
 */
export function scopedPipelineRequest<T extends { frameId: string; token: number }>(
  request: T | null,
  frameId: string
): Omit<T, "frameId"> | undefined {
  if (!request || request.frameId !== frameId) return undefined;
  const { frameId: _frameId, ...rest } = request;
  return rest;
}
