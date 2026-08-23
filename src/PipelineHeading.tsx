import { GitBranch, RefreshCw } from "lucide-react";

export function PipelineHeading({
  hasGeneratedColumns,
  refreshing,
  onRefresh,
}: {
  hasGeneratedColumns: boolean;
  refreshing: boolean;
  onRefresh: () => void;
}) {
  return <div className="section-heading">
    <GitBranch size={16} />
    <strong>Transformations</strong>
    {hasGeneratedColumns && <button
      type="button"
      className="pipeline-refresh-generated"
      disabled={refreshing}
      onClick={onRefresh}
    >
      <RefreshCw className={refreshing ? "spinning" : ""} size={12} />
      {refreshing ? "Refreshing…" : "Refresh generated columns"}
    </button>}
  </div>;
}
