//! Map location markers (Haxe LOCATION_SAYS / mother markers).

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MapMarker {
    pub x: i32,
    pub y: i32,
    pub label: String,
    pub kind: MarkerKind,
    pub owner_p_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Mother,
    Leader,
    Home,
    Custom,
}

impl MarkerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MarkerKind::Mother => "MOTHER",
            MarkerKind::Leader => "leader",
            MarkerKind::Home => "HOME",
            MarkerKind::Custom => "MARK",
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct MarkerState {
    pub markers: HashMap<i32, Vec<MapMarker>>, // viewer p_id → markers they should see
}

impl MarkerState {
    pub fn set_mother_marker(&mut self, child_p_id: i32, mother_x: i32, mother_y: i32, mother_p_id: i32) {
        let m = MapMarker {
            x: mother_x,
            y: mother_y,
            label: "MOTHER".into(),
            kind: MarkerKind::Mother,
            owner_p_id: mother_p_id,
        };
        self.markers.entry(child_p_id).or_default().push(m);
    }

    /// Custom map pin at `(x, y)` visible to `viewer_p_id` (typically self).
    /// Label is the user text after `SAY MARK …`.
    pub fn add_custom_marker(
        &mut self,
        viewer_p_id: i32,
        x: i32,
        y: i32,
        label: impl Into<String>,
        owner_p_id: i32,
    ) {
        let m = MapMarker {
            x,
            y,
            label: label.into(),
            kind: MarkerKind::Custom,
            owner_p_id,
        };
        self.markers.entry(viewer_p_id).or_default().push(m);
    }

    /// Haxe LOCATION_SAYS style: `x y ! text`
    pub fn wire_lines_for(&self, p_id: i32) -> Vec<String> {
        self.markers
            .get(&p_id)
            .map(|list| {
                list.iter()
                    .map(|m| format!("{} {} ! {}", m.x, m.y, m.label))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mother_marker_wire() {
        let mut s = MarkerState::default();
        s.set_mother_marker(2, 10, 20, 1);
        let lines = s.wire_lines_for(2);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("10 20"));
        assert!(lines[0].contains("MOTHER"));
    }

    #[test]
    fn custom_marker_for_self_wire() {
        let mut s = MarkerState::default();
        s.add_custom_marker(7, 3, 9, "camp", 7);
        let lines = s.wire_lines_for(7);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "3 9 ! camp");
        // Not visible to other viewers unless stored for them.
        assert!(s.wire_lines_for(1).is_empty());
        let list = s.markers.get(&7).unwrap();
        assert_eq!(list[0].kind, MarkerKind::Custom);
        assert_eq!(list[0].owner_p_id, 7);
    }
}
