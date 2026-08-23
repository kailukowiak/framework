export type JoinState = {
  primaryFrameId: string;
  x: number;
  y: number;
  lookupFrameId?: string;
  primaryKeyId?: string;
  lookupKeyId?: string;
  /** A canvas lookup gesture brings only these columns across. */
  lookupOutputColumnIds?: string[];
} | null;
