use crate::Id;
use crate::error::CoreError;
use crate::model::frame::FrameObject;
use crate::model::plot::PlotObject;
use crate::model::value::{
    BlockLine, BlockObject, ContainerObject, DataType, FrozenValue, ResultObject, SeriesObject,
    TextObject, ValueObject,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;
use uuid::Uuid;

pub fn id() -> Id {
    Uuid::new_v4().to_string()
}

/// Mints the physical name a column carries through formulas and Polars.
///
/// A UUID solved uniqueness and made every plan, parquet schema, event, and
/// error unnecessarily opaque. The name at creation is the useful part when
/// somebody is reading those things, while a short random suffix is the part
/// that lets two columns begin with the same name, be renamed, deleted, or
/// recreated without accidentally becoming the same column. The slug never
/// changes after creation; [`Column::name`](crate::Column::name) is the
/// editable label.
pub fn column_id(name: &str) -> Id {
    let mut slug = String::new();
    let mut separated = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if separated && !slug.is_empty() {
                slug.push('_');
            }
            slug.push(character.to_ascii_lowercase());
            separated = false;
        } else {
            separated = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }
    let slug = slug.trim_end_matches('_');
    let slug = if slug.is_empty() { "column" } else { slug };

    // Six base-32 characters carry thirty random bits: enough that ordinary
    // documents are overwhelmingly unlikely to collide, without giving the
    // diagnostic suffix more visual weight than the readable slug. Operation
    // preparation remains the single minting point, so a resolved operation
    // carries the chosen id to every replica.
    const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let bytes = Uuid::new_v4().into_bytes();
    let random = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) >> 2;
    let suffix = (0..6)
        .rev()
        .map(|shift| ALPHABET[((random >> (shift * 5)) & 31) as usize] as char)
        .collect::<String>();
    format!("{slug}~{suffix}")
}

#[cfg(test)]
mod column_id_tests {
    use super::column_id;

    #[test]
    fn column_ids_keep_a_readable_slug_and_a_unique_suffix() {
        let first = column_id("Net Revenue ($)");
        let second = column_id("Net Revenue ($)");
        assert!(first.starts_with("net_revenue~"));
        assert_eq!(first.len(), "net_revenue~".len() + 6);
        assert_ne!(first, second);
        assert!(column_id("   ").starts_with("column~"));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Document {
    pub id: Id,
    pub name: String,
    /// ts-rs renders a `u64` as `bigint`, which is not what crosses the wire:
    /// serde_json writes this as a plain JSON number and the interface has
    /// always read it as one. A revision counter incremented once per edit is
    /// nowhere near the range where the distinction could matter.
    #[ts(as = "u32")]
    pub revision: u64,
    pub objects: Vec<DataObject>,
    pub views: Vec<CanvasView>,
    /// Answers computed from live data once and written down, keyed by the
    /// result or block line that computed them.
    ///
    /// Kept here rather than on the objects themselves because both kinds of
    /// holder — a result on a card, a line inside a block — are addressed by
    /// the same sort of id, and a block's lines are rebuilt wholesale every
    /// time its text is retyped. A frozen answer that lived on the line
    /// would have to be carried through that rebuild by hand; one that lives
    /// beside the document survives it by having the same id it always had.
    // Optional on the TypeScript side for the same reason `paged` is: serde
    // accepts the key missing, and this field is new to the mirror — a reader
    // that never knew about frozen values should not have to start naming it.
    #[serde(default)]
    #[ts(optional, as = "Option<BTreeMap<Id, FrozenValue>>")]
    pub frozen_values: BTreeMap<Id, FrozenValue>,
}

/// Measured: 704 bytes, all of it `FrameObject`. The other variants are
/// `ValueObject` 80, `PlotObject` 144, `TextObject` smaller still, so every
/// element of `Document::objects` is sized for a frame whether or not it is
/// one.
///
/// Not boxed, deliberately. A document holds tens of objects, so the waste is
/// a few tens of KB, and the allocation `Box<FrameObject>` would add sits on
/// the path every frame read takes. The memory that actually matters here is
/// `Store::undo`, which clones whole documents — fixing that is worth more
/// than this, and is tracked separately.
///
/// If this is ever revisited, boxing *this* variant is the high-leverage
/// change: it also shrinks `ReplicatedOperation`, whose largest variant is
/// `AddObject { object: DataObject, view: CanvasView }` at 704 + 136 = 840.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum DataObject {
    Value(ValueObject),
    Result(ResultObject),
    Block(BlockObject),
    Series(SeriesObject),
    Container(ContainerObject),
    Frame(FrameObject),
    Text(TextObject),
    Plot(PlotObject),
}

impl DataObject {
    pub fn id(&self) -> &str {
        match self {
            Self::Value(value) => &value.id,
            Self::Result(result) => &result.id,
            Self::Block(block) => &block.id,
            Self::Series(series) => &series.id,
            Self::Container(container) => &container.id,
            Self::Frame(frame) => &frame.id,
            Self::Text(text) => &text.id,
            Self::Plot(plot) => &plot.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Value(value) => &value.name,
            Self::Result(result) => &result.name,
            Self::Block(block) => &block.name,
            Self::Series(series) => &series.name,
            Self::Container(container) => &container.name,
            Self::Frame(frame) => &frame.name,
            Self::Text(text) => &text.name,
            Self::Plot(plot) => &plot.name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CanvasView {
    pub id: Id,
    /// The object this card shows. On a card with tabs this is the selected
    /// tab's object — switching tabs moves it.
    pub object_id: Id,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub collapsed: bool,
    /// The objects this card offers as tabs, in strip order.
    ///
    /// Empty means the card shows `object_id` alone and draws no strip.
    /// Non-empty must contain `object_id`; that membership is what makes
    /// "which tab is active" the same question as "what does this card
    /// show", rather than a second piece of state that can disagree.
    ///
    /// Frames and plots both qualify: a plot of a frame on the card reads
    /// the same data through the same lineage, so it belongs on the same
    /// card. What may *not* appear is anything reading from somewhere the
    /// card does not show — see `Document::validate_tab_target`.
    ///
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tab_object_ids: Vec<Id>,
}

impl CanvasView {
    /// The tab strip as the UI draws it: a card with no explicit tabs still
    /// shows its own object as the single selected tab.
    pub fn tabs(&self) -> &[Id] {
        if self.tab_object_ids.is_empty() {
            std::slice::from_ref(&self.object_id)
        } else {
            &self.tab_object_ids
        }
    }

    /// Replaces the strip and selects `active`. A strip of one is stored as
    /// no strip at all, so "does this card have tabs" has a single answer
    /// however the card got there.
    pub(crate) fn set_tabs(&mut self, tabs: Vec<Id>, active: Id) {
        self.object_id = active;
        self.tab_object_ids = if tabs.len() <= 1 { Vec::new() } else { tabs };
    }
}

impl Document {
    pub fn blank(name: impl Into<String>) -> Self {
        Self {
            id: id(),
            name: name.into(),
            revision: 0,
            objects: Vec::new(),
            views: Vec::new(),
            frozen_values: BTreeMap::new(),
        }
    }

    pub fn frame(&self, frame_id: &str) -> Result<&FrameObject, CoreError> {
        self.objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
                _ => None,
            })
            .ok_or(CoreError::FrameNotFound)
    }

    pub(crate) fn frame_mut(&mut self, frame_id: &str) -> Result<&mut FrameObject, CoreError> {
        self.objects
            .iter_mut()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
                _ => None,
            })
            .ok_or(CoreError::FrameNotFound)
    }

    pub fn text_object(&self, text_id: &str) -> Result<&TextObject, CoreError> {
        self.objects
            .iter()
            .find_map(|object| match object {
                DataObject::Text(text) if text.id == text_id => Some(text),
                _ => None,
            })
            .ok_or(CoreError::ObjectNotFound)
    }

    pub(crate) fn text_object_mut(&mut self, text_id: &str) -> Result<&mut TextObject, CoreError> {
        self.objects
            .iter_mut()
            .find_map(|object| match object {
                DataObject::Text(text) if text.id == text_id => Some(text),
                _ => None,
            })
            .ok_or(CoreError::ObjectNotFound)
    }

    pub fn block(&self, block_id: &str) -> Result<&BlockObject, CoreError> {
        self.objects
            .iter()
            .find_map(|object| match object {
                DataObject::Block(block) if block.id == block_id => Some(block),
                _ => None,
            })
            .ok_or(CoreError::ObjectNotFound)
    }

    pub(crate) fn block_mut(&mut self, block_id: &str) -> Result<&mut BlockObject, CoreError> {
        self.objects
            .iter_mut()
            .find_map(|object| match object {
                DataObject::Block(block) if block.id == block_id => Some(block),
                _ => None,
            })
            .ok_or(CoreError::ObjectNotFound)
    }

    /// The block holding the line with this id, and the line's position in
    /// it. Line ids are document-unique, so a formula can hold one the way
    /// it holds a result's — this is how such a reference is read back.
    pub(crate) fn block_line(&self, line_id: &str) -> Option<(&BlockObject, usize)> {
        self.objects.iter().find_map(|object| match object {
            DataObject::Block(block) => block
                .lines
                .iter()
                .position(|line| line.id == line_id)
                .map(|index| (block, index)),
            _ => None,
        })
    }

    /// What a column holds, wherever in the document that column lives.
    ///
    /// Column ids are unique across the whole document — the same fact that
    /// lets a formula name a column of another frame by id alone — so this
    /// needs no frame to ask in, which is what makes it usable from a
    /// scratchpad line that sits in no frame at all.
    pub(crate) fn column_type(&self, column_id: &str) -> Option<DataType> {
        self.objects.iter().find_map(|object| match object {
            DataObject::Frame(frame) => frame
                .columns
                .iter()
                .find(|column| column.id == column_id)
                .map(|column| column.data_type),
            _ => None,
        })
    }

    /// How many rows a frame's snapshot holds, or `None` for a frame with
    /// no snapshot.
    ///
    /// Written into the document when the snapshot is taken, so asking is
    /// free — which is what lets the length rule be settled while a formula
    /// is being parsed, rather than discovered by Polars after the fact.
    pub(crate) fn snapshot_row_count(&self, frame_id: &str) -> Option<usize> {
        self.frame(frame_id)
            .ok()?
            .materialization
            .as_ref()
            .map(|materialization| materialization.artifact.row_count)
    }

    /// Which formula elsewhere reads `frame_id`, named the way a refusal
    /// names it, or `None` when nothing does.
    ///
    /// The guard behind three refusals: deleting the frame, deleting one of
    /// its columns, and dropping the snapshot that made it readable. All
    /// three would leave a formula somewhere else pointing at nothing —
    /// which is worth saying out loud, so the answer carries a name rather
    /// than a yes. The first one found; a refusal is a refusal, and the
    /// second one is a problem for after the first is dealt with.
    pub(crate) fn frame_read_by(&self, frame_id: &str) -> Option<String> {
        self.objects.iter().find_map(|object| match object {
            DataObject::Frame(frame) if frame.id != frame_id => frame
                .foreign_frames()
                .contains(&frame_id)
                .then(|| as_named(&frame.name)),
            DataObject::Result(result) => {
                let mut frames = Vec::new();
                result.formula.expression.foreign_frames(&mut frames);
                frames.contains(&frame_id).then(|| as_named(&result.name))
            }
            DataObject::Block(block) => block.lines.iter().find_map(|line| {
                let mut frames = Vec::new();
                line.expression()?.foreign_frames(&mut frames);
                frames
                    .contains(&frame_id)
                    .then(|| as_line_named(block, line))
            }),
            _ => None,
        })
    }

    /// The same question about one column: does a formula in some *other*
    /// frame name it. Column ids are unique document-wide, so a frame that
    /// does not own the column can only be naming it across the boundary.
    pub(crate) fn column_read_by(&self, owner_id: &str, column_id: &str) -> Option<String> {
        self.objects.iter().find_map(|object| match object {
            DataObject::Frame(frame) if frame.id != owner_id => frame
                .expressions()
                .any(|expression| expression.references_column(column_id))
                .then(|| as_named(&frame.name)),
            DataObject::Result(result) => result
                .formula
                .expression
                .references_column(column_id)
                .then(|| as_named(&result.name)),
            DataObject::Block(block) => block.lines.iter().find_map(|line| {
                line.expression()?
                    .references_column(column_id)
                    .then(|| as_line_named(block, line))
            }),
            _ => None,
        })
    }

    /// The container holding `object_id`, if any.
    pub fn container_of(&self, object_id: &str) -> Option<&ContainerObject> {
        self.objects.iter().find_map(|object| match object {
            DataObject::Container(container)
                if container.member_ids.iter().any(|id| id == object_id) =>
            {
                Some(container)
            }
            _ => None,
        })
    }

    /// Whether a card should be drawn for this object on the canvas.
    ///
    /// Something inside a container is shown by that container's card, so
    /// drawing its own as well would put it in two places at once. The rule
    /// lives here rather than in the interface so that every reader of the
    /// document agrees about it.
    pub fn is_on_canvas(&self, object_id: &str) -> bool {
        self.container_of(object_id).is_none()
    }

    /// Whether `container_id` holds `object_id`, at any depth.
    pub(crate) fn container_holds(&self, container_id: &str, object_id: &str) -> bool {
        let Ok(DataObject::Container(container)) = self.object(container_id) else {
            return false;
        };
        container
            .member_ids
            .iter()
            .any(|member| member == object_id || self.container_holds(member, object_id))
    }

    /// A frame and one of its columns by the names a person gave them, for
    /// saying which reference an error is about. Falls back to the ids,
    /// which is at least something to search for.
    pub(crate) fn foreign_names(&self, frame_id: &str, column_id: &str) -> (String, String) {
        let Ok(frame) = self.frame(frame_id) else {
            return (frame_id.to_string(), column_id.to_string());
        };
        let column = frame
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.name.clone())
            .unwrap_or_else(|| column_id.to_string());
        (frame.name.clone(), column)
    }

    pub fn object(&self, object_id: &str) -> Result<&DataObject, CoreError> {
        self.objects
            .iter()
            .find(|object| object.id() == object_id)
            .ok_or(CoreError::ObjectNotFound)
    }

    pub(crate) fn object_mut(&mut self, object_id: &str) -> Result<&mut DataObject, CoreError> {
        self.objects
            .iter_mut()
            .find(|object| object.id() == object_id)
            .ok_or(CoreError::ObjectNotFound)
    }

    pub fn view(&self, view_id: &str) -> Result<&CanvasView, CoreError> {
        self.views
            .iter()
            .find(|view| view.id == view_id)
            .ok_or(CoreError::ViewNotFound)
    }

    pub(crate) fn view_mut(&mut self, view_id: &str) -> Result<&mut CanvasView, CoreError> {
        self.views
            .iter_mut()
            .find(|view| view.id == view_id)
            .ok_or(CoreError::ViewNotFound)
    }

    /// A frame name is a formula address, so two frames may not publish the
    /// same one. The first keeps the requested spelling and later collisions
    /// receive a stable numeric suffix. Generated `Frame N` names continue
    /// their visible count instead of becoming `Frame 1_2`.
    pub(crate) fn unique_frame_name(&self, base: &str, except: Option<&str>) -> String {
        let taken = |candidate: &str| {
            self.objects.iter().any(|object| {
                matches!(object, DataObject::Frame(_))
                    && Some(object.id()) != except
                    && object.name() == candidate
            })
        };
        untaken_frame_name(base, taken)
    }

    /// Repairs documents written before frame names became unique. Formula
    /// expressions already hold frame IDs, so changing only the display
    /// spelling preserves what every saved formula reads.
    pub(crate) fn normalize_frame_names(&mut self) {
        let mut used = Vec::<String>::new();
        for object in &mut self.objects {
            let DataObject::Frame(frame) = object else {
                continue;
            };
            let name = untaken_frame_name(&frame.name, |candidate| {
                used.iter().any(|taken| taken == candidate)
            });
            frame.name = name.clone();
            used.push(name);
        }
    }
}

fn untaken_frame_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    if let Some((prefix, number)) = ["Frame ", "Frame "].into_iter().find_map(|prefix| {
        base.strip_prefix(prefix)
            .and_then(|suffix| suffix.parse::<usize>().ok())
            .map(|number| (prefix, number))
    }) {
        return ((number + 1)..)
            .map(|suffix| format!("{prefix}{suffix}"))
            .find(|candidate| !taken(candidate))
            .expect("an unbounded range always yields an untaken name");
    }
    let (root, start) = base
        .rsplit_once('_')
        .and_then(|(root, suffix)| {
            suffix
                .parse::<usize>()
                .ok()
                .map(|suffix| (root, suffix + 1))
        })
        .unwrap_or((base, 2));
    (start..)
        .map(|suffix| format!("{root}_{suffix}"))
        .find(|candidate| !taken(candidate))
        .expect("an unbounded range always yields an untaken name")
}

/// A name as a refusal writes it, ready to drop into a sentence.
pub(crate) fn as_named(name: &str) -> String {
    format!("‘{name}’")
}

/// The same for a formula that lives on a line of a block. The block alone
/// would be barely narrower than saying nothing — a block is forty lines,
/// and the one that reads this is the one worth walking to.
pub(crate) fn as_line_named(block: &BlockObject, line: &BlockLine) -> String {
    match line.name.is_empty() {
        true => as_named(&block.name),
        false => format!("‘{}’ in ‘{}’", line.name, block.name),
    }
}
