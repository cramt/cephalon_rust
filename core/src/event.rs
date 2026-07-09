use crate::items::items::Item;

/// One reward card slot, indexed to match `geometry::reward_card_regions`.
#[derive(Debug, Clone, PartialEq)]
pub enum RewardSlot {
    /// OCR hasn't identified this card yet
    Pending,
    /// forma blueprint — has no market price
    Forma,
    /// identified item; price is None if the market lookup failed
    Item { item: Item, price: Option<u32> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    RewardScreenOpened { count: usize },
    RewardsResolved(Vec<RewardSlot>),
    RewardScreenClosed,
}
