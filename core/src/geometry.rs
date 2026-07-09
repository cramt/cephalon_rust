/// screen-space rect of the game window, global/virtual-desktop coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Screen-space region of one reward card's name text, in the same pixel space
/// as the value passed for `screen_width`/`screen_height`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardRegion {
    /// left edge of the card
    pub x: u32,
    /// card width
    pub width: u32,
    /// bottom edge of the item-name text block
    pub text_bottom: u32,
    /// height of one line of item-name text
    pub line_height: u32,
}

/// Positions of the reward cards on the relic reward screen, scaled from the
/// 1920x1080 reference layout. `count` is the squad size (1-4); anything else
/// yields no regions. Index i is the parser's OCR slot i.
pub fn reward_card_regions(screen_width: u32, screen_height: u32, count: usize) -> Vec<CardRegion> {
    let middle = screen_width / 2;
    let frame_width = (screen_width * 243) / 1920;
    let frame_bottom = (screen_height * 460) / 1080;
    let text_height = (screen_height * 24) / 1080;
    let start_points = match count {
        4 => vec![
            middle - frame_width * 2,
            middle - frame_width,
            middle,
            middle + frame_width,
        ],
        3 => vec![
            middle - ((3 * frame_width) / 2),
            middle + (frame_width / 2),
            middle - (frame_width / 2),
        ],
        2 => vec![middle - frame_width, middle],
        1 => vec![middle - (frame_width / 2)],
        _ => Vec::new(),
    };
    start_points
        .into_iter()
        .map(|x| CardRegion {
            x,
            width: frame_width,
            text_bottom: frame_bottom,
            line_height: text_height,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_cards_1080p() {
        let r = reward_card_regions(1920, 1080, 4);
        assert_eq!(r.iter().map(|c| c.x).collect::<Vec<_>>(), vec![474, 717, 960, 1203]);
        assert!(r.iter().all(|c| c.width == 243));
        assert!(r.iter().all(|c| c.text_bottom == 460));
        assert!(r.iter().all(|c| c.line_height == 24));
    }

    #[test]
    fn four_cards_1440p_scales() {
        let r = reward_card_regions(2560, 1440, 4);
        assert_eq!(r.iter().map(|c| c.x).collect::<Vec<_>>(), vec![632, 956, 1280, 1604]);
        assert!(r.iter().all(|c| c.width == 324));
        assert!(r.iter().all(|c| c.text_bottom == 613));
        assert!(r.iter().all(|c| c.line_height == 32));
    }

    // NOTE: the 3-card order is intentionally non-monotonic — it mirrors the slot order
    // the original parser used (left, right, middle). Preserved verbatim; both parser
    // crops and overlay labels use the same slot->position mapping so they stay consistent.
    #[test]
    fn three_cards_preserves_original_slot_order() {
        let r = reward_card_regions(1920, 1080, 3);
        assert_eq!(r.iter().map(|c| c.x).collect::<Vec<_>>(), vec![596, 1081, 839]);
    }

    #[test]
    fn two_and_one_cards() {
        let two = reward_card_regions(1920, 1080, 2);
        assert_eq!(two.iter().map(|c| c.x).collect::<Vec<_>>(), vec![717, 960]);
        let one = reward_card_regions(1920, 1080, 1);
        assert_eq!(one.iter().map(|c| c.x).collect::<Vec<_>>(), vec![839]);
        assert!(reward_card_regions(1920, 1080, 5).is_empty());
        assert!(reward_card_regions(1920, 1080, 0).is_empty());
    }
}
