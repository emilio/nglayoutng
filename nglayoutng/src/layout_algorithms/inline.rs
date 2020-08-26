use app_units::Au;
use crate::fragment_tree::{ChildFragment, Fragment, FragmentKind, ContainerFragmentKind};
use crate::logical_geometry::*;
use crate::style::{ComputedStyle, LengthPercentage, LengthPercentageOrAuto};
use super::{ConstraintSpace, LayoutContext, LayoutResult};
use crate::layout_tree::{LayoutNodeKind, LeafKind, ContainerKind, LayoutNode, LayoutNodeId};
use smallbitvec::SmallBitVec;
use std::borrow::Cow;

/// A position for a given line break.
#[derive(Clone)]
struct InlineItemPosition {
    /// The index of the item, may be one past the end to represent "the whole
    /// range".
    item_index: usize,
}

impl InlineItemPosition {
    fn start() -> Self {
        Self {
            item_index: 0,
        }
    }

    fn advance_item(&mut self) {
        self.item_index += 1;
    }
}

struct OpenInlineBox {
    /// The layout node that generates this inline box.
    node: LayoutNodeId,
    /// Whether we've generated at least one fragment for the currently open
    /// inline box. This is useful so as to not double-account for margins /
    /// padding and such, for example.
    generated_fragment: bool,
    /// The children that are not yet placed in the line.
    children: Vec<ChildFragment>,
}

struct LineBreaker<'a, 'b, 'c> {
    fc: &'a InlineFormattingContext<'b, 'c>,
    constraints: &'a ConstraintSpace,
    lines: Vec<ChildFragment>,
    consumed_block_offset: Au,
    current_line: Vec<ChildFragment>,
    current_line_available_size: Au,
    current_line_max_block_size: Au,
    current_position: InlineItemPosition,
    at_break_opportunity: bool,
    /// An stack of currently open inline boxes.
    open_boxes: Vec<OpenInlineBox>,
}

impl<'a, 'b, 'c> LineBreaker<'a, 'b, 'c> {
    fn new(fc: &'a mut InlineFormattingContext<'b, 'c>, constraints: &'a ConstraintSpace) -> Self {
        Self {
            fc,
            constraints,
            lines: vec![],
            consumed_block_offset: Au(0),
            current_line: vec![],
            current_line_available_size: constraints.available_size.inline(),
            current_line_max_block_size: Au(0),
            current_position: InlineItemPosition::start(),
            at_break_opportunity: false,
            open_boxes: vec![],
        }
    }

    fn wm(&self) -> WritingMode {
        self.fc.input_node.style.writing_mode
    }

    fn close_box(&mut self) {
        // TODO: We know this is the last fragment of the line, and whether it's
        // the first, but we should keep that information in the fragment too.
        let box_ = self.open_boxes.pop().unwrap();
        let style = &self.fc.context.layout_tree[box_.node].style;
        let wm = style.writing_mode;
        let children = box_.children.into_boxed_slice();

        let fragment = ChildFragment {
            // XXX: current_inline_offset + margin_inline_start?
            offset: LogicalPoint::zero(wm),
            fragment: Box::new(Fragment {
                // XXX: max height of the fragments + padding + border?
                size: LogicalSize::zero(wm),
                style: style.clone(),
                kind: FragmentKind::Container {
                    kind: ContainerFragmentKind::Box {},
                    children,
                },
            })
        };

        self.push_fragment_to_line(fragment);
    }

    fn push_fragment_to_line(&mut self, fragment: ChildFragment) {
        if let Some(ref mut last) = self.open_boxes.last_mut() {
            last.children.push(fragment);
        } else {
            self.current_line.push(fragment);
        }
    }

    fn flush_open_boxes_to_line(&mut self) {
        let mut pending_fragment = None;

        for b in self.open_boxes.iter_mut().rev() {
            if let Some(pending_fragment) = pending_fragment.take() {
                b.children.push(pending_fragment);
            }

            let style = &self.fc.context.layout_tree[b.node].style;
            let wm = style.writing_mode;
            let children = std::mem::replace(&mut b.children, vec![]).into_boxed_slice();
            let fragment = ChildFragment {
                // XXX: current_inline_offset + margin_inline_start?
                offset: LogicalPoint::zero(wm),
                fragment: Box::new(Fragment {
                    // XXX: max height of the fragments + padding + border?
                    size: LogicalSize::zero(wm),
                    style: style.clone(),
                    kind: FragmentKind::Container {
                        kind: ContainerFragmentKind::Box {},
                        children,
                    },
                })
            };

            b.generated_fragment = true;
            pending_fragment = Some(fragment);
        }

        if let Some(pending_fragment) = pending_fragment {
            self.current_line.push(pending_fragment);
        }
    }

    fn flush_line(&mut self) {
        self.flush_open_boxes_to_line();

        if self.current_line.is_empty() {
            return; // XXX Do we need to create empty lines in any case?
        }

        let line_fragments = std::mem::replace(&mut self.current_line, vec![]);
        let max_block_size = std::mem::replace(&mut self.current_line_max_block_size, Au(0));

        // TODO: first-line style if appropriate?
        let style = &self.fc.input_node.style;
        let wm = self.wm();

        // TODO: Compute line box size from contents.
        //
        // TODO: Account for line-height?
        //
        // TODO: Line box size may be affected by floats, may not always be the
        // avail inline size.
        let size = LogicalSize::new(wm, self.constraints.available_size.inline(), max_block_size);

        // TODO: Vertical alignment of items? Here or when we're done with all
        // lines?
        let offset = LogicalPoint::new(wm, Au(0), self.consumed_block_offset);

        self.consumed_block_offset += size.block;

        self.lines.push(ChildFragment {
            offset,
            fragment: Box::new(Fragment {
                size,
                style: style.clone(),
                kind: FragmentKind::Container {
                    kind: ContainerFragmentKind::Line {},
                    children: line_fragments.into_boxed_slice(),
                },
            }),
        });

        // Go to the next line.
        self.current_line_available_size = self.constraints.available_size.inline();
        self.at_break_opportunity = false;
    }

    fn layout_atomic_inline(&mut self, _: LayoutNodeId) {
        unimplemented!()
    }

    fn layout_replaced(&mut self, _: LayoutNodeId) {
        unimplemented!()
    }

    fn can_fit(&self, inline_size: Au) -> bool {
        !self.at_break_opportunity ||
            self.current_line_available_size >= inline_size
    }

    fn resolve_padding(&self, lp: &LengthPercentage) -> Au {
        self.resolve_padding_margin(lp)
    }

    fn resolve_padding_margin(&self, lp: &LengthPercentage) -> Au {
        lp.resolve(self.constraints.percentage_resolution_size.inline())
    }

    fn resolve_margin(&self, margin: &LengthPercentageOrAuto) -> Au {
        match *margin {
            LengthPercentageOrAuto::Auto => Au(0),
            LengthPercentageOrAuto::LengthPercentage(ref lp) => {
                self.resolve_padding_margin(lp)
            },
        }
    }

    fn layout_inline_box(&mut self, style: &ComputedStyle, start_text_item_text: &str) {
        let mut paragraph = Cow::Borrowed(start_text_item_text);

        // let wm = style.writing_mode;

        let margin_start = self.resolve_margin(style.margin().inline_end);

        let mut mbp_start = margin_start +
            style.border_widths().inline_start +
            self.resolve_padding(style.padding().inline_start);

        let mut mbp_end = Au(0);

        // Look to following elements for text to collect.
        // A text run may be made of various text items, or various
        // inline items etc. We try to shape in "paragraph"
        // boundaries (as in, around hard breaks, or the whole thing
        // if there are none).
        //
        // Line-breaking may make us re-shape some of that text, as
        // needed, as a result of breaking. For example, consider
        // the following:
        //
        // <style>
        // p { font-size: 10px; }
        // ::first-line { font-size: 30px }
        // </style>
        // <p>This is a not-very-long paragraph</p>
        //                           ^
        //                           |
        //                           +--- Break here.
        //
        // We need to shape with the first-line style, until we hit
        // a break.  Once we know where to break, then we need to
        // shape the rest of the run with the non-first-line style.
        // This kinda sucks in multiple ways.
        //
        // If we know we're not dealing with ::first-line, we may
        // still need to re-shape, if we happen to break inside a
        // ligature, or a kerned space, see
        // https://bugzilla.mozilla.org/show_bug.cgi?id=479829.
        // Though that is less common.
        //
        // Still, in the common case, the breakpoint happens to be
        // in a e.g. space, or other place where we can slice the
        // shaping result, and carry on.
        let mut advance = 1;
        let mut break_if_not_end = false;

        loop {
            let following_item = match self.fc.items.get(self.current_position.item_index + advance) {
                Some(item) => item,
                None => break,
            };

            if let InlineItem::TagEnd(node) = *following_item {
                let end_style = &self.fc.context.layout_tree[node].style;
                if !break_if_not_end && !can_continue_run(style, end_style, /* at_beginning = */ false) {
                    break_if_not_end = true;
                }

                mbp_end += self.resolve_margin(end_style.margin().inline_end) +
                           end_style.border_widths().inline_end +
                           self.resolve_padding(end_style.padding().inline_end);

                self.close_box();
                advance += 1;
                continue;
            }

            if break_if_not_end {
                break;
            }

            match *following_item {
                InlineItem::TagStart(node) => {
                    if !paragraph.is_empty() &&
                       !can_continue_run(style, &self.fc.context.layout_tree[node].style, /* at_beginning = */ true) {
                        trace!("Can't continue run with {:?} at start", following_item);
                        break;
                    }

                    self.open_boxes.push(OpenInlineBox {
                        node,
                        generated_fragment: false,
                        children: vec![],
                    });

                    // TODO(emilio): We probably need to record some
                    // state here about where in `paragraph` the current box
                    // maps, or defer the opening / closing of boxes in this
                    // loop, or something, to create the right fragment tree.
                },
                InlineItem::TagEnd(..) => unreachable!(),
                InlineItem::Text(node, ref s) => {
                    let text_style = &self.fc.context.layout_tree[node].style;
                    if !can_continue_run(style, text_style, /* at_beginning = */ true) {
                        trace!("Can't continue run with text {:?} at start", following_item);
                        break;
                    }
                    if paragraph.is_empty() {
                        paragraph = Cow::Borrowed(s);
                    } else {
                        paragraph.to_mut().push_str(&s);
                    }

                    // This can only really happen with display: contents, and
                    // text can't have margin/border/padding, so at_beginning
                    // shouldn't matter here.
                    debug_assert!(can_continue_run(style, text_style, /* at_beginning = */ false));
                },
                InlineItem::AtomicInline(..) | InlineItem::Replaced(..) => {
                    break;
                }
            }
            advance += 1;
        }

        // Now we have a run of text on which we can compute break
        // opportunities, and which we can shape with a given style.
        //
        // Do that, and see what fits. If stuff doesn't fit, we
        // break at the first opportunity before that. We may need
        // to re-shape later, but we can try to re-use the shape
        // results from the previous run if appropriate to avoid
        // O(n^2) algorithms.
        //
        // We also have unbreakable sizes at the start and end (mbp_start and
        // mbp_end).
        let mut break_opportunities = SmallBitVec::new();
        break_opportunities.resize(paragraph.len(), false);

        // Try to grab a whole text run and line-break / shape it.
        let mut breaker = xi_unicode::LineBreakLeafIter::new(&*paragraph, 0);

        // TODO(emilio): There are optimizations here we could do to
        // avoid doing this, or do a simplified version of this,
        // when different white-space values are in-use like nowrap
        // and so on...
        trace!("Breaking {:?}", paragraph);
        loop {
            // TODO: Account for white-space and other similar shenanigans.
            let (result, _hard_break) = breaker.next(&*paragraph);
            if result == paragraph.len() {
                break;
            }
            break_opportunities.set(result, true);
            // XXX Do we need to use the hard_break bit somehow?
            // Maybe just truncating the paragraph and carrying on?
        }

        if log_enabled!(log::Level::Trace) {
            trace!("Broken:");
            let mut start = 0;
            for i in 0..paragraph.as_bytes().len() {
                if !break_opportunities[i] {
                    continue;
                }
                trace!("{}", &paragraph[start..i]);
                start = i;
            }
            trace!("{}", &paragraph[start..]);
        }

        if !self.can_fit(mbp_start) {

        }

        let shaped_runs = crate::fonts::shaping::shape(&paragraph, style);
        let mut inline_size = mbp_start;
        self.at_break_opportunity = false;
        for glyph in shaped_runs.glyphs() {
            inline_size += glyph.advance;
            if !self.can_fit(inline_size) {
                // TODO: Create fragments, reset mbp_start, carry on!
                self.flush_line();
            }
            self.at_break_opportunity = break_opportunities[glyph.byte_offset];
        }
        // TODO:
        // break_and_shape_text(text, style);
        // advance_as_needed()
        // return Some(break);
        // unimplemented!()
        // Advance past all the items that we've collected items from.
        self.current_position.item_index += advance;
    }

    fn layout_and_break(&mut self) -> Option<()> {
        loop {
            let item = self.fc.items.get(self.current_position.item_index)?;
            match *item {
                InlineItem::TagStart(node) => {
                    self.open_boxes.push(OpenInlineBox {
                        node,
                        generated_fragment: false,
                        children: vec![],
                    });
                    self.layout_inline_box(&self.fc.context.layout_tree[node].style, "");
                },
                InlineItem::TagEnd(..) => {
                    self.close_box();
                    self.current_position.advance_item();
                },
                InlineItem::AtomicInline(node) => {
                    self.layout_atomic_inline(node);
                    self.current_position.advance_item();
                },
                InlineItem::Replaced(node) => {
                    self.layout_replaced(node);
                    self.current_position.advance_item();
                },
                InlineItem::Text(node, ref text) => {
                    self.layout_inline_box(&self.fc.context.layout_tree[node].style, text);
                },
            }
        }

        self.flush_line();
    }

    fn break_and_finish(mut self) -> LayoutResult {
        self.layout_and_break();
        self.finish()
    }

    fn finish(self) -> LayoutResult {
        let wm = self.wm();
        // TODO: Vertical align, line positioning, block size.
        LayoutResult {
            root_fragment: ChildFragment {
                offset: LogicalPoint::zero(wm),
                fragment: Box::new(Fragment {
                    size: LogicalSize::new(wm, self.constraints.available_size.inline(), self.current_line_max_block_size),
                    style: self.fc.input_node.style.clone(),
                    kind: FragmentKind::Container {
                        kind: ContainerFragmentKind::Box {},
                        children: self.lines.into_boxed_slice(),
                    },
                }),
            },
        }
    }
}

pub struct InlineFormattingContext<'a, 'b> {
    context: &'a LayoutContext<'b>,
    input_node: &'a LayoutNode,
    items: Vec<InlineItem>,
}

/// An item we do inline layout on. Each of these correspond roughly to the
/// (rendered) DOM.
#[derive(Debug)]
enum InlineItem {
    // The start of a nested inline box.
    TagStart(LayoutNodeId),
    // TODO(emilio): Probably want to reference-count this somehow, or something
    // something.
    Text(LayoutNodeId, String),
    Replaced(LayoutNodeId),
    AtomicInline(LayoutNodeId),
    TagEnd(LayoutNodeId),
}

// https://drafts.csswg.org/css-text-3/#space-discard-set
fn is_space_discarding(c: char) -> bool {
    match c {
        '\u{2E80}'..='\u{2EFF}' |
        '\u{2F00}'..='\u{2FDD}' |
        '\u{2FF0}'..='\u{2FFF}' |
        '\u{3000}'..='\u{303F}' |
        '\u{3040}'..='\u{309F}' |
        '\u{30A0}'..='\u{30FF}' |
        '\u{3130}'..='\u{318F}' |
        '\u{3190}'..='\u{319F}' |
        '\u{31C0}'..='\u{31EF}' |
        '\u{31F0}'..='\u{31FF}' |
        '\u{3300}'..='\u{33FF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{4E00}'..='\u{9FFF}' |
        '\u{A000}'..='\u{A48F}' |
        '\u{A490}'..='\u{A4CF}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{FE10}'..='\u{FE1F}' |
        '\u{FE30}'..='\u{FE4F}' |
        '\u{FE50}'..='\u{FE6F}' |
        '\u{FF00}'..='\u{FFEF}' |
        '\u{1B000}'..='\u{1B0FF}' |
        '\u{1B100}'..='\u{1B12F}' |
        '\u{1B130}'..='\u{1B16F}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2B73F}' |
        '\u{2B740}'..='\u{2B81F}' |
        '\u{2B820}'..='\u{2CEAF}' |
        '\u{2CEB0}'..='\u{2EBEF}' |
        '\u{2F800}'..='\u{2FA1F}' |
        '\u{30000}'..='\u{3134F}' => true,
        _ => false
    }
}

// https://searchfox.org/mozilla-central/rev/3d39d3b7dd1b2be30692d4541ea681614e34c786/layout/generic/nsTextFrame.cpp#1826-1827
// https://drafts.csswg.org/css-text/#boundary-shaping
fn can_continue_run(run_style: &ComputedStyle, new_style: &ComputedStyle, at_beginning: bool) -> bool {
    if run_style.writing_mode != new_style.writing_mode {
        return false;
    }

    // Any of margin/border/padding separating the two typographic character
    // units in the inline axis is non-zero.
    let margin = new_style.margin();
    let padding = new_style.padding();
    let border = new_style.border_widths();
    let (margin, padding, border) = if at_beginning {
        (margin.inline_start, padding.inline_start, border.inline_start)
    } else {
        (margin.inline_end, padding.inline_end, border.inline_end)
    };

    if !margin.is_zero_or_auto() || !padding.is_zero() || border.0 != 0 {
        return false;
    }

    // TODO: vertical-align is not baseline
    // TODO: The boundary is a bidi isolation boundary.
    // TODO: line-break / word-break / maybe white-space: nowrap-ness?
    // TODO: Definitely different fonts and such.

    true
}

impl<'a, 'b> InlineFormattingContext<'a, 'b> {
    pub fn new(context: &'a LayoutContext<'b>, input_node: &'a LayoutNode) -> Self {
        debug_assert!(input_node.establishes_ifc(context.layout_tree));
        Self {
            context,
            input_node,
            items: vec![],
        }
    }

    // TODO(emilio): Maybe merge this with whitespace processing?
    fn collect_inline_items_in(&mut self, node: &LayoutNode) {
        for (id, child) in node.children_and_id(self.context.layout_tree) {
            match child.kind {
                LayoutNodeKind::Leaf { ref kind } => {
                    match kind {
                        LeafKind::Replaced { .. } => self.items.push(InlineItem::Replaced(id)),
                        LeafKind::Text { ref text } => self.items.push(InlineItem::Text(id, text.clone().into())),
                    }
                }
                LayoutNodeKind::Container { ref kind, .. } => {
                    match kind {
                        ContainerKind::Inline { .. } => {
                            self.items.push(InlineItem::TagStart(id));
                            self.collect_inline_items_in(child);
                            self.items.push(InlineItem::TagEnd(id));
                        },
                        ContainerKind::Block { .. } => {
                            debug_assert!(
                                !child.style.display.is_block_outside(),
                                "Should've been split",
                            );
                            debug_assert!(
                                child.has_independent_layout(self.context),
                                "Should be atomic",
                            );
                            self.items.push(InlineItem::AtomicInline(id));
                        }
                    }
                }
            }
        }
    }

    fn collapse_spaces_in_string(
        text: String,
        collapses_newlines: bool,
        after_collapsible_space: &mut bool,
        after_break: &mut bool,
    ) -> String {
        let mut result = String::with_capacity(text.len());
        // The rules are relatively simple:
        //
        // 1. Any sequence of collapsible spaces and tabs immediately preceding
        //    or following a segment break is removed.
        // 2. Collapsible segment breaks are transformed for rendering according
        //    to the segment break transformation rules.
        // 3. Every collapsible tab is converted to a collapsible space (U+0020).
        // 4. Any collapsible space immediately following another collapsible
        //    space - even one outside the boundary of the inline containing
        //    that space, provided both spaces are within the same inline
        //    formatting context - is collapsed to have zero advance width. (It
        //    is invisible, but retains its soft wrap opportunity, if any.)
        //
        // In order to do this in one pass, as some of the rules are dependent
        // with each other (see rule 1. and rule 2, which depends on spaces
        // around segment breaks getting transformed), we only put on the string
        // the characters that _definitely_ end up in it, and flush the
        // characters as needed.

        // Whether we need to insert a collapsible space if we se a
        // non-collapsible character.
        let mut last_non_collapsible_char = None;
        let after_collapsible_space_at_start = *after_collapsible_space;
        const ZWSP: char = '\u{200B}';
        for c in text.chars() {
            match c {
                '\t' | ' ' => {
                    if !*after_break {
                        *after_collapsible_space = true;
                    }
                },
                '\n' => {
                    // We found a break, thus previous collapsible space
                    // characters just go away, rule 4 doesn't (necessarily)
                    // apply.
                    if !collapses_newlines {
                        result.push('\n');
                    }
                    *after_break = true;
                    *after_collapsible_space = false;
                },
                _ => {
                    // https://drafts.csswg.org/css-text-3/#line-break-transform
                    if *after_break {
                        debug_assert!(
                            !*after_collapsible_space,
                            "Collapsible space after a segment break is removed per rule 1",
                        );

                        // Note that the `!collapses_newlines` case we've
                        // already handled when finding it.
                        let suppress = !collapses_newlines || c == ZWSP || last_non_collapsible_char.take().map_or(false, |last| {
                            if last == ZWSP {
                                return true;
                            }
                            is_space_discarding(last) && is_space_discarding(c)
                        });

                        if !suppress {
                            *after_collapsible_space = true;
                        }
                    }

                    if *after_collapsible_space {
                        if after_collapsible_space_at_start {
                            result.push(ZWSP);
                        } else {
                            result.push(' ');
                        }
                    }
                    result.push(c);
                    *after_break = false;
                    *after_collapsible_space = false;
                    last_non_collapsible_char = Some(c);
                }
            }
        }
        result
    }


    // https://drafts.csswg.org/css-text-3/#white-space-phase-1
    fn collapse_spaces(&mut self) {
        let mut after_break = true;
        let mut after_collapsible_space = false;
        for item in &mut self.items {
            let (node, text) = match *item {
                InlineItem::Replaced(..) |
                InlineItem::AtomicInline(..) |
                InlineItem::TagEnd(..) |
                InlineItem::TagStart(..) => continue,
                InlineItem::Text(node, ref mut s) => (node, s),
            };

            let style = &self.context.layout_tree[node].style;
            if style.white_space.collapses_spaces() {
                let new_text = Self::collapse_spaces_in_string(
                    std::mem::replace(text, String::new()),
                    style.white_space.collapses_newlines(),
                    &mut after_collapsible_space,
                    &mut after_break,
                );
                *text = new_text;
            } else {
                debug_assert!(!style.white_space.collapses_newlines());
                // We don't have to collapse spaces here but we still need to
                // know if we're after a segment break.
                for &b in text.as_bytes().iter().rev() {
                    match b {
                        b'\t' | b' ' => {},
                        _ => {
                            after_break = b == b'\n';
                            break;
                        }
                    }
                }
            }
        }
    }

    fn split_bidi(&mut self) {
        // TODO: Keep track of bidi levels within the text items.
        // https://drafts.csswg.org/css-writing-modes-3/#bidi-algo
        // https://drafts.csswg.org/css-writing-modes-3/#unicode-bidi
    }

    fn do_layout(&mut self, constraints: &ConstraintSpace) -> LayoutResult {
        LineBreaker::new(self, constraints).break_and_finish()
    }
}

impl<'a, 'b> super::LayoutAlgorithm for InlineFormattingContext<'a, 'b> {
    fn layout(&mut self, constraints: &ConstraintSpace) -> LayoutResult {
        debug_assert!(self.input_node.establishes_ifc(self.context.layout_tree));

        self.collect_inline_items_in(self.input_node);
        self.collapse_spaces();
        self.split_bidi();
        self.do_layout(constraints)
    }
}
