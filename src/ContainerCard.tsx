import { Plus } from "lucide-react";
import { ResultCard, SeriesCard, ValueCard, objectFormulaToken } from "./ScalarCards";
import type { OperationHandler } from "./lib/handlers";
import type { ContainerObject, DataObject, ComputedFrame, ComputedResult, ComputedValue, FormulaFunction } from "./lib/types";

/**
 * A heading and what is kept under it.
 *
 * Members are drawn by the same cards they would get on the canvas, so a
 * value inside a container is edited exactly the way a value outside one
 * is — being in a container is about where it sits and what it is called,
 * not about what you can do to it.
 */
export function ContainerCard({
  container,
  objects,
  computedFrames,
  computedResults,
  computedValues,
  formulaFunctions,
  onOperation,
  onFreeze,
  onAddList,
  onSelectMember,
}: {
  container: ContainerObject;
  objects: DataObject[];
  computedFrames: Record<string, ComputedFrame>;
  computedResults: Record<string, ComputedResult>;
  computedValues: Record<string, ComputedValue>;
  formulaFunctions: FormulaFunction[];
  onOperation: OperationHandler;
  onFreeze: (objectId: string) => Promise<void>;
  onAddList: (containerId: string) => void;
  /** A press on a member selects that member, not the container around it. */
  onSelectMember: (memberId: string) => void;
}) {
  const members = container.memberIds
    .map((memberId) => objects.find((object) => object.id === memberId))
    .filter((member): member is DataObject => Boolean(member));
  return (
    <div className="container-card">
      <input
        className="object-name-input"
        defaultValue={container.name}
        key={container.name}
        onBlur={(event) => {
          if (event.target.value !== container.name)
            onOperation({
              type: "renameObject",
              objectId: container.id,
              name: event.target.value,
            });
        }}
        // Return commits here too: a container's name is read by every
        // formula that reaches through it, and leaving the rename pending
        // in the field made those read stale.
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
          else if (event.key === "Escape") {
            event.currentTarget.value = container.name;
            event.currentTarget.blur();
          }
        }}
      />
      <div className="container-members">
        {members.length === 0 && (
          <p className="container-empty">
            Nothing in here yet. Add a value or a vector below, or drop one in
            from its own menu.
          </p>
        )}
        {members.map((member) => (
          <div
            className="container-member"
            data-object-id={member.id}
            key={member.id}
            // Before the card's own handler, which would widen the selection
            // back to the container; a right-click already reached the member
            // through the context menu, and a left-click should not do less.
            onPointerDown={(event) => {
              if (event.button !== 0) return;
              event.stopPropagation();
              onSelectMember(member.id);
            }}
          >
            {member.kind === "value" && (
              <ValueCard
                value={member}
                computed={computedValues[member.id]}
                formula={objectFormulaToken(objects, member.id)}
                onOperation={onOperation}
              />
            )}
            {member.kind === "result" && (
              <ResultCard
                result={member}
                formula={objectFormulaToken(objects, member.id)}
                computed={computedResults[member.id]}
                objects={objects}
                computedFrames={computedFrames}
                formulaFunctions={formulaFunctions}
                onOperation={onOperation}
                onFreeze={onFreeze}
              />
            )}
            {member.kind === "series" && (
              <SeriesCard
                series={member}
                formula={
                  objectFormulaToken(objects, member.id) || `\`${member.name}\``
                }
                onOperation={onOperation}
              />
            )}
            {member.kind === "container" && (
              <ContainerCard
                container={member}
                objects={objects}
                computedFrames={computedFrames}
                computedResults={computedResults}
                computedValues={computedValues}
                formulaFunctions={formulaFunctions}
                onOperation={onOperation}
                onFreeze={onFreeze}
                onAddList={onAddList}
                onSelectMember={onSelectMember}
              />
            )}
          </div>
        ))}
      </div>
      <ContainerActions objects={objects} container={container} onOperation={onOperation} onAddList={onAddList} />
    </div>
  );
}

/** A name nothing in this container has taken yet. */
function nextMemberName(
  objects: DataObject[],
  container: ContainerObject,
  stem: string
): string {
  const taken = new Set(
    container.memberIds
      .map((memberId) => objects.find((object) => object.id === memberId)?.name)
      .filter(Boolean)
  );
  if (!taken.has(stem)) return stem;
  let suffix = 2;
  while (taken.has(`${stem} ${suffix}`)) suffix += 1;
  return `${stem} ${suffix}`;
}

function ContainerActions({ objects, container, onOperation, onAddList }: {
  objects: DataObject[]; container: ContainerObject; onOperation: OperationHandler; onAddList: (id: string) => void;
}) {
  return (
      <div className="container-actions">
        <button
          className="secondary-action"
          onClick={() =>
            onOperation({
              type: "addValue",
              name: nextMemberName(objects, container, "Value"),
              raw: "0",
              x: 0,
              y: 0,
              containerId: container.id,
            })
          }
        >
          <Plus size={13} />
          Value
        </button>
        <button
          className="secondary-action"
          onClick={() =>
            onOperation({
              type: "addResult",
              name: nextMemberName(objects, container, "Result"),
              formula: "0",
              x: 0,
              y: 0,
              containerId: container.id,
            })
          }
        >
          <Plus size={13} />
          Result
        </button>
        <button className="secondary-action" onClick={() => onAddList(container.id)}>
          <Plus size={13} />
          Vector
        </button>
      </div>
  );
}
