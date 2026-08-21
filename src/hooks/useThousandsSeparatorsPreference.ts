import { useCallback, useState } from "react";
import {
  THOUSANDS_SEPARATORS_PREFERENCE,
  readThousandsSeparatorsPreference,
} from "../FrameGrid";

/** Whether large numbers are grouped ("1,000" vs "1000"), persisted locally. */
export function useThousandsSeparatorsPreference(): [
  boolean,
  (useGrouping: boolean) => void,
] {
  const [useThousandsSeparators, setUseThousandsSeparatorsState] = useState(
    readThousandsSeparatorsPreference
  );
  const setUseThousandsSeparators = useCallback((useGrouping: boolean) => {
    setUseThousandsSeparatorsState(useGrouping);
    try {
      window.localStorage.setItem(
        THOUSANDS_SEPARATORS_PREFERENCE,
        String(useGrouping)
      );
    } catch {
      // The current window still follows the choice; only persistence failed.
    }
  }, []);
  return [useThousandsSeparators, setUseThousandsSeparators];
}
