use app_units::Au;
use crate::fragment_tree::{ChildFragment, Fragment, FragmentKind, ContainerFragmentKind};
use crate::logical_geometry::*;
use crate::style::ComputedStyle;
use super::{ConstraintSpace, LayoutContext, LayoutResult};
use crate::layout_tree::{LayoutNodeKind, LeafKind, ContainerKind, LayoutNode, LayoutNodeId};
use smallvec::SmallVec;
use smallbitvec::SmallBitVec;
use std::borrow::Cow;

/// A position for a given line break.
#[derive(Clone)]
struct InlineItemPosition {
    /// The index of the item, may be one past the end to represent "the whole
    /// range".
    item_index: usize,
    /// The byte index of the line break inside the given text string.
    text_start: usize,
}

impl InlineItemPosition {
    fn start() -> Self {
        Self {
            item_index: 0,
            text_start: 0,
        }
    }

    fn advance_item(&mut self) {
        self.item_index += 1;
        self.text_start = 0;
    }
}

struct LineBreaker<'a, 'b, 'c> {
    fc: &'a InlineFormattingContext<'b, 'c>,
    constraints: &'a ConstraintSpace,
    style_stack: Vec<&'a ComputedStyle>,
    lines: Vec<ChildFragment>,
    current_line: Vec<ChildFragment>,
    current_line_available_size: Au,
    current_line_max_block_size: Au,
    current_position: InlineItemPosition,
    /// The last item we laid out, which didn't fit.
    last_laid_out_item: Option<ChildFragment>,
}

impl<'a, 'b, 'c> LineBreaker<'a, 'b, 'c> {
    fn new(fc: &'a mut InlineFormattingContext<'b, 'c>, constraints: &'a ConstraintSpace) -> Self {
        Self {
            fc,
            constraints,
            style_stack: vec![],
            lines: vec![],
            current_line: vec![],
            current_line_available_size: constraints.available_size.inline(),
            current_line_max_block_size: Au(0),
            current_position: InlineItemPosition::start(),
            last_laid_out_item: None,
        }
    }

    fn wm(&self) -> WritingMode {
        self.fc.input_node.style.writing_mode
    }

    fn flush_line(&mut self) {
        if self.current_line.is_empty() {
            return;
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
        let size = LogicalSize::new(wm, self.constraints.available_size.inline(), self.current_line_max_block_size);

        // TODO: Vertical alignment of items. Here or when we're done with all
        // lines?

        self.lines.push(ChildFragment {
            offset: LogicalPoint::zero(wm),
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
    }

    fn layout_atomic_inline(&mut self, node: LayoutNodeId) -> ChildFragment {
        unimplemented!()
    }

    fn layout_replaced(&mut self, node: LayoutNodeId) -> ChildFragment {
        unimplemented!()
    }

    fn next_item_fragment(&mut self) -> Option<ChildFragment> {
        loop {
            if let Some(f) = self.last_laid_out_item.take() {
                return Some(f);
            }
            let item = self.fc.items.get(self.current_position.item_index)?;
            match *item {
                InlineItem::TagStart(node) => {
                    self.style_stack.push(&self.fc.context.layout_tree[node].style);
                    self.current_position.advance_item();
                    continue;
                },
                InlineItem::TagEnd(node) => {
                    self.style_stack.pop();
                    self.current_position.advance_item();
                    continue;
                },
                InlineItem::AtomicInline(node) => {
                    let fragment = self.layout_atomic_inline(node);
                    self.current_position.advance_item();
                    return Some(fragment);
                },
                InlineItem::Replaced(node) => {
                    let fragment = self.layout_replaced(node);
                    self.current_position.advance_item();
                    return Some(fragment);
                },
                InlineItem::Text(ref text) => {
                    let mut paragraph = Cow::Borrowed(&text[self.current_position.text_start..]);

                    let style = self.style_stack.last().unwrap();
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
                    loop {
                        let item = match self.fc.items.get(self.current_position.item_index + advance) {
                            Some(item) => item,
                            None => break,
                        };

                        match *item {
                            InlineItem::TagStart(node) => {
                                if !can_continue_run(style, &self.fc.context.layout_tree[node].style, /* at_beginning = */ true) {
                                    break;
                                }
                                // TODO(emilio): We probably need to record some
                                // state here to create the right fragment tree.
                            },
                            InlineItem::TagEnd(node) => {
                                if !can_continue_run(style, &self.fc.context.layout_tree[node].style, /* at_beginning = */ false) {
                                    break;
                                }
                                // TODO(emilio): We probably need to record some
                                // state here to create the right fragment tree.
                            },
                            InlineItem::Text(ref s) => {
                                paragraph.to_mut().push_str(&s);
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
                    let mut break_opportunities = SmallBitVec::new();
                    break_opportunities.resize(paragraph.len(), false);

                    // Try to grab a whole text run and line-break / shape it.
                    let mut breaker = xi_unicode::LineBreakLeafIter::new("", 0);

                    // TODO(emilio): There are optimizations here we could do to
                    // avoid doing this, or do a simplified version of this,
                    // when different white-space values are in-use like nowrap
                    // and so on...
                    loop {
                        let (result, _hard_break) = breaker.next(&*paragraph);
                        if result == paragraph.len() {
                            break;
                        }
                        break_opportunities.set(result, true);
                        // XXX Do we need to use the hard_break bit somehow?
                        // Maybe just truncating the paragraph and carrying on?
                    }

                    let shaped_run = super::shaping::shape(&paragraph, style);

                    // TODO:
                    // break_and_shape_text(text, style);
                    // advance_as_needed()
                    // return Some(break);
                    unimplemented!()
                }
            }
        }
    }

    fn break_next(&mut self) -> bool {
        loop {
            // Take our next item to fit.
            let next_fragment = match self.next_item_fragment() {
                Some(fragment) => fragment,
                None => {
                    // No more fragments, we're all done.
                    self.flush_line();
                    return false;
                },
            };

            // If we're in an empty line, everything fits.
            if self.current_line.is_empty() {
                self.current_line.push(next_fragment);
                continue;
            }
        }

        true
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
enum InlineItem {
    // The start of a nested inline box.
    TagStart(LayoutNodeId),
    // TODO(emilio): Probably want to reference-count this somehow, or something
    // something.
    Text(String),
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

    if !margin.is_zero() || !padding.is_zero() || border.0 != 0 {
        return false;
    }

    // TODO: vertical-align is not baseline
    // TODO: The boundary is a bidi isolation boundary.
    //
    // TODO: line-break / word-break / maybe white-space: nowrap-ness?

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
                        LeafKind::Text { ref text } => self.items.push(InlineItem::Text(text.clone().into())),
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
        let mut style_stack = SmallVec::<[&_; 5]>::new();
        style_stack.push(&self.input_node.style);
        let mut after_break = true;
        let mut after_collapsible_space = false;
        for item in &mut self.items {
            let text = match *item {
                InlineItem::Replaced(..) |
                InlineItem::AtomicInline(..) => continue,
                InlineItem::TagStart(node) => {
                    style_stack.push(&self.context.layout_tree[node].style);
                    continue;
                },
                InlineItem::TagEnd(..) => {
                    style_stack.pop();
                    continue;
                },
                InlineItem::Text(ref mut s) => s,
            };

            let style = style_stack.last().unwrap();
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
        let mut breaker = LineBreaker::new(self, constraints);
        while breaker.break_next() {
            // TODO(emilio): At some point we'll have to signal a forced stop
            // (as in, ran out of space in the fragmentainer), and propagate the
            // necessary data back up, saving up as much stuff as necessary.
        }
        breaker.finish()
    }
}

impl<'a, 'b> super::LayoutAlgorithm for InlineFormattingContext<'a, 'b> {
    fn layout(&mut self, constraints: &ConstraintSpace) -> LayoutResult {
        debug_assert!(!self.input_node.establishes_ifc(self.context.layout_tree));

        self.collect_inline_items_in(self.input_node);
        self.collapse_spaces();
        self.split_bidi();
        self.do_layout(constraints)
    }
}
