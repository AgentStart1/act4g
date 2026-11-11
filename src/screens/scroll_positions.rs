use crate::api::ItemFilter;
use gpui::ScrollHandle;
use std::collections::HashMap;

#[derive(Default)]
pub struct ScrollPositions {
    pub inbox: ScrollHandle,
    pub repositories: ScrollHandle,
    items: HashMap<String, [ScrollHandle; 2]>,
}
impl ScrollPositions {
    pub fn items(&mut self, repository: &str, filter: ItemFilter) -> ScrollHandle {
        let handles = self.items.entry(repository.to_string()).or_default();
        handles[match filter {
            ItemFilter::Issues => 0,
            ItemFilter::PullRequests => 1,
        }]
        .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px};
    #[test]
    fn returning_to_a_repository_retains_its_own_filter_position() {
        let mut positions = ScrollPositions::default();
        let handle = positions.items("sample/one", ItemFilter::Issues);
        handle.set_offset(point(px(0.), px(-240.)));
        drop(handle);
        assert_eq!(
            positions.items("sample/one", ItemFilter::Issues).offset(),
            point(px(0.), px(-240.))
        );
        assert_eq!(
            positions
                .items("sample/one", ItemFilter::PullRequests)
                .offset(),
            point(px(0.), px(0.))
        );
        assert_eq!(
            positions.items("sample/two", ItemFilter::Issues).offset(),
            point(px(0.), px(0.))
        );
    }
}
