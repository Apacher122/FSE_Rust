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
