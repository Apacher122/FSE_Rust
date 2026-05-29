//! Traversal stack implementation.

const INLINE_TRAVERSAL_STACK_CAPACITY: usize = 16;

/// Internal traversal stack frame.
///
/// # Runtime Role
///
/// A frame carries the node id and whether the node is inside a subtree already
/// proven to be fully covered by the query. Covered descendants do not need
/// another bounds classification pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TraversalFrame {
    pub(super) node_id: usize,
    pub(super) inherited_covered: bool,
}

impl TraversalFrame {
    #[inline]
    pub(super) fn normal(node_id: usize) -> Self {
        Self {
            node_id,
            inherited_covered: false,
        }
    }

    #[inline]
    pub(super) fn covered(node_id: usize) -> Self {
        Self {
            node_id,
            inherited_covered: true,
        }
    }
}

/// Small LIFO stack used by hierarchy traversal.
///
/// # Runtime Role
///
/// Selective range queries usually keep only a few child frames active at once.
/// `TraversalStack` stores those common frames inline and only allocates an
/// overflow vector if a wider or deeper future hierarchy exceeds the inline
/// capacity.
pub(super) struct TraversalStack {
    inline_frames: [Option<TraversalFrame>; INLINE_TRAVERSAL_STACK_CAPACITY],
    inline_len: usize,
    overflow_frames: Vec<TraversalFrame>,
}

impl TraversalStack {
    pub(super) fn new() -> Self {
        Self {
            inline_frames: [None; INLINE_TRAVERSAL_STACK_CAPACITY],
            inline_len: 0,
            overflow_frames: Vec::new(),
        }
    }

    #[inline]
    pub(super) fn push(&mut self, frame: TraversalFrame) {
        if self.overflow_frames.is_empty() && self.inline_len < INLINE_TRAVERSAL_STACK_CAPACITY {
            self.inline_frames[self.inline_len] = Some(frame);
            self.inline_len += 1;
            return;
        }

        // rare path for deeper traversals, keep the common path allocation free
        self.overflow_frames.push(frame);
    }

    #[inline]
    pub(super) fn pop(&mut self) -> Option<TraversalFrame> {
        if let Some(frame) = self.overflow_frames.pop() {
            return Some(frame);
        }

        if self.inline_len == 0 {
            return None;
        }

        self.inline_len -= 1;
        self.inline_frames[self.inline_len].take()
    }
}

/// Pushes child node frames so traversal preserves left-to-right pop order.
///
/// # Runtime Role
///
/// Hierarchy traversal uses a LIFO stack. Children are pushed in reverse order
/// so the leftmost child is visited first when frames are popped. The 1-child
/// and 2-child branches keep the current binary hierarchy hot path direct,
/// while the generic fallback preserves correctness for wider future fanout.
#[inline]
pub(super) fn push_child_frames(
    children: &[usize],
    inherited_covered: bool,
    stack: &mut TraversalStack,
) {
    match children.len() {
        0 => {}
        1 => {
            stack.push(child_frame(children[0], inherited_covered));
        }
        2 => {
            // preserve left to right pop order without the iterator path
            stack.push(child_frame(children[1], inherited_covered));
            stack.push(child_frame(children[0], inherited_covered));
        }
        _ => {
            // keep the generic fallback in case future splitters use wider fanout
            for child in children.iter().rev() {
                stack.push(child_frame(*child, inherited_covered));
            }
        }
    }
}

#[inline]
fn child_frame(node_id: usize, inherited_covered: bool) -> TraversalFrame {
    if inherited_covered {
        TraversalFrame::covered(node_id)
    } else {
        TraversalFrame::normal(node_id)
    }
}
