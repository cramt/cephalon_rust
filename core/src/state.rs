use crate::items::items::Item;

pub struct State {
    pub relic_rewards: Vec<Option<(Item, u32)>>,
}
