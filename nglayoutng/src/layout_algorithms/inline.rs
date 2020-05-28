use app_units::Au;
use crate::fragment_tree::{ChildFragment, Fragment, FragmentKind, ContainerFragmentKind};
use super::{ConstraintSpace, LayoutContext, LayoutResult};
use crate::layout_tree::{LayoutNodeKind, LeafKind, ContainerKind, LayoutNode, LayoutNodeId};
use smallvec::SmallVec;

pub struct InlineFormattingContext<'a, 'b> {
    context: &'a LayoutContext<'b>,
    input_node: &'a LayoutNode,
    items: Vec<InlineItem>,
}

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
}

impl<'a, 'b> super::LayoutAlgorithm for InlineFormattingContext<'a, 'b> {
    fn layout(&mut self, constraints: &ConstraintSpace) -> LayoutResult {
        debug_assert!(!self.input_node.establishes_ifc(self.context.layout_tree));

        self.collect_inline_items_in(self.input_node);
        self.collapse_spaces();
        self.split_bidi();

        unimplemented!();
    }
}
