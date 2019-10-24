use std::alloc::Layout;
use std::io::{Error, ErrorKind};

#[inline]
pub fn extend(layout: Layout, next: Layout) -> (Layout, usize) {
    let new_align = std::cmp::max(layout.align(), next.align());
    let pad = padding_needed_for(layout, next.align());

    let offset = layout
        .size()
        .checked_add(pad)
        .ok_or(Error::new(
            ErrorKind::Other,
            "Padding overflow check failed",
        ))
        .unwrap();
    let new_size = offset
        .checked_add(next.size())
        .ok_or(Error::new(ErrorKind::Other, "New size can't be computed"))
        .unwrap();

    let layout = Layout::from_size_align(new_size, new_align).unwrap();
    (layout, offset)
}

#[inline]
pub(crate) fn padding_needed_for(layout: Layout, align: usize) -> usize {
    let len = layout.size();
    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    len_rounded_up.wrapping_sub(len)
}
